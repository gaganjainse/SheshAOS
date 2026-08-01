use rusqlite::{Connection, OptionalExtension, Transaction};
use std::sync::Mutex;
use uuid::Uuid;
use crate::meta::{MetaMap, merge_meta};
use crate::oref::{ORef, VALID_OTYPES, is_valid_otype};
use crate::types::{WaveObj, Tab, Workspace, Window, Block};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("invalid object type: {0}")]
    InvalidOType(String),
    #[error("version conflict for {0}: expected {1}")]
    VersionConflict(String, i64),
    #[error("lock poisoned")]
    PoisonError,
}

impl<T> From<std::sync::PoisonError<T>> for StoreError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        StoreError::PoisonError
    }
}

pub struct WaveStore {
    conn: Mutex<Connection>,
}

impl WaveStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;"
        )?;
        let store = Self { conn: Mutex::new(conn) };
        store.ensure_tables()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;"
        )?;
        let store = Self { conn: Mutex::new(conn) };
        store.ensure_tables()?;
        Ok(store)
    }

    fn ensure_tables(&self) -> Result<(), StoreError> {
        let conn = self.conn.lock()?;
        for otype in VALID_OTYPES {
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS db_{} (
                    oid TEXT PRIMARY KEY,
                    version INTEGER NOT NULL,
                    data TEXT NOT NULL
                )",
                otype
            );
            conn.execute(&sql, [])?;
        }
        Ok(())
    }

    pub fn db_insert<T: WaveObj + serde::Serialize>(&self, obj: &mut T) -> Result<(), StoreError> {
        let otype = T::otype();
        if !is_valid_otype(otype) {
            return Err(StoreError::InvalidOType(otype.to_string()));
        }
        obj.set_version(1);
        let data = serde_json::to_string(obj)?;
        let conn = self.conn.lock()?;
        let sql = format!("INSERT INTO db_{} (oid, version, data) VALUES (?1, 1, ?2)", otype);
        conn.execute(&sql, rusqlite::params![obj.oid().to_string(), data])?;
        Ok(())
    }

    pub fn db_get<T: WaveObj + serde::de::DeserializeOwned>(&self, oid: &Uuid) -> Result<Option<T>, StoreError> {
        let otype = T::otype();
        if !is_valid_otype(otype) {
            return Err(StoreError::InvalidOType(otype.to_string()));
        }
        let conn = self.conn.lock()?;
        let sql = format!("SELECT data FROM db_{} WHERE oid = ?1", otype);
        let data: Option<String> = conn.query_row(&sql, rusqlite::params![oid.to_string()], |row| row.get(0)).optional()?;
        if let Some(d) = data {
            let obj: T = serde_json::from_str(&d)?;
            Ok(Some(obj))
        } else {
            Ok(None)
        }
    }

    pub fn db_must_get<T: WaveObj + serde::de::DeserializeOwned>(&self, oid: &Uuid) -> Result<T, StoreError> {
        self.db_get(oid)?.ok_or_else(|| StoreError::NotFound(oid.to_string()))
    }

    pub fn db_update<T: WaveObj + serde::Serialize>(&self, obj: &mut T) -> Result<(), StoreError> {
        let otype = T::otype();
        if !is_valid_otype(otype) {
            return Err(StoreError::InvalidOType(otype.to_string()));
        }
        let new_version = obj.version() + 1;
        obj.set_version(new_version);
        let data = serde_json::to_string(obj)?;
        let conn = self.conn.lock()?;
        let sql = format!("UPDATE db_{} SET data = ?1, version = ?2 WHERE oid = ?3", otype);
        let count = conn.execute(&sql, rusqlite::params![data, new_version, obj.oid().to_string()])?;
        if count > 0 {
            Ok(())
        } else {
            Err(StoreError::NotFound(obj.oid().to_string()))
        }
    }

    pub fn db_delete(&self, otype: &str, oid: &Uuid) -> Result<bool, StoreError> {
        if !is_valid_otype(otype) {
            return Err(StoreError::InvalidOType(otype.to_string()));
        }
        let conn = self.conn.lock()?;
        let sql = format!("DELETE FROM db_{} WHERE oid = ?1", otype);
        let count = conn.execute(&sql, rusqlite::params![oid.to_string()])?;
        Ok(count > 0)
    }

    pub fn db_get_all<T: WaveObj + serde::de::DeserializeOwned>(&self) -> Result<Vec<T>, StoreError> {
        let otype = T::otype();
        if !is_valid_otype(otype) {
            return Err(StoreError::InvalidOType(otype.to_string()));
        }
        let conn = self.conn.lock()?;
        let sql = format!("SELECT data FROM db_{}", otype);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut res = Vec::new();
        for r in rows {
            let data = r?;
            let obj: T = serde_json::from_str(&data)?;
            res.push(obj);
        }
        Ok(res)
    }

    pub fn db_get_by_oref(&self, oref: &ORef) -> Result<Option<serde_json::Value>, StoreError> {
        if !is_valid_otype(&oref.otype) {
            return Err(StoreError::InvalidOType(oref.otype.clone()));
        }
        let conn = self.conn.lock()?;
        let sql = format!("SELECT data FROM db_{} WHERE oid = ?1", oref.otype);
        let data: Option<String> = conn.query_row(&sql, rusqlite::params![oref.oid.to_string()], |row| row.get(0)).optional()?;
        if let Some(d) = data {
            let val: serde_json::Value = serde_json::from_str(&d)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub fn find_tab_for_block(&self, block_oid: &Uuid) -> Result<Option<Tab>, StoreError> {
        let mut curr_oid = *block_oid;
        for _ in 0..5 {
            let block_opt = self.db_get::<Block>(&curr_oid)?;
            if let Some(block) = block_opt {
                if let Some(parent_oref_str) = block.parent_oref {
                    let parent_oref = ORef::parse(&parent_oref_str).map_err(|_| StoreError::NotFound("invalid oref".to_string()))?;
                    if parent_oref.otype == "tab" {
                        return self.db_get::<Tab>(&parent_oref.oid);
                    } else if parent_oref.otype == "block" {
                        curr_oid = parent_oref.oid;
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        }
        Ok(None)
    }

    pub fn find_workspace_for_tab(&self, tab_oid: &Uuid) -> Result<Option<Workspace>, StoreError> {
        let conn = self.conn.lock()?;
        let sql = "SELECT data FROM db_workspace WHERE EXISTS (SELECT 1 FROM json_each(json_extract(data, '$.tab_ids')) WHERE value = ?1)";
        let data: Option<String> = conn.query_row(sql, rusqlite::params![tab_oid.to_string()], |row| row.get(0)).optional()?;
        if let Some(d) = data {
            Ok(Some(serde_json::from_str(&d)?))
        } else {
            Ok(None)
        }
    }

    pub fn find_window_for_workspace(&self, workspace_oid: &Uuid) -> Result<Option<Window>, StoreError> {
        let conn = self.conn.lock()?;
        let sql = "SELECT data FROM db_window WHERE json_extract(data, '$.workspace_id') = ?1";
        let data: Option<String> = conn.query_row(sql, rusqlite::params![workspace_oid.to_string()], |row| row.get(0)).optional()?;
        if let Some(d) = data {
            Ok(Some(serde_json::from_str(&d)?))
        } else {
            Ok(None)
        }
    }

    pub fn update_object_meta(&self, otype: &str, oid: &Uuid, updates: &MetaMap) -> Result<(), StoreError> {
        if !is_valid_otype(otype) {
            return Err(StoreError::InvalidOType(otype.to_string()));
        }
        let conn = self.conn.lock()?;
        let sql_select = format!("SELECT data FROM db_{} WHERE oid = ?1", otype);
        let data_str: String = conn.query_row(&sql_select, rusqlite::params![oid.to_string()], |row| row.get(0))
            .optional()?
            .ok_or_else(|| StoreError::NotFound(oid.to_string()))?;
        
        let mut obj_json: serde_json::Value = serde_json::from_str(&data_str)?;
        let mut meta = if let Some(meta_val) = obj_json.get("meta").and_then(|m| m.as_object()) {
            MetaMap(meta_val.clone())
        } else {
            MetaMap::new()
        };
        
        merge_meta(&mut meta, updates);
        
        obj_json["meta"] = serde_json::Value::Object(meta.0);
        let current_version = obj_json.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
        let new_version = current_version + 1;
        obj_json["version"] = serde_json::json!(new_version);
        let new_data_str = serde_json::to_string(&obj_json)?;
        
        let sql_update = format!("UPDATE db_{} SET data = ?1, version = ?2 WHERE oid = ?3", otype);
        let _count = conn.execute(&sql_update, rusqlite::params![new_data_str, new_version, oid.to_string()])?;
        
        Ok(())
    }

    pub fn with_tx<F, R>(&self, f: F) -> Result<R, StoreError>
    where
        F: FnOnce(&Transaction<'_>) -> Result<R, StoreError>,
    {
        let mut conn = self.conn.lock()?;
        let tx = conn.transaction()?;
        match f(&tx) {
            Ok(r) => {
                tx.commit()?;
                Ok(r)
            }
            Err(e) => {
                tx.rollback()?;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Point, WinSize};

    #[test]
    fn test_open_in_memory() {
        let store = WaveStore::open_in_memory().unwrap();
        assert!(store.ensure_tables().is_ok());
    }

    #[test]
    fn test_insert_get_block() {
        let store = WaveStore::open_in_memory().unwrap();
        let mut block = Block {
            oid: Uuid::new_v4(),
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![],
            job_id: None,
        };
        store.db_insert(&mut block).unwrap();
        assert_eq!(block.version, 1);
        
        let fetched: Block = store.db_get(&block.oid).unwrap().unwrap();
        assert_eq!(fetched.oid, block.oid);
        assert_eq!(fetched.version, 1);
    }

    #[test]
    fn test_insert_workspace() {
        let store = WaveStore::open_in_memory().unwrap();
        let mut workspace = Workspace {
            oid: Uuid::new_v4(),
            version: 0,
            name: Some("test".to_string()),
            icon: None,
            color: None,
            tab_ids: vec!["tab1".to_string()],
            active_tab_id: "tab1".to_string(),
            meta: MetaMap::new(),
        };
        store.db_insert(&mut workspace).unwrap();
        
        let fetched: Workspace = store.db_get(&workspace.oid).unwrap().unwrap();
        assert_eq!(fetched.tab_ids, vec!["tab1".to_string()]);
    }

    #[test]
    fn test_update() {
        let store = WaveStore::open_in_memory().unwrap();
        let mut block = Block {
            oid: Uuid::new_v4(),
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![],
            job_id: None,
        };
        store.db_insert(&mut block).unwrap();
        
        block.sub_block_ids = vec!["sub1".to_string()];
        store.db_update(&mut block).unwrap();
        assert_eq!(block.version, 2);
        
        let fetched: Block = store.db_get(&block.oid).unwrap().unwrap();
        assert_eq!(fetched.version, 2);
        assert_eq!(fetched.sub_block_ids, vec!["sub1".to_string()]);
    }

    #[test]
    fn test_delete() {
        let store = WaveStore::open_in_memory().unwrap();
        let mut block = Block {
            oid: Uuid::new_v4(),
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![],
            job_id: None,
        };
        store.db_insert(&mut block).unwrap();
        
        let deleted = store.db_delete("block", &block.oid).unwrap();
        assert!(deleted);
        
        let fetched: Option<Block> = store.db_get(&block.oid).unwrap();
        assert!(fetched.is_none());
        
        let deleted_again = store.db_delete("block", &block.oid).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_db_must_get() {
        let store = WaveStore::open_in_memory().unwrap();
        let result = store.db_must_get::<Block>(&Uuid::new_v4());
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[test]
    fn test_db_get_all() {
        let store = WaveStore::open_in_memory().unwrap();
        let mut block1 = Block {
            oid: Uuid::new_v4(),
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![],
            job_id: None,
        };
        let mut block2 = Block {
            oid: Uuid::new_v4(),
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![],
            job_id: None,
        };
        store.db_insert(&mut block1).unwrap();
        store.db_insert(&mut block2).unwrap();
        
        let all: Vec<Block> = store.db_get_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_find_workspace_for_tab() {
        let store = WaveStore::open_in_memory().unwrap();
        let tab_oid = Uuid::new_v4();
        let mut workspace = Workspace {
            oid: Uuid::new_v4(),
            version: 0,
            name: None,
            icon: None,
            color: None,
            tab_ids: vec![tab_oid.to_string()],
            active_tab_id: tab_oid.to_string(),
            meta: MetaMap::new(),
        };
        store.db_insert(&mut workspace).unwrap();
        
        let fetched = store.find_workspace_for_tab(&tab_oid).unwrap().unwrap();
        assert_eq!(fetched.oid, workspace.oid);
    }

    #[test]
    fn test_find_window_for_workspace() {
        let store = WaveStore::open_in_memory().unwrap();
        let workspace_oid = Uuid::new_v4();
        let mut window = Window {
            oid: Uuid::new_v4(),
            version: 0,
            workspace_id: workspace_oid.to_string(),
            is_new: None,
            pos: Point { x: 0, y: 0 },
            win_size: WinSize { width: 100, height: 100 },
            last_focus_ts: 0,
            meta: MetaMap::new(),
        };
        store.db_insert(&mut window).unwrap();
        
        let fetched = store.find_window_for_workspace(&workspace_oid).unwrap().unwrap();
        assert_eq!(fetched.oid, window.oid);
    }

    #[test]
    fn test_update_object_meta() {
        let store = WaveStore::open_in_memory().unwrap();
        let mut block = Block {
            oid: Uuid::new_v4(),
            parent_oref: None,
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![],
            job_id: None,
        };
        store.db_insert(&mut block).unwrap();
        
        let mut updates = MetaMap::new();
        updates.set("test_key", "test_val");
        store.update_object_meta("block", &block.oid, &updates).unwrap();
        
        let fetched: Block = store.db_get(&block.oid).unwrap().unwrap();
        assert_eq!(fetched.version, 2);
        assert_eq!(fetched.meta.get_string("test_key").unwrap(), "test_val");
    }

    #[test]
    fn test_transaction_rollback() {
        let store = WaveStore::open_in_memory().unwrap();
        let block_oid = Uuid::new_v4();
        
        let _ = store.with_tx(|tx| {
            let sql = format!("INSERT INTO db_block (oid, version, data) VALUES (?1, 1, ?2)");
            tx.execute(&sql, rusqlite::params![block_oid.to_string(), "{}"]).unwrap();
            Err::<(), _>(StoreError::NotFound("test".to_string()))
        });
        
        let fetched: Option<Block> = store.db_get(&block_oid).unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_transaction_commit() {
        let store = WaveStore::open_in_memory().unwrap();
        let block_oid = Uuid::new_v4();
        let block = Block {
            oid: block_oid,
            parent_oref: None,
            version: 1,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![],
            job_id: None,
        };
        
        store.with_tx(|tx| {
            let data = serde_json::to_string(&block).unwrap();
            let sql = format!("INSERT INTO db_block (oid, version, data) VALUES (?1, 1, ?2)");
            tx.execute(&sql, rusqlite::params![block_oid.to_string(), data]).unwrap();
            Ok::<(), StoreError>(())
        }).unwrap();
        
        let fetched: Option<Block> = store.db_get(&block_oid).unwrap();
        assert!(fetched.is_some());
    }

    #[test]
    fn test_full_hierarchy() {
        let store = WaveStore::open_in_memory().unwrap();
        
        let window_oid = Uuid::new_v4();
        let workspace_oid = Uuid::new_v4();
        let tab_oid = Uuid::new_v4();
        let block1_oid = Uuid::new_v4();
        let block2_oid = Uuid::new_v4();
        
        let mut window = Window {
            oid: window_oid,
            version: 0,
            workspace_id: workspace_oid.to_string(),
            is_new: None,
            pos: Point { x: 0, y: 0 },
            win_size: WinSize { width: 100, height: 100 },
            last_focus_ts: 0,
            meta: MetaMap::new(),
        };
        
        let mut workspace = Workspace {
            oid: workspace_oid,
            version: 0,
            name: None,
            icon: None,
            color: None,
            tab_ids: vec![tab_oid.to_string()],
            active_tab_id: tab_oid.to_string(),
            meta: MetaMap::new(),
        };
        
        let mut tab = Tab {
            oid: tab_oid,
            version: 0,
            name: "tab1".to_string(),
            layout_state: Uuid::new_v4().to_string(),
            block_ids: vec![block1_oid.to_string()],
            meta: MetaMap::new(),
        };
        
        let mut block1 = Block {
            oid: block1_oid,
            parent_oref: Some(format!("tab:{}", tab_oid)),
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![block2_oid.to_string()],
            job_id: None,
        };
        
        let mut block2 = Block {
            oid: block2_oid,
            parent_oref: Some(format!("block:{}", block1_oid)),
            version: 0,
            runtime_opts: None,
            stickers: None,
            meta: MetaMap::new(),
            sub_block_ids: vec![],
            job_id: None,
        };
        
        store.db_insert(&mut window).unwrap();
        store.db_insert(&mut workspace).unwrap();
        store.db_insert(&mut tab).unwrap();
        store.db_insert(&mut block1).unwrap();
        store.db_insert(&mut block2).unwrap();
        
        let found_tab = store.find_tab_for_block(&block2_oid).unwrap().unwrap();
        assert_eq!(found_tab.oid, tab_oid);
        
        let found_workspace = store.find_workspace_for_tab(&tab_oid).unwrap().unwrap();
        assert_eq!(found_workspace.oid, workspace_oid);
        
        let found_window = store.find_window_for_workspace(&workspace_oid).unwrap().unwrap();
        assert_eq!(found_window.oid, window_oid);
    }
}
