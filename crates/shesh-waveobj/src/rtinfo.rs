use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

pub const STATUS_INIT: &str = "init";
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_DONE: &str = "done";
pub const STATUS_ERROR: &str = "error";

/// Runtime information for a live Wave object.
/// This is ephemeral (not persisted to SQLite).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjRTInfo {
    /// Block ID this info belongs to
    pub block_id: String,

    /// Shell process status: "running", "done", "init", "error"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_proc_status: Option<String>,

    /// Connection name for the shell process
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_proc_conn_name: Option<String>,

    /// Shell process exit code (if completed)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_proc_exit_code: Option<i32>,

    /// Tsunami app port (for web app blocks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tsunami_port: Option<u16>,

    /// Wave AI chat status
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wave_ai_status: Option<String>,

    /// Builder status
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builder_status: Option<String>,

    /// Extra metadata (extensible)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Thread-safe in-memory store for runtime object info.
#[derive(Debug, Default)]
pub struct RTInfoStore {
    data: RwLock<HashMap<String, ObjRTInfo>>,
}

impl RTInfoStore {
    pub fn new() -> Self {
        Self { data: RwLock::new(HashMap::new()) }
    }

    pub fn get(&self, block_id: &str) -> Option<ObjRTInfo> {
        self.data.read().unwrap_or_else(|e| e.into_inner()).get(block_id).cloned()
    }

    pub fn set(&self, info: ObjRTInfo) {
        self.data.write().unwrap_or_else(|e| e.into_inner()).insert(info.block_id.clone(), info);
    }

    pub fn delete(&self, block_id: &str) -> bool {
        self.data.write().unwrap_or_else(|e| e.into_inner()).remove(block_id).is_some()
    }

    pub fn update<F>(&self, block_id: &str, f: F) -> bool
    where
        F: FnOnce(&mut ObjRTInfo),
    {
        let mut guard = self.data.write().unwrap_or_else(|e| e.into_inner());
        if let Some(info) = guard.get_mut(block_id) {
            f(info);
            true
        } else {
            false
        }
    }

    pub fn get_all(&self) -> Vec<ObjRTInfo> {
        self.data.read().unwrap_or_else(|e| e.into_inner()).values().cloned().collect()
    }

    pub fn clear(&self) {
        self.data.write().unwrap_or_else(|e| e.into_inner()).clear();
    }

    pub fn len(&self) -> usize {
        self.data.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap_or_else(|e| e.into_inner()).is_empty()
    }

