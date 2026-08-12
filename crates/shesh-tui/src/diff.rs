//! Colorized unified code diff visualizer.

pub struct DiffViewer;

impl DiffViewer {
    /// Render colorized unified diff lines for proposed file changes.
    pub fn render_diff(filename: &str, diff_text: &str) -> String {
        let mut output = String::new();
        output.push_str(&format!("--- a/{}\n+++ b/{}\n", filename, filename));

        for line in diff_text.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                output.push_str(&format!("\x1b[32m{}\x1b[0m\n", line)); // Green for additions
            } else if line.starts_with('-') && !line.starts_with("---") {
                output.push_str(&format!("\x1b[31m{}\x1b[0m\n", line)); // Red for deletions
            } else if line.starts_with('@') {
                output.push_str(&format!("\x1b[36m{}\x1b[0m\n", line)); // Cyan for chunk headers
            } else {
                output.push_str(&format!("{}\n", line));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_diff() {
        let diff = "-old_line\n+new_line";
        let rendered = DiffViewer::render_diff("test.rs", diff);
        assert!(rendered.contains("\x1b[31m-old_line\x1b[0m"));
        assert!(rendered.contains("\x1b[32m+new_line\x1b[0m"));
    }

    #[test]
    fn test_render_diff_with_chunk_header() {
        let diff = "@@ -1,2 +1,2 @@\n-old\n+new";
        let rendered = DiffViewer::render_diff("file.txt", diff);
        assert!(rendered.contains("\x1b[36m@@ -1,2 +1,2 @@\x1b[0m"));
    }

    #[test]
    fn test_render_diff_context_lines() {
        let diff = " unchanged\n-old\n+new\n unchanged";
        let rendered = DiffViewer::render_diff("file.txt", diff);
        assert!(rendered.contains(" unchanged\n"));
        assert!(rendered.contains("\x1b[31m-old\x1b[0m"));
        assert!(rendered.contains("\x1b[32m+new\x1b[0m"));
    }

    #[test]
    fn test_render_diff_empty() {
        let rendered = DiffViewer::render_diff("file.txt", "");
        assert!(rendered.contains("--- a/file.txt"));
        assert!(rendered.contains("+++ b/file.txt"));
    }
}
