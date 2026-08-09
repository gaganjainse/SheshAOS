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
    fn test_wave_event_new_with_string_topic() {
        let topic = String::from("my_topic");
        let ev = WaveEvent::new(topic.clone(), vec![], json!(null));
        assert_eq!(ev.topic, topic);
        assert_eq!(ev.persist, 0);
        assert!(!ev.event_id.is_nil());
        assert!(ev.timestamp <= Utc::now());
    }

    #[test]
    fn test_wave_event_new_with_multiple_scopes() {
        let scopes = vec!["scope1".into(), "scope2".into(), "scope3".into()];
        let ev = WaveEvent::new("topic", scopes.clone(), json!({}));
        assert_eq!(ev.scopes, scopes);
    }

    #[test]
    fn test_wave_event_global_creates_empty_scopes() {
        let ev = WaveEvent::global("topic", json!({"key": "value"}));
        assert!(ev.scopes.is_empty());
        assert_eq!(ev.topic, "topic");
        assert_eq!(ev.data["key"], "value");
        assert_eq!(ev.persist, 0);
    }

    #[test]
    fn test_wave_event_with_persist_zero() {
        let ev = WaveEvent::new("topic", vec![], json!(1)).with_persist(0);
        assert_eq!(ev.persist, 0);
    }

    #[test]
    fn test_wave_event_with_persist_max_u32() {
        let ev = WaveEvent::new("topic", vec![], json!(1)).with_persist(u32::MAX);
        assert_eq!(ev.persist, u32::MAX);
    }

    #[test]
    fn test_wave_event_with_persist_chaining() {
        let ev = WaveEvent::new("topic", vec![], json!(1))
            .with_persist(1)
            .with_persist(2)
            .with_persist(3);
        assert_eq!(ev.persist, 3);
    }

    #[test]
    fn test_wave_event_serde_roundtrip() {
        let ev = WaveEvent::new("test", vec!["scope1".to_string()], json!({"nested": {"a": 1}}))
            .with_persist(42);
        let json_str = serde_json::to_string(&ev).unwrap();
        let decoded: WaveEvent = serde_json::from_str(&json_str).unwrap();
        assert_eq!(decoded.topic, ev.topic);
        assert_eq!(decoded.scopes, ev.scopes);
        assert_eq!(decoded.data, ev.data);
        assert_eq!(decoded.persist, 42);
        assert_eq!(decoded.event_id, ev.event_id);
    }

    #[test]
    fn test_wave_event_event_id_is_unique() {
        let ev1 = WaveEvent::new("topic", vec![], json!(1));
        let ev2 = WaveEvent::new("topic", vec![], json!(1));
        assert_ne!(ev1.event_id, ev2.event_id);
    }

    #[test]
    fn test_wave_event_timestamps_close() {
        let ev1 = WaveEvent::new("topic", vec![], json!(1));
        let ev2 = WaveEvent::new("topic", vec![], json!(1));
        assert!(ev2.timestamp >= ev1.timestamp);
    }

    #[test]
    fn test_file_event_data_with_none_data64() {
        let d = FileEventData {
            zone_id: "z1".to_string(),
            file_name: "f1".to_string(),
            file_op: FILE_OP_APPEND.to_string(),
            data64: None,
        };
        let s = serde_json::to_string(&d).unwrap();
        assert!(!s.contains("data64"));
        let d2: FileEventData = serde_json::from_str(&s).unwrap();
        assert!(d2.data64.is_none());
    }

    #[test]
    fn test_file_event_data_all_operations() {
        for op in [FILE_OP_APPEND, FILE_OP_TRUNCATE, FILE_OP_INVALIDATE] {
            let d = FileEventData {
                zone_id: "z1".to_string(),
                file_name: "f1".to_string(),
                file_op: op.to_string(),
                data64: None,
            };
            assert_eq!(d.file_op, op);
            let s = serde_json::to_string(&d).unwrap();
            let d2: FileEventData = serde_json::from_str(&s).unwrap();
            assert_eq!(d2.file_op, op);
        }
    }

    #[test]
    fn test_file_event_data_serde_with_all_fields() {
        let d = FileEventData {
            zone_id: "zone".to_string(),
            file_name: "file.txt".to_string(),
            file_op: FILE_OP_TRUNCATE.to_string(),
            data64: Some("SGVsbG8=".to_string()),
        };
        let s = serde_json::to_string(&d).unwrap();
        let d2: FileEventData = serde_json::from_str(&s).unwrap();
        assert_eq!(d2.zone_id, "zone");
        assert_eq!(d2.file_name, "file.txt");
        assert_eq!(d2.file_op, FILE_OP_TRUNCATE);
        assert_eq!(d2.data64.unwrap(), "SGVsbG8=");
    }

    #[test]
    fn test_subscription_request_construction() {
        let req = SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec!["scope1".to_string(), "scope2".to_string()],
        };
        assert_eq!(req.topic, "test");
        assert_eq!(req.scopes.len(), 2);
    }

    #[test]
    fn test_subscription_request_empty_scopes() {
        let req = SubscriptionRequest {
            topic: "test".to_string(),
            scopes: vec![],
        };
        assert!(req.scopes.is_empty());
    }

    #[test]
    fn test_file_event_data_clone() {
        let d = FileEventData {
            zone_id: "z".to_string(),
            file_name: "f".to_string(),
            file_op: FILE_OP_APPEND.to_string(),
            data64: Some("data".to_string()),
        };
        let d2 = d.clone();
        assert_eq!(d.zone_id, d2.zone_id);
        assert_eq!(d.file_name, d2.file_name);
        assert_eq!(d.file_op, d2.file_op);
        assert_eq!(d.data64, d2.data64);
    }

    #[test]
    fn test_wave_event_clone() {
        let ev = WaveEvent::new("t", vec!["s".to_string()], json!(1)).with_persist(10);
        let ev2 = ev.clone();
        assert_eq!(ev.topic, ev2.topic);
        assert_eq!(ev.scopes, ev2.scopes);
        assert_eq!(ev.data, ev2.data);
        assert_eq!(ev.persist, ev2.persist);
        assert_eq!(ev.event_id, ev2.event_id);
    }
}
