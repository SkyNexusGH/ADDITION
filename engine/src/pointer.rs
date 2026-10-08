//! Pointer chains: `[["game.exe"+base]+o1]+o2`. Games allocate their objects
//! on the heap at a different address every run; a chain that starts from a
//! static address inside the .exe leads back to the same value every time.

use crate::error::{EngineError, Result};
use crate::memory::{Memory, MemoryExt};
use crate::value::parse_int;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PointerPath {
    /// Module the base offset is relative to. `None` means `base` is absolute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(with = "hex_i64")]
    pub base: i64,
    /// Each step reads a pointer at the current address, then adds the offset.
    #[serde(default, with = "hex_vec")]
    pub offsets: Vec<i64>,
}

impl PointerPath {
    pub fn absolute(addr: u64) -> Self {
        PointerPath { module: None, base: addr as i64, offsets: vec![] }
    }

    pub fn resolve(&self, mem: &dyn Memory) -> Result<u64> {
        let mut addr = match &self.module {
            Some(m) => mem.module(m)?.base.wrapping_add(self.base as u64),
            None => self.base as u64,
        };
        for off in &self.offsets {
            let p = mem.read_ptr(addr)?;
            if p == 0 {
                return Err(EngineError::Other("null pointer in chain (is the game still loading?)".into()));
            }
            addr = p.wrapping_add(*off as u64);
        }
        Ok(addr)
    }
}

impl std::fmt::Display for PointerPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = match &self.module {
            Some(m) => format!("\"{m}\"+{}", hex(self.base)),
            None => hex(self.base),
        };
        for o in &self.offsets {
            s = format!("[{s}]+{}", hex(*o));
        }
        f.write_str(&s)
    }
}

pub fn hex(v: i64) -> String {
    if v < 0 {
        format!("-0x{:X}", v.unsigned_abs())
    } else {
        format!("0x{v:X}")
    }
}

fn de_int<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<i64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum N {
        Num(i64),
        Str(String),
    }
    match N::deserialize(d)? {
        N::Num(n) => Ok(n),
        N::Str(s) => parse_int(&s).ok_or_else(|| serde::de::Error::custom(format!("bad number '{s}'"))),
    }
}

/// Offsets are written as hex strings in trainer files ("0x5578"), matching
/// how Cheat Engine and every guide show them. Plain numbers are accepted too.
mod hex_i64 {
    use super::*;
    pub fn serialize<S: Serializer>(v: &i64, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&hex(*v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<i64, D::Error> {
        de_int(d)
    }
}

mod hex_vec {
    use super::*;
    pub fn serialize<S: Serializer>(v: &[i64], s: S) -> std::result::Result<S::Ok, S::Error> {
        s.collect_seq(v.iter().map(|x| hex(*x)))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<i64>, D::Error> {
        #[derive(Deserialize)]
        struct W(#[serde(deserialize_with = "de_int")] i64);
        Ok(Vec::<W>::deserialize(d)?.into_iter().map(|w| w.0).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeMemory;

    #[test]
    fn resolves_chain() {
        let mut m = FakeMemory::new(8);
        m.add_module("game.exe", 0x400000, 0x1000);
        m.map(0x10000, 0x1000);
        m.put_u64(0x400100, 0x10000); // static -> object
        m.put_u64(0x10020, 0x10800); // object+0x20 -> sub-object
        let p = PointerPath { module: Some("game.exe".into()), base: 0x100, offsets: vec![0x20, 0x8] };
        assert_eq!(p.resolve(&m).unwrap(), 0x10808);
        assert_eq!(p.to_string(), "[[\"game.exe\"+0x100]+0x20]+0x8");
    }

    #[test]
    fn json_hex_roundtrip() {
        let p: PointerPath = serde_json::from_str(r#"{"module":"a.exe","base":"0x10","offsets":["0x8",16,"-0x4"]}"#).unwrap();
        assert_eq!(p.offsets, vec![8, 16, -4]);
        let s = serde_json::to_string(&p).unwrap();
        assert!(s.contains("\"-0x4\""));
        assert_eq!(serde_json::from_str::<PointerPath>(&s).unwrap(), p);
    }
}
