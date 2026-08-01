use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TermSettings {
    #[serde(rename = "term:fontsize", skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(rename = "term:fontfamily", skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(rename = "term:theme", skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(rename = "term:scrollback", skip_serializing_if = "Option::is_none")]
    pub scrollback: Option<i64>,
}

impl Default for TermSettings {
    fn default() -> Self {
        Self {
            font_size: Some(14.0),
            font_family: None,
            theme: Some("dark".to_string()),
            scrollback: Some(10000),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiSettings {
    #[serde(rename = "ai:model", skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(rename = "ai:maxtokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
    #[serde(rename = "ai:baseurl", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            model: Some("gpt-4".to_string()),
            max_tokens: None,
            base_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorSettings {
    #[serde(rename = "editor:minimap", skip_serializing_if = "Option::is_none")]
    pub minimap: Option<bool>,
    #[serde(rename = "editor:wordwrap", skip_serializing_if = "Option::is_none")]
    pub word_wrap: Option<bool>,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            minimap: Some(true),
            word_wrap: Some(false),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GlobalSettings {
    #[serde(flatten)]
    pub term: TermSettings,
    #[serde(flatten)]
    pub ai: AiSettings,
    #[serde(flatten)]
    pub editor: EditorSettings,
    
    #[serde(flatten)]
    pub extras: std::collections::HashMap<String, serde_json::Value>,
}

pub trait MergeSettings {
    fn merge(&mut self, other: Self);
}

impl MergeSettings for TermSettings {
    fn merge(&mut self, other: Self) {
        if other.font_size.is_some() { self.font_size = other.font_size; }
        if other.font_family.is_some() { self.font_family = other.font_family; }
        if other.theme.is_some() { self.theme = other.theme; }
        if other.scrollback.is_some() { self.scrollback = other.scrollback; }
    }
}

impl MergeSettings for AiSettings {
    fn merge(&mut self, other: Self) {
        if other.model.is_some() { self.model = other.model; }
        if other.max_tokens.is_some() { self.max_tokens = other.max_tokens; }
        if other.base_url.is_some() { self.base_url = other.base_url; }
    }
}

impl MergeSettings for EditorSettings {
    fn merge(&mut self, other: Self) {
        if other.minimap.is_some() { self.minimap = other.minimap; }
        if other.word_wrap.is_some() { self.word_wrap = other.word_wrap; }
    }
}

impl MergeSettings for GlobalSettings {
    fn merge(&mut self, other: Self) {
        self.term.merge(other.term);
        self.ai.merge(other.ai);
        self.editor.merge(other.editor);
        for (k, v) in other.extras {
            self.extras.insert(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let default_settings = GlobalSettings::default();
        assert_eq!(default_settings.term.font_size, Some(14.0));
        assert_eq!(default_settings.term.theme, Some("dark".to_string()));
        assert_eq!(default_settings.term.scrollback, Some(10000));
        assert_eq!(default_settings.editor.minimap, Some(true));
        assert_eq!(default_settings.editor.word_wrap, Some(false));
        assert_eq!(default_settings.ai.model, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_json_parsing() {
        let json_str = r#"{
            "term:fontsize": 16.5,
            "term:fontfamily": "Fira Code",
            "ai:model": "gpt-4o",
            "editor:wordwrap": true,
            "unknown:key": "value"
        }"#;

        let settings: GlobalSettings = serde_json::from_str(json_str).unwrap();
        assert_eq!(settings.term.font_size, Some(16.5));
        assert_eq!(settings.term.font_family, Some("Fira Code".to_string()));
        assert_eq!(settings.term.theme, None); // Not provided
        assert_eq!(settings.ai.model, Some("gpt-4o".to_string()));
        assert_eq!(settings.editor.word_wrap, Some(true));
        assert_eq!(settings.editor.minimap, None); // Not provided
        assert_eq!(settings.extras.get("unknown:key").unwrap(), &serde_json::json!("value"));
    }

    #[test]
    fn test_merging() {
        let mut base = GlobalSettings::default();
        let override_json = r#"{
            "term:theme": "light",
            "ai:model": "claude-3-opus"
        }"#;
        let overrides: GlobalSettings = serde_json::from_str(override_json).unwrap();
        
        base.merge(overrides);
        
        assert_eq!(base.term.theme, Some("light".to_string()));
        assert_eq!(base.term.font_size, Some(14.0)); // From default
        assert_eq!(base.ai.model, Some("claude-3-opus".to_string()));
        assert_eq!(base.editor.minimap, Some(true)); // From default
    }
}
