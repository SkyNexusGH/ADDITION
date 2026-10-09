//! Pointer-path finder. Given the address of a value found with the scanner,
//! looks for chains of pointers that start at a static address inside a module
//! and lead to it. Static starts survive a game restart, heap addresses don't.

use crate::error::Result;
use crate::memory::{Memory, Region};
use crate::pointer::PointerPath;
use crate::scan::ScanControl;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PointerScanOptions {
    pub max_depth: usize,
    /// Largest field offset considered inside one object.
    pub max_offset: u64,
    pub max_results: usize,
}

impl Default for PointerScanOptions {
    fn default() -> Self {
        PointerScanOptions { max_depth: 4, max_offset: 0x1000, max_results: 200 }
    }
}

/// Caps work per level so a huge game can't run the finder forever.
const MAX_FRONTIER: usize = 50_000;

/// Paths collected before ranking. Heap bookkeeping inside system DLLs also
/// points at game objects, so the pool must be much bigger than what's shown,
/// or the game's own chains can be cut off before ranking sees them.
const POOL: usize = 20_000;

fn contains(regions: &[Region], v: u64) -> bool {
    let i = regions.partition_point(|r| r.end() <= v);
    i < regions.len() && regions[i].base <= v
}

pub fn find_paths(
    mem: &dyn Memory,
    target: u64,
    opts: &PointerScanOptions,
    ctl: &ScanControl,
) -> Result<Vec<PointerPath>> {
    ctl.reset();
    let ptr = mem.pointer_size();
    let modules = mem.modules()?;
    let mut readable = mem.regions()?;
    readable.sort_by_key(|r| r.base);
    let holders: Vec<Region> = readable.iter().copied().filter(|r| r.writable).collect();

    // 1. Pointer map: every aligned slot whose value points into mapped memory,
    //    sorted by the value it points to.
    let total: u64 = holders.iter().map(|r| r.size).sum();
    let mut done = 0u64;
    let mut map: Vec<(u64, u64)> = Vec::new();
    for r in &holders {
        ctl.check(done / 2, total)?;
        done += r.size;
        let mut buf = vec![0u8; r.size as usize];
        if mem.read(r.base, &mut buf).is_err() {
            continue;
        }
        for (i, slot) in buf.chunks_exact(ptr).enumerate() {
            let mut b = [0u8; 8];
            b[..ptr].copy_from_slice(slot);
            let v = u64::from_le_bytes(b);
            if v != 0 && contains(&readable, v) {
                map.push((v, r.base + (i * ptr) as u64));
            }
        }
    }
    map.sort_unstable();

    // 2. Walk backwards from the target, one pointer level at a time.
    let mut results = Vec::new();
    let mut frontier: Vec<(u64, Vec<i64>)> = vec![(target, vec![])];
    let mut seen: HashSet<u64> = HashSet::from([target]);
    for depth in 1..=opts.max_depth {
        ctl.check(total / 2 + (total / 2) * depth as u64 / opts.max_depth as u64, total)?;
        let mut next = Vec::new();
        for (addr, offs) in &frontier {
            let lo = addr.saturating_sub(opts.max_offset);
            let start = map.partition_point(|(v, _)| *v < lo);
            for &(v, slot) in map[start..].iter().take_while(|(v, _)| v <= addr) {
                let mut chain = Vec::with_capacity(offs.len() + 1);
                chain.push((addr - v) as i64);
                chain.extend_from_slice(offs);
                if let Some(m) = modules.iter().find(|m| m.contains(slot)) {
                    results.push(PointerPath {
                        module: Some(m.name.clone()),
                        base: (slot - m.base) as i64,
                        offsets: chain,
                    });
                    if results.len() >= POOL {
                        return Ok(rank(results, &modules, opts.max_results));
                    }
                } else if depth < opts.max_depth && next.len() < MAX_FRONTIER && seen.insert(slot) {
                    next.push((slot, chain));
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    Ok(rank(results, &modules, opts.max_results))
}

/// Chains from the game's own .exe first, shortest first: they're the most
/// likely to keep working across restarts and patches.
fn rank(mut paths: Vec<PointerPath>, modules: &[crate::memory::Module], keep: usize) -> Vec<PointerPath> {
    let main = modules.first().map(|m| m.name.clone());
    paths.sort_by_key(|p| (p.module != main, p.offsets.len(), p.offsets.iter().map(|o| o.unsigned_abs()).sum::<u64>()));
    paths.truncate(keep);
    paths
}

/// After restarting the game and finding the value again, keep only the
/// paths that lead to the new address.
pub fn filter_paths(mem: &dyn Memory, paths: &[PointerPath], new_target: u64) -> Vec<PointerPath> {
    paths.iter().filter(|p| p.resolve(mem).ok() == Some(new_target)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeMemory;

    #[test]
    fn finds_two_level_chain() {
        let mut m = FakeMemory::new(8);
        m.add_module("game.exe", 0x400000, 0x1000);
        m.map(0x10000, 0x1000);
        m.map(0x20000, 0x1000);
        m.put_u64(0x400100, 0x10000); // static -> world
        m.put_u64(0x10040, 0x20000); // world+0x40 -> player
        let target = 0x20010; // player.health
        let paths = find_paths(&m, target, &PointerScanOptions::default(), &ScanControl::default()).unwrap();
        let want = PointerPath { module: Some("game.exe".into()), base: 0x100, offsets: vec![0x40, 0x10] };
        assert_eq!(paths.first(), Some(&want));
        assert_eq!(filter_paths(&m, &paths, target).len(), paths.len());
        assert!(filter_paths(&m, &paths, target + 4).is_empty());
    }
}
