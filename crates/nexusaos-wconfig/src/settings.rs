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
    use std::collections::HashMap;

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

    #[test]
    fn test_term_settings_default() {
        let term = TermSettings::default();
        assert_eq!(term.font_size, Some(14.0));
        assert_eq!(term.font_family, None);
        assert_eq!(term.theme, Some("dark".to_string()));
        assert_eq!(term.scrollback, Some(10000));
    }

    #[test]
    fn test_ai_settings_default() {
        let ai = AiSettings::default();
        assert_eq!(ai.model, Some("gpt-4".to_string()));
        assert_eq!(ai.max_tokens, None);
        assert_eq!(ai.base_url, None);
    }

    #[test]
    fn test_editor_settings_default() {
        let editor = EditorSettings::default();
        assert_eq!(editor.minimap, Some(true));
        assert_eq!(editor.word_wrap, Some(false));
    }

    #[test]
    fn test_global_settings_default_extras_empty() {
        let global = GlobalSettings::default();
        assert!(global.extras.is_empty());
    }

    #[test]
    fn test_term_settings_merge_preserves_some_values() {
        let mut base = TermSettings::default();
        let override_settings = TermSettings {
            font_size: Some(18.0),
            font_family: Some("JetBrains Mono".to_string()),
            theme: None,
            scrollback: None,
        };
        base.merge(override_settings);
        assert_eq!(base.font_size, Some(18.0));
        assert_eq!(base.font_family, Some("JetBrains Mono".to_string()));
        assert_eq!(base.theme, Some("dark".to_string())); // preserved from default
        assert_eq!(base.scrollback, Some(10000)); // preserved from default
    }

    #[test]
    fn test_term_settings_merge_all_none_preserves_base() {
        let mut base = TermSettings {
            font_size: Some(16.0),
            font_family: Some("Fira Code".to_string()),
            theme: Some("light".to_string()),
            scrollback: Some(5000),
        };
        let override_settings = TermSettings {
            font_size: None,
            font_family: None,
            theme: None,
            scrollback: None,
        };
        base.merge(override_settings);
        assert_eq!(base.font_size, Some(16.0));
        assert_eq!(base.font_family, Some("Fira Code".to_string()));
        assert_eq!(base.theme, Some("light".to_string()));
        assert_eq!(base.scrollback, Some(5000));
    }

    #[test]
    fn test_ai_settings_merge_preserves_some_values() {
        let mut base = AiSettings::default();
        let override_settings = AiSettings {
            model: Some("claude-3".to_string()),
            max_tokens: Some(4096),
            base_url: None,
        };
        base.merge(override_settings);
        assert_eq!(base.model, Some("claude-3".to_string()));
        assert_eq!(base.max_tokens, Some(4096));
        assert_eq!(base.base_url, None);
    }

    #[test]
    fn test_editor_settings_merge_preserves_some_values() {
        let mut base = EditorSettings::default();
        let override_settings = EditorSettings {
            minimap: Some(false),
            word_wrap: None,
        };
        base.merge(override_settings);
        assert_eq!(base.minimap, Some(false));
        assert_eq!(base.word_wrap, Some(false)); // preserved from default
    }

    #[test]
    fn test_global_settings_merge_preserves_extras() {
        let mut base = GlobalSettings::default();
        base.extras.insert("custom:key".to_string(), serde_json::json!("base_value"));

        let overrides_json = r#"{"term:theme": "light"}"#;
        let overrides: GlobalSettings = serde_json::from_str(overrides_json).unwrap();

        base.merge(overrides);
        assert_eq!(base.extras.get("custom:key").unwrap(), &serde_json::json!("base_value"));
        assert_eq!(base.term.theme, Some("light".to_string()));
    }

    #[test]
    fn test_global_settings_merge_overrides_extras() {
        let mut base = GlobalSettings::default();
        base.extras.insert("key".to_string(), serde_json::json!("old"));

        let overrides_json = r#"{"extra:key": "new"}"#;
        let overrides: GlobalSettings = serde_json::from_str(overrides_json).unwrap();

        base.merge(overrides);
        assert_eq!(base.extras.get("extra:key").unwrap(), &serde_json::json!("new"));
        assert_eq!(base.extras.get("key").unwrap(), &serde_json::json!("old"));
    }

    #[test]
    fn test_term_settings_serialization_roundtrip() {
        let term = TermSettings {
            font_size: Some(16.0),
            font_family: Some("Monaco".to_string()),
            theme: Some("light".to_string()),
            scrollback: Some(5000),
        };
        let json = serde_json::to_string(&term).unwrap();
        let decoded: TermSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.font_size, term.font_size);
        assert_eq!(decoded.font_family, term.font_family);
        assert_eq!(decoded.theme, term.theme);
        assert_eq!(decoded.scrollback, term.scrollback);
    }

    #[test]
    fn test_ai_settings_serialization_roundtrip() {
        let ai = AiSettings {
            model: Some("gpt-4".to_string()),
            max_tokens: Some(2048),
            base_url: Some("https://api.openai.com".to_string()),
        };
        let json = serde_json::to_string(&ai).unwrap();
        let decoded: AiSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.model, ai.model);
        assert_eq!(decoded.max_tokens, ai.max_tokens);
        assert_eq!(decoded.base_url, ai.base_url);
    }

    #[test]
    fn test_editor_settings_serialization_roundtrip() {
        let editor = EditorSettings {
            minimap: Some(false),
            word_wrap: Some(true),
        };
        let json = serde_json::to_string(&editor).unwrap();
        let decoded: EditorSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.minimap, editor.minimap);
        assert_eq!(decoded.word_wrap, editor.word_wrap);
    }

    #[test]
    fn test_global_settings_serialization_roundtrip() {
        let global = GlobalSettings {
            term: TermSettings {
                font_size: Some(14.0),
                font_family: None,
                theme: Some("dark".to_string()),
                scrollback: Some(10000),
            },
            ai: AiSettings {
                model: Some("gpt-4".to_string()),
                max_tokens: None,
                base_url: None,
            },
            editor: EditorSettings {
                minimap: Some(true),
                word_wrap: Some(false),
            },
            extras: HashMap::new(),
        };
        let json = serde_json::to_string(&global).unwrap();
        let decoded: GlobalSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.term.font_size, global.term.font_size);
        assert_eq!(decoded.term.theme, global.term.theme);
        assert_eq!(decoded.ai.model, global.ai.model);
        assert_eq!(decoded.editor.minimap, global.editor.minimap);
    }

    #[test]
    fn test_term_settings_partial_serialization() {
        let term = TermSettings {
            font_size: Some(16.0),
            font_family: None,
            theme: None,
            scrollback: None,
        };
        let json = serde_json::to_string(&term).unwrap();
        let decoded: TermSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.font_size, Some(16.0));
        assert_eq!(decoded.font_family, None);
        assert_eq!(decoded.theme, None);
        assert_eq!(decoded.scrollback, None);
    }

    #[test]
    fn test_global_settings_clone() {
        let base = GlobalSettings::default();
        let cloned = base.clone();
        assert_eq!(base.term.font_size, cloned.term.font_size);
        assert_eq!(base.ai.model, cloned.ai.model);
        assert_eq!(base.editor.minimap, cloned.editor.minimap);
    }

    #[test]
    fn test_global_settings_debug() {
        let global = GlobalSettings::default();
        let debug_str = format!("{:?}", global);
        assert!(debug_str.contains("GlobalSettings"));
    }

    #[test]
    fn test_term_settings_serde_rename() {
        let json_str = r#"{"term:fontsize": 20.0}"#;
        let term: TermSettings = serde_json::from_str(json_str).unwrap();
        assert_eq!(term.font_size, Some(20.0));
    }

    #[test]
    fn test_ai_settings_serde_rename() {
        let json_str = r#"{"ai:model": "claude-3", "ai:maxtokens": 8192}"#;
        let ai: AiSettings = serde_json::from_str(json_str).unwrap();
        assert_eq!(ai.model, Some("claude-3".to_string()));
        assert_eq!(ai.max_tokens, Some(8192));
    }

    #[test]
    fn test_editor_settings_serde_rename() {
        let json_str = r#"{"editor:minimap": false, "editor:wordwrap": true}"#;
        let editor: EditorSettings = serde_json::from_str(json_str).unwrap();
        assert_eq!(editor.minimap, Some(false));
        assert_eq!(editor.word_wrap, Some(true));
    }

    #[test]
    fn test_global_settings_empty_json() {
        let json_str = r#"{}"#;
        let settings: GlobalSettings = serde_json::from_str(json_str).unwrap();
        assert_eq!(settings.term.font_size, None);
        assert_eq!(settings.term.theme, None);
        assert_eq!(settings.ai.model, None);
        assert_eq!(settings.editor.minimap, None);
    }

    #[test]
    fn test_global_settings_partial_json_override() {
        let mut base = GlobalSettings::default();
        let override_json = r#"{"term:scrollback": 50000}"#;
        let overrides: GlobalSettings = serde_json::from_str(override_json).unwrap();
        base.merge(overrides);
        assert_eq!(base.term.scrollback, Some(50000));
        assert_eq!(base.term.font_size, Some(14.0)); // unchanged
        assert_eq!(base.term.theme, Some("dark".to_string())); // unchanged
    }
}