    pub fn merge_update(&self, partial: ObjRTInfo) {
        let mut guard = self.data.write().unwrap_or_else(|e| e.into_inner());
        let existing = guard.get_mut(&partial.block_id);

        match existing {
            Some(existing) => {
                if partial.shell_proc_status.is_some() {
                    existing.shell_proc_status = partial.shell_proc_status;
                }
                if partial.shell_proc_conn_name.is_some() {
                    existing.shell_proc_conn_name = partial.shell_proc_conn_name;
                }
                if partial.shell_proc_exit_code.is_some() {
                    existing.shell_proc_exit_code = partial.shell_proc_exit_code;
                }
                if partial.tsunami_port.is_some() {
                    existing.tsunami_port = partial.tsunami_port;
                }
                if partial.wave_ai_status.is_some() {
                    existing.wave_ai_status = partial.wave_ai_status;
                }
                if partial.builder_status.is_some() {
                    existing.builder_status = partial.builder_status;
                }
                for (k, v) in partial.extra {
                    existing.extra.insert(k, v);
                }
            }
            None => {
                guard.insert(partial.block_id.clone(), partial);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_status_constants() {
        assert_eq!(STATUS_INIT, "init");
        assert_eq!(STATUS_RUNNING, "running");
        assert_eq!(STATUS_DONE, "done");
        assert_eq!(STATUS_ERROR, "error");
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut extra = HashMap::new();
        extra.insert("key1".to_string(), serde_json::json!("value1"));

        let info = ObjRTInfo {
            block_id: "block_1".to_string(),
            shell_proc_status: Some(STATUS_RUNNING.to_string()),
            shell_proc_conn_name: Some("local".to_string()),
            shell_proc_exit_code: None,
            tsunami_port: Some(8080),
            wave_ai_status: None,
            builder_status: Some("building".to_string()),
            extra,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ObjRTInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.block_id, info.block_id);
        assert_eq!(deserialized.shell_proc_status, info.shell_proc_status);
        assert_eq!(deserialized.tsunami_port, info.tsunami_port);
        assert_eq!(deserialized.extra.get("key1"), Some(&serde_json::json!("value1")));
    }

    #[test]
    fn test_store_basic_operations() {
        let store = RTInfoStore::new();

        let info = ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_INIT.to_string()),
            ..Default::default()
        };

        // set and get
        store.set(info.clone());
        let retrieved = store.get("b1").unwrap();
        assert_eq!(retrieved.block_id, "b1");
        assert_eq!(retrieved.shell_proc_status, Some(STATUS_INIT.to_string()));

        // update
        let updated = store.update("b1", |i| {
            i.shell_proc_status = Some(STATUS_RUNNING.to_string());
        });
        assert!(updated);
        let retrieved2 = store.get("b1").unwrap();
        assert_eq!(retrieved2.shell_proc_status, Some(STATUS_RUNNING.to_string()));

        // get_all, len, is_empty
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
        let all = store.get_all();
        assert_eq!(all.len(), 1);

        // delete
        assert!(store.delete("b1"));
        assert!(!store.delete("b1"));
        assert!(store.get("b1").is_none());

        // clear
        store.set(info);
        store.clear();
        assert!(store.is_empty());
    }

    #[test]
    fn test_merge_update() {
        let store = RTInfoStore::new();

        // 5. merge_update: new entry is inserted
        let info = ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_INIT.to_string()),
            ..Default::default()
        };
        store.merge_update(info.clone());

        let retrieved = store.get("b1").unwrap();
        assert_eq!(retrieved.shell_proc_status, Some(STATUS_INIT.to_string()));
        assert_eq!(retrieved.shell_proc_conn_name, None);

