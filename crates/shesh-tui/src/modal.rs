//! Interactive approval modal ([Y/n]) for security-gated tool execution.

use std::io::{self, Write};

pub struct ApprovalModal;

impl ApprovalModal {
    /// Render a terminal confirmation modal for a proposed action or command.
    pub fn confirm_prompt(action_name: &str, details: &str) -> bool {
        println!("\n╔═════════════════════════════════════════════════════════════════╗");
        println!("║ ⚠️  SHESH SECURITY POLICY CONFIRMATION REQUIRED                 ║");
        println!("╠═════════════════════════════════════════════════════════════════╣");
        println!("  Action:  {}", action_name);
        println!("  Details: {}", details);
        println!("╚═════════════════════════════════════════════════════════════════╝");
        print!("Allow execution? [y/N]: ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            Self::parse_answer(&input)
        } else {
            false
        }
    }

    /// Pure decision: accept only explicit y/yes (everything else denies).
    pub fn parse_answer(input: &str) -> bool {
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_answer_accepts_explicit_yes() {
        for yes in ["y", "yes", "Y", "YES", "  yes  ", "y\n"] {
            assert!(ApprovalModal::parse_answer(yes), "{yes:?} must approve");
        }
    }

    #[test]
    fn test_parse_answer_denies_by_default() {
        for no in ["", "n", "no", "nope", "ye", "yess", "1", "\n"] {
            assert!(!ApprovalModal::parse_answer(no), "{no:?} must deny");
        }
    }
}
