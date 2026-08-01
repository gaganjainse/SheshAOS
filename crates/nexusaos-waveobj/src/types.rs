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
        ORef {
            otype: Self::otype().to_string(),
            oid: *self.oid(),
        }
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
    use std::collections::HashMap;

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

    #[test]
    fn test_otype_to_table_all_types() {
        assert_eq!(otype_to_table("client"), "db_client");
        assert_eq!(otype_to_table("window"), "db_window");
        assert_eq!(otype_to_table("workspace"), "db_workspace");
        assert_eq!(otype_to_table("tab"), "db_tab");
        assert_eq!(otype_to_table("layoutstate"), "db_layoutstate");
        assert_eq!(otype_to_table("block"), "db_block");
        assert_eq!(otype_to_table("job"), "db_job");
    }

    #[test]
    fn test_otype_to_table_empty_string() {
        assert_eq!(otype_to_table(""), "db_");
    }

    #[test]
    fn test_point_default() {
        let p = Point::default();
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);
    }

    #[test]
    fn test_point_partial_eq() {
        assert_eq!(Point { x: 1, y: 2 }, Point { x: 1, y: 2 });
        assert_ne!(Point { x: 1, y: 2 }, Point { x: 1, y: 3 });
        assert_ne!(Point { x: 0, y: 0 }, Point { x: 1, y: 0 });
    }

    #[test]
    fn test_point_clone() {
        let p = Point { x: 42, y: -10 };
        let cloned = p.clone();
        assert_eq!(p, cloned);
    }

    #[test]
    fn test_point_serde() {
        let p = Point { x: -5, y: 100 };
        let json = serde_json::to_string(&p).unwrap();
        let deserialized: Point = serde_json::from_str(&json).unwrap();
        assert_eq!(p, deserialized);
    }

    #[test]
    fn test_point_serde_field_names() {
        let p = Point { x: 1, y: 2 };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["x"], 1);
        assert_eq!(json["y"], 2);
    }

    #[test]
    fn test_winsize_default() {
        let s = WinSize::default();
        assert_eq!(s.width, 0);
        assert_eq!(s.height, 0);
    }

    #[test]
    fn test_winsize_partial_eq() {
        assert_eq!(WinSize { width: 100, height: 200 }, WinSize { width: 100, height: 200 });
        assert_ne!(WinSize { width: 100, height: 200 }, WinSize { width: 100, height: 300 });
    }

    #[test]
    fn test_winsize_clone() {
        let s = WinSize { width: 1920, height: 1080 };
        assert_eq!(s.clone(), s);
    }

    #[test]
    fn test_winsize_serde() {
        let s = WinSize { width: 1920, height: 1080 };
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: WinSize = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }

    #[test]
    fn test_termsize_default() {
        let t = TermSize::default();
        assert_eq!(t.rows, 0);
        assert_eq!(t.cols, 0);
    }

    #[test]
    fn test_termsize_partial_eq() {
        assert_eq!(TermSize { rows: 24, cols: 80 }, TermSize { rows: 24, cols: 80 });
        assert_ne!(TermSize { rows: 24, cols: 80 }, TermSize { rows: 25, cols: 80 });
    }

    #[test]
    fn test_termsize_clone() {
        let t = TermSize { rows: 50, cols: 120 };
        assert_eq!(t.clone(), t);
    }

    #[test]
    fn test_termsize_serde() {
        let t = TermSize { rows: 24, cols: 80 };
        let json = serde_json::to_string(&t).unwrap();
        let deserialized: TermSize = serde_json::from_str(&json).unwrap();
        assert_eq!(t, deserialized);
    }

    #[test]
    fn test_runtimeopts_serde() {
        let opts = RuntimeOpts {
            term_size: TermSize { rows: 24, cols: 80 },
            env: Some(HashMap::from([("KEY".to_string(), "val".to_string())])),
        };
        let json = serde_json::to_string(&opts).unwrap();
        let deserialized: RuntimeOpts = serde_json::from_str(&json).unwrap();
        assert_eq!(opts, deserialized);
    }

    #[test]
    fn test_runtimeopts_env_none_skipped() {
        let opts = RuntimeOpts {
            term_size: TermSize { rows: 24, cols: 80 },
            env: None,
        };
        let json = serde_json::to_value(&opts).unwrap();
        assert!(json.get("env").is_none());
    }

    #[test]
    fn test_runtimeopts_env_some_present() {
        let opts = RuntimeOpts {
            term_size: TermSize { rows: 24, cols: 80 },
            env: Some(HashMap::from([("PATH".to_string(), "/usr/bin".to_string())])),
        };
        let json = serde_json::to_value(&opts).unwrap();
        assert!(json.get("env").is_some());
    }

    #[test]
    fn test_runtimeopts_partial_eq() {
        let opts1 = RuntimeOpts {
            term_size: TermSize { rows: 24, cols: 80 },
            env: None,
        };
        let opts2 = RuntimeOpts {
            term_size: TermSize { rows: 24, cols: 80 },
            env: None,
        };
        let opts3 = RuntimeOpts {
            term_size: TermSize { rows: 30, cols: 80 },
            env: None,
        };
        assert_eq!(opts1, opts2);
        assert_ne!(opts1, opts3);
    }

    #[test]
    fn test_stickertype_serde() {
        let sticker = StickerType {
            sticker_type: "cmd".to_string(),
            style: serde_json::json!({"color": "red"}),
        };
        let json = serde_json::to_string(&sticker).unwrap();
        let deserialized: StickerType = serde_json::from_str(&json).unwrap();
        assert_eq!(sticker, deserialized);
    }

    #[test]
    fn test_stickertype_serde_field_renamed() {
        let sticker = StickerType {
            sticker_type: "cmd".to_string(),
            style: serde_json::json!({"color": "red"}),
        };
        let json = serde_json::to_value(&sticker).unwrap();
        assert_eq!(json["stickertype"], "cmd");
    }

    #[test]
    fn test_stickertype_partial_eq() {
        let s1 = StickerType {
            sticker_type: "cmd".to_string(),
            style: serde_json::json!({"color": "red"}),
        };
        let s2 = StickerType {
            sticker_type: "cmd".to_string(),
            style: serde_json::json!({"color": "red"}),
        };
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_leaforderentry_serde() {
        let entry = LeafOrderEntry {
            node_id: "node1".to_string(),
            block_id: "block1".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: LeafOrderEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_layoutactiondata_serde() {
        let action = LayoutActionData {
            action_type: "resize".to_string(),
            block_id: Some("block1".to_string()),
            node_size: Some(0.5),
        };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: LayoutActionData = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_layoutactiondata_optional_none_skipped() {
        let action = LayoutActionData {
            action_type: "resize".to_string(),
            block_id: None,
            node_size: None,
        };
        let json = serde_json::to_value(&action).unwrap();
        assert!(json.get("block_id").is_none());
        assert!(json.get("node_size").is_none());
        assert_eq!(json["actiontype"], "resize");
    }

    #[test]
    fn test_client_otype() {
        assert_eq!(Client::otype(), "client");
    }

    #[test]
    fn test_window_otype() {
        assert_eq!(Window::otype(), "window");
    }

    #[test]
    fn test_workspace_otype() {
        assert_eq!(Workspace::otype(), "workspace");
    }

    #[test]
    fn test_tab_otype() {
        assert_eq!(Tab::otype(), "tab");
    }

    #[test]
    fn test_layoutstate_otype() {
        assert_eq!(LayoutState::otype(), "layoutstate");
    }

    #[test]
    fn test_block_otype() {
        assert_eq!(Block::otype(), "block");
    }

    #[test]
    fn test_job_otype() {
        assert_eq!(Job::otype(), "job");
    }

    #[test]
    fn test_client_oid() {
        let oid = Uuid::new_v4();
        let client = Client {
            oid,
            version: 0,
            window_ids: vec![],
            meta: MetaMap::default(),
            tos_agreed: None,
            has_old_history: None,
            temp_oid: None,
            install_id: None,
        };
        assert_eq!(client.oid(), &oid);
    }

    #[test]
    fn test_client_version_and_set_version() {
        let mut client = Client {
            oid: Uuid::new_v4(),
            version: 5,
            window_ids: vec![],
            meta: MetaMap::default(),
            tos_agreed: None,
            has_old_history: None,
            temp_oid: None,
            install_id: None,
        };
        assert_eq!(client.version(), 5);
        client.set_version(10);
        assert_eq!(client.version(), 10);
    }

    #[test]
    fn test_client_meta_and_meta_mut() {
        let mut client = Client {
            oid: Uuid::new_v4(),
            version: 0,
            window_ids: vec![],
            meta: MetaMap::default(),
            tos_agreed: None,
            has_old_history: None,
            temp_oid: None,
            install_id: None,
        };
        client.meta_mut().set("key", "val");
        assert_eq!(client.meta().get_string("key"), Some("val".to_string()));
    }

    #[test]
    fn test_client_oref() {
        let oid = Uuid::new_v4();
        let client = Client {
            oid,
            version: 0,
            window_ids: vec![],
            meta: MetaMap::default(),
            tos_agreed: None,
            has_old_history: None,
            temp_oid: None,
            install_id: None,
        };
        let oref = client.oref();
        assert_eq!(oref.otype, "client");
        assert_eq!(oref.oid, oid);
    }

    #[test]
    fn test_window_oid_version_set_version() {
        let oid = Uuid::new_v4();
        let mut window = Window {
            oid,
            version: 1,
            workspace_id: "ws1".to_string(),
            is_new: None,
            pos: Point::default(),
            win_size: WinSize::default(),
            last_focus_ts: 0,
            meta: MetaMap::default(),
        };
        assert_eq!(window.oid(), &oid);
        assert_eq!(window.version(), 1);
        window.set_version(42);
        assert_eq!(window.version(), 42);
    }

    #[test]
    fn test_window_meta_and_oref() {
        let oid = Uuid::new_v4();
        let mut window = Window {
            oid,
            version: 0,
            workspace_id: "ws1".to_string(),
            is_new: None,
            pos: Point::default(),
            win_size: WinSize::default(),
            last_focus_ts: 123,
            meta: MetaMap::default(),
        };
        window.meta_mut().set("theme", "dark");
        assert_eq!(window.meta().get_string("theme"), Some("dark".to_string()));

        let oref = window.oref();
        assert_eq!(oref.otype, "window");
        assert_eq!(oref.oid, oid);
    }

    #[test]
    fn test_window_serde_is_new_some() {
        let window = Window {
            oid: Uuid::new_v4(),
            version: 1,
            workspace_id: "ws1".to_string(),
            is_new: Some(true),
            pos: Point { x: 10, y: 20 },
            win_size: WinSize { width: 800, height: 600 },
            last_focus_ts: 1000,
            meta: MetaMap::default(),
        };
        let json = serde_json::to_string(&window).unwrap();
        let deserialized: Window = serde_json::from_str(&json).unwrap();
        assert_eq!(window, deserialized);
    }

    #[test]
    fn test_window_serde_is_new_none_skipped() {
        let window = Window {
            oid: Uuid::new_v4(),
            version: 1,
            workspace_id: "ws1".to_string(),
            is_new: None,
            pos: Point::default(),
            win_size: WinSize::default(),
            last_focus_ts: 0,
            meta: MetaMap::default(),
        };
        let json = serde_json::to_value(&window).unwrap();
        assert!(json.get("is_new").is_none());
    }

    #[test]
    fn test_workspace_oid_version_set_version() {
        let oid = Uuid::new_v4();
        let mut ws = Workspace {
            oid,
            version: 3,
            name: Some("My Workspace".to_string()),
            icon: None,
            color: None,
            tab_ids: vec!["tab1".to_string()],
            active_tab_id: "tab1".to_string(),
            meta: MetaMap::default(),
        };
        assert_eq!(ws.oid(), &oid);
        assert_eq!(ws.version(), 3);
        ws.set_version(7);
        assert_eq!(ws.version(), 7);
    }

    #[test]
    fn test_workspace_meta_and_oref() {
        let oid = Uuid::new_v4();
        let mut ws = Workspace {
            oid,
            version: 0,
            name: Some("ws".to_string()),
            icon: None,
            color: None,
            tab_ids: vec![],
            active_tab_id: String::new(),
            meta: MetaMap::default(),
        };
        ws.meta_mut().set("key", "val");
        assert_eq!(ws.meta().get_string("key"), Some("val".to_string()));

        let oref = ws.oref();
        assert_eq!(oref.otype, "workspace");
        assert_eq!(oref.oid, oid);
    }

    #[test]
    fn test_workspace_serde_optional_none() {
        let ws = Workspace {
            oid: Uuid::new_v4(),
            version: 1,
            name: None,
            icon: None,
            color: None,
            tab_ids: vec![],
            active_tab_id: "active".to_string(),
            meta: MetaMap::default(),
        };
        let json = serde_json::to_value(&ws).unwrap();
        assert!(json.get("name").is_none());
        assert!(json.get("icon").is_none());
        assert!(json.get("color").is_none());

        let deserialized: Workspace = serde_json::from_str(&serde_json::to_string(&ws).unwrap()).unwrap();
        assert_eq!(ws, deserialized);
    }

    #[test]
    fn test_workspace_serde_full() {
        let ws = Workspace {
            oid: Uuid::new_v4(),
            version: 5,
            name: Some("Full WS".to_string()),
            icon: Some("star".to_string()),
            color: Some("#ff0000".to_string()),
            tab_ids: vec!["tab1".to_string(), "tab2".to_string()],
            active_tab_id: "tab2".to_string(),
            meta: MetaMap::default(),
        };
        let json = serde_json::to_string(&ws).unwrap();
        let deserialized: Workspace = serde_json::from_str(&json).unwrap();
        assert_eq!(ws, deserialized);
    }

    #[test]
    fn test_tab_oid_version_set_version() {
        let oid = Uuid::new_v4();
        let mut tab = Tab {
            oid,
            version: 2,
            name: "tab1".to_string(),
            layout_state: Uuid::new_v4().to_string(),
            block_ids: vec![],
            meta: MetaMap::default(),
        };
        assert_eq!(tab.oid(), &oid);
        assert_eq!(tab.version(), 2);
        tab.set_version(99);
        assert_eq!(tab.version(), 99);
    }

    #[test]
    fn test_tab_meta_and_oref() {
        let oid = Uuid::new_v4();
        let mut tab = Tab {
            oid,
            version: 0,
            name: "tab".to_string(),
            layout_state: "layout_uuid".to_string(),
            block_ids: vec![],
            meta: MetaMap::default(),
        };
        tab.meta_mut().set("key", "val");
        assert_eq!(tab.meta().get_string("key"), Some("val".to_string()));

        let oref = tab.oref();
        assert_eq!(oref.otype, "tab");
        assert_eq!(oref.oid, oid);
    }

    #[test]
    fn test_tab_block_orefs_empty() {
        let tab = Tab {
            oid: Uuid::new_v4(),
            version: 0,
            name: "empty".to_string(),
            layout_state: Uuid::new_v4().to_string(),
            block_ids: vec![],
            meta: MetaMap::default(),
        };
        let orefs = tab.block_orefs();
        assert!(orefs.is_empty());
    }

    #[test]
    fn test_tab_block_orefs_all_invalid() {
        let tab = Tab {
            oid: Uuid::new_v4(),
            version: 0,
            name: "invalid".to_string(),
            layout_state: Uuid::new_v4().to_string(),
            block_ids: vec!["not-a-uuid".to_string(), "also-not".to_string()],
            meta: MetaMap::default(),
        };
        let orefs = tab.block_orefs();
        assert!(orefs.is_empty());
    }

    #[test]
    fn test_tab_block_orefs_mixed_valid_invalid() {
        let valid_id = Uuid::new_v4();
        let tab = Tab {
            oid: Uuid::new_v4(),
            version: 0,
            name: "mixed".to_string(),
            layout_state: Uuid::new_v4().to_string(),
            block_ids: vec![
                valid_id.to_string(),
                "invalid".to_string(),
                Uuid::new_v4().to_string(),
            ],
            meta: MetaMap::default(),
        };
        let orefs = tab.block_orefs();
        assert_eq!(orefs.len(), 2);
        for oref in &orefs {
            assert_eq!(oref.otype, "block");
        }
        assert_eq!(orefs[0].oid, valid_id);
    }

    #[test]
    fn test_tab_serde() {
        let tab = Tab {
            oid: Uuid::new_v4(),
            version: 1,
            name: "tab1".to_string(),
            layout_state: Uuid::new_v4().to_string(),
            block_ids: vec!["id1".to_string(), "id2".to_string()],
            meta: MetaMap::default(),
        };
        let json = serde_json::to_string(&tab).unwrap();
        let deserialized: Tab = serde_json::from_str(&json).unwrap();
        assert_eq!(tab, deserialized);
    }

    #[test]
    fn test_layoutstate_oid_version_set_version() {
        let oid = Uuid::new_v4();
        let mut ls = LayoutState {
            oid,
            version: 0,
            root_node: None,
            magnified_node_id: None,
            focused_node_id: None,
            leaf_order: None,
            pending_backend_actions: None,
            meta: MetaMap::default(),
        };
        assert_eq!(ls.oid(), &oid);
        assert_eq!(ls.version(), 0);
        ls.set_version(3);
        assert_eq!(ls.version(), 3);
    }

    #[test]
    fn test_layoutstate_meta_and_oref() {
        let oid = Uuid::new_v4();
        let mut ls = LayoutState {
            oid,
            version: 0,
            root_node: None,
            magnified_node_id: None,
            focused_node_id: None,
            leaf_order: None,
            pending_backend_actions: None,
            meta: MetaMap::default(),
        };
        ls.meta_mut().set("key", "val");
        assert_eq!(ls.meta().get_string("key"), Some("val".to_string()));

        let oref = ls.oref();
        assert_eq!(oref.otype, "layoutstate");
        assert_eq!(oref.oid, oid);
    }

    #[test]
    fn test_layoutstate_serde_all_none() {
        let ls = LayoutState {
            oid: Uuid::new_v4(),
            version: 0,
            root_node: None,
            magnified_node_id: None,
            focused_node_id: None,
            leaf_order: None,
            pending_backend_actions: None,
            meta: MetaMap::default(),
        };
        let json = serde_json::to_value(&ls).unwrap();
        assert!(json.get("root_node").is_none());
        assert!(json.get("magnified_node_id").is_none());
        assert_eq!(json["version"], 0);
    }

    #[test]
    fn test_layoutstate_serde_full() {
        let ls = LayoutState {
            oid: Uuid::new_v4(),
            version: 4,
            root_node: Some(serde_json::json!({"type": "row", "children": []})),
            magnified_node_id: Some("node1".to_string()),
            focused_node_id: Some("node2".to_string()),
            leaf_order: Some(vec![
                LeafOrderEntry { node_id: "n1".to_string(), block_id: "b1".to_string() },
                LeafOrderEntry { node_id: "n2".to_string(), block_id: "b2".to_string() },
            ]),
            pending_backend_actions: Some(vec![
                LayoutActionData {
                    action_type: "resize".to_string(),
                    block_id: Some("b1".to_string()),
                    node_size: Some(0.5),
                },
            ]),
            meta: MetaMap::default(),
        };
        let json = serde_json::to_string(&ls).unwrap();
        let deserialized: LayoutState = serde_json::from_str(&json).unwrap();
        assert_eq!(ls, deserialized);
    }

    #[test]
    fn test_block_oid_version_set_version() {
        let oid = Uuid::new_v4();
        let mut block = Block {
            oid,
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::default(),
            sub_block_ids: vec![],
            job_id: None,
        };
        assert_eq!(block.oid(), &oid);
        assert_eq!(block.version(), 0);
        block.set_version(5);
        assert_eq!(block.version(), 5);
    }

    #[test]
    fn test_block_meta_and_oref() {
        let oid = Uuid::new_v4();
        let mut block = Block {
            oid,
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::default(),
            sub_block_ids: vec![],
            job_id: None,
        };
        block.meta_mut().set("key", "val");
        assert_eq!(block.meta().get_string("key"), Some("val".to_string()));

        let oref = block.oref();
        assert_eq!(oref.otype, "block");
        assert_eq!(oref.oid, oid);
    }

    #[test]
    fn test_block_serde_all_none() {
        let block = Block {
            oid: Uuid::new_v4(),
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::default(),
            sub_block_ids: vec![],
            job_id: None,
        };
        let json = serde_json::to_value(&block).unwrap();
        assert!(json.get("parent_oref").is_none());
        assert!(json.get("runtime_opts").is_none());
        assert!(json.get("stickers").is_none());
        assert!(json.get("sub_block_ids").is_none());
        assert!(json.get("job_id").is_none());
    }

    #[test]
    fn test_block_serde_full() {
        let block = Block {
            oid: Uuid::new_v4(),
            parent_oref: Some("tab:00000000-0000-0000-0000-000000000000".to_string()),
            version: 1,
            runtime_opts: Some(RuntimeOpts {
                term_size: TermSize { rows: 24, cols: 80 },
                env: Some(HashMap::from([("PATH".to_string(), "/bin".to_string())])),
            }),
            stickers: Some(vec![
                StickerType {
                    sticker_type: "cmd".to_string(),
                    style: serde_json::json!({"color": "blue"}),
                },
            ]),
            meta: MetaMap::default(),
            sub_block_ids: vec!["sub1".to_string(), "sub2".to_string()],
            job_id: Some("job123".to_string()),
        };
        let json = serde_json::to_string(&block).unwrap();
        let deserialized: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(block, deserialized);
    }

    #[test]
    fn test_block_sub_block_ids_present_when_non_empty() {
        let block = Block {
            oid: Uuid::new_v4(),
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::default(),
            sub_block_ids: vec!["sub1".to_string()],
            job_id: None,
        };
        let json = serde_json::to_value(&block).unwrap();
        assert!(json.get("sub_block_ids").is_some());
        assert_eq!(json["sub_block_ids"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_job_oid_version_set_version() {
        let oid = Uuid::new_v4();
        let mut job = Job {
            oid,
            version: 0,
            connection: "conn".to_string(),
            job_kind: "shell".to_string(),
            cmd: "bash".to_string(),
            cmd_args: vec![],
            cmd_env: HashMap::new(),
            job_auth_token: "token".to_string(),
            attached_block_id: None,
            job_manager_status: None,
            cmd_pid: None,
            cmd_term_size: TermSize { rows: 24, cols: 80 },
            cmd_exit_code: None,
            cmd_exit_signal: None,
            cmd_exit_error: None,
            stream_done: false,
            meta: MetaMap::default(),
        };
        assert_eq!(job.oid(), &oid);
        assert_eq!(job.version(), 0);
        job.set_version(1);
        assert_eq!(job.version(), 1);
    }

    #[test]
    fn test_job_meta_and_oref() {
        let oid = Uuid::new_v4();
        let mut job = Job {
            oid,
            version: 0,
            connection: "conn".to_string(),
            job_kind: "shell".to_string(),
            cmd: "bash".to_string(),
            cmd_args: vec![],
            cmd_env: HashMap::new(),
            job_auth_token: "token".to_string(),
            attached_block_id: None,
            job_manager_status: None,
            cmd_pid: None,
            cmd_term_size: TermSize::default(),
            cmd_exit_code: None,
            cmd_exit_signal: None,
            cmd_exit_error: None,
            stream_done: false,
            meta: MetaMap::default(),
        };
        job.meta_mut().set("key", "val");
        assert_eq!(job.meta().get_string("key"), Some("val".to_string()));

        let oref = job.oref();
        assert_eq!(oref.otype, "job");
        assert_eq!(oref.oid, oid);
    }

    #[test]
    fn test_job_serde_full() {
        let job = Job {
            oid: Uuid::new_v4(),
            version: 7,
            connection: "conn".to_string(),
            job_kind: "shell".to_string(),
            cmd: "bash".to_string(),
            cmd_args: vec!["-c".to_string(), "echo hello".to_string()],
            cmd_env: HashMap::from([("HOME".to_string(), "/root".to_string())]),
            job_auth_token: "secret".to_string(),
            attached_block_id: Some("block123".to_string()),
            job_manager_status: Some("running".to_string()),
            cmd_pid: Some(1234),
            cmd_term_size: TermSize { rows: 40, cols: 120 },
            cmd_exit_code: Some(0),
            cmd_exit_signal: Some("SIGTERM".to_string()),
            cmd_exit_error: Some("error msg".to_string()),
            stream_done: true,
            meta: MetaMap::default(),
        };
        let json = serde_json::to_string(&job).unwrap();
        let deserialized: Job = serde_json::from_str(&json).unwrap();
        assert_eq!(job, deserialized);
    }

    #[test]
    fn test_job_serde_optional_none_skipped() {
        let job = Job {
            oid: Uuid::new_v4(),
            version: 0,
            connection: "conn".to_string(),
            job_kind: "shell".to_string(),
            cmd: "bash".to_string(),
            cmd_args: vec![],
            cmd_env: HashMap::new(),
            job_auth_token: "token".to_string(),
            attached_block_id: None,
            job_manager_status: None,
            cmd_pid: None,
            cmd_term_size: TermSize::default(),
            cmd_exit_code: None,
            cmd_exit_signal: None,
            cmd_exit_error: None,
            stream_done: false,
            meta: MetaMap::default(),
        };
        let json = serde_json::to_value(&job).unwrap();
        assert!(json.get("attached_block_id").is_none());
        assert!(json.get("job_manager_status").is_none());
        assert!(json.get("cmd_pid").is_none());
        assert!(json.get("cmd_exit_code").is_none());
        assert!(json.get("cmd_exit_signal").is_none());
        assert!(json.get("cmd_exit_error").is_none());
    }

    #[test]
    fn test_job_serde_cmd_args_empty_skipped() {
        let job = Job {
            oid: Uuid::new_v4(),
            version: 0,
            connection: "conn".to_string(),
            job_kind: "shell".to_string(),
            cmd: "bash".to_string(),
            cmd_args: vec![],
            cmd_env: HashMap::new(),
            job_auth_token: "token".to_string(),
            attached_block_id: None,
            job_manager_status: None,
            cmd_pid: None,
            cmd_term_size: TermSize::default(),
            cmd_exit_code: None,
            cmd_exit_signal: None,
            cmd_exit_error: None,
            stream_done: false,
            meta: MetaMap::default(),
        };
        let json = serde_json::to_value(&job).unwrap();
        assert!(json.get("cmd_args").is_none());
        assert!(json.get("cmd_env").is_none());
    }

    #[test]
    fn test_job_serde_stream_done_false() {
        let job = Job {
            oid: Uuid::new_v4(),
            version: 0,
            connection: "conn".to_string(),
            job_kind: "shell".to_string(),
            cmd: "bash".to_string(),
            cmd_args: vec![],
            cmd_env: HashMap::new(),
            job_auth_token: "token".to_string(),
            attached_block_id: None,
            job_manager_status: None,
            cmd_pid: None,
            cmd_term_size: TermSize::default(),
            cmd_exit_code: None,
            cmd_exit_signal: None,
            cmd_exit_error: None,
            stream_done: false,
            meta: MetaMap::default(),
        };
        let json = serde_json::to_value(&job).unwrap();
        // stream_done has #[serde(default)] so it should be present even when false
        assert_eq!(json["stream_done"], false);
    }

    #[test]
    fn test_all_otypes_match_valid_list() {
        use crate::oref::VALID_OTYPES;
        let otypes = vec![
            Client::otype(),
            Window::otype(),
            Workspace::otype(),
            Tab::otype(),
            LayoutState::otype(),
            Block::otype(),
            Job::otype(),
        ];
        for ot in &otypes {
            assert!(VALID_OTYPES.contains(ot), "otype '{}' not in VALID_OTYPES", ot);
        }
    }
}
