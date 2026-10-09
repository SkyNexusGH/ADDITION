//! A trainer attached to a running game.

use crate::aob::{self, Pattern};
use crate::anticheat;
use crate::error::{Addr, EngineError, Result};
use crate::freezer::Freezer;
use crate::memory::{Memory, MemoryExt, ProcessInfo};
use crate::pointer::PointerPath;
use crate::sys;
use crate::trainer::{Action, Cheat, Trainer};
use crate::value::Value;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Finds a running process by executable name (case-insensitive). Earlier
/// names win, so a trainer can list the real game before its launcher.
pub fn find_process(names: &[String]) -> Result<Option<ProcessInfo>> {
    let procs = sys::list_processes()?;
    Ok(names
        .iter()
        .find_map(|n| procs.iter().find(|p| n.eq_ignore_ascii_case(&p.name)))
        .cloned())
}

pub fn list_processes() -> Result<Vec<ProcessInfo>> {
    sys::list_processes()
}

/// Opens a process for reading and writing.
pub fn open_process(pid: u32) -> Result<Arc<dyn Memory>> {
    Ok(Arc::from(sys::open(pid)?))
}

/// Refuses to touch anything while kernel anti-cheat is around.
pub fn check_anticheat(mem: Option<&dyn Memory>) -> Result<()> {
    if let Some(found) = anticheat::detect_in_processes(&sys::list_processes()?) {
        return Err(EngineError::AntiCheat(format!("{found} is running")));
    }
    if let Some(mem) = mem {
        if let Some(found) = anticheat::detect_in_modules(&mem.modules()?) {
            return Err(EngineError::AntiCheat(format!("{found} is loaded in the game")));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct CheatState {
    pub id: String,
    pub active: bool,
    /// Current value for value cheats, when it can be read.
    pub value: Option<Value>,
    pub error: Option<String>,
}

struct AppliedPatch {
    addr: u64,
    original: Vec<u8>,
}

pub struct Session {
    mem: Arc<dyn Memory>,
    trainer: Trainer,
    freezer: Freezer,
    patches: HashMap<String, AppliedPatch>,
    /// Patch sites found so far, so toggling doesn't rescan the module.
    patch_sites: HashMap<String, u64>,
    errors: HashMap<String, String>,
}

impl Session {
    pub fn attach(trainer: Trainer, pid: u32) -> Result<Self> {
        let mem = open_process(pid)?;
        Self::with_memory(trainer, mem)
    }

    pub fn with_memory(trainer: Trainer, mem: Arc<dyn Memory>) -> Result<Self> {
        check_anticheat(Some(&*mem))?;
        Ok(Session {
            freezer: Freezer::start(mem.clone()),
            mem,
            trainer,
            patches: HashMap::new(),
            patch_sites: HashMap::new(),
            errors: HashMap::new(),
        })
    }

    pub fn trainer(&self) -> &Trainer {
        &self.trainer
    }

    pub fn pid(&self) -> u32 {
        self.mem.pid()
    }

    pub fn is_alive(&self) -> bool {
        self.mem.is_alive()
    }

    pub fn memory(&self) -> Arc<dyn Memory> {
        self.mem.clone()
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.freezer.contains(id) || self.patches.contains_key(id)
    }

    /// Turns a toggle cheat on. For `set` cheats this locks `value`.
    pub fn enable(&mut self, id: &str, value: Option<Value>) -> Result<()> {
        let cheat = self.trainer.cheat(id)?.clone();
        let r = self.enable_inner(&cheat, value);
        self.record(id, &r);
        r
    }

    fn enable_inner(&mut self, cheat: &Cheat, value: Option<Value>) -> Result<()> {
        match &cheat.action {
            Action::Freeze { target, vtype, value: v } => {
                let bytes = vtype.encode(value.unwrap_or(*v));
                self.write_now(target, &bytes)?;
                self.freezer.set(&cheat.id, target.clone(), bytes);
            }
            Action::Set { target, vtype, default, .. } => {
                let v = value.or(*default).ok_or_else(|| EngineError::Invalid("no value to lock".into()))?;
                let bytes = vtype.encode(v);
                self.write_now(target, &bytes)?;
                self.freezer.set(&cheat.id, target.clone(), bytes);
            }
            Action::Patch { module, aob: sig, offset, bytes } => {
                if self.patches.contains_key(&cheat.id) {
                    return Ok(());
                }
                let addr = self.patch_site(&cheat.id, module.as_deref(), sig)?.wrapping_add(*offset as u64);
                let new = aob::parse_bytes(bytes)?;
                let original = self.mem.read_vec(addr, new.len())?;
                self.mem.write(addr, &new)?;
                self.patches.insert(cheat.id.clone(), AppliedPatch { addr, original });
            }
        }
        Ok(())
    }

    fn patch_site(&mut self, id: &str, module: Option<&str>, sig: &str) -> Result<u64> {
        if let Some(a) = self.patch_sites.get(id) {
            return Ok(*a);
        }
        let module = match module {
            Some(name) => self.mem.module(name)?,
            None => self.mem.modules()?.into_iter().next().ok_or_else(|| EngineError::ModuleNotFound("main executable".into()))?,
        };
        let pattern = Pattern::parse(sig)?;
        let hits = aob::scan_module(&*self.mem, &module, &pattern)?;
        let addr = *hits.first().ok_or_else(|| {
            EngineError::PatternNotFound(format!("{pattern} in {} (wrong game version, or already patched by another tool?)", module.name))
        })?;
        self.patch_sites.insert(id.to_string(), addr);
        Ok(addr)
    }

    pub fn disable(&mut self, id: &str) -> Result<()> {
        self.freezer.remove(id);
        if let Some(p) = self.patches.remove(id) {
            let r = self.mem.write(p.addr, &p.original);
            if r.is_err() {
                // Keep it so detach can try again.
                self.patches.insert(id.to_string(), p);
            }
            self.record(id, &r);
            return r;
        }
        self.errors.remove(id);
        Ok(())
    }

    pub fn toggle(&mut self, id: &str) -> Result<bool> {
        if self.is_active(id) {
            self.disable(id)?;
            Ok(false)
        } else {
            self.enable(id, None)?;
            Ok(true)
        }
    }

    /// One-shot write for `set` cheats. Updates the lock value if locked.
    pub fn set_value(&mut self, id: &str, value: Value) -> Result<()> {
        let cheat = self.trainer.cheat(id)?.clone();
        let Action::Set { target, vtype, min, max, .. } = &cheat.action else {
            return Err(EngineError::Invalid(format!("'{}' is not a value cheat", cheat.name)));
        };
        let x = value.as_f64();
        if min.is_some_and(|m| x < m) || max.is_some_and(|m| x > m) {
            return Err(EngineError::Invalid(format!("{value} is outside the allowed range")));
        }
        let bytes = vtype.encode(value);
        let r = self.write_now(target, &bytes);
        if r.is_ok() && self.freezer.contains(id) {
            self.freezer.set(id, target.clone(), bytes);
        }
        self.record(id, &r);
        r
    }

    /// What the hotkey for a cheat does: toggle, or apply the default value.
    pub fn hotkey(&mut self, id: &str) -> Result<()> {
        let cheat = self.trainer.cheat(id)?.clone();
        match &cheat.action {
            Action::Set { default: Some(v), .. } => self.set_value(id, *v),
            Action::Set { .. } => Ok(()),
            _ => self.toggle(id).map(|_| ()),
        }
    }

    fn write_now(&self, target: &PointerPath, bytes: &[u8]) -> Result<()> {
        let addr = target.resolve(&*self.mem)?;
        self.mem.write(addr, bytes).map_err(|_| EngineError::Write(Addr(addr)))
    }

    fn record(&mut self, id: &str, r: &Result<()>) {
        match r {
            Ok(()) => self.errors.remove(id),
            Err(e) => self.errors.insert(id.to_string(), e.to_string()),
        };
    }

    pub fn states(&self) -> Vec<CheatState> {
        self.trainer
            .cheats
            .iter()
            .map(|c| {
                let value = match &c.action {
                    Action::Freeze { target, vtype, .. } | Action::Set { target, vtype, .. } => target
                        .resolve(&*self.mem)
                        .and_then(|a| self.mem.read_vec(a, vtype.size()))
                        .ok()
                        .map(|b| vtype.decode(&b)),
                    Action::Patch { .. } => None,
                };
                CheatState {
                    id: c.id.clone(),
                    active: self.is_active(&c.id),
                    value,
                    error: self.errors.get(&c.id).cloned(),
                }
            })
            .collect()
    }

    /// Turns everything off and restores patched code.
    pub fn disable_all(&mut self) {
        for k in self.freezer.keys() {
            self.freezer.remove(&k);
        }
        let ids: Vec<String> = self.patches.keys().cloned().collect();
        for id in ids {
            let _ = self.disable(&id);
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.mem.is_alive() {
            self.disable_all();
        }
    }
}
