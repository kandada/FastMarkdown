// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use pulldown_cmark::Parser;

use crate::options::MarkdownOptions;

pub fn markdown_to_plain_text(md: &str, options: &MarkdownOptions) -> String {
    let parser = Parser::new_ext(md, options.to_pulldown_options());
    let mut output = String::with_capacity(md.len());

    for event in parser {
        use pulldown_cmark::Event;
        match event {
            Event::Text(text) | Event::Code(text) => {
                output.push_str(&text);
            }
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                output.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => {
                output.push(' ');
            }
            Event::Start(tag) => {
                use pulldown_cmark::Tag;
                match tag {
                    Tag::Item => {
                        output.push_str("• ");
                    }
                    Tag::TableHead | Tag::TableRow | Tag::TableCell => {}
                    _ => {}
                }
            }
            Event::End(_) => {}
            Event::Rule => {}
            Event::TaskListMarker(checked) => {
                if checked {
                    output.push_str("[x] ");
                } else {
                    output.push_str("[ ] ");
                }
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                // Strip HTML tags in plain text mode
                let stripped = crate::strip_html_tags(&html);
                output.push_str(&stripped);
            }
            _ => {}
        }
    }

    // Clean up extra whitespace
    let result = output.trim().to_string();
    // Collapse multiple spaces
    collapse_spaces(&result)
}

fn collapse_spaces(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let result = markdown_to_plain_text("Hello **world**", &Default::default());
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_link() {
        let result =
            markdown_to_plain_text("[click here](https://example.com)", &Default::default());
        assert_eq!(result, "click here");
    }

    #[test]
    fn test_code() {
        let result = markdown_to_plain_text("Use `println!` macro", &Default::default());
        assert_eq!(result, "Use println! macro");
    }

    #[test]
    fn test_heading() {
        let result = markdown_to_plain_text("# Heading", &Default::default());
        assert_eq!(result, "Heading");
    }

    #[test]
    fn test_table() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |";
        let result = markdown_to_plain_text(md, &Default::default());
        assert!(!result.is_empty());
    }

    #[test]
    fn test_html_stripped() {
        let result = markdown_to_plain_text("<b>bold</b> text", &Default::default());
        assert_eq!(result, "bold text");
    }

    #[test]
    fn test_math_in_literal_mode() {
        let result = markdown_to_plain_text("$HOME is /path", &Default::default());
        assert!(result.contains("$HOME"));
    }

    #[test]
    fn test_empty() {
        let result = markdown_to_plain_text("", &Default::default());
        assert_eq!(result, "");
    }
}
