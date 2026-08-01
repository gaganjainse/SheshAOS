//! Dynamic placeholder parameter resolver for commands (<container_id>, <branch_name>).

use std::collections::HashMap;

use regex::Regex;

/// Parameter resolver for placeholder substitution in command templates.
pub struct ParameterResolver;

impl ParameterResolver {
    /// Extract all placeholder parameters (e.g. `<container>`, `<port>`) from a template.
    pub fn extract_placeholders(template: &str) -> Vec<String> {
        let re = Regex::new(r"<([a-zA-Z0-9_]+)>").unwrap();
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
    fn test_extract_placeholders() {
        let template = "docker exec -it <container_id> ffmpeg -i <input_file> -p <port>";
        let params = ParameterResolver::extract_placeholders(template);
        assert_eq!(params, vec!["container_id", "input_file", "port"]);
    }

    #[test]
    fn test_resolve_template() {
        let template = "docker exec -it <container> /bin/bash";
        let mut params = HashMap::new();
        params.insert("container".to_string(), "my-app-1".to_string());

        let resolved = ParameterResolver::resolve(template, &params);
        assert_eq!(resolved, "docker exec -it my-app-1 /bin/bash");
    }
}
