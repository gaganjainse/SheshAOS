use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::meta::MetaMap;
use crate::oref::ORef;

/// Trait that all Wave objects must implement.
/// Provides typed access to the common fields (oid, version, meta).
pub trait WaveObj: std::fmt::Debug + Send + Sync {
    fn otype() -> &'static str
    where
        Self: Sized;
    fn oid(&self) -> &Uuid;
    fn version(&self) -> i64;
    fn set_version(&mut self, v: i64);
    fn meta(&self) -> &MetaMap;
    fn meta_mut(&mut self) -> &mut MetaMap;
    fn oref(&self) -> ORef
    where
        Self: Sized,
    {
        ORef::new(Self::otype().to_string(), *self.oid()).unwrap()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WinSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TermSize {
    pub rows: i32,
    pub cols: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeOpts {
    pub term_size: TermSize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StickerType {
    #[serde(rename = "stickertype")]
    pub sticker_type: String,
    pub style: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeafOrderEntry {
    pub node_id: String,
    pub block_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutActionData {
    #[serde(rename = "actiontype")]
    pub action_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_size: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub oid: Uuid,
    pub version: i64,
    pub window_ids: Vec<String>,
    pub meta: MetaMap,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tos_agreed: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_old_history: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temp_oid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_id: Option<String>,
}

impl WaveObj for Client {
    fn otype() -> &'static str {
        "client"
    }
    fn oid(&self) -> &Uuid {
        &self.oid
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, v: i64) {
        self.version = v;
    }
    fn meta(&self) -> &MetaMap {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut MetaMap {
        &mut self.meta
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Window {
    pub oid: Uuid,
    pub version: i64,
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_new: Option<bool>,
    pub pos: Point,
    pub win_size: WinSize,
    #[serde(default)]
    pub last_focus_ts: i64,
    pub meta: MetaMap,
}

impl WaveObj for Window {
    fn otype() -> &'static str {
        "window"
    }
    fn oid(&self) -> &Uuid {
        &self.oid
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, v: i64) {
        self.version = v;
    }
    fn meta(&self) -> &MetaMap {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut MetaMap {
        &mut self.meta
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub oid: Uuid,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub tab_ids: Vec<String>,
    pub active_tab_id: String,
    pub meta: MetaMap,
}

impl WaveObj for Workspace {
    fn otype() -> &'static str {
        "workspace"
    }
    fn oid(&self) -> &Uuid {
        &self.oid
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, v: i64) {
        self.version = v;
    }
    fn meta(&self) -> &MetaMap {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut MetaMap {
        &mut self.meta
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub oid: Uuid,
    pub version: i64,
    pub name: String,
    pub layout_state: String, // OID reference to LayoutState
    pub block_ids: Vec<String>,
    pub meta: MetaMap,
}

impl WaveObj for Tab {
    fn otype() -> &'static str {
        "tab"
    }
    fn oid(&self) -> &Uuid {
        &self.oid
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, v: i64) {
        self.version = v;
    }
    fn meta(&self) -> &MetaMap {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut MetaMap {
        &mut self.meta
    }
}

impl Tab {
    pub fn block_orefs(&self) -> Vec<ORef> {
        self.block_ids
            .iter()
            .filter_map(|id| id.parse::<Uuid>().ok())
            .filter_map(|id| ORef::new("block".to_string(), id).ok())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutState {
    pub oid: Uuid,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_node: Option<serde_json::Value>, // flexible tree node
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magnified_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaf_order: Option<Vec<LeafOrderEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_backend_actions: Option<Vec<LayoutActionData>>,
    #[serde(default)]
    pub meta: MetaMap,
}

impl WaveObj for LayoutState {
    fn otype() -> &'static str {
        "layoutstate"
    }
    fn oid(&self) -> &Uuid {
        &self.oid
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, v: i64) {
        self.version = v;
    }
    fn meta(&self) -> &MetaMap {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut MetaMap {
        &mut self.meta
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub oid: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_oref: Option<String>, // "tab:uuid" or "block:uuid"
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_opts: Option<RuntimeOpts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stickers: Option<Vec<StickerType>>,
    pub meta: MetaMap,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub_block_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

impl WaveObj for Block {
    fn otype() -> &'static str {
        "block"
    }
    fn oid(&self) -> &Uuid {
        &self.oid
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, v: i64) {
        self.version = v;
    }
    fn meta(&self) -> &MetaMap {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut MetaMap {
        &mut self.meta
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub oid: Uuid,
    pub version: i64,
    pub connection: String,
    pub job_kind: String,
    pub cmd: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cmd_args: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub cmd_env: std::collections::HashMap<String, String>,
    pub job_auth_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attached_block_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_manager_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd_pid: Option<i32>,
    pub cmd_term_size: TermSize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd_exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd_exit_signal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd_exit_error: Option<String>,
    #[serde(default)]
    pub stream_done: bool,
    pub meta: MetaMap,
}

impl WaveObj for Job {
    fn otype() -> &'static str {
        "job"
    }
    fn oid(&self) -> &Uuid {
        &self.oid
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, v: i64) {
        self.version = v;
    }
    fn meta(&self) -> &MetaMap {
        &self.meta
    }
    fn meta_mut(&mut self) -> &mut MetaMap {
        &mut self.meta
    }
}

/// Maps an OType string to its SQLite table name.
pub fn otype_to_table(otype: &str) -> String {
    format!("db_{}", otype)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_roundtrip() {
        let mut client = Client {
            oid: Uuid::now_v7(),
            version: 1,
            window_ids: vec!["win1".to_string()],
            meta: MetaMap::default(),
            tos_agreed: Some(12345),
            has_old_history: Some(false),
            temp_oid: None,
            install_id: Some("install1".to_string()),
        };
        client.meta_mut().0.insert("key".to_string(), serde_json::json!("val"));
        assert_eq!(Client::otype(), "client");
        assert_eq!(client.version(), 1);
        client.set_version(2);
        assert_eq!(client.version(), 2);
        
        let json = serde_json::to_string(&client).unwrap();
        let client_deser: Client = serde_json::from_str(&json).unwrap();
        assert_eq!(client.oid, client_deser.oid);
        assert_eq!(client.version, client_deser.version);
        assert_eq!(client.meta.0.get("key"), Some(&serde_json::json!("val")));
    }

    #[test]
    fn test_tab_block_orefs() {
        let block_id1 = Uuid::now_v7();
        let block_id2 = Uuid::now_v7();
        let tab = Tab {
            oid: Uuid::now_v7(),
            version: 1,
            name: "tab1".to_string(),
            layout_state: Uuid::now_v7().to_string(),
            block_ids: vec![block_id1.to_string(), "invalid_uuid".to_string(), block_id2.to_string()],
            meta: MetaMap::default(),
        };

        let orefs = tab.block_orefs();
        assert_eq!(orefs.len(), 2);
        assert_eq!(orefs[0].otype, "block");
        assert_eq!(orefs[0].oid, block_id1);
        assert_eq!(orefs[1].otype, "block");
        assert_eq!(orefs[1].oid, block_id2);
    }

    #[test]
    fn test_otype_to_table() {
        assert_eq!(otype_to_table("client"), "db_client");
        assert_eq!(otype_to_table("block"), "db_block");
    }
}
