//! The `Memory` trait is the only thing the rest of the engine knows about a
//! process. Windows (the real target) and Linux (tests/CI) implement it in
//! `sys/`, and unit tests implement it with a fake in-memory address space.

use crate::error::{Addr, EngineError, Result};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

/// A loaded executable image (the game .exe or a .dll).
#[derive(Clone, Debug, Serialize)]
pub struct Module {
    pub name: String,
    pub base: u64,
    pub size: u64,
}

impl Module {
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.base + self.size
    }
}

/// A committed, readable range of the target's address space.
#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub base: u64,
    pub size: u64,
    pub writable: bool,
    /// Backed by an executable image (exe/dll) rather than heap/stack.
    pub image: bool,
}

impl Region {
    pub fn end(&self) -> u64 {
        self.base + self.size
    }
}

pub trait Memory: Send + Sync {
    fn pid(&self) -> u32;

    /// 4 for 32-bit games, 8 for 64-bit games.
    fn pointer_size(&self) -> usize;

    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<()>;

    /// Writes even to read-only/code pages (needed for byte patches).
    fn write(&self, addr: u64, data: &[u8]) -> Result<()>;

    fn modules(&self) -> Result<Vec<Module>>;

    fn regions(&self) -> Result<Vec<Region>>;

    fn is_alive(&self) -> bool;
}

/// Convenience helpers available on every `Memory`.
pub trait MemoryExt: Memory {
    fn read_vec(&self, addr: u64, len: usize) -> Result<Vec<u8>> {
        let mut v = vec![0u8; len];
        self.read(addr, &mut v)?;
        Ok(v)
    }

    fn read_ptr(&self, addr: u64) -> Result<u64> {
        let mut b = [0u8; 8];
        let n = self.pointer_size();
        self.read(addr, &mut b[..n])?;
        Ok(u64::from_le_bytes(b))
    }

    fn module(&self, name: &str) -> Result<Module> {
        self.modules()?
            .into_iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| EngineError::ModuleNotFound(name.to_string()))
    }

    /// Reads a module's readable pages; unreadable gaps are left zeroed.
    fn read_module(&self, m: &Module) -> Result<Vec<u8>> {
        let mut out = vec![0u8; m.size as usize];
        let mut any = false;
        for r in self.regions()? {
            let start = r.base.max(m.base);
            let end = r.end().min(m.base + m.size);
            if start >= end {
                continue;
            }
            let off = (start - m.base) as usize;
            if self.read(start, &mut out[off..off + (end - start) as usize]).is_ok() {
                any = true;
            }
        }
        if any {
            Ok(out)
        } else {
            Err(EngineError::Read(Addr(m.base)))
        }
    }
}

impl<T: Memory + ?Sized> MemoryExt for T {}
