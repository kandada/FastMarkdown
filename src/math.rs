// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

/// A detected math region in the source text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathRegion {
    pub start: usize,
    pub end: usize, // exclusive
    pub is_display: bool,
}

/// Detect $...$ and $$...$$ math regions in markdown text.
///
/// Rules for inline math ($...$):
/// - Both $ on the same line (no newline between)
/// - Content between $ is non-empty
/// - First char after opening $ is not a digit and not whitespace
/// - Last char before closing $ is not whitespace
/// - Opening $ is not escaped with backslash
///
/// Rules for display math ($$...$$):
/// - Content spans any number of lines
/// - Content is non-empty after trimming
pub fn detect_math_regions(text: &str) -> Vec<MathRegion> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut regions = Vec::new();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }

        // Check for escaped $
        if i > 0 && bytes[i - 1] == b'\\' {
            i += 1;
            continue;
        }

        // Check for display math: $$
        if i + 1 < len && bytes[i + 1] == b'$' {
            if let Some(end) = find_closing_display_math(bytes, i + 2) {
                let content = &text[i + 2..end];
                if !content.trim().is_empty() {
                    regions.push(MathRegion {
                        start: i,
                        end: end + 2,
                        is_display: true,
                    });
                }
                i = end + 2;
                continue;
            }
        }

        // Check for inline math: $...$
        if let Some(end) = find_closing_inline_math(bytes, i + 1) {
            if end > i + 1 {
                let after_open = bytes[i + 1];
                let before_close = bytes[end - 1];
                if !after_open.is_ascii_digit()
                    && !after_open.is_ascii_whitespace()
                    && !before_close.is_ascii_whitespace()
                {
                    regions.push(MathRegion {
                        start: i,
                        end: end + 1,
                        is_display: false,
                    });
                    i = end + 1;
                    continue;
                }
            }
        }

        i += 1;
    }

    regions
}

fn find_closing_display_math(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'$' && bytes[i + 1] == b'$' {
            // Check not escaped
            if i > 0 && bytes[i - 1] == b'\\' {
                i += 2;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_closing_inline_math(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\n' || bytes[i] == b'\r' {
            // Inline math must be on a single line; CR or LF terminates it
            return None;
        }
        if bytes[i] == b'$' {
            // Check not escaped
            if i > 0 && bytes[i - 1] == b'\\' {
                i += 1;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_dollar() {
        let regions = detect_math_regions("Price: $5.99 for each");
        assert!(regions.is_empty());
    }

    #[test]
    fn test_path_dollar() {
        let regions = detect_math_regions("Path: $HOME/.config");
        assert!(regions.is_empty());
    }

    #[test]
    fn test_single_dollar() {
        let regions = detect_math_regions("This costs $100");
        assert!(regions.is_empty());
    }

    #[test]
    fn test_inline_math() {
        let regions = detect_math_regions("The formula is $x^2 + y^2 = z^2$ here");
        assert_eq!(regions.len(), 1);
        assert!(!regions[0].is_display);
        assert!(regions[0].start < regions[0].end);
    }

    #[test]
    fn test_display_math() {
        let regions = detect_math_regions("Some text\n$$\nx^2 + y^2 = z^2\n$$\nMore text");
        assert_eq!(regions.len(), 1);
        assert!(regions[0].is_display);
    }

    #[test]
    fn test_mixed_dollars() {
        let text = "Price $5.99 but formula $E = mc^2$ is cool";
        let regions = detect_math_regions(text);
        assert_eq!(regions.len(), 1); // only $E = mc^2$
        assert!(!regions[0].is_display);
    }

    #[test]
    fn test_no_math_with_digit() {
        let regions = detect_math_regions("$5.99"); // starts with digit
        assert!(regions.is_empty());
    }

    #[test]
    fn test_escaped_dollar() {
        let regions = detect_math_regions(r"\$not math\$");
        assert!(regions.is_empty());
    }

    #[test]
    fn test_inline_math_no_newline() {
        let regions = detect_math_regions("$x\ny$");
        assert!(regions.is_empty());
    }

    #[test]
    fn test_multiple_inline_math() {
        let text = "$a+b$ and $c+d$";
        let regions = detect_math_regions(text);
        assert_eq!(regions.len(), 2);
    }
}
