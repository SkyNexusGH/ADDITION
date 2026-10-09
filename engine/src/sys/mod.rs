//! Platform backends. Windows is the real target; Linux exists so the engine
//! can be exercised by automated tests on any CI runner.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{list_processes, open};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{list_processes, open};

#[cfg(not(any(windows, target_os = "linux")))]
mod unsupported {
    use crate::error::{EngineError, Result};
    use crate::memory::{Memory, ProcessInfo};

    pub fn list_processes() -> Result<Vec<ProcessInfo>> {
        Ok(vec![])
    }

    pub fn open(pid: u32) -> Result<Box<dyn Memory>> {
        Err(EngineError::OpenFailed { pid, reason: "unsupported platform".into() })
    }
}
#[cfg(not(any(windows, target_os = "linux")))]
pub use unsupported::{list_processes, open};
