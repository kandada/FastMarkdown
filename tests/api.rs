// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

/// Tests that verify API consistency across different functions.

#[test]
fn test_html_and_plain_consistency() {
    let md = "Hello **world** `code` [link](https://example.com)";

    let html = to_html(md);
    let text = to_plain_text(md);

    // Plain text should contain the text content without markup
    assert!(text.contains("Hello"));
    assert!(text.contains("world"));
    assert!(text.contains("code"));
    assert!(text.contains("link"));

    // HTML should contain markup tags
    assert!(html.contains("<strong>"));
    assert!(html.contains("<code>"));
}

#[test]
fn test_html_and_spans_consistency() {
    let md = "**bold** and *italic*";

    let output = to_spans(md);
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };

    // Should have both bold and italic spans
    assert!(spans.iter().any(|s| s.is_bold()));
    assert!(spans.iter().any(|s| s.is_italic()));

    unsafe { fastmarkdown::free_spans(output); }
}

#[test]
fn test_span_output_covers_full_text() {
    let tests = vec![
        "Hello",
        "**bold** *italic*",
        "# Title\n\nParagraph",
        "1. One\n2. Two\n3. Three",
        "| A | B |\n|---|---|\n| 1 | 2 |",
        "`code` and **bold** and ~~strike~~",
    ];

    for md in tests {
        let output = to_spans(md);
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        let text = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
                .unwrap()
        };

        if output.span_count > 0 {
            // Last span should end at or before text length
            let last = spans.last().unwrap();
            assert!(
                (last.offset + last.length) as usize <= text.len(),
                "Span overflows text for input: {:?}", md
            );
        }

        unsafe { fastmarkdown::free_spans(output); }
    }
}

#[test]
fn test_options_independent() {
    // Verify that different options combinations don't crash
    let md = "# Test\n\n$E=mc^2$\n\n<b>html</b>\n\n```rust\nfn main() {}\n```";

    for html_mode in &[HtmlMode::Strip, HtmlMode::Safe, HtmlMode::AllowAll] {
        for math_mode in &[MathMode::Literal, MathMode::Detect] {
            let opts = MarkdownOptions {
                html_mode: *html_mode,
                math_mode: *math_mode,
                ..Default::default()
            };

            // HTML
            let _html = to_html_with_options(md, &opts);

            // Spans
            let output = to_spans_with_options(md, &opts);
            unsafe { fastmarkdown::free_spans(output); }
        }
    }
}

#[test]
fn test_version() {
    let version = crate_version();
    assert!(!version.is_empty());
    assert!(version.contains("fastmarkdown"));
}

#[test]
fn test_html_with_math_detect() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;
    opts.html_mode = HtmlMode::Safe;

    let md = "Price $5.99 and $E=mc^2$";
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("math"));
    assert!(html.contains("5.99"));
}

#[test]
fn test_plain_text_strips_markdown() {
    let text = to_plain_text("**bold** and *italic*");
    assert!(!text.contains("**"));
    assert!(!text.contains("*"));
    assert!(text.contains("bold"));
    assert!(text.contains("italic"));
}

#[test]
fn test_plain_text_strips_links() {
    let text = to_plain_text("[click here](https://example.com)");
    assert!(!text.contains("https://"));
    assert!(text.contains("click here"));
}

#[test]
fn test_to_spans_includes_link_urls() {
    let output = to_spans("[GitHub](https://github.com)");
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
    let link_span = spans.iter().find(|s| s.is_link()).unwrap();
    assert!(link_span.extra_len > 0);

    // Verify extra data exists
    let _extra_start = output.extra_data as usize;
    assert!(output.extra_data_len > 0);
    unsafe { fastmarkdown::free_spans(output); }
}

#[test]
fn test_free_spans_safety() {
    // free_spans should not crash when called on valid output
    let output = to_spans("Hello");
    unsafe { fastmarkdown::free_spans(output); }
    // No crash = pass

    // free_spans on empty
    let output = to_spans("");
    unsafe { fastmarkdown::free_spans(output); }
    // No crash = pass
}

fn crate_version() -> String {
    let ptr = fastmarkdown::ffi::fastmarkdown_version();
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        std::ffi::CStr::from_ptr(ptr)
            .to_string_lossy()
            .into_owned()
    }
}
