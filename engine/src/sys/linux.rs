//! Linux backend via `/proc/<pid>/mem` and `/proc/<pid>/maps`.
//! Writes through `/proc/<pid>/mem` ignore page protection, like
//! `WriteProcessMemory` + `VirtualProtectEx` on Windows.

use crate::error::{Addr, EngineError, Result};
use crate::memory::{Memory, Module, ProcessInfo, Region};
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::path::Path;

pub fn list_processes() -> Result<Vec<ProcessInfo>> {
    let mut out = Vec::new();
    for entry in fs::read_dir("/proc")?.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let name = fs::read_link(entry.path().join("exe"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .or_else(|| fs::read_to_string(entry.path().join("comm")).ok().map(|s| s.trim().to_string()));
        if let Some(name) = name {
            out.push(ProcessInfo { pid, name });
        }
    }
    Ok(out)
}

pub fn open(pid: u32) -> Result<Box<dyn Memory>> {
    let fail = |e: std::io::Error| EngineError::OpenFailed { pid, reason: e.to_string() };
    let mem = OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("/proc/{pid}/mem"))
        .map_err(fail)?;
    let mut hdr = [0u8; 5];
    File::open(format!("/proc/{pid}/exe"))
        .and_then(|f| f.read_exact_at(&mut hdr, 0))
        .map_err(fail)?;
    let pointer_size = if hdr[4] == 1 { 4 } else { 8 };
    Ok(Box::new(LinuxProcess { pid, mem, pointer_size }))
}

struct LinuxProcess {
    pid: u32,
    mem: File,
    pointer_size: usize,
}

struct MapLine {
    start: u64,
    end: u64,
    readable: bool,
    writable: bool,
    path: String,
}

impl LinuxProcess {
    fn maps(&self) -> Result<Vec<MapLine>> {
        let text = fs::read_to_string(format!("/proc/{}/maps", self.pid))
            .map_err(|_| EngineError::ProcessExited)?;
        Ok(text.lines().filter_map(parse_map_line).collect())
    }
}

fn parse_map_line(line: &str) -> Option<MapLine> {
    let mut parts = line.split_whitespace();
    let range = parts.next()?;
    let perms = parts.next()?;
    let path = parts.nth(3).unwrap_or("").to_string();
    let (a, b) = range.split_once('-')?;
    Some(MapLine {
        start: u64::from_str_radix(a, 16).ok()?,
        end: u64::from_str_radix(b, 16).ok()?,
        readable: perms.starts_with('r'),
        writable: perms.as_bytes().get(1) == Some(&b'w'),
        path,
    })
}

impl Memory for LinuxProcess {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn pointer_size(&self) -> usize {
        self.pointer_size
    }

    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<()> {
        self.mem.read_exact_at(buf, addr).map_err(|_| EngineError::Read(Addr(addr)))
    }

    fn write(&self, addr: u64, data: &[u8]) -> Result<()> {
        self.mem.write_all_at(data, addr).map_err(|_| EngineError::Write(Addr(addr)))
    }

    fn modules(&self) -> Result<Vec<Module>> {
        let maps = self.maps()?;
        let mut mods: Vec<Module> = Vec::new();
        for (i, m) in maps.iter().enumerate() {
            if !m.path.starts_with('/') {
                // An anonymous mapping directly after a file mapping is that
                // file's .bss, which holds statics, so it belongs to the module.
                let prev = i.checked_sub(1).map(|j| &maps[j]);
                if let (Some(prev), Some(last)) = (prev, mods.last_mut()) {
                    if m.path.is_empty() && prev.end == m.start && last.base + last.size == m.start {
                        last.size = m.end - last.base;
                    }
                }
                continue;
            }
            let name = Path::new(&m.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            match mods.last_mut() {
                Some(last) if last.name == name => last.size = m.end - last.base,
                _ => mods.push(Module { name, base: m.start, size: m.end - m.start }),
            }
        }
        Ok(mods)
    }

    fn regions(&self) -> Result<Vec<Region>> {
        let mods = self.modules()?;
        Ok(self
            .maps()?
            .into_iter()
            .filter(|m| m.readable && !m.path.starts_with("[v"))
            .map(|m| Region {
                base: m.start,
                size: m.end - m.start,
                writable: m.writable,
                image: mods.iter().any(|md| md.contains(m.start)),
            })
            .collect())
    }

    fn is_alive(&self) -> bool {
        // A zombie still has a /proc entry; check its state too.
        match fs::read_to_string(format!("/proc/{}/stat", self.pid)) {
            Ok(s) => !matches!(s.rsplit(')').next().map(str::trim_start).and_then(|r| r.chars().next()), Some('Z' | 'X')),
            Err(_) => false,
        }
    }
}
