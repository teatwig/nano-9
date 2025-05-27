use std::io::{self, BufRead, Read};

/// Parse front matter if present
///
/// Returns a tuple:
/// - `Some(&str)` containing the front matter (excluding delimiters),
/// - `&str` containing the rest of the body.
pub fn parse_front_matter(input: &str) -> (Option<&str>, &str) {
    // Strip BOM if present.
    let input = input.strip_prefix("\u{FEFF}").unwrap_or(input);

    // Must start with "===\n" (front matter opening).
    if !input.starts_with("===\n") {
        return (None, input);
    }

    // Position just after the opening delimiter.
    let after_start = 4;

    // Look for the closing delimiter.
    let rest = &input[after_start..];
    if let Some(end_pos) = rest.find("\n===\n") {
        let front_matter = &rest[..end_pos];
        // Skip "\n===\n" with + 5.
        let body_start = after_start + end_pos + 5;
        let body = &input[body_start..];
        return (Some(front_matter), body);
    }

    // No closing delimiter found.
    (None, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_front_matter() {
        let input = "\
===
title: Test
date: 2025-05-27
===
# Content
This is the body.";
        let (fm, body) = parse_front_matter(input);

        assert_eq!(
            fm,
            Some("title: Test\ndate: 2025-05-27")
        );
        assert_eq!(
            body,
            "# Content\nThis is the body."
        );
    }

    #[test]
    fn test_without_front_matter() {
        let input = "# Content\nNo front matter here.";
        let (fm, body) = parse_front_matter(input);

        assert!(fm.is_none());
        assert_eq!(body, input);
    }

    #[test]
    fn test_front_matter_without_end() {
        let input = "\
===
title: Test
Still in front matter
No closing delimiter";
        let (fm, body) = parse_front_matter(input);

        assert!(fm.is_none());
        assert_eq!(body, input);
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let (fm, body) = parse_front_matter(input);

        assert!(fm.is_none());
        assert_eq!(body, "");
    }

    #[test]
    fn test_only_front_matter() {
        let input = "\
===
foo: bar
===
";
        let (fm, body) = parse_front_matter(input);

        assert_eq!(fm, Some("foo: bar"));
        assert_eq!(body, "");
    }
}
