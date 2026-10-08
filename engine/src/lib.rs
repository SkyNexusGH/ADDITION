//! ADDITION's trainer engine: attach to a game process, read and write its
//! memory, follow pointer chains, patch code by byte signature, keep values
//! frozen, and scan memory to find new cheats.
//!
//! Windows is the real target. The Linux backend exists so all of this can
//! be tested automatically against `test-game`.

pub mod anticheat;
pub mod aob;
pub mod error;
pub mod freezer;
pub mod memory;
pub mod pointer;
pub mod ptrscan;
pub mod scan;
pub mod session;
mod sys;
pub mod trainer;
pub mod value;

#[cfg(test)]
pub(crate) mod testing;

pub use error::{EngineError, Result};
pub use memory::{Memory, MemoryExt, Module, ProcessInfo, Region};
pub use pointer::PointerPath;
pub use session::{check_anticheat, find_process, list_processes, open_process, CheatState, Session};
pub use trainer::{Action, Cheat, Trainer};
pub use value::{Value, ValueType};
