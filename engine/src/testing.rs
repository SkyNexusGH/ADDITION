//! An in-memory fake process for unit tests.

use crate::error::{Addr, EngineError, Result};
use crate::memory::{Memory, Module, Region};
use std::sync::Mutex;

pub struct FakeMemory {
    ptr: usize,
    regions: Vec<(Region, Mutex<Vec<u8>>)>,
    modules: Vec<Module>,
}

impl FakeMemory {
    pub fn new(pointer_size: usize) -> Self {
        FakeMemory { ptr: pointer_size, regions: vec![], modules: vec![] }
    }

    pub fn map(&mut self, base: u64, size: u64) {
        let r = Region { base, size, writable: true, image: false };
        self.regions.push((r, Mutex::new(vec![0; size as usize])));
    }

    pub fn add_module(&mut self, name: &str, base: u64, size: u64) {
        self.map(base, size);
        self.regions.last_mut().unwrap().0.image = true;
        self.modules.push(Module { name: name.into(), base, size });
    }

    pub fn put(&self, addr: u64, data: &[u8]) {
        self.write(addr, data).unwrap();
    }

    pub fn put_u64(&self, addr: u64, v: u64) {
        self.put(addr, &v.to_le_bytes()[..self.ptr]);
    }

    fn locate(&self, addr: u64, len: usize) -> Option<(usize, usize)> {
        self.regions.iter().enumerate().find_map(|(i, (r, _))| {
            (addr >= r.base && addr + len as u64 <= r.end()).then(|| (i, (addr - r.base) as usize))
        })
    }
}

impl Memory for FakeMemory {
    fn pid(&self) -> u32 {
        1
    }
    fn pointer_size(&self) -> usize {
        self.ptr
    }
    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<()> {
        let (i, off) = self.locate(addr, buf.len()).ok_or(EngineError::Read(Addr(addr)))?;
        buf.copy_from_slice(&self.regions[i].1.lock().unwrap()[off..off + buf.len()]);
        Ok(())
    }
    fn write(&self, addr: u64, data: &[u8]) -> Result<()> {
        let (i, off) = self.locate(addr, data.len()).ok_or(EngineError::Write(Addr(addr)))?;
        self.regions[i].1.lock().unwrap()[off..off + data.len()].copy_from_slice(data);
        Ok(())
    }
    fn modules(&self) -> Result<Vec<Module>> {
        Ok(self.modules.clone())
    }
    fn regions(&self) -> Result<Vec<Region>> {
        Ok(self.regions.iter().map(|(r, _)| *r).collect())
    }
    fn is_alive(&self) -> bool {
        true
    }
}
