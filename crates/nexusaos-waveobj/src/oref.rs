use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

pub const OTYPE_CLIENT: &str = "client";
pub const OTYPE_WINDOW: &str = "window";
pub const OTYPE_WORKSPACE: &str = "workspace";
pub const OTYPE_TAB: &str = "tab";
pub const OTYPE_LAYOUT: &str = "layout";
pub const OTYPE_BLOCK: &str = "block";
pub const OTYPE_MAINSERVER: &str = "mainserver";
pub const OTYPE_JOB: &str = "job";
pub const OTYPE_TEMP: &str = "temp";

pub const VALID_OTYPES: &[&str] = &[
    OTYPE_CLIENT,
    OTYPE_WINDOW,
    OTYPE_WORKSPACE,
    OTYPE_TAB,
    OTYPE_LAYOUT,
    OTYPE_BLOCK,
    OTYPE_MAINSERVER,
    OTYPE_JOB,
    OTYPE_TEMP,
];

pub fn is_valid_otype(otype: &str) -> bool {
    VALID_OTYPES.contains(&otype)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ORefError {
    #[error("invalid ORef format: expected 'otype:oid', got '{0}'")]
    InvalidFormat(String),
    #[error("invalid object type: '{0}'")]
    InvalidOType(String),
    #[error("invalid OID (not a valid UUID): '{0}'")]
    InvalidOid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ORef {
    pub otype: String,
    pub oid: Uuid,
}

impl ORef {
    pub fn new(otype: String, oid: Uuid) -> Result<Self, ORefError> {
        if !otype.chars().all(|c| c.is_ascii_lowercase()) || !is_valid_otype(&otype) {
            return Err(ORefError::InvalidOType(otype));
        }
        Ok(Self { otype, oid })
    }

    pub fn parse(s: &str) -> Result<Self, ORefError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(ORefError::InvalidFormat(s.to_string()));
        }

        let otype = parts[0].to_string();
        if !otype.chars().all(|c| c.is_ascii_lowercase()) || !is_valid_otype(&otype) {
            return Err(ORefError::InvalidOType(otype));
        }

        let oid_str = parts[1];
        let oid = Uuid::parse_str(oid_str).map_err(|_| ORefError::InvalidOid(oid_str.to_string()))?;

        Ok(Self { otype, oid })
    }
}

impl fmt::Display for ORef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.otype, self.oid)
    }
}

impl FromStr for ORef {
    type Err = ORefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ORef::parse(s)
    }
}

impl Serialize for ORef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ORef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ORef::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[test]
    fn test_parse_valid() {
        let oid = Uuid::new_v4();
        let s = format!("block:{}", oid);
        let oref = ORef::parse(&s).unwrap();
        assert_eq!(oref.otype, "block");
        assert_eq!(oref.oid, oid);
    }

    #[test]
    fn test_parse_invalid_format() {
        assert!(matches!(ORef::parse("block"), Err(ORefError::InvalidFormat(_))));
        assert!(matches!(ORef::parse("block:uuid:extra"), Err(ORefError::InvalidFormat(_))));
        assert!(matches!(ORef::parse(""), Err(ORefError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_invalid_otype() {
        assert!(matches!(ORef::parse("Block:00000000-0000-0000-0000-000000000000"), Err(ORefError::InvalidOType(_))));
        assert!(matches!(ORef::parse("unknown:00000000-0000-0000-0000-000000000000"), Err(ORefError::InvalidOType(_))));
        assert!(matches!(ORef::parse("bl0ck:00000000-0000-0000-0000-000000000000"), Err(ORefError::InvalidOType(_))));
    }

    #[test]
    fn test_parse_invalid_oid() {
        assert!(matches!(ORef::parse("block:invalid-uuid"), Err(ORefError::InvalidOid(_))));
    }

    #[test]
    fn test_display_round_trip() {
        let oid = Uuid::new_v4();
        let original = ORef::new("workspace".to_string(), oid).unwrap();
        let s = original.to_string();
        let parsed = ORef::parse(&s).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_serde_json() {
        let oid = Uuid::new_v4();
        let original = ORef::new("tab".to_string(), oid).unwrap();
        
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, format!("\"tab:{}\"", oid));
        
        let deserialized: ORef = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_hash_eq() {
        let oid1 = Uuid::new_v4();
        let oid2 = Uuid::new_v4();
        
        let oref1 = ORef::new("block".to_string(), oid1).unwrap();
        let oref2 = ORef::new("block".to_string(), oid1).unwrap();
        let oref3 = ORef::new("block".to_string(), oid2).unwrap();
        let oref4 = ORef::new("window".to_string(), oid1).unwrap();

        assert_eq!(oref1, oref2);
        assert_ne!(oref1, oref3);
        assert_ne!(oref1, oref4);

        let mut hasher1 = DefaultHasher::new();
        oref1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        oref2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }
    
    #[test]
    fn test_all_valid_otypes() {
        let oid = Uuid::new_v4();
        for &otype in VALID_OTYPES {
            let oref = ORef::new(otype.to_string(), oid);
            assert!(oref.is_ok(), "Failed for otype: {}", otype);
        }
    }
}
