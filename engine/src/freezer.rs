//! Keeps values pinned by rewriting them on a background thread. Pointer
//! chains are re-resolved every tick, because games rebuild objects on level
//! loads and the final address moves.

use crate::memory::Memory;
use crate::pointer::PointerPath;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub const INTERVAL: Duration = Duration::from_millis(50);

type Entries = Arc<Mutex<HashMap<String, (PointerPath, Vec<u8>)>>>;

pub struct Freezer {
    entries: Entries,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Freezer {
    pub fn start(mem: Arc<dyn Memory>) -> Self {
        let entries: Entries = Default::default();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (entries, stop) = (entries.clone(), stop.clone());
            std::thread::Builder::new()
                .name("addition-freezer".into())
                .spawn(move || {
                    while !stop.load(Ordering::Relaxed) && mem.is_alive() {
                        {
                            // Hold the lock for the whole tick so that once
                            // `remove` returns, no stale write can follow it.
                            let entries = entries.lock().unwrap();
                            for (path, bytes) in entries.values() {
                                // Failures are normal while the game is loading.
                                if let Ok(addr) = path.resolve(&*mem) {
                                    let _ = mem.write(addr, bytes);
                                }
                            }
                        }
                        std::thread::sleep(INTERVAL);
                    }
                })
                .expect("spawn freezer thread")
        };
        Freezer { entries, stop, thread: Some(thread) }
    }

    pub fn set(&self, key: &str, path: PointerPath, bytes: Vec<u8>) {
        self.entries.lock().unwrap().insert(key.to_string(), (path, bytes));
    }

    pub fn remove(&self, key: &str) -> bool {
        self.entries.lock().unwrap().remove(key).is_some()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.lock().unwrap().contains_key(key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.entries.lock().unwrap().keys().cloned().collect()
    }
}

impl Drop for Freezer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
