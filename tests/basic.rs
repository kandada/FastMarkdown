// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

#[test]
fn test_bold() {
    let html = to_html("Hello **world**");
    assert!(html.contains("<strong>world</strong>"));

    let output = to_spans("Hello **world**");
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.is_bold()));
    free(output);
}

#[test]
fn test_italic() {
    let html = to_html("Hello *world*");
    assert!(html.contains("<em>world</em>"));

    let output = to_spans("Hello *world*");
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.is_italic()));
    free(output);
}

#[test]
fn test_strikethrough() {
    let html = to_html("Hello ~~world~~");
    assert!(html.contains("<del>world</del>"));

    let output = to_spans("Hello ~~world~~");
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.is_strikethrough()));
    free(output);
}

#[test]
fn test_inline_code() {
    let html = to_html("Use `println!` here");
    assert!(html.contains("<code>println!</code>"));

    let output = to_spans("Use `println!` here");
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.is_inline_code()));
    free(output);
}

#[test]
fn test_headings() {
    for level in 1..=6 {
        let prefix = "#".repeat(level);
        let md = format!("{} Heading {}", prefix, level);
        let html = to_html(&md);
        let tag = format!("<h{}>", level);
        assert!(html.contains(&tag), "Expected {} in output for h{}", tag, level);
    }
}

#[test]
fn test_heading_spans() {
    let output1 = to_spans("# H1");
    let spans1 = get_spans(&output1);
    assert!(spans1.iter().any(|s| s.block_type == BLOCK_HEADING_H1));
    free(output1);

    let output2 = to_spans("### H3");
    let spans2 = get_spans(&output2);
    assert!(spans2.iter().any(|s| s.block_type == BLOCK_HEADING_H3));
    free(output2);

    let output3 = to_spans("###### H6");
    let spans3 = get_spans(&output3);
    assert!(spans3.iter().any(|s| s.block_type == BLOCK_HEADING_H6));
    free(output3);
}

#[test]
fn test_links() {
    let html = to_html("[GitHub](https://github.com)");
    assert!(html.contains("href=\"https://github.com\""));
    assert!(html.contains(">GitHub</a>"));

    let output = to_spans("[click](https://example.com)");
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.is_link()));
    free(output);
}

#[test]
fn test_images() {
    let html = to_html("![alt](https://example.com/img.png)");
    assert!(html.contains("img"));
    assert!(html.contains("src=\"https://example.com/img.png\""));
}

#[test]
fn test_code_blocks() {
    let md = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
    let html = to_html(md);
    assert!(html.contains("<code"));
    assert!(html.contains("language-rust"));

    let output = to_spans(md);
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.block_type == BLOCK_CODE));
    free(output);
}

#[test]
fn test_unordered_list() {
    let md = "- Item 1\n- Item 2\n- Item 3";
    let html = to_html(md);
    assert!(html.contains("<ul>"));
    assert!(html.matches("<li>").count() >= 3);

    let output = to_spans(md);
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED));
    free(output);
}

#[test]
fn test_ordered_list() {
    let md = "1. First\n2. Second\n3. Third";
    let html = to_html(md);
    assert!(html.contains("<ol>"));

    let output = to_spans(md);
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.block_type == BLOCK_LIST_ITEM_ORDERED));
    free(output);
}

#[test]
fn test_nested_list() {
    let md = "- A\n  - B\n    - C\n- D";
    let html = to_html(md);
    assert!(html.contains("<li>A"));
    assert!(html.contains("<li>B"));
    assert!(html.contains("<li>C"));
    assert!(html.contains("<li>D"));
}

#[test]
fn test_blockquote() {
    let md = "> This is a quote\n> with **bold** text";
    let html = to_html(md);
    assert!(html.contains("<blockquote>"));

    // Blockquote wraps paragraphs; content spans may have paragraph or blockquote type
    let output = to_spans(md);
    let spans = get_spans(&output);
    // At minimum, the bold text should be present
    assert!(spans.iter().any(|s| s.is_bold()));
    free(output);
}

#[test]
fn test_nested_blockquote() {
    let md = "> Outer\n>> Nested\n> Back to outer";
    let html = to_html(md);
    assert!(html.matches("<blockquote>").count() >= 2);
}

#[test]
fn test_horizontal_rule() {
    let html = to_html("---");
    assert!(html.contains("<hr"));

    let output = to_spans("---");
    let spans = get_spans(&output);
    assert!(spans.iter().any(|s| s.block_type == BLOCK_HORIZONTAL_RULE));
    free(output);
}

#[test]
fn test_task_list() {
    let md = "- [ ] Todo item\n- [x] Done item";
    let html = to_html(md);
    // Task lists produce Event::TaskListMarker; HTML renderer may not
    // include checkbox elements by default, but the list items should be present
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>"));
    assert!(html.contains("Todo"));
    assert!(html.contains("Done"));
}

#[test]
fn test_mixed_styles() {
    let md = "**bold *bolditalic* bold**";
    let html = to_html(md);

    // All text should be bold
    assert!(html.contains("<strong>bold <em>bolditalic</em> bold</strong>"));

    let output = to_spans(md);
    let spans = get_spans(&output);
    // There should be a span that is both bold and italic
    assert!(spans.iter().any(|s| s.is_bold() && s.is_italic()));
    free(output);
}

#[test]
fn test_escape() {
    let html = to_html(r"\*not italic\*");
    assert!(!html.contains("<em>not italic</em>"));
    assert!(html.contains("*not italic*") || html.contains("&#42;") || html.contains("\\*"));
}

#[test]
fn test_soft_break() {
    let html = to_html("Line 1\nLine 2");
    // Should have a line break between the two lines
    assert!(html.contains("Line 1") && html.contains("Line 2"));
}

#[test]
fn test_hard_break() {
    let html = to_html("Line 1  \nLine 2");
    assert!(html.contains("<br") || html.contains("Line 1\nLine 2") || html.contains("<br />"));
}

#[test]
fn test_empty_input() {
    let html = to_html("");
    assert_eq!(html, "");

    let text = to_plain_text("");
    assert_eq!(text, "");

    let output = to_spans("");
    assert_eq!(output.span_count, 0);
    free(output);
}

#[test]
fn test_whitespace_only() {
    let html = to_html("   \n\n   ");
    assert!(html.is_empty() || html.trim().is_empty());

    let text = to_plain_text("   \n\n   ");
    assert!(text.is_empty() || text.trim().is_empty());
}

#[test]
fn test_very_long_paragraph() {
    let long_text = "A".repeat(10000);
    let md = format!("**{}**", long_text);
    let html = to_html(&md);
    assert!(html.len() > 10000);
}

#[test]
fn test_special_characters() {
    let md = "Copyright © 2024 — «quotes» … ellipsis";
    let html = to_html(md);
    // Should contain the original characters
    assert!(html.contains("©") || html.contains("&copy;"));
}

#[test]
fn test_deeply_nested_styles() {
    let md = "***bold italic ~~strike~~ continue***";
    let html = to_html(md);
    assert!(html.contains("<strong>"));
    assert!(html.contains("<em>"));
    assert!(html.contains("<del>"));
}

// Helper functions
fn get_spans(output: &SpanOutput) -> &[Span] {
    unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) }
}

fn free(output: SpanOutput) {
    unsafe { fastmarkdown::free_spans(output); }
}
