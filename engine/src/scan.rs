//! Cheat Engine-style value scanner: find where a number lives in memory by
//! scanning, changing it in game, and scanning again until few addresses are
//! left.

use crate::error::{EngineError, Result};
use crate::memory::{Memory, Region};
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Large regions are read in pieces so one bad page doesn't lose the region.
const CHUNK: usize = 4 << 20;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ScanFilter {
    /// The value is exactly this (floats match after rounding to the
    /// precision typed, so "100" finds 100.3).
    Exact(String),
    /// First scan only: remember everything and narrow down later.
    Unknown,
    Changed,
    Unchanged,
    Increased,
    Decreased,
}

/// Shared with the UI thread for progress bars and the cancel button.
#[derive(Default)]
pub struct ScanControl {
    cancel: AtomicBool,
    progress: AtomicU32,
}

impl ScanControl {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// 0..=1000
    pub fn progress(&self) -> u32 {
        self.progress.load(Ordering::Relaxed)
    }

    pub(crate) fn reset(&self) {
        self.cancel.store(false, Ordering::Relaxed);
        self.progress.store(0, Ordering::Relaxed);
    }

    pub(crate) fn check(&self, done: u64, total: u64) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(EngineError::Cancelled);
        }
        if let Some(p) = (done * 1000).checked_div(total) {
            self.progress.store(p as u32, Ordering::Relaxed);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanHit {
    pub address: u64,
    pub value: Value,
    /// "game.exe+0x1234" when the address is inside a module, so it's static
    /// and can be saved without a pointer chain.
    pub module_offset: Option<(String, u64)>,
}

enum State {
    Empty,
    /// "Unknown initial value": a full copy of writable memory.
    Snapshot(Vec<(u64, Vec<u8>)>),
    /// Matching addresses (sorted) and the value each had at the last scan.
    Hits { addrs: Vec<u64>, prev: Vec<u8> },
}

pub struct Scanner {
    vtype: ValueType,
    state: State,
}

struct Matcher {
    filter: ScanFilter,
    vtype: ValueType,
    exact: Option<(Value, f64)>,
}

impl Matcher {
    fn new(filter: ScanFilter, vtype: ValueType) -> Result<Self> {
        let exact = match &filter {
            ScanFilter::Exact(s) => {
                let v = vtype.parse(s)?;
                let decimals = s.trim().split_once('.').map_or(0, |(_, d)| d.len()) as i32;
                Some((v, 0.5 * 10f64.powi(-decimals)))
            }
            _ => None,
        };
        Ok(Matcher { filter, vtype, exact })
    }

    fn is_match(&self, old: Option<&[u8]>, new: &[u8]) -> bool {
        let t = self.vtype;
        let cur = t.decode(new);
        if let Value::Float(f) = cur {
            if !f.is_finite() {
                return false;
            }
        }
        let prev = || t.decode(old.expect("relative scans need a previous value"));
        match (&self.filter, cur) {
            (ScanFilter::Exact(_), Value::Int(i)) => Some(Value::Int(i)) == self.exact.map(|e| e.0),
            (ScanFilter::Exact(_), Value::Float(f)) => {
                let (v, tol) = self.exact.unwrap();
                (f - v.as_f64()).abs() < tol
            }
            (ScanFilter::Unknown, _) => true,
            (ScanFilter::Changed, _) => old.unwrap() != new,
            (ScanFilter::Unchanged, _) => old.unwrap() == new,
            (ScanFilter::Increased, c) => c.as_f64() > prev().as_f64(),
            (ScanFilter::Decreased, c) => c.as_f64() < prev().as_f64(),
        }
    }
}

fn scan_regions(mem: &dyn Memory) -> Result<Vec<Region>> {
    Ok(mem.regions()?.into_iter().filter(|r| r.writable).collect())
}

impl Scanner {
    pub fn new(vtype: ValueType) -> Self {
        Scanner { vtype, state: State::Empty }
    }

    pub fn value_type(&self) -> ValueType {
        self.vtype
    }

    fn align(&self) -> usize {
        self.vtype.size().min(4)
    }

    pub fn count(&self) -> usize {
        match &self.state {
            State::Empty => 0,
            State::Snapshot(chunks) => chunks.iter().map(|(_, d)| d.len() / self.align()).sum(),
            State::Hits { addrs, .. } => addrs.len(),
        }
    }

    pub fn has_results(&self) -> bool {
        !matches!(self.state, State::Empty)
    }

    pub fn first_scan(&mut self, mem: &dyn Memory, filter: ScanFilter, ctl: &ScanControl) -> Result<usize> {
        ctl.reset();
        let m = Matcher::new(filter.clone(), self.vtype)?;
        if matches!(filter, ScanFilter::Changed | ScanFilter::Unchanged | ScanFilter::Increased | ScanFilter::Decreased) {
            return Err(EngineError::Invalid("the first scan must be an exact value or unknown initial value".into()));
        }
        let regions = scan_regions(mem)?;
        let total: u64 = regions.iter().map(|r| r.size).sum();
        let (size, align) = (self.vtype.size(), self.align());
        let mut done = 0u64;
        let mut snapshot = Vec::new();
        let (mut addrs, mut prev) = (Vec::new(), Vec::new());

        for r in &regions {
            let mut off = 0u64;
            while off < r.size {
                ctl.check(done, total)?;
                let len = (r.size - off).min(CHUNK as u64) as usize;
                let base = r.base + off;
                off += len as u64;
                done += len as u64;
                let mut buf = vec![0u8; len];
                if mem.read(base, &mut buf).is_err() {
                    continue;
                }
                if matches!(filter, ScanFilter::Unknown) {
                    snapshot.push((base, buf));
                    continue;
                }
                let mut i = 0;
                while i + size <= len {
                    if m.is_match(None, &buf[i..i + size]) {
                        addrs.push(base + i as u64);
                        prev.extend_from_slice(&buf[i..i + size]);
                    }
                    i += align;
                }
            }
        }
        self.state = if matches!(filter, ScanFilter::Unknown) {
            State::Snapshot(snapshot)
        } else {
            State::Hits { addrs, prev }
        };
        ctl.progress.store(1000, Ordering::Relaxed);
        Ok(self.count())
    }

    pub fn next_scan(&mut self, mem: &dyn Memory, filter: ScanFilter, ctl: &ScanControl) -> Result<usize> {
        ctl.reset();
        if matches!(filter, ScanFilter::Unknown) {
            return Err(EngineError::Invalid("'unknown initial value' is only for the first scan".into()));
        }
        let m = Matcher::new(filter, self.vtype)?;
        let (size, align) = (self.vtype.size(), self.align());
        let (mut addrs, mut prev) = (Vec::new(), Vec::new());

        match std::mem::replace(&mut self.state, State::Empty) {
            State::Empty => return Err(EngineError::Invalid("run a first scan before a next scan".into())),
            State::Snapshot(chunks) => {
                let total: u64 = chunks.iter().map(|(_, d)| d.len() as u64).sum();
                let mut done = 0u64;
                for (base, old) in chunks {
                    ctl.check(done, total)?;
                    done += old.len() as u64;
                    let mut cur = vec![0u8; old.len()];
                    if mem.read(base, &mut cur).is_err() {
                        continue;
                    }
                    let mut i = 0;
                    while i + size <= cur.len() {
                        if m.is_match(Some(&old[i..i + size]), &cur[i..i + size]) {
                            addrs.push(base + i as u64);
                            prev.extend_from_slice(&cur[i..i + size]);
                        }
                        i += align;
                    }
                }
            }
            State::Hits { addrs: old_addrs, prev: old_prev } => {
                let total = old_addrs.len() as u64;
                let mut idx = 0;
                // Read nearby addresses together instead of one call per hit.
                while idx < old_addrs.len() {
                    ctl.check(idx as u64, total)?;
                    let start = old_addrs[idx];
                    let mut end_idx = idx;
                    while end_idx + 1 < old_addrs.len() && old_addrs[end_idx + 1] + size as u64 - start <= 64 * 1024 {
                        end_idx += 1;
                    }
                    let span = (old_addrs[end_idx] + size as u64 - start) as usize;
                    let mut buf = vec![0u8; span];
                    let window_ok = mem.read(start, &mut buf).is_ok();
                    for j in idx..=end_idx {
                        let a = old_addrs[j];
                        let mut single = [0u8; 8];
                        let cur: &[u8] = if window_ok {
                            let o = (a - start) as usize;
                            &buf[o..o + size]
                        } else if mem.read(a, &mut single[..size]).is_ok() {
                            &single[..size]
                        } else {
                            continue;
                        };
                        let old = &old_prev[j * size..(j + 1) * size];
                        if m.is_match(Some(old), cur) {
                            addrs.push(a);
                            prev.extend_from_slice(cur);
                        }
                    }
                    idx = end_idx + 1;
                }
            }
        }
        self.state = State::Hits { addrs, prev };
        ctl.progress.store(1000, Ordering::Relaxed);
        Ok(self.count())
    }

    /// The first `limit` hits with their current values.
    pub fn hits(&self, mem: &dyn Memory, limit: usize) -> Vec<ScanHit> {
        let State::Hits { addrs, .. } = &self.state else {
            return vec![];
        };
        let modules = mem.modules().unwrap_or_default();
        let size = self.vtype.size();
        addrs
            .iter()
            .take(limit)
            .filter_map(|&a| {
                let mut b = [0u8; 8];
                mem.read(a, &mut b[..size]).ok()?;
                Some(ScanHit {
                    address: a,
                    value: self.vtype.decode(&b[..size]),
                    module_offset: modules.iter().find(|m| m.contains(a)).map(|m| (m.name.clone(), a - m.base)),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeMemory;

    fn setup() -> FakeMemory {
        let mut m = FakeMemory::new(8);
        m.map(0x1000, 0x2000);
        m.put(0x1100, &100i32.to_le_bytes());
        m.put(0x1200, &100i32.to_le_bytes());
        m.put(0x2004, &100i32.to_le_bytes());
        m
    }

    #[test]
    fn exact_then_narrow() {
        let m = setup();
        let ctl = ScanControl::default();
        let mut s = Scanner::new(ValueType::I32);
        assert_eq!(s.first_scan(&m, ScanFilter::Exact("100".into()), &ctl).unwrap(), 3);
        m.put(0x1200, &93i32.to_le_bytes());
        assert_eq!(s.next_scan(&m, ScanFilter::Decreased, &ctl).unwrap(), 1);
        assert_eq!(s.hits(&m, 10)[0].address, 0x1200);
        assert_eq!(s.next_scan(&m, ScanFilter::Exact("93".into()), &ctl).unwrap(), 1);
    }

    #[test]
    fn unknown_initial_value() {
        let m = setup();
        let ctl = ScanControl::default();
        let mut s = Scanner::new(ValueType::I32);
        s.first_scan(&m, ScanFilter::Unknown, &ctl).unwrap();
        m.put(0x2004, &150i32.to_le_bytes());
        assert_eq!(s.next_scan(&m, ScanFilter::Increased, &ctl).unwrap(), 1);
        assert_eq!(s.next_scan(&m, ScanFilter::Unchanged, &ctl).unwrap(), 1);
        assert_eq!(s.hits(&m, 10)[0].value, Value::Int(150));
    }

    #[test]
    fn float_rounding() {
        let mut m = FakeMemory::new(8);
        m.map(0x1000, 0x100);
        m.put(0x1010, &100.3f32.to_le_bytes());
        let ctl = ScanControl::default();
        let mut s = Scanner::new(ValueType::F32);
        assert_eq!(s.first_scan(&m, ScanFilter::Exact("100".into()), &ctl).unwrap(), 1);
        let mut s = Scanner::new(ValueType::F32);
        assert_eq!(s.first_scan(&m, ScanFilter::Exact("100.0".into()), &ctl).unwrap(), 0);
    }
}
