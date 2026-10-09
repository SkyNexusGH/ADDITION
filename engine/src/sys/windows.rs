//! Windows backend: OpenProcess + Read/WriteProcessMemory, VirtualQueryEx for
//! the region list and Toolhelp32 snapshots for processes and modules.
//! A 64-bit ADDITION can drive 32-bit (WOW64) games; pointer size is detected
//! per process.

use crate::error::{Addr, EngineError, Result};
use crate::memory::{Memory, Module, ProcessInfo, Region};
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, BOOL, HANDLE, HMODULE, INVALID_HANDLE_VALUE, STILL_ACTIVE,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
    FlushInstructionCache, ReadProcessMemory, WriteProcessMemory,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Memory::{
    VirtualProtectEx, VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, MEM_IMAGE,
    MEM_MAPPED, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS,
    PAGE_PROTECTION_FLAGS, PAGE_READWRITE, PAGE_WRITECOPY,
};
use windows_sys::Win32::System::ProcessStatus::{
    EnumProcessModulesEx, GetModuleBaseNameW, GetModuleInformation, LIST_MODULES_ALL, MODULEINFO,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, IsWow64Process, OpenProcess, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_BAD_LENGTH: u32 = 24;

fn wide_to_string(w: &[u16]) -> String {
    let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..end])
}

struct Snapshot(HANDLE);

