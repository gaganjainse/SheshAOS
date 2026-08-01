use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A metadata map wrapping serde_json::Map for typed access.
/// This is the Rust equivalent of Wave Terminal's MetaMapType (map[string]any in Go).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MetaMap(pub serde_json::Map<String, serde_json::Value>);

impl MetaMap {
    pub fn new() -> Self {
        Self(serde_json::Map::new())
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.0.get(key).and_then(|v| v.as_i64())
    }

    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.0.get(key).and_then(|v| v.as_f64())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.0.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_string_list(&self, key: &str) -> Option<Vec<String>> {
        let arr = self.0.get(key)?.as_array()?;
        let mut list = Vec::with_capacity(arr.len());
        for item in arr {
            if let Some(s) = item.as_str() {
                list.push(s.to_string());
            } else {
                return None;
            }
        }
        Some(list)
    }

    pub fn get_string_map(&self, key: &str) -> Option<HashMap<String, String>> {
        let obj = self.0.get(key)?.as_object()?;
        let mut map = HashMap::with_capacity(obj.len());
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                map.insert(k.to_string(), s.to_string());
            } else {
                return None;
            }
        }
        Some(map)
    }

    pub fn set<V: Into<serde_json::Value>>(&mut self, key: impl Into<String>, value: V) {
        self.0.insert(key.into(), value.into());
    }

    pub fn remove(&mut self, key: &str) {
        self.0.remove(key);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
}

/// Merge updates into a base MetaMap following Wave Terminal's merge rules:
/// 1. If a key in `updates` has a JSON null value, DELETE that key from `base`
/// 2. If a key in `updates` ends with `:*` and has a null value, DELETE ALL keys
///    from `base` that start with that prefix (section wildcard reset)
///    e.g. "ai:*" = null deletes "ai:model", "ai:temperature", etc.
/// 3. Otherwise, SET the key in `base` to the value from `updates`
pub fn merge_meta(base: &mut MetaMap, updates: &MetaMap) {
    for (k, v) in &updates.0 {
        if v.is_null() {
            if k.ends_with(":*") {
                let prefix = &k[0..k.len() - 1]; // Includes the ':'
                base.0.retain(|base_k, _| !base_k.starts_with(prefix));
            } else {
                base.remove(k);
            }
        } else {
            base.set(k.clone(), v.clone());
        }
    }
}

