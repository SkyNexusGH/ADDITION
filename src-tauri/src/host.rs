//! Runs the trainer the user has open: waits for the game to start, attaches,
//! keeps the session alive, and wires hotkeys to cheats.

use crate::error::{AppError, AppResult};
use addition_engine::{check_anticheat, find_process, CheatState, Session, Trainer, Value};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub const UPDATED_EVENT: &str = "trainer-updated";

#[derive(Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    /// Trainer chosen, game not running yet.
    Waiting,
    Attached,
    /// Anti-cheat present; nothing will be touched.
    Blocked,
    Error,
}

#[derive(Clone, Serialize)]
pub struct HostStatus {
    pub trainer_id: Option<String>,
    pub phase: Phase,
    pub message: Option<String>,
    pub pid: Option<u32>,
    pub cheats: Vec<CheatState>,
    /// Hotkeys that couldn't be registered (taken by another app, bad name).
    pub hotkey_errors: Vec<String>,
}

#[derive(Default)]
struct Inner {
    trainer: Option<Trainer>,
    session: Option<Session>,
    phase: Option<Phase>,
    message: Option<String>,
    hotkeys: HashMap<u32, String>,
    hotkey_errors: Vec<String>,
}

#[derive(Default)]
pub struct Host(Mutex<Inner>);

impl Host {
    /// Opens a trainer. Detaches from any other game first.
    pub fn select(&self, app: &AppHandle, trainer: Trainer) {
        let mut g = self.0.lock().unwrap();
        // Re-opening the same, unchanged trainer keeps the running session.
        // An edited trainer reattaches so the new cheats take effect.
        let same = g.trainer.as_ref().is_some_and(|t| t.to_json() == trainer.to_json());
        if same && g.session.is_some() {
            return;
        }
        Self::detach(app, &mut g);
        g.trainer = Some(trainer);
        g.phase = Some(Phase::Waiting);
        g.message = None;
    }

    pub fn stop(&self, app: &AppHandle) {
        let mut g = self.0.lock().unwrap();
        Self::detach(app, &mut g);
        g.trainer = None;
        g.phase = None;
        g.message = None;
    }

    fn detach(app: &AppHandle, g: &mut Inner) {
        // Dropping the session restores patched code.
        g.session = None;
        if !g.hotkeys.is_empty() {
            let _ = app.global_shortcut().unregister_all();
            g.hotkeys.clear();
        }
        g.hotkey_errors.clear();
    }

    /// Called by the UI about once a second: attaches when the game appears,
    /// notices when it closes.
    pub fn poll(&self, app: &AppHandle) -> HostStatus {
        let mut g = self.0.lock().unwrap();
        if g.session.as_ref().is_some_and(|s| !s.is_alive()) {
            Self::detach(app, &mut g);
            g.phase = Some(Phase::Waiting);
            g.message = Some("The game closed. Waiting for it to start again.".into());
        }
        if g.session.is_none() && g.trainer.is_some() {
            Self::try_attach(app, &mut g);
        }
        Self::status(&g)
    }

    fn try_attach(app: &AppHandle, g: &mut Inner) {
        let trainer = g.trainer.clone().expect("checked by caller");
        let found = match find_process(&trainer.process) {
            Ok(f) => f,
            Err(e) => return Self::fail(g, Phase::Error, e.to_string()),
        };
        let Some(proc_) = found else {
            if g.phase != Some(Phase::Waiting) {
                g.phase = Some(Phase::Waiting);
            }
            return;
        };
        if let Err(e) = check_anticheat(None) {
            return Self::fail(g, Phase::Blocked, e.to_string());
        }
        match Session::attach(trainer.clone(), proc_.pid) {
            Ok(s) => {
                g.session = Some(s);
                g.phase = Some(Phase::Attached);
                g.message = None;
                Self::register_hotkeys(app, g, &trainer);
            }
            Err(addition_engine::EngineError::AntiCheat(m)) => Self::fail(g, Phase::Blocked, m),
            Err(e) => Self::fail(g, Phase::Error, e.to_string()),
        }
    }

    fn fail(g: &mut Inner, phase: Phase, msg: String) {
        g.phase = Some(phase);
        g.message = Some(msg);
    }

    fn register_hotkeys(app: &AppHandle, g: &mut Inner, trainer: &Trainer) {
        for c in &trainer.cheats {
            let Some(key) = &c.hotkey else { continue };
            match key.parse::<Shortcut>() {
                Ok(sc) => match app.global_shortcut().register(sc) {
                    Ok(()) => {
                        g.hotkeys.insert(sc.id(), c.id.clone());
                    }
                    Err(e) => g.hotkey_errors.push(format!("{key} ({}): {e}", c.name)),
                },
                Err(e) => g.hotkey_errors.push(format!("{key} ({}): {e}", c.name)),
            }
        }
    }

    /// Global shortcut handler.
    pub fn on_hotkey(&self, app: &AppHandle, shortcut: &Shortcut) {
        {
            let mut g = self.0.lock().unwrap();
            let Some(id) = g.hotkeys.get(&shortcut.id()).cloned() else { return };
            if let Some(s) = g.session.as_mut() {
                // Errors are recorded per cheat and shown in the UI.
                let _ = s.hotkey(&id);
            }
        }
        let _ = app.emit(UPDATED_EVENT, ());
    }

    fn status(g: &Inner) -> HostStatus {
        HostStatus {
            trainer_id: g.trainer.as_ref().map(|t| t.id.clone()),
            phase: g.phase.unwrap_or(Phase::Idle),
            message: g.message.clone(),
            pid: g.session.as_ref().map(|s| s.pid()),
            cheats: g.session.as_ref().map(|s| s.states()).unwrap_or_default(),
            hotkey_errors: g.hotkey_errors.clone(),
        }
    }

    fn with_session<T>(&self, f: impl FnOnce(&mut Session) -> addition_engine::Result<T>) -> AppResult<HostStatus> {
        let mut g = self.0.lock().unwrap();
        let s = g.session.as_mut().ok_or_else(|| AppError::Other("the game isn't running yet".into()))?;
        // Per-cheat errors also land in the status, so the UI shows them inline.
        let _ = f(s);
        Ok(Self::status(&g))
    }

    pub fn enable(&self, id: &str, value: Option<Value>) -> AppResult<HostStatus> {
        self.with_session(|s| s.enable(id, value))
    }

    pub fn disable(&self, id: &str) -> AppResult<HostStatus> {
        self.with_session(|s| s.disable(id))
    }

    pub fn set_value(&self, id: &str, value: Value) -> AppResult<HostStatus> {
        self.with_session(|s| s.set_value(id, value))
    }

    /// Restores patched code before the app exits.
    pub fn shutdown(&self) {
        if let Ok(mut g) = self.0.lock() {
            g.session = None;
        }
    }
}
