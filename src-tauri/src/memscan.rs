//! State behind the Scanner tab: one process, one scan in progress, a watch
//! list of frozen addresses, and the pointer finder.

use crate::error::{AppError, AppResult};
use addition_engine::freezer::Freezer;
use addition_engine::ptrscan::{self, PointerScanOptions};
use addition_engine::scan::{ScanControl, ScanFilter, ScanHit, Scanner};
use addition_engine::{check_anticheat, open_process, Memory, MemoryExt, PointerPath, Value, ValueType};
use serde::Serialize;
use std::sync::{Arc, Mutex};

struct Attached {
    mem: Arc<dyn Memory>,
    freezer: Freezer,
    scanner: Option<Scanner>,
}

#[derive(Default)]
pub struct MemScan {
    attached: Mutex<Option<Attached>>,
    /// Outside the mutex so progress/cancel work while a scan holds it.
    control: Arc<ScanControl>,
}

#[derive(Serialize)]
pub struct ScanSummary {
    pub count: usize,
    pub hits: Vec<ScanHit>,
}

#[derive(Serialize)]
pub struct PathView {
    pub path: PointerPath,
    pub display: String,
}

const SHOWN_HITS: usize = 500;

impl MemScan {
    pub fn open(&self, pid: u32) -> AppResult<()> {
        let mem = open_process(pid)?;
        check_anticheat(Some(&*mem))?;
        let freezer = Freezer::start(mem.clone());
        *self.attached.lock().unwrap() = Some(Attached { mem, freezer, scanner: None });
        Ok(())
    }

    pub fn close(&self) {
        *self.attached.lock().unwrap() = None;
    }

    fn mem(&self) -> AppResult<Arc<dyn Memory>> {
        let g = self.attached.lock().unwrap();
        let a = g.as_ref().ok_or_else(|| AppError::Other("pick a process first".into()))?;
        if !a.mem.is_alive() {
            return Err(AppError::Other("the process has exited".into()));
        }
        Ok(a.mem.clone())
    }

    pub fn scan(&self, first: bool, vtype: ValueType, filter: ScanFilter) -> AppResult<ScanSummary> {
        let mem = self.mem()?;
        let mut g = self.attached.lock().unwrap();
        let a = g.as_mut().ok_or_else(|| AppError::Other("pick a process first".into()))?;
        if first || a.scanner.is_none() {
            let mut s = Scanner::new(vtype);
            s.first_scan(&*mem, filter, &self.control)?;
            a.scanner = Some(s);
        } else {
            a.scanner.as_mut().unwrap().next_scan(&*mem, filter, &self.control)?;
        }
        let s = a.scanner.as_ref().unwrap();
        Ok(ScanSummary { count: s.count(), hits: s.hits(&*mem, SHOWN_HITS) })
    }

    pub fn results(&self) -> AppResult<ScanSummary> {
        let mem = self.mem()?;
        let g = self.attached.lock().unwrap();
        match g.as_ref().and_then(|a| a.scanner.as_ref()) {
            Some(s) => Ok(ScanSummary { count: s.count(), hits: s.hits(&*mem, SHOWN_HITS) }),
            None => Ok(ScanSummary { count: 0, hits: vec![] }),
        }
    }

    pub fn reset(&self) {
        if let Some(a) = self.attached.lock().unwrap().as_mut() {
            a.scanner = None;
        }
    }

    pub fn progress(&self) -> u32 {
        self.control.progress()
    }

    pub fn cancel(&self) {
        self.control.cancel();
    }

    pub fn read(&self, targets: Vec<(PointerPath, ValueType)>) -> AppResult<Vec<Option<Value>>> {
        let mem = self.mem()?;
        Ok(targets
            .into_iter()
            .map(|(p, t)| {
                let a = p.resolve(&*mem).ok()?;
                mem.read_vec(a, t.size()).ok().map(|b| t.decode(&b))
            })
            .collect())
    }

    pub fn write(&self, path: &PointerPath, vtype: ValueType, value: Value) -> AppResult<()> {
        let mem = self.mem()?;
        let addr = path.resolve(&*mem)?;
        mem.write(addr, &vtype.encode(value))?;
        Ok(())
    }

    pub fn freeze(&self, key: &str, path: PointerPath, vtype: ValueType, value: Option<Value>) -> AppResult<()> {
        let g = self.attached.lock().unwrap();
        let a = g.as_ref().ok_or_else(|| AppError::Other("pick a process first".into()))?;
        match value {
            Some(v) => a.freezer.set(key, path, vtype.encode(v)),
            None => {
                a.freezer.remove(key);
            }
        }
        Ok(())
    }

    pub fn find_pointers(&self, address: u64, opts: PointerScanOptions) -> AppResult<Vec<PathView>> {
        let mem = self.mem()?;
        let paths = ptrscan::find_paths(&*mem, address, &opts, &self.control)?;
        Ok(paths.into_iter().map(view).collect())
    }

    pub fn filter_pointers(&self, paths: Vec<PointerPath>, address: u64) -> AppResult<Vec<PathView>> {
        let mem = self.mem()?;
        Ok(ptrscan::filter_paths(&*mem, &paths, address).into_iter().map(view).collect())
    }
}

fn view(path: PointerPath) -> PathView {
    PathView { display: path.to_string(), path }
}
