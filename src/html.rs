// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use pulldown_cmark::{html, Parser};

use crate::options::{HtmlMode, MarkdownOptions};

pub fn markdown_to_html(md: &str, options: &MarkdownOptions) -> String {
    if md.len() > u32::MAX as usize {
        return String::new();
    }
    let md = if options.html_mode == HtmlMode::Strip {
        strip_html_tags_full(md)
    } else {
        preprocess_math(md, options)
    };

    let parser = Parser::new_ext(&md, options.to_pulldown_options());
    let mut output = String::with_capacity(md.len() * 2);
    html::push_html(&mut output, parser);

    let output = sanitize_if_needed(output, options);
    let output = apply_code_class_prefix(&output, options);
    let output = resolve_relative_links(&output, options);
    output
}

fn preprocess_math(md: &str, options: &MarkdownOptions) -> String {
    use crate::options::MathMode;

    if options.math_mode != MathMode::Detect {
        return md.to_string();
    }

    let regions = crate::math::detect_math_regions(md);
    if regions.is_empty() {
        return md.to_string();
    }

    // Build result linearly: iterate through md, copying non-math text directly
    // and replacing math regions inline. This avoids stale indices from replace_range.
    let mut result = String::with_capacity(md.len());
    let mut pos = 0;
    for region in &regions {
        // Copy text before the math region
        result.push_str(&md[pos..region.start]);
        if region.is_display {
            result.push_str("\n<div class=\"math math-block\">");
            result.push_str(&md[region.start + 2..region.end - 2]);
            result.push_str("</div>\n");
        } else {
            result.push_str("<span class=\"math math-inline\">");
            result.push_str(&md[region.start + 1..region.end - 1]);
            result.push_str("</span>");
        }
        pos = region.end;
    }
    // Copy remaining text
    result.push_str(&md[pos..]);

    result
}

fn sanitize_if_needed(output: String, options: &MarkdownOptions) -> String {
    use crate::options::HtmlMode;

    match options.html_mode {
        HtmlMode::Safe => sanitize_html(&output),
        HtmlMode::AllowAll => sanitize_html_allow_all(&output),
        HtmlMode::Strip => output, // HTML already stripped from input
    }
}

fn strip_html_tags_full(html: &str) -> String {
    crate::strip_html_tags(html)
}

fn apply_code_class_prefix(html: &str, options: &MarkdownOptions) -> String {
    let prefix = match &options.code_class_prefix {
        Some(p) if !p.is_empty() => p.as_str(),
        _ => return html.to_string(),
    };

    // pulldown-cmark defaults "language-rust"; replace with custom prefix
    if prefix == "language-" {
        return html.to_string();
    }

    // Simple replacement: swap "language-" prefix in class attributes
    html.replace("class=\"language-", &format!("class=\"{prefix}"))
}

fn resolve_relative_links(html: &str, options: &MarkdownOptions) -> String {
    let base = match &options.base_url {
        Some(b) if !b.is_empty() => b.trim_end_matches('/'),
        _ => return html.to_string(),
    };

    let mut result = String::with_capacity(html.len());
    let mut last_end = 0;

    let attr_pairs = [("href=\"", 6), ("src=\"", 5)];

    loop {
        let mut earliest: Option<(usize, usize, usize)> = None;
        for &(attr, attr_len) in &attr_pairs {
            if let Some(idx) = html[last_end..].find(attr) {
                let abs_idx = last_end + idx;
                let after_attr = abs_idx + attr_len;
                match earliest {
                    None => earliest = Some((abs_idx, attr_len, after_attr)),
                    Some((e, _, _)) if abs_idx < e => earliest = Some((abs_idx, attr_len, after_attr)),
                    _ => {}
                }
            }
        }

        let (_, _, after_attr) = match earliest {
            Some(e) => e,
            None => break,
        };

        result.push_str(&html[last_end..after_attr]);

        // Check if the URL is already absolute
        let rest = &html[after_attr..];
        let is_absolute = rest.starts_with("https://")
            || rest.starts_with("http://")
            || rest.starts_with("//")
            || rest.starts_with("ftp://")
            || rest.starts_with("mailto:");

        if is_absolute {
            last_end = after_attr;
        } else {
            // Prepend base URL for relative paths
            result.push_str(base);
            // Ensure path separator if the relative URL doesn't start with /
            if !rest.starts_with('/') {
                result.push('/');
            }
            last_end = after_attr;
        }
    }

    result.push_str(&html[last_end..]);
    result
}

