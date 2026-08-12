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

    #[test]
    fn test_explain_flags_empty_command() {
        let flags = FlagInspector::explain_flags("");
        assert!(flags.is_empty());
    }

    #[test]
    fn test_explain_flags_no_flags() {
        let cmd = "ls -la /tmp";
        let flags = FlagInspector::explain_flags(cmd);
        // -l and -a are flags
        assert!(!flags.is_empty());
    }

    #[test]
    fn test_explain_flags_plain_command() {
        let cmd = "echo hello world";
        let flags = FlagInspector::explain_flags(cmd);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_explain_flags_single_long_flag() {
        let cmd = "ls --all";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].0, "--all");
        assert_eq!(flags[0].1, "Command line long option parameter");
    }

    #[test]
    fn test_explain_flags_single_short_flag() {
        let cmd = "ls -l";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].0, "-l");
        assert_eq!(flags[0].1, "Command line short option flag");
    }

    #[test]
    fn test_explain_flags_combined_short_flags() {
        let cmd = "ls -la";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 2);
        assert!(flags.iter().any(|(f, _)| f == "-l"));
        assert!(flags.iter().any(|(f, _)| f == "-a"));
    }

    #[test]
    fn test_explain_flags_mixed_long_and_short() {
        let cmd = "rsync -avz --recursive --verbose src/ dst/";
        let flags = FlagInspector::explain_flags(cmd);
        assert!(flags.iter().any(|(f, _)| f == "-a"));
        assert!(flags.iter().any(|(f, _)| f == "-v"));
        assert!(flags.iter().any(|(f, _)| f == "-z"));
        assert!(flags.iter().any(|(f, _)| f == "--recursive"));
        assert!(flags.iter().any(|(f, _)| f == "--verbose"));
    }

    #[test]
    fn test_explain_flags_known_long_flags() {
        let cmd = "rm --force --recursive --verbose --yes --delete --archive /tmp";
        let flags = FlagInspector::explain_flags(cmd);
        assert!(flags
            .iter()
            .any(|(f, e)| f == "--force" && e == "Force operation without prompt or error"));
        assert!(flags
            .iter()
            .any(|(f, e)| f == "--recursive" && e == "Operate recursively on subdirectories"));
        assert!(flags
            .iter()
            .any(|(f, e)| f == "--verbose" && e == "Enable verbose progress output"));
        assert!(flags
            .iter()
            .any(|(f, e)| f == "--yes" && e == "Automatically answer yes to all prompts"));
        assert!(flags.iter().any(
            |(f, e)| f == "--delete" && e == "Delete extraneous files from target destination"
        ));
        assert!(flags.iter().any(|(f, e)| f == "--archive"
            && e == "Archive mode; preserves permissions, times, and symlinks"));
    }

    #[test]
    fn test_explain_flags_known_short_flags() {
        let cmd = "rsync -avryzf src dst";
        let flags = FlagInspector::explain_flags(cmd);
        assert!(flags
            .iter()
            .any(|(f, e)| f == "-a"
                && e == "Archive mode; preserves permissions, times, and symlinks"));
        assert!(flags.iter().any(|(f, e)| f == "-v" && e == "Enable verbose progress output"));
        assert!(flags
            .iter()
            .any(|(f, e)| f == "-r" && e == "Operate recursively on subdirectories"));
        assert!(flags
            .iter()
            .any(|(f, e)| f == "-y" && e == "Automatically answer yes to all prompts"));
        assert!(flags.iter().any(|(f, e)| f == "-z" && e == "Compress file data during transfer"));
    }

    #[test]
    fn test_explain_flags_case_sensitive_short_flags() {
        let cmd = "rsync -Rav src dst";
        let flags = FlagInspector::explain_flags(cmd);
        assert!(flags.iter().any(|(f, _)| f == "-R"));
        assert!(flags.iter().any(|(f, _)| f == "-a"));
        assert!(flags.iter().any(|(f, _)| f == "-v"));
    }

    #[test]
    fn test_explain_flags_unknown_long_flag() {
        let cmd = "cmd --unknown-flag";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].0, "--unknown-flag");
        assert_eq!(flags[0].1, "Command line long option parameter");
    }

    #[test]
    fn test_explain_flags_unknown_short_flag() {
        let cmd = "cmd -x";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].0, "-x");
        assert_eq!(flags[0].1, "Command line short option flag");
    }

    #[test]
    fn test_explain_flags_single_dash_single_char() {
        let cmd = "cmd -a";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].0, "-a");
    }

    #[test]
    fn test_explain_flags_single_dash_multiple_chars() {
        let cmd = "cmd -abc";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 3);
        assert!(flags.iter().any(|(f, _)| f == "-a"));
        assert!(flags.iter().any(|(f, _)| f == "-b"));
        assert!(flags.iter().any(|(f, _)| f == "-c"));
    }

    #[test]
    fn test_explain_flags_no_duplicates_for_combined() {
        let cmd = "cmd -aa";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 2);
        assert!(flags.iter().all(|(f, _)| f == "-a"));
    }

    #[test]
    fn test_explain_flags_multiple_long_flags() {
        let cmd = "cmd --flag1 --flag2 --flag3";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 3);
    }

    #[test]
    fn test_explain_flags_with_non_flag_arguments() {
        let cmd = "cp -r /src /dst --preserve";
        let flags = FlagInspector::explain_flags(cmd);
        assert!(flags.iter().any(|(f, _)| f == "-r"));
        assert!(flags.iter().any(|(f, _)| f == "--preserve"));
        assert_eq!(flags.len(), 2);
    }

    #[test]
    fn test_explain_flags_returns_vec_of_tuples() {
        let cmd = "ls -l --all";
        let flags = FlagInspector::explain_flags(cmd);
        assert_eq!(flags.len(), 2);
        for (flag, explanation) in &flags {
            assert!(flag.starts_with('-'));
            assert!(!explanation.is_empty());
        }
    }
}
