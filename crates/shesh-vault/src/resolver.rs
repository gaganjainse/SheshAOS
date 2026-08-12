//! Dynamic placeholder parameter resolver for commands (<container_id>, <branch_name>).

use std::collections::HashMap;

use regex::Regex;

/// Parameter resolver for placeholder substitution in command templates.
pub struct ParameterResolver;

impl ParameterResolver {
    /// Extract all placeholder parameters (e.g. `<container>`, `<port>`) from a template.
    pub fn extract_placeholders(template: &str) -> Vec<String> {
        // The pattern is a fixed constant, so a compile error here is a bug in
        // this file, not runtime input — test_placeholder_pattern_compiles
        // covers it. Returning an empty list keeps the production path total
        // without panicking (workspace lint forbids unwrap/panic).
        let Ok(re) = Regex::new(r"<([a-zA-Z0-9_]+)>") else {
            return Vec::new();
        };
        re.captures_iter(template)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }

    /// Substitute placeholders in a template with supplied parameter values.
    pub fn resolve(template: &str, params: &HashMap<String, String>) -> String {
        let mut resolved = template.to_string();
        for (key, value) in params {
            let placeholder = format!("<{}>", key);
            resolved = resolved.replace(&placeholder, value);
        }
        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_pattern_compiles() {
        // Guards the `let Ok(re)` guard in extract_placeholders: if this fails,
        // the extractor silently returns empty vectors everywhere.
        assert!(regex::Regex::new(r"<([a-zA-Z0-9_]+)>").is_ok());
    }

    #[test]
    fn test_extract_placeholders() {
        let template = "docker exec -it <container_id> ffmpeg -i <input_file> -p <port>";
        let params = ParameterResolver::extract_placeholders(template);
        assert_eq!(params, vec!["container_id", "input_file", "port"]);
    }

    #[test]
    fn test_extract_placeholders_empty_template() {
        let params = ParameterResolver::extract_placeholders("");
        assert!(params.is_empty());
    }

    #[test]
    fn test_extract_placeholders_no_placeholders() {
        let template = "docker exec -it mycontainer ffmpeg -i input.mp4";
        let params = ParameterResolver::extract_placeholders(template);
        assert!(params.is_empty());
    }

    #[test]
    fn test_extract_placeholders_single_placeholder() {
        let template = "echo <message>";
        let params = ParameterResolver::extract_placeholders(template);
        assert_eq!(params, vec!["message"]);
    }

    #[test]
    fn test_extract_placeholders_repeated_names() {
        let template = "<x> and <x> and <y>";
        let params = ParameterResolver::extract_placeholders(template);
        assert_eq!(params, vec!["x", "x", "y"]);
    }

    #[test]
    fn test_extract_placeholders_with_numbers() {
        let template = "arg1 <arg_1> arg2 <arg_2>";
        let params = ParameterResolver::extract_placeholders(template);
        assert_eq!(params, vec!["arg_1", "arg_2"]);
    }

    #[test]
    fn test_extract_placeholders_with_underscores() {
        let template = "process <my_var> with <other_var>";
        let params = ParameterResolver::extract_placeholders(template);
        assert_eq!(params, vec!["my_var", "other_var"]);
    }

    #[test]
    fn test_extract_placeholders_adjacent() {
        let template = "<a><b><c>";
        let params = ParameterResolver::extract_placeholders(template);
        assert_eq!(params, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_resolve_template() {
        let template = "docker exec -it <container> /bin/bash";
        let mut params = HashMap::new();
        params.insert("container".to_string(), "my-app-1".to_string());

        let resolved = ParameterResolver::resolve(template, &params);
        assert_eq!(resolved, "docker exec -it my-app-1 /bin/bash");
    }

    #[test]
    fn test_resolve_template_empty_params() {
        let template = "echo hello world";
        let params = HashMap::new();
        let resolved = ParameterResolver::resolve(template, &params);
        assert_eq!(resolved, "echo hello world");
    }

    #[test]
    fn test_resolve_template_no_placeholders() {
        let template = "ls -la /tmp";
        let mut params = HashMap::new();
        params.insert("unused".to_string(), "value".to_string());
        let resolved = ParameterResolver::resolve(template, &params);
        assert_eq!(resolved, "ls -la /tmp");
    }

    #[test]
    fn test_resolve_template_multiple_placeholders() {
        let template = "scp <user>@<host>:<path> <dest>";
        let mut params = HashMap::new();
        params.insert("user".to_string(), "alice".to_string());
        params.insert("host".to_string(), "192.168.1.1".to_string());
        params.insert("path".to_string(), "/data/file.txt".to_string());
        params.insert("dest".to_string(), "/local/".to_string());

        let resolved = ParameterResolver::resolve(template, &params);
        assert_eq!(resolved, "scp alice@192.168.1.1:/data/file.txt /local/");
    }

    #[test]
    fn test_resolve_template_special_characters_in_values() {
        let template = "echo <text>";
        let mut params = HashMap::new();
        params.insert("text".to_string(), "hello world! @#$%".to_string());
        let resolved = ParameterResolver::resolve(template, &params);
        assert_eq!(resolved, "echo hello world! @#$%");
    }

    #[test]
    fn test_resolve_template_overlapping_placeholders() {
        let template = "<port> <port_range>";
        let mut params = HashMap::new();
        params.insert("port".to_string(), "8080".to_string());
        params.insert("port_range".to_string(), "8000-9000".to_string());
        let resolved = ParameterResolver::resolve(template, &params);
        assert_eq!(resolved, "8080 8000-9000");
    }

    #[test]
    fn test_resolve_template_empty_template() {
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());
        let resolved = ParameterResolver::resolve("", &params);
        assert_eq!(resolved, "");
    }

    #[test]
    fn test_resolve_template_placeholder_at_boundaries() {
        let template = "<start>middle<end>";
        let mut params = HashMap::new();
        params.insert("start".to_string(), "A".to_string());
        params.insert("end".to_string(), "Z".to_string());
        let resolved = ParameterResolver::resolve(template, &params);
        assert_eq!(resolved, "AmiddleZ");
    }

    #[test]
    fn test_extract_and_resolve_roundtrip() {
        let template = "cmd <arg1> <arg2> --flag";
        let params = ParameterResolver::extract_placeholders(template);
        let mut map = HashMap::new();
        for (i, p) in params.iter().enumerate() {
            map.insert(p.clone(), format!("val{}", i));
        }
        let resolved = ParameterResolver::resolve(template, &map);
        assert!(resolved.contains("val0"));
        assert!(resolved.contains("val1"));
        assert!(resolved.contains("--flag"));
    }
}
