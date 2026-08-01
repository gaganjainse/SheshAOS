use ratatui::layout::{Constraint, Direction, Layout, Rect};
use serde_json::Value;

pub fn map_layout_tree(node: &Value, area: Rect) -> Vec<(String, Rect)> {
    let mut results = Vec::new();
    
    // If it's a leaf node containing a block_id
    if let Some(block_id) = node.get("blockId").and_then(|v| v.as_str()) {
        results.push((block_id.to_string(), area));
        return results;
    }
    
    // If it's a split node
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        let direction = match node.get("direction").and_then(|v| v.as_str()) {
            Some("horizontal") => Direction::Horizontal,
            _ => Direction::Vertical,
        };
        
        if children.is_empty() {
            return results;
        }
        
        let mut constraints = Vec::new();
        for _ in 0..children.len() {
            constraints.push(Constraint::Ratio(1, children.len() as u32));
        }
        
        let chunks = Layout::default()
            .direction(direction)
            .constraints(constraints)
            .split(area);
            
        for (child_node, chunk) in children.iter().zip(chunks.iter()) {
            results.extend(map_layout_tree(child_node, *chunk));
        }
    } else {
        // Fallback for an unknown leaf, we just collect it if it has an id
        if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
            results.push((id.to_string(), area));
        }
    }
    
    results
}

use nexusaos_waveobj::store::WaveStore;
use nexusaos_waveobj::types::{Workspace, Tab, LayoutState};
use uuid::Uuid;
use std::sync::Arc;
use tracing::warn;

pub fn get_current_layout(store: Arc<WaveStore>) -> Option<LayoutState> {
    let workspaces: Vec<Workspace> = match store.db_get_all() {
        Ok(ws) => ws,
        Err(e) => {
            warn!("Failed to query workspaces: {}", e);
            return None;
        }
    };
    
    let current_ws = workspaces.into_iter().next()?;
    let active_tab_id = current_ws.active_tab_id.clone();
    
    let tab_uuid = match Uuid::parse_str(&active_tab_id) {
        Ok(u) => u,
        Err(_) => return None,
    };
    
    let tab: Tab = store.db_get(&tab_uuid).ok().flatten()?;
    
    let layout_uuid = match Uuid::parse_str(&tab.layout_state) {
        Ok(u) => u,
        Err(_) => return None,
    };
    
    store.db_get(&layout_uuid).ok().flatten()
}