fn sanitize_html(html: &str) -> String {
    let mut builder = ammonia::Builder::new();
    builder
        .add_tags(&[
            "b", "strong", "i", "em", "u", "s", "del", "code", "pre",
            "a", "img", "br", "p", "h1", "h2", "h3", "h4", "h5", "h6",
            "ul", "ol", "li", "blockquote", "table", "thead", "tbody",
            "tr", "th", "td", "div", "span", "sub", "sup", "mark", "hr",
            "details", "summary", "dl", "dt", "dd",
        ])
        .add_generic_attribute_prefixes(&["class", "id", "data-", "aria-", "role"])
        .clean(html)
        .to_string()
}

fn sanitize_html_allow_all(html: &str) -> String {
    ammonia::Builder::new()
        .add_generic_attribute_prefixes(&["class", "id", "data-", "aria-", "role", "style"])
        .rm_tags(&["script", "style", "iframe", "object", "embed"])
        .rm_clean_content_tags(&["script", "style"])
        .clean(html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::*;

    fn opts(html_mode: HtmlMode) -> MarkdownOptions {
        MarkdownOptions {
            html_mode,
            ..Default::default()
        }
    }

    #[test]
    fn test_basic_html() {
        let html = markdown_to_html("Hello **world**", &Default::default());
        assert!(html.contains("<strong>world</strong>"));
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let html = markdown_to_html(md, &Default::default());
        assert!(html.contains("<code"));
        assert!(html.contains("language-rust"));
    }

    #[test]
    fn test_table() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |";
        let html = markdown_to_html(md, &Default::default());
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn test_link() {
        let html = markdown_to_html("[click](https://example.com)", &Default::default());
        assert!(html.contains("href=\"https://example.com\""));
    }

    #[test]
    fn test_math_in_detect_mode() {
        let mut opt = MarkdownOptions::default();
        opt.math_mode = MathMode::Detect;
        let html = markdown_to_html("$x^2$", &opt);
        assert!(html.contains("math"));
    }

    #[test]
    fn test_math_literal_mode() {
        let html = markdown_to_html("$HOME is /path", &Default::default());
        assert!(!html.contains("math")); // $HOME not treated as math
    }

    #[test]
    fn test_html_sanitize_script() {
        let md = "<script>alert(1)</script>**safe**";
        let html = markdown_to_html(md, &opts(HtmlMode::Safe));
        assert!(!html.contains("script"));
        assert!(html.contains("safe"));
    }

    #[test]
    fn test_html_sanitize_onclick() {
        let md = "<a onclick='alert(1)' href='/'>link</a>";
        let html = markdown_to_html(md, &opts(HtmlMode::Safe));
        assert!(!html.contains("onclick"));
        assert!(html.contains("href"));
    }

    #[test]
    fn test_allow_all_keeps_non_dangerous() {
        let md = "<div style='color:red'>text</div>";
        let html = markdown_to_html(md, &opts(HtmlMode::AllowAll));
        assert!(html.contains("style"));
    }

    #[test]
    fn test_strip_removes_html() {
        let md = "<b>bold</b> and **bold2**";
        let html = markdown_to_html(md, &opts(HtmlMode::Strip));
        assert!(!html.contains("<b>"));
        assert!(html.contains("<strong>bold2</strong>"));
    }

    #[test]
    fn test_empty_input() {
        let html = markdown_to_html("", &Default::default());
        assert_eq!(html, "");
    }

    #[test]
    fn test_nested_list() {
        let md = "- item1\n  - nested\n- item2";
        let html = markdown_to_html(md, &Default::default());
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
    }
}