        // 4. merge_update: partial update doesn't overwrite existing fields
        let partial = ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_conn_name: Some("local".to_string()),
            ..Default::default()
        };
        store.merge_update(partial);

        let retrieved2 = store.get("b1").unwrap();
        // The original shell_proc_status should remain
        assert_eq!(retrieved2.shell_proc_status, Some(STATUS_INIT.to_string()));
        // The new field should be set
        assert_eq!(retrieved2.shell_proc_conn_name, Some("local".to_string()));
    }

    #[test]
    fn test_thread_safety() {
        let store = Arc::new(RTInfoStore::new());
        let mut handles = vec![];

        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                let block_id = format!("block_{}", i);
                let info = ObjRTInfo {
                    block_id: block_id.clone(),
                    shell_proc_status: Some(STATUS_INIT.to_string()),
                    ..Default::default()
                };

                // Write
                store_clone.set(info);

                // Update
                store_clone.update(&block_id, |i| {
                    i.shell_proc_status = Some(STATUS_RUNNING.to_string());
                });

                // Read
                let _ = store_clone.get(&block_id);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(store.len(), 10);
    }

    #[test]
    fn test_get_nonexistent() {
        let store = RTInfoStore::new();
        assert!(store.get("nonexistent").is_none());
        assert!(store.get("").is_none());
    }

    #[test]
    fn test_get_all_empty_store() {
        let store = RTInfoStore::new();
        let all = store.get_all();
        assert!(all.is_empty());
    }

    #[test]
    fn test_len_and_is_empty() {
        let store = RTInfoStore::new();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());

        let info = ObjRTInfo { block_id: "b1".to_string(), ..Default::default() };
        store.set(info);
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
    }

    #[test]
    fn test_clear_empty_store() {
        let store = RTInfoStore::new();
        store.clear();
        assert!(store.is_empty());
    }

    #[test]
    fn test_clear_after_set() {
        let store = RTInfoStore::new();
        for i in 0..5 {
            store.set(ObjRTInfo { block_id: format!("b{}", i), ..Default::default() });
        }
        assert_eq!(store.len(), 5);
        store.clear();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let store = RTInfoStore::new();
        assert!(!store.delete("nonexistent"));
        assert!(!store.delete(""));
    }

    #[test]
    fn test_update_nonexistent() {
        let store = RTInfoStore::new();
        let result = store.update("nonexistent", |i| {
            i.shell_proc_status = Some(STATUS_RUNNING.to_string());
        });
        assert!(!result);
    }

    #[test]
    fn test_update_multiple_calls() {
        let store = RTInfoStore::new();
        store.set(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_INIT.to_string()),
            ..Default::default()
        });

        // First update succeeds
        let ok = store.update("b1", |i| {
            i.shell_proc_status = Some(STATUS_RUNNING.to_string());
        });
        assert!(ok);

        // Second update also succeeds (re-inserting doesn't happen, it's the same key)
        let ok2 = store.update("b1", |i| {
            i.shell_proc_status = Some(STATUS_DONE.to_string());
        });
        assert!(ok2);

        let retrieved = store.get("b1").unwrap();
        assert_eq!(retrieved.shell_proc_status, Some(STATUS_DONE.to_string()));
    }

    #[test]
    fn test_set_overwrite() {
        let store = RTInfoStore::new();
        store.set(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_INIT.to_string()),
            ..Default::default()
        });

        // Setting again with same block_id overwrites the old entry
        store.set(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_RUNNING.to_string()),
            tsunami_port: Some(3000),
            ..Default::default()
        });

        let retrieved = store.get("b1").unwrap();
        assert_eq!(retrieved.shell_proc_status, Some(STATUS_RUNNING.to_string()));
        assert_eq!(retrieved.tsunami_port, Some(3000));
        assert_eq!(retrieved.shell_proc_conn_name, None);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_objrtinfo_default() {
        let info = ObjRTInfo::default();
        assert_eq!(info.block_id, String::new());
        assert!(info.shell_proc_status.is_none());
        assert!(info.shell_proc_conn_name.is_none());
        assert!(info.shell_proc_exit_code.is_none());
        assert!(info.tsunami_port.is_none());
        assert!(info.wave_ai_status.is_none());
        assert!(info.builder_status.is_none());
        assert!(info.extra.is_empty());
    }

    #[test]
    fn test_rtinfostore_default() {
        let store = RTInfoStore::default();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_merge_update_overwrites_all_fields() {
        let store = RTInfoStore::new();

        store.set(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_INIT.to_string()),
            shell_proc_conn_name: Some("old_conn".to_string()),
            shell_proc_exit_code: Some(0),
            tsunami_port: Some(8080),
            wave_ai_status: Some("idle".to_string()),
            builder_status: Some("done".to_string()),
            extra: HashMap::new(),
        });

        // Merge partial with all Some fields - should overwrite all
        store.merge_update(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_RUNNING.to_string()),
            shell_proc_conn_name: Some("new_conn".to_string()),
            shell_proc_exit_code: Some(42),
            tsunami_port: Some(3000),
            wave_ai_status: Some("busy".to_string()),
            builder_status: Some("building".to_string()),
            extra: HashMap::new(),
        });

        let retrieved = store.get("b1").unwrap();
        assert_eq!(retrieved.shell_proc_status, Some(STATUS_RUNNING.to_string()));
        assert_eq!(retrieved.shell_proc_conn_name, Some("new_conn".to_string()));
        assert_eq!(retrieved.shell_proc_exit_code, Some(42));
        assert_eq!(retrieved.tsunami_port, Some(3000));
        assert_eq!(retrieved.wave_ai_status, Some("busy".to_string()));
        assert_eq!(retrieved.builder_status, Some("building".to_string()));
    }

    #[test]
    fn test_merge_update_preserves_none_fields() {
        let store = RTInfoStore::new();

        store.set(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_RUNNING.to_string()),
            shell_proc_conn_name: Some("conn1".to_string()),
            shell_proc_exit_code: Some(0),
            ..Default::default()
        });

        // Merge partial with None for some fields - existing values should be preserved
        store.merge_update(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: None,
            shell_proc_conn_name: Some("conn2".to_string()),
            shell_proc_exit_code: None,
            ..Default::default()
        });

        let retrieved = store.get("b1").unwrap();
        // None fields preserve original
        assert_eq!(retrieved.shell_proc_status, Some(STATUS_RUNNING.to_string()));
        assert_eq!(retrieved.shell_proc_exit_code, Some(0));
        // Some fields get updated
        assert_eq!(retrieved.shell_proc_conn_name, Some("conn2".to_string()));
    }

    #[test]
    fn test_merge_update_extra_hashmap() {
        let store = RTInfoStore::new();

        let mut initial_extra = HashMap::new();
        initial_extra.insert("key1".to_string(), serde_json::json!("val1"));
        initial_extra.insert("key2".to_string(), serde_json::json!("val2"));

        store.set(ObjRTInfo {
            block_id: "b1".to_string(),
            extra: initial_extra.clone(),
            ..Default::default()
        });

        // Merge partial with new extra entries
        let mut new_extra = HashMap::new();
        new_extra.insert("key3".to_string(), serde_json::json!("val3"));
        new_extra.insert("key1".to_string(), serde_json::json!("overwritten"));

        store.merge_update(ObjRTInfo {
            block_id: "b1".to_string(),
            extra: new_extra,
            ..Default::default()
        });

        let retrieved = store.get("b1").unwrap();
        // key1 overwritten
        assert_eq!(retrieved.extra.get("key1"), Some(&serde_json::json!("overwritten")));
        // key2 preserved
        assert_eq!(retrieved.extra.get("key2"), Some(&serde_json::json!("val2")));
        // key3 added
        assert_eq!(retrieved.extra.get("key3"), Some(&serde_json::json!("val3")));
    }

    #[test]
    fn test_merge_update_inserts_new_entry() {
        let store = RTInfoStore::new();
        assert!(store.is_empty());

        store.merge_update(ObjRTInfo {
            block_id: "new_block".to_string(),
            shell_proc_status: Some(STATUS_INIT.to_string()),
            ..Default::default()
        });

        assert_eq!(store.len(), 1);
        let retrieved = store.get("new_block").unwrap();
        assert_eq!(retrieved.block_id, "new_block");
        assert_eq!(retrieved.shell_proc_status, Some(STATUS_INIT.to_string()));
    }

    #[test]
    fn test_merge_update_empty_extra_no_change() {
        let store = RTInfoStore::new();
        store.set(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_INIT.to_string()),
            ..Default::default()
        });

        store.merge_update(ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_RUNNING.to_string()),
            extra: HashMap::new(),
            ..Default::default()
        });

        let retrieved = store.get("b1").unwrap();
        assert!(retrieved.extra.is_empty());
    }

    #[test]
    fn test_serde_objrtinfo_all_none() {
        let info = ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: None,
            shell_proc_conn_name: None,
            shell_proc_exit_code: None,
            tsunami_port: None,
            wave_ai_status: None,
            builder_status: None,
            extra: HashMap::new(),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ObjRTInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.block_id, "b1");
        assert!(deserialized.shell_proc_status.is_none());
        assert!(deserialized.tsunami_port.is_none());
        assert!(deserialized.extra.is_empty());
    }

    #[test]
    fn test_serde_objrtinfo_all_fields() {
        let mut extra = HashMap::new();
        extra.insert("key1".to_string(), serde_json::json!("val1"));
        extra.insert("key2".to_string(), serde_json::json!(42));

        let info = ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_RUNNING.to_string()),
            shell_proc_conn_name: Some("local".to_string()),
            shell_proc_exit_code: Some(0),
            tsunami_port: Some(8080),
            wave_ai_status: Some("idle".to_string()),
            builder_status: Some("building".to_string()),
            extra,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ObjRTInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.block_id, info.block_id);
        assert_eq!(deserialized.shell_proc_status, info.shell_proc_status);
        assert_eq!(deserialized.shell_proc_conn_name, info.shell_proc_conn_name);
        assert_eq!(deserialized.shell_proc_exit_code, info.shell_proc_exit_code);
        assert_eq!(deserialized.tsunami_port, info.tsunami_port);
        assert_eq!(deserialized.wave_ai_status, info.wave_ai_status);
        assert_eq!(deserialized.builder_status, info.builder_status);
        assert_eq!(deserialized.extra.get("key1"), Some(&serde_json::json!("val1")));
        assert_eq!(deserialized.extra.get("key2"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_serde_skip_serializing_if_none() {
        let info = ObjRTInfo { block_id: "b1".to_string(), ..Default::default() };

        let json = serde_json::to_value(&info).unwrap();
        // None fields should be skipped
        assert!(json.get("shell_proc_status").is_none());
        assert!(json.get("tsunami_port").is_none());
        assert!(json.get("builder_status").is_none());
        // block_id (no skip) should be present
        assert_eq!(json["block_id"], "b1");
    }

    #[test]
    fn test_serde_extra_empty_skipped() {
        let info = ObjRTInfo { block_id: "b1".to_string(), ..Default::default() };

        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("extra").is_none());
    }

    #[test]
    fn test_serde_extra_present_when_non_empty() {
        let mut extra = HashMap::new();
        extra.insert("key".to_string(), serde_json::json!("val"));

        let info = ObjRTInfo { block_id: "b1".to_string(), extra, ..Default::default() };

        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("extra").is_some());
    }

    #[test]
    fn test_get_all_multiple_entries() {
        let store = RTInfoStore::new();
        for i in 0..5 {
            store.set(ObjRTInfo {
                block_id: format!("block_{}", i),
                shell_proc_status: Some(STATUS_INIT.to_string()),
                ..Default::default()
            });
        }

        let all = store.get_all();
        assert_eq!(all.len(), 5);

        let mut ids: Vec<String> = all.iter().map(|i| i.block_id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["block_0", "block_1", "block_2", "block_3", "block_4"]);
    }

    #[test]
    fn test_clone_objrtinfo() {
        let mut extra = HashMap::new();
        extra.insert("key".to_string(), serde_json::json!("val"));

        let info = ObjRTInfo {
            block_id: "b1".to_string(),
            shell_proc_status: Some(STATUS_RUNNING.to_string()),
            shell_proc_conn_name: Some("conn-1".to_string()),
            shell_proc_exit_code: Some(0),
            tsunami_port: Some(8080),
            wave_ai_status: Some("idle".to_string()),
            builder_status: Some("success".to_string()),
            extra,
        };

        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn test_concurrent_get_and_set() {
        let store = Arc::new(RTInfoStore::new());
        let mut handles = vec![];

        for i in 0..20 {
            let store_clone = Arc::clone(&store);
            let op = i % 2;
            handles.push(thread::spawn(move || {
                let block_id = format!("block_{}", i);
                if op == 0 {
                    store_clone.set(ObjRTInfo {
                        block_id: block_id.clone(),
                        shell_proc_status: Some(STATUS_INIT.to_string()),
                        ..Default::default()
                    });
                } else {
                    store_clone.merge_update(ObjRTInfo {
                        block_id: block_id.clone(),
                        shell_proc_status: Some(STATUS_RUNNING.to_string()),
                        ..Default::default()
                    });
                }
                let _ = store_clone.get(&block_id);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(store.len(), 20);
    }

    #[test]
    fn test_concurrent_delete() {
        let store = Arc::new(RTInfoStore::new());

        for i in 0..10 {
            store.set(ObjRTInfo { block_id: format!("block_{}", i), ..Default::default() });
        }

        let mut handles = vec![];
        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                store_clone.delete(&format!("block_{}", i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(store.len(), 0);
    }
}
