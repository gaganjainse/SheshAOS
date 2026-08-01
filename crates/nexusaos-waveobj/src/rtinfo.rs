use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

pub const STATUS_INIT: &str = "init";
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_DONE: &str = "done";
pub const STATUS_ERROR: &str = "error";

/// Runtime information for a live Wave object.
/// This is ephemeral (not persisted to SQLite).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, block_id: &str) -> Option<ObjRTInfo> {
        self.data.read().unwrap().get(block_id).cloned()
    }

    pub fn set(&self, info: ObjRTInfo) {
        self.data.write().unwrap().insert(info.block_id.clone(), info);
    }

    pub fn delete(&self, block_id: &str) -> bool {
        self.data.write().unwrap().remove(block_id).is_some()
    }

    pub fn update<F>(&self, block_id: &str, f: F) -> bool
    where
        F: FnOnce(&mut ObjRTInfo),
    {
        let mut guard = self.data.write().unwrap();
        if let Some(info) = guard.get_mut(block_id) {
            f(info);
            true
        } else {
            false
        }
    }

    pub fn get_all(&self) -> Vec<ObjRTInfo> {
        self.data.read().unwrap().values().cloned().collect()
    }

    pub fn clear(&self) {
        self.data.write().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }

    pub fn merge_update(&self, partial: ObjRTInfo) {
        let mut guard = self.data.write().unwrap();
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
}
