use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const EVENT_BLOCK_CLOSE: &str = "blockclose";
pub const EVENT_CONN_CHANGE: &str = "connchange";
pub const EVENT_SYSINFO: &str = "sysinfo";
pub const EVENT_CONTROLLER_STATUS: &str = "controllerstatus";
pub const EVENT_BUILDER_STATUS: &str = "builderstatus";
pub const EVENT_BUILDER_OUTPUT: &str = "builderoutput";
pub const EVENT_WAVEOBJ_UPDATE: &str = "waveobj:update";
pub const EVENT_BLOCK_FILE: &str = "blockfile";
pub const EVENT_BLOCK_UPDATE: &str = "blockupdate";
pub const EVENT_CONFIG: &str = "config";
pub const EVENT_USER_INPUT: &str = "userinput";
pub const EVENT_ROUTE_UP: &str = "route:up";
pub const EVENT_ROUTE_DOWN: &str = "route:down";
pub const EVENT_WORKSPACE_UPDATE: &str = "workspace:update";
pub const EVENT_WAVEAI_RATELIMIT: &str = "waveai:ratelimit";
pub const EVENT_BLOCK_JOB_STATUS: &str = "block:jobstatus";
pub const EVENT_BADGE: &str = "badge";

pub const FILE_OP_APPEND: &str = "append";
pub const FILE_OP_TRUNCATE: &str = "truncate";
pub const FILE_OP_INVALIDATE: &str = "invalidate";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEventData {
    pub zone_id: String,
    pub file_name: String,
    pub file_op: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub topic: String,
    pub scopes: Vec<String>,
}

/// A Wave event — the atomic unit of the pub/sub system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveEvent {
    pub topic: String,
    pub scopes: Vec<String>,
    pub data: serde_json::Value,
    #[serde(default)]
    pub persist: u32,
    pub timestamp: DateTime<Utc>,
    pub event_id: Uuid,
}

impl WaveEvent {
    pub fn new(topic: impl Into<String>, scopes: Vec<String>, data: serde_json::Value) -> Self {
        Self {
            topic: topic.into(),
            scopes,
            data,
            persist: 0,
            timestamp: Utc::now(),
            event_id: Uuid::now_v7(),
        }
    }

    pub fn with_persist(mut self, persist: u32) -> Self {
        self.persist = persist;
        self
    }

    /// Create an event with no scopes (global broadcast)
    pub fn global(topic: impl Into<String>, data: serde_json::Value) -> Self {
        Self::new(topic, vec![], data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_wave_event_creation() {
        let ev = WaveEvent::new("test", vec!["scope1".to_string()], json!({"foo": "bar"}));
        assert_eq!(ev.topic, "test");
        assert_eq!(ev.scopes, vec!["scope1"]);
        assert_eq!(ev.data["foo"], "bar");
        assert_eq!(ev.persist, 0);

        let ev2 = WaveEvent::global("global", json!(1));
        assert!(ev2.scopes.is_empty());
        
        let ev3 = ev2.with_persist(5);
        assert_eq!(ev3.persist, 5);
    }

    #[test]
    fn test_file_event_data_serde() {
        let d = FileEventData {
            zone_id: "z1".to_string(),
            file_name: "f1".to_string(),
            file_op: FILE_OP_APPEND.to_string(),
            data64: Some("dGVzdA==".to_string()),
        };
        let s = serde_json::to_string(&d).unwrap();
        assert!(s.contains("dGVzdA=="));
        let d2: FileEventData = serde_json::from_str(&s).unwrap();
        assert_eq!(d2.data64.unwrap(), "dGVzdA==");
    }
}
