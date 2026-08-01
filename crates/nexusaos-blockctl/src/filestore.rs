use std::collections::HashMap;
use std::sync::RwLock;

pub struct BlockFileStore {
    zones: RwLock<HashMap<String, ZoneData>>,
}

struct ZoneData {
    data: Vec<u8>,
    max_size: usize,
}

impl BlockFileStore {
    pub fn new() -> Self {
        Self {
            zones: RwLock::new(HashMap::new()),
        }
    }

    pub fn append(&self, block_id: &str, data: &[u8]) {
        let mut zones = self.zones.write().unwrap();
        let zone = zones.entry(block_id.to_string()).or_insert_with(|| ZoneData {
            data: Vec::new(),
            max_size: 1_048_576,
        });

        zone.data.extend_from_slice(data);
        if zone.data.len() > zone.max_size {
            let overflow = zone.data.len() - zone.max_size;
            zone.data.drain(..overflow);
        }
    }

    pub fn read_all(&self, block_id: &str) -> Option<Vec<u8>> {
        let zones = self.zones.read().unwrap();
        zones.get(block_id).map(|zone| zone.data.clone())
    }

    pub fn read_tail(&self, block_id: &str, max_bytes: usize) -> Option<Vec<u8>> {
        let zones = self.zones.read().unwrap();
        zones.get(block_id).map(|zone| {
            let start = if zone.data.len() > max_bytes {
                zone.data.len() - max_bytes
            } else {
                0
            };
            zone.data[start..].to_vec()
        })
    }

    pub fn truncate(&self, block_id: &str) {
        let mut zones = self.zones.write().unwrap();
        if let Some(zone) = zones.get_mut(block_id) {
            zone.data.clear();
        }
    }

    pub fn delete_zone(&self, block_id: &str) {
        let mut zones = self.zones.write().unwrap();
        zones.remove(block_id);
    }

    pub fn zone_size(&self, block_id: &str) -> usize {
        let zones = self.zones.read().unwrap();
        zones.get(block_id).map(|zone| zone.data.len()).unwrap_or(0)
    }

    pub fn set_max_size(&self, block_id: &str, max_size: usize) {
        let mut zones = self.zones.write().unwrap();
        let zone = zones.entry(block_id.to_string()).or_insert_with(|| ZoneData {
            data: Vec::new(),
            max_size,
        });
        zone.max_size = max_size;
        if zone.data.len() > max_size {
            let overflow = zone.data.len() - max_size;
            zone.data.drain(..overflow);
        }
    }
}

impl Default for BlockFileStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_read_all() {
        let store = BlockFileStore::new();
        store.append("blk1", b"hello");
        assert_eq!(store.read_all("blk1").unwrap(), b"hello");
    }

    #[test]
    fn test_truncate_exceeding_max_size() {
        let store = BlockFileStore::new();
        store.set_max_size("blk1", 5);
        store.append("blk1", b"hello world");
        assert_eq!(store.read_all("blk1").unwrap(), b"world");
    }

    #[test]
    fn test_read_tail() {
        let store = BlockFileStore::new();
        store.append("blk1", b"1234567890");
        assert_eq!(store.read_tail("blk1", 3).unwrap(), b"890");
    }

    #[test]
    fn test_truncate_and_delete() {
        let store = BlockFileStore::new();
        store.append("blk1", b"123");
        store.truncate("blk1");
        assert_eq!(store.read_all("blk1").unwrap(), b"");
        store.delete_zone("blk1");
        assert!(store.read_all("blk1").is_none());
    }

    #[test]
    fn test_multiple_zones() {
        let store = BlockFileStore::new();
        store.append("blk1", b"aaa");
        store.append("blk2", b"bbb");
        assert_eq!(store.read_all("blk1").unwrap(), b"aaa");
        assert_eq!(store.read_all("blk2").unwrap(), b"bbb");
    }
}