pub const META_KEY_VIEW: &str = "view";
pub const META_KEY_CONTROLLER: &str = "controller";
pub const META_KEY_CONNECTION: &str = "connection";
pub const META_KEY_CMD: &str = "cmd";
pub const META_KEY_CMD_ENV: &str = "cmd:env";
pub const META_KEY_TERM_FONT_SIZE: &str = "term:fontsize";
pub const META_KEY_TERM_FONT_FAMILY: &str = "term:fontfamily";
pub const META_KEY_TERM_THEME: &str = "term:theme";
pub const META_KEY_TERM_LOCAL_SHELL_PATH: &str = "term:localshellpath";
pub const META_KEY_TERM_SCROLL_BACK: &str = "term:scrollback";
pub const META_KEY_AI_MODEL: &str = "ai:model";
pub const META_KEY_AI_MAXTOKENS: &str = "ai:maxtokens";
pub const META_KEY_AI_BASE_URL: &str = "ai:baseurl";
pub const META_KEY_AI_API_TOKEN: &str = "ai:apitoken";
pub const META_KEY_EDITOR_MINIMAP: &str = "editor:minimap";
pub const META_KEY_EDITOR_WORD_WRAP: &str = "editor:wordwrap";
pub const META_KEY_WEB_URL: &str = "web:url";
pub const META_KEY_BG_COLOR: &str = "bg";
pub const META_KEY_ICON: &str = "icon";
pub const META_KEY_ICON_COLOR: &str = "icon:color";
pub const META_KEY_FRAME: &str = "frame";
pub const META_KEY_FRAME_CLEAR: &str = "frame:*";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_metamap_new_and_empty() {
        let mut meta = MetaMap::new();
        assert!(meta.is_empty());
        assert_eq!(meta.len(), 0);
        
        meta.set("test", "value");
        assert!(!meta.is_empty());
        assert_eq!(meta.len(), 1);
    }

    #[test]
    fn test_typed_getters() {
        let mut meta = MetaMap::new();
        meta.set("str", "hello");
        meta.set("int", 42);
        meta.set("float", 3.14);
        meta.set("bool", true);
        meta.set("list", json!(["a", "b", "c"]));
        meta.set("map", json!({"key": "value"}));

        assert_eq!(meta.get_string("str"), Some("hello".to_string()));
        assert_eq!(meta.get_string("int"), None); // Non-string returns None
        
        assert_eq!(meta.get_int("int"), Some(42));
        assert_eq!(meta.get_int("str"), None);

        assert_eq!(meta.get_float("float"), Some(3.14));
        
        assert_eq!(meta.get_bool("bool"), Some(true));
        assert_eq!(meta.get_bool("int"), None);

        assert_eq!(meta.get_string_list("list"), Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]));
        assert_eq!(meta.get_string_list("str"), None);

        let mut expected_map = HashMap::new();
        expected_map.insert("key".to_string(), "value".to_string());
        assert_eq!(meta.get_string_map("map"), Some(expected_map));
        assert_eq!(meta.get_string_map("str"), None);
    }

    #[test]
    fn test_merge_meta_simple_set() {
        let mut base = MetaMap::new();
        base.set("a", "1");
        
        let mut updates = MetaMap::new();
        updates.set("a", "2");
        updates.set("b", "3");

        merge_meta(&mut base, &updates);
        assert_eq!(base.get_string("a").unwrap(), "2");
        assert_eq!(base.get_string("b").unwrap(), "3");
    }

    #[test]
    fn test_merge_meta_delete() {
        let mut base = MetaMap::new();
        base.set("a", "1");
        base.set("b", "2");
        
        let mut updates = MetaMap::new();
        updates.set("a", serde_json::Value::Null);

        merge_meta(&mut base, &updates);
        assert!(!base.contains_key("a"));
        assert!(base.contains_key("b"));
    }

    #[test]
    fn test_merge_meta_wildcard_delete() {
        let mut base = MetaMap::new();
        base.set("ai:model", "gpt-4");
        base.set("ai:maxtokens", 1000);
        base.set("bg", "red");
        
        let mut updates = MetaMap::new();
        updates.set("ai:*", serde_json::Value::Null);

        merge_meta(&mut base, &updates);
        assert!(!base.contains_key("ai:model"));
        assert!(!base.contains_key("ai:maxtokens"));
        assert!(base.contains_key("bg"));
    }

    #[test]
    fn test_merge_meta_mixed() {
        let mut base = MetaMap::new();
        base.set("ai:model", "gpt-4");
        base.set("ai:maxtokens", 1000);
        base.set("term:theme", "dark");
        base.set("keep", "me");
        
        let mut updates = MetaMap::new();
        updates.set("ai:*", serde_json::Value::Null);
        updates.set("term:theme", serde_json::Value::Null);
        updates.set("new_key", "hello");

        merge_meta(&mut base, &updates);
        assert!(!base.contains_key("ai:model"));
        assert!(!base.contains_key("ai:maxtokens"));
        assert!(!base.contains_key("term:theme"));
        assert!(base.contains_key("keep"));
        assert_eq!(base.get_string("new_key"), Some("hello".to_string()));
    }

    #[test]
    fn test_serde() {
        let mut meta = MetaMap::new();
        meta.set("key", "value");
        let serialized = serde_json::to_string(&meta).unwrap();
        assert_eq!(serialized, r#"{"key":"value"}"#);
        
        let deserialized: MetaMap = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, meta);
    }
}
