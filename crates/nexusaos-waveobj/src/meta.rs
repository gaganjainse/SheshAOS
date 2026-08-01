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
            let s = item.as_str()?;
            list.push(s.to_string());
        }
        Some(list)
    }

    pub fn get_string_map(&self, key: &str) -> Option<HashMap<String, String>> {
        let obj = self.0.get(key)?.as_object()?;
        let mut map = HashMap::with_capacity(obj.len());
        for (k, v) in obj {
            let s = v.as_str()?;
            map.insert(k.to_string(), s.to_string());
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
        meta.set("float", std::f64::consts::PI);
        meta.set("bool", true);
        meta.set("list", json!(["a", "b", "c"]));
        meta.set("map", json!({"key": "value"}));

        assert_eq!(meta.get_string("str"), Some("hello".to_string()));
        assert_eq!(meta.get_string("int"), None); // Non-string returns None
        
        assert_eq!(meta.get_int("int"), Some(42));
        assert_eq!(meta.get_int("str"), None);

        assert_eq!(meta.get_float("float"), Some(std::f64::consts::PI));
        
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

    #[test]
    fn test_remove_existing_and_nonexistent() {
        let mut meta = MetaMap::new();
        meta.set("a", 1);
        meta.set("b", 2);
        assert!(meta.contains_key("a"));

        meta.remove("a");
        assert!(!meta.contains_key("a"));
        assert!(meta.contains_key("b"));
        assert_eq!(meta.len(), 1);

        // Removing a non-existent key is a no-op
        meta.remove("nonexistent");
        assert_eq!(meta.len(), 1);
        assert!(meta.contains_key("b"));
    }

    #[test]
    fn test_keys_iterator() {
        let mut meta = MetaMap::new();
        meta.set("alpha", 1);
        meta.set("beta", 2);
        meta.set("gamma", 3);

        let mut keys: Vec<&String> = meta.keys().collect();
        keys.sort();
        assert_eq!(keys, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_keys_empty() {
        let meta = MetaMap::new();
        assert_eq!(meta.keys().count(), 0);
    }

    #[test]
    fn test_contains_key() {
        let mut meta = MetaMap::new();
        meta.set("present", "yes");
        assert!(meta.contains_key("present"));
        assert!(!meta.contains_key("absent"));
    }

    #[test]
    fn test_default() {
        let meta = MetaMap::default();
        assert!(meta.is_empty());
        assert_eq!(meta.len(), 0);
    }

    #[test]
    fn test_clone() {
        let mut original = MetaMap::new();
        original.set("key", "value");
        original.set("num", 42);

        let cloned = original.clone();
        assert_eq!(original, cloned);

        // Mutating clone should not affect original
        cloned.remove("key");
        assert!(original.contains_key("key"));
        assert!(!cloned.contains_key("key"));
    }

    #[test]
    fn test_get_int_negative_and_float() {
        let mut meta = MetaMap::new();
        meta.set("neg", -99);
        meta.set("zero", 0);
        meta.set("float", 3.14);
        meta.set("bool", true);

        assert_eq!(meta.get_int("neg"), Some(-99));
        assert_eq!(meta.get_int("zero"), Some(0));
        // Floats are NOT valid integers
        assert_eq!(meta.get_int("float"), None);
        assert_eq!(meta.get_int("bool"), None);
    }

    #[test]
    fn test_get_float_from_int() {
        let mut meta = MetaMap::new();
        meta.set("int_val", 42);
        meta.set("float_val", -3.14);

        // Integer stored as JSON should be retrievable as f64
        assert_eq!(meta.get_float("int_val"), Some(42.0));
        assert_eq!(meta.get_float("float_val"), Some(-3.14));
    }

    #[test]
    fn test_get_bool_false() {
        let mut meta = MetaMap::new();
        meta.set("true_val", true);
        meta.get_string("true_val"); // consume
        meta.set("false_val", false);

        assert_eq!(meta.get_bool("true_val"), Some(true));
        assert_eq!(meta.get_bool("false_val"), Some(false));
    }

    #[test]
    fn test_get_string_list_empty_array() {
        let mut meta = MetaMap::new();
        meta.set("empty_list", json!([]));

        assert_eq!(meta.get_string_list("empty_list"), Some(vec![]));
    }

    #[test]
    fn test_get_string_list_mixed_items() {
        let mut meta = MetaMap::new();
        // Array with non-string items returns None (first non-string causes failure)
        meta.set("mixed", json!(["a", 42, "b"]));

        assert_eq!(meta.get_string_list("mixed"), None);
    }

    #[test]
    fn test_get_string_list_all_non_strings() {
        let mut meta = MetaMap::new();
        meta.set("ints", json!([1, 2, 3]));

        assert_eq!(meta.get_string_list("ints"), None);
    }

    #[test]
    fn test_get_string_list_null_value() {
        let mut meta = MetaMap::new();
        meta.set("null_key", serde_json::Value::Null);

        assert_eq!(meta.get_string_list("null_key"), None);
    }

    #[test]
    fn test_get_string_map_empty_object() {
        let mut meta = MetaMap::new();
        meta.set("empty_map", json!({}));

        assert_eq!(meta.get_string_map("empty_map"), Some(HashMap::new()));
    }

    #[test]
    fn test_get_string_map_non_string_values() {
        let mut meta = MetaMap::new();
        // Object with non-string value returns None
        meta.set("mixed_map", json!({"key": 42}));

        assert_eq!(meta.get_string_map("mixed_map"), None);
    }

    #[test]
    fn test_get_string_map_partial_non_strings() {
        let mut meta = MetaMap::new();
        meta.set("partial_mixed", json!({"a": "ok", "b": 42}));

        assert_eq!(meta.get_string_map("partial_mixed"), None);
    }

    #[test]
    fn test_get_nonexistent_key_returns_none() {
        let meta = MetaMap::new();
        assert_eq!(meta.get_string("missing"), None);
        assert_eq!(meta.get_int("missing"), None);
        assert_eq!(meta.get_float("missing"), None);
        assert_eq!(meta.get_bool("missing"), None);
        assert_eq!(meta.get_string_list("missing"), None);
        assert_eq!(meta.get_string_map("missing"), None);
        assert_eq!(meta.get_string("empty"), None);
    }

    #[test]
    fn test_get_on_key_with_wrong_type() {
        let mut meta = MetaMap::new();
        meta.set("num", 42);
        meta.set("str_val", "hello");
        meta.set("obj", json!({"a": 1}));
        meta.set("arr", json!([1, 2, 3]));

        // Type mismatch returns None
        assert_eq!(meta.get_string("num"), None);
        assert_eq!(meta.get_int("str_val"), None);
        assert_eq!(meta.get_bool("str_val"), None);
        assert_eq!(meta.get_string_map("arr"), None);
        assert_eq!(meta.get_string_list("obj"), None);
    }

    #[test]
    fn test_set_overwrites_existing() {
        let mut meta = MetaMap::new();
        meta.set("key", "first");
        assert_eq!(meta.get_string("key"), Some("first".to_string()));

        meta.set("key", "second");
        assert_eq!(meta.get_string("key"), Some("second".to_string()));
        assert_eq!(meta.len(), 1);
    }

    #[test]
    fn test_set_various_value_types() {
        let mut meta = MetaMap::new();
        meta.set("string", "text");
        meta.set("i64", 9223372036854775807i64);
        meta.set("f64", std::f64::consts::PI);
        meta.set("bool", true);
        meta.set("null", serde_json::Value::Null);
        meta.set("array", json!([1, "two", true]));
        meta.set("object", json!({"inner": "value"}));

        assert_eq!(meta.get_string("string"), Some("text".to_string()));
        assert_eq!(meta.get_int("i64"), Some(9223372036854775807));
        assert_eq!(meta.get_float("f64"), Some(std::f64::consts::PI));
        assert_eq!(meta.get_bool("bool"), Some(true));
        assert_eq!(meta.get_string_list("array"), Some(vec!["two".to_string()]));
    }

    #[test]
    fn test_merge_with_empty_updates() {
        let mut base = MetaMap::new();
        base.set("a", "1");
        base.set("b", "2");

        let updates = MetaMap::new();
        merge_meta(&mut base, &updates);

        // Base should be unchanged
        assert_eq!(base.len(), 2);
        assert_eq!(base.get_string("a"), Some("1".to_string()));
        assert_eq!(base.get_string("b"), Some("2".to_string()));
    }

    #[test]
    fn test_merge_into_empty_base() {
        let mut base = MetaMap::new();

        let mut updates = MetaMap::new();
        updates.set("new", "value");
        updates.set("num", 42);

        merge_meta(&mut base, &updates);

        assert_eq!(base.len(), 2);
        assert_eq!(base.get_string("new"), Some("value".to_string()));
        assert_eq!(base.get_int("num"), Some(42));
    }

    #[test]
    fn test_merge_null_value_not_wildcard() {
        let mut base = MetaMap::new();
        base.set("to_delete", "val");
        base.set("to_keep", "val2");

        let mut updates = MetaMap::new();
        updates.set("to_delete", serde_json::Value::Null);

        merge_meta(&mut base, &updates);

        assert!(!base.contains_key("to_delete"));
        assert!(base.contains_key("to_keep"));
    }

    #[test]
    fn test_merge_wildcard_sets_non_null_value() {
        let mut base = MetaMap::new();
        base.set("ai:model", "gpt-4");

        let mut updates = MetaMap::new();
        // "ai:*" with a non-null value should just SET "ai:*" as a key
        updates.set("ai:*", "overwritten");

        merge_meta(&mut base, &updates);

        assert_eq!(base.get_string("ai:*"), Some("overwritten".to_string()));
        // The actual key "ai:model" should NOT be deleted since the value was not null
        assert_eq!(base.get_string("ai:model"), Some("gpt-4".to_string()));
    }

    #[test]
    fn test_merge_wildcard_no_matching_prefix() {
        let mut base = MetaMap::new();
        base.set("other:key", "val");
        base.set("foo:bar", "val2");

        let mut updates = MetaMap::new();
        updates.set("ai:*", serde_json::Value::Null);

        merge_meta(&mut base, &updates);

        // No keys start with "ai:", so base is unchanged
        assert_eq!(base.len(), 2);
        assert!(base.contains_key("other:key"));
        assert!(base.contains_key("foo:bar"));
    }

    #[test]
    fn test_merge_wildcard_exact_prefix_match() {
        let mut base = MetaMap::new();
        base.set("ai:model", "gpt-4");
        base.set("ai:temp", 0.7);
        base.set("aide:val", "kept"); // starts with "ai" but not "ai:"

        let mut updates = MetaMap::new();
        updates.set("ai:*", serde_json::Value::Null);

        merge_meta(&mut base, &updates);

        assert!(!base.contains_key("ai:model"));
        assert!(!base.contains_key("ai:temp"));
        // "aide:val" should NOT be deleted because it doesn't start with "ai:"
        assert!(base.contains_key("aide:val"));
    }

    #[test]
    fn test_merge_overwrites_existing_keys() {
        let mut base = MetaMap::new();
        base.set("a", "old");
        base.set("b", 1);

        let mut updates = MetaMap::new();
        updates.set("a", "new");
        updates.set("b", 2);

        merge_meta(&mut base, &updates);

        assert_eq!(base.get_string("a"), Some("new".to_string()));
        assert_eq!(base.get_int("b"), Some(2));
    }

    #[test]
    fn test_merge_empty_string_key_delete() {
        let mut base = MetaMap::new();
        base.set("", "val");

        let mut updates = MetaMap::new();
        updates.set("", serde_json::Value::Null);

        merge_meta(&mut base, &updates);

        assert!(!base.contains_key(""));
    }

    #[test]
    fn test_serde_empty_metamap() {
        let meta = MetaMap::new();
        let serialized = serde_json::to_string(&meta).unwrap();
        assert_eq!(serialized, r#"{}"#);

        let deserialized: MetaMap = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, meta);
        assert!(deserialized.is_empty());
    }

    #[test]
    fn test_serde_complex_values() {
        let mut meta = MetaMap::new();
        meta.set("nested", json!({"outer": {"inner": [1, 2, 3]}}));
        meta.set("array_of_objects", json!([{"name": "a"}, {"name": "b"}]));

        let serialized = serde_json::to_string(&meta).unwrap();
        let deserialized: MetaMap = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, meta);
    }

    #[test]
    fn test_serde_invalid_json_fails() {
        let result: Result<MetaMap, _> = serde_json::from_str("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_metamap_partial_eq_with_different_values() {
        let mut m1 = MetaMap::new();
        m1.set("key", "val1");

        let mut m2 = MetaMap::new();
        m2.set("key", "val2");

        assert_ne!(m1, m2);
    }

    #[test]
    fn test_metamap_partial_eq_with_different_lengths() {
        let mut m1 = MetaMap::new();
        m1.set("key", "val");

        let mut m2 = MetaMap::new();
        m2.set("key", "val");
        m2.set("extra", "val2");

        assert_ne!(m1, m2);
    }

    #[test]
    fn test_merge_multiple_keys_same_operation() {
        let mut base = MetaMap::new();
        base.set("x", 1);
        base.set("y", 2);
        base.set("z", 3);

        let mut updates = MetaMap::new();
        updates.set("x", 10);
        updates.set("y", serde_json::Value::Null);
        updates.set("z", 30);

        merge_meta(&mut base, &updates);

        assert_eq!(base.get_int("x"), Some(10));
        assert!(!base.contains_key("y"));
        assert_eq!(base.get_int("z"), Some(30));
        assert_eq!(base.len(), 2);
    }

    #[test]
    fn test_merge_preserves_unrelated_keys() {
        let mut base = MetaMap::new();
        base.set("keep1", "a");
        base.set("keep2", "b");
        base.set("ai:model", "gpt-4");
        base.set("term:theme", "dark");

        let mut updates = MetaMap::new();
        updates.set("ai:*", serde_json::Value::Null);
        updates.set("new", "added");

        merge_meta(&mut base, &updates);

        assert!(base.contains_key("keep1"));
        assert!(base.contains_key("keep2"));
        assert!(base.contains_key("term:theme"));
        assert_eq!(base.get_string("new"), Some("added".to_string()));
        assert_eq!(base.len(), 4);
    }
}
