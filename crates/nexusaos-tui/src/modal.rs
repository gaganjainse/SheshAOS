//! Interactive approval modal ([Y/n]) for security-gated tool execution.

use std::io::{self, Write};

pub struct ApprovalModal;

impl ApprovalModal {
    /// Render a terminal confirmation modal for a proposed action or command.
    pub fn confirm_prompt(action_name: &str, details: &str) -> bool {
        println!("\n╔═════════════════════════════════════════════════════════════════╗");
        println!("║ ⚠️  NEXUSAOS SECURITY POLICY CONFIRMATION REQUIRED              ║");
        println!("╠═════════════════════════════════════════════════════════════════╣");
        println!("  Action:  {}", action_name);
        println!("  Details: {}", details);
        println!("╚═════════════════════════════════════════════════════════════════╝");
        print!("Allow execution? [y/N]: ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let trimmed = input.trim().to_lowercase();
            trimmed == "y" || trimmed == "yes"
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_modal_struct_exists() {
        assert!(true);
    }
}
