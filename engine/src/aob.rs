//! Array-of-bytes signatures: `"48 8B ?? 05 ?? ?? ?? ??"`. A signature
//! survives game patches as long as the surrounding instructions don't change,
//! which is why patch cheats locate code this way instead of by address.

use crate::error::{EngineError, Result};
use crate::memory::{Memory, MemoryExt, Module};

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    /// `None` is a wildcard byte.
    bytes: Vec<Option<u8>>,
    source: String,
}

impl Pattern {
    pub fn parse(s: &str) -> Result<Self> {
        let bytes = s
            .split_whitespace()
            .map(|tok| match tok {
                "?" | "??" => Ok(None),
                _ => u8::from_str_radix(tok, 16)
                    .map(Some)
                    .map_err(|_| EngineError::Invalid(format!("bad byte '{tok}' in pattern '{s}'"))),
            })
            .collect::<Result<Vec<_>>>()?;
        if bytes.iter().all(Option::is_none) {
            return Err(EngineError::Invalid(format!("pattern '{s}' has no fixed bytes")));
        }
        Ok(Pattern { bytes, source: s.to_string() })
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Offsets of every match in `hay`.
    pub fn find_all(&self, hay: &[u8]) -> Vec<usize> {
        let n = self.bytes.len();
        if hay.len() < n {
            return vec![];
        }
        // Anchor on the first fixed byte so the inner loop rarely runs.
        let (anchor_i, anchor) = self
            .bytes
            .iter()
            .enumerate()
            .find_map(|(i, b)| b.map(|b| (i, b)))
            .expect("pattern has a fixed byte");
        let mut out = Vec::new();
        let last = hay.len() - n;
        let mut i = 0;
        while i <= last {
            match hay[i + anchor_i..=last + anchor_i].iter().position(|&b| b == anchor) {
                None => break,
                Some(p) => i += p,
            }
            if self.matches_at(hay, i) {
                out.push(i);
            }
            i += 1;
        }
        out
    }

    fn matches_at(&self, hay: &[u8], at: usize) -> bool {
        self.bytes
            .iter()
            .zip(&hay[at..at + self.bytes.len()])
            .all(|(p, h)| p.map_or(true, |p| p == *h))
    }
}

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.source)
    }
}

/// Finds a signature inside one module and returns absolute addresses.
pub fn scan_module(mem: &dyn Memory, module: &Module, pattern: &Pattern) -> Result<Vec<u64>> {
    let image = mem.read_module(module)?;
    Ok(pattern.find_all(&image).into_iter().map(|o| module.base + o as u64).collect())
}

/// Parses a plain hex byte string (`"90 90 90"`) with no wildcards.
pub fn parse_bytes(s: &str) -> Result<Vec<u8>> {
    s.split_whitespace()
        .map(|t| u8::from_str_radix(t, 16).map_err(|_| EngineError::Invalid(format!("bad byte '{t}' in '{s}'"))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_with_wildcards() {
        let p = Pattern::parse("8B ?? 05").unwrap();
        let hay = [0x00, 0x8B, 0x11, 0x05, 0x8B, 0x22, 0x06, 0x8B, 0x33, 0x05];
        assert_eq!(p.find_all(&hay), vec![1, 7]);
    }

    #[test]
    fn leading_wildcard() {
        let p = Pattern::parse("?? AA").unwrap();
        assert_eq!(p.find_all(&[0xAA, 0x01, 0xAA]), vec![1]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Pattern::parse("ZZ").is_err());
        assert!(Pattern::parse("?? ??").is_err());
    }
}
