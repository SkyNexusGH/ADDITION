//! Typed values as they live in game memory.

use crate::error::{EngineError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    U8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

impl ValueType {
    pub fn size(self) -> usize {
        match self {
            ValueType::U8 => 1,
            ValueType::I16 => 2,
            ValueType::I32 | ValueType::F32 => 4,
            ValueType::I64 | ValueType::F64 => 8,
        }
    }

    pub fn is_float(self) -> bool {
        matches!(self, ValueType::F32 | ValueType::F64)
    }

    /// Decodes `size()` little-endian bytes. Integers are widened to i64,
    /// floats to f64, so comparisons can work on one representation.
    pub fn decode(self, b: &[u8]) -> Value {
        match self {
            ValueType::U8 => Value::Int(b[0] as i64),
            ValueType::I16 => Value::Int(i16::from_le_bytes([b[0], b[1]]) as i64),
            ValueType::I32 => Value::Int(i32::from_le_bytes(b[..4].try_into().unwrap()) as i64),
            ValueType::I64 => Value::Int(i64::from_le_bytes(b[..8].try_into().unwrap())),
            ValueType::F32 => Value::Float(f32::from_le_bytes(b[..4].try_into().unwrap()) as f64),
            ValueType::F64 => Value::Float(f64::from_le_bytes(b[..8].try_into().unwrap())),
        }
    }

    pub fn encode(self, v: Value) -> Vec<u8> {
        let (i, f) = match v {
            Value::Int(i) => (i, i as f64),
            Value::Float(f) => (f as i64, f),
        };
        match self {
            ValueType::U8 => vec![i as u8],
            ValueType::I16 => (i as i16).to_le_bytes().to_vec(),
            ValueType::I32 => (i as i32).to_le_bytes().to_vec(),
            ValueType::I64 => i.to_le_bytes().to_vec(),
            ValueType::F32 => (f as f32).to_le_bytes().to_vec(),
            ValueType::F64 => f.to_le_bytes().to_vec(),
        }
    }

    /// Parses user input ("100", "-5", "1.5", "0x64") for this type.
    pub fn parse(self, s: &str) -> Result<Value> {
        let s = s.trim();
        let bad = || EngineError::Invalid(format!("'{s}' is not a valid {self:?} value"));
        if self.is_float() {
            s.parse::<f64>().map(Value::Float).map_err(|_| bad())
        } else {
            parse_int(s).map(Value::Int).ok_or_else(bad)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Int(i64),
    Float(f64),
}

impl Value {
    pub fn as_f64(self) -> f64 {
        match self {
            Value::Int(i) => i as f64,
            Value::Float(f) => f,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
        }
    }
}

/// Parses decimal or `0x` hex, with an optional leading `-`.
pub fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim().replace('_', "");
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest.to_string()),
        None => (false, s),
    };
    let v = match body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        Some(hex) => i64::from_str_radix(hex, 16).ok()?,
        None => body.parse::<i64>().ok()?,
    };
    Some(if neg { -v } else { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for t in [ValueType::U8, ValueType::I16, ValueType::I32, ValueType::I64] {
            assert_eq!(t.decode(&t.encode(Value::Int(100))), Value::Int(100));
        }
        assert_eq!(ValueType::F32.decode(&ValueType::F32.encode(Value::Float(1.5))), Value::Float(1.5));
        assert_eq!(ValueType::I32.decode(&ValueType::I32.encode(Value::Int(-7))), Value::Int(-7));
    }

    #[test]
    fn parsing() {
        assert_eq!(parse_int("0x10"), Some(16));
        assert_eq!(parse_int("-0x10"), Some(-16));
        assert_eq!(parse_int("42"), Some(42));
        assert!(ValueType::I32.parse("abc").is_err());
        assert_eq!(ValueType::F32.parse("2.5").unwrap(), Value::Float(2.5));
    }
}