impl Snapshot {
    fn new(flags: u32, pid: u32) -> Result<Self> {
        // Module snapshots can fail with ERROR_BAD_LENGTH while the target is
        // loading DLLs; retrying is the documented fix.
        for _ in 0..10 {
            let h = unsafe { CreateToolhelp32Snapshot(flags, pid) };
            if h != INVALID_HANDLE_VALUE {
                return Ok(Snapshot(h));
            }
            if unsafe { GetLastError() } != ERROR_BAD_LENGTH {
                break;
            }
        }
        Err(EngineError::Other(format!("Toolhelp snapshot failed (error {})", unsafe { GetLastError() })))
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

pub fn list_processes() -> Result<Vec<ProcessInfo>> {
    let snap = Snapshot::new(TH32CS_SNAPPROCESS, 0)?;
    let mut out = Vec::new();
    let mut e: PROCESSENTRY32W = unsafe { zeroed() };
    e.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut ok = unsafe { Process32FirstW(snap.0, &mut e) };
    while ok != 0 {
        out.push(ProcessInfo { pid: e.th32ProcessID, name: wide_to_string(&e.szExeFile) });
        ok = unsafe { Process32NextW(snap.0, &mut e) };
    }
    Ok(out)
}

pub fn open(pid: u32) -> Result<Box<dyn Memory>> {
    let access = PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION;
    let handle = unsafe { OpenProcess(access, 0, pid) };
    if handle.is_null() {
        let code = unsafe { GetLastError() };
        let reason = if code == ERROR_ACCESS_DENIED {
            "access denied. The game may be running as administrator; run ADDITION as administrator too".to_string()
        } else {
            format!("OpenProcess error {code}")
        };
        return Err(EngineError::OpenFailed { pid, reason });
    }
    let mut wow64: BOOL = 0;
    unsafe { IsWow64Process(handle, &mut wow64) };
    let pointer_size = if wow64 != 0 || cfg!(target_pointer_width = "32") { 4 } else { 8 };
    Ok(Box::new(WinProcess { pid, handle, pointer_size }))
}

struct WinProcess {
    pid: u32,
    handle: HANDLE,
    pointer_size: usize,
}

// The process handle is a kernel object usable from any thread.
unsafe impl Send for WinProcess {}
unsafe impl Sync for WinProcess {}

impl Drop for WinProcess {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

fn is_writable(p: PAGE_PROTECTION_FLAGS) -> bool {
    p & (PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY) != 0
}

impl WinProcess {
    fn raw_write(&self, addr: u64, data: &[u8]) -> bool {
        let mut written = 0usize;
        let ok = unsafe {
            WriteProcessMemory(
                self.handle,
                addr as usize as *const c_void,
                data.as_ptr() as *const c_void,
                data.len(),
                &mut written,
            )
        };
        ok != 0 && written == data.len()
    }

    fn toolhelp_modules(&self) -> Result<Vec<Module>> {
        let snap = Snapshot::new(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, self.pid)?;
        let mut out = Vec::new();
        let mut e: MODULEENTRY32W = unsafe { zeroed() };
        e.dwSize = size_of::<MODULEENTRY32W>() as u32;
        let mut ok = unsafe { Module32FirstW(snap.0, &mut e) };
        while ok != 0 {
            out.push(Module {
                name: wide_to_string(&e.szModule),
                base: e.modBaseAddr as usize as u64,
                size: e.modBaseSize as u64,
            });
            ok = unsafe { Module32NextW(snap.0, &mut e) };
        }
        Ok(out)
    }

    /// Fallback for when Toolhelp can't snapshot the target (seen with
    /// 32-bit games under some compatibility layers).
    fn psapi_modules(&self) -> Result<Vec<Module>> {
        let mut handles: Vec<HMODULE> = vec![std::ptr::null_mut(); 1024];
        let mut needed = 0u32;
        let ok = unsafe {
            EnumProcessModulesEx(
                self.handle,
                handles.as_mut_ptr(),
                (handles.len() * size_of::<HMODULE>()) as u32,
                &mut needed,
                LIST_MODULES_ALL,
            )
        };
        if ok == 0 {
            return Err(EngineError::Other(format!("EnumProcessModulesEx failed (error {})", unsafe { GetLastError() })));
        }
        handles.truncate((needed as usize / size_of::<HMODULE>()).min(handles.len()));
        let mut out = Vec::new();
        for h in handles {
            let mut info: MODULEINFO = unsafe { zeroed() };
            let mut name = [0u16; 260];
            let got_info = unsafe { GetModuleInformation(self.handle, h, &mut info, size_of::<MODULEINFO>() as u32) };
            let len = unsafe { GetModuleBaseNameW(self.handle, h, name.as_mut_ptr(), name.len() as u32) };
            if got_info != 0 && len > 0 {
                out.push(Module {
                    name: String::from_utf16_lossy(&name[..len as usize]),
                    base: info.lpBaseOfDll as usize as u64,
                    size: info.SizeOfImage as u64,
                });
            }
        }
        Ok(out)
    }
}

impl Memory for WinProcess {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn pointer_size(&self) -> usize {
        self.pointer_size
    }

    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<()> {
        let mut read = 0usize;
        let ok = unsafe {
            ReadProcessMemory(
                self.handle,
                addr as usize as *const c_void,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                &mut read,
            )
        };
        if ok != 0 && read == buf.len() {
            Ok(())
        } else {
            Err(EngineError::Read(Addr(addr)))
        }
    }

    fn write(&self, addr: u64, data: &[u8]) -> Result<()> {
        if self.raw_write(addr, data) {
            return Ok(());
        }
        // Code and read-only data: lift protection, write, put it back.
        let mut old: PAGE_PROTECTION_FLAGS = 0;
        let lifted = unsafe {
            VirtualProtectEx(self.handle, addr as usize as *const c_void, data.len(), PAGE_EXECUTE_READWRITE, &mut old)
        };
        if lifted == 0 {
            return Err(EngineError::Write(Addr(addr)));
        }
        let ok = self.raw_write(addr, data);
        unsafe {
            let mut dummy: PAGE_PROTECTION_FLAGS = 0;
            VirtualProtectEx(self.handle, addr as usize as *const c_void, data.len(), old, &mut dummy);
            FlushInstructionCache(self.handle, addr as usize as *const c_void, data.len());
        }
        if ok {
            Ok(())
        } else {
            Err(EngineError::Write(Addr(addr)))
        }
    }

    fn modules(&self) -> Result<Vec<Module>> {
        match self.toolhelp_modules() {
            Ok(m) if !m.is_empty() => Ok(m),
            first => self.psapi_modules().or(first),
        }
    }

    fn regions(&self) -> Result<Vec<Region>> {
        let mut out = Vec::new();
        let mut addr: u64 = 0;
        loop {
            let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { zeroed() };
            let n = unsafe {
                VirtualQueryEx(
                    self.handle,
                    addr as usize as *const c_void,
                    &mut mbi,
                    size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            if n == 0 {
                break;
            }
            let base = mbi.BaseAddress as usize as u64;
            let size = mbi.RegionSize as u64;
            let readable = mbi.State == MEM_COMMIT
                && mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD) == 0
                && mbi.Protect != 0;
            // Mapped files (fonts, shared sections) never hold game state.
            if readable && mbi.Type != MEM_MAPPED {
                out.push(Region {
                    base,
                    size,
                    writable: is_writable(mbi.Protect),
                    image: mbi.Type == MEM_IMAGE,
                });
            }
            match base.checked_add(size) {
                Some(next) if next > addr => addr = next,
                _ => break,
            }
        }
        Ok(out)
    }

    fn is_alive(&self) -> bool {
        let mut code = 0u32;
        let ok = unsafe { GetExitCodeProcess(self.handle, &mut code) };
        ok != 0 && code == STILL_ACTIVE as u32
    }
}
