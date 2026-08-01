//! AI flag inspector & dry-run explanation engine.

/// Flag inspector for breaking down complex CLI options before execution.
pub struct FlagInspector;

impl FlagInspector {
    /// Explain flags in a CLI command string for dry-run inspection.
    pub fn explain_flags(command: &str) -> Vec<(String, String)> {
        let mut explanations = Vec::new();
        let tokens: Vec<&str> = command.split_whitespace().collect();

        for token in tokens {
            if token.starts_with("--") {
                let explanation = match token {
                    "--recursive" => "Operate recursively on subdirectories",
                    "--force" => "Force operation without prompt or error",
                    "--verbose" => "Enable verbose progress output",
                    "--yes" => "Automatically answer yes to all prompts",
                    "--delete" => "Delete extraneous files from target destination",
                    "--archive" => "Archive mode; preserves permissions, times, and symlinks",
                    _ => "Command line long option parameter",
                };
                explanations.push((token.to_string(), explanation.to_string()));
            } else if token.starts_with('-') && token.len() > 1 {
                for ch in token[1..].chars() {
                    let flag_str = format!("-{}", ch);
                    let explanation = match ch {
                        'r' | 'R' => "Operate recursively on subdirectories",
                        'f' => "Force operation without prompt or error",
                        'v' => "Enable verbose progress output",
                        'y' => "Automatically answer yes to all prompts",
                        'a' => "Archive mode; preserves permissions, times, and symlinks",
                        'z' => "Compress file data during transfer",
                        _ => "Command line short option flag",
                    };
                    explanations.push((flag_str, explanation.to_string()));
                }
            }
        }

        explanations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_explanation() {
        let cmd = "rsync -avz --delete dir1/ dir2/";
        let flags = FlagInspector::explain_flags(cmd);
        assert!(flags.iter().any(|(f, _)| f == "-a"));
        assert!(flags.iter().any(|(f, _)| f == "--delete"));
    }
}
