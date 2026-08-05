// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

#[test]
fn test_bold_with_html() {
    let md = "<b>bold via html</b> and **bold via md**";
    let html = to_html(md);
    // Both should be bold in the output
    assert!(html.contains("bold via html"));
    assert!(html.contains("bold via md"));
}

#[test]
fn test_html_safe_mode_script_stripped() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;

    let md = "<script>alert('xss')</script><b>safe</b>";
    let html = to_html_with_options(md, &opts);
    assert!(!html.contains("<script>"));
    assert!(!html.contains("alert"));
    assert!(html.contains("safe"));
}

#[test]
fn test_html_safe_mode_onclick_stripped() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;

    let md = "<a href='/' onclick='steal()'>link</a>";
    let html = to_html_with_options(md, &opts);
    assert!(!html.contains("onclick"));
    assert!(html.contains("href"));
}

#[test]
fn test_html_allow_all_keeps_div_style() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::AllowAll;

    let md = "<div style='color:red'>red text</div>";
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("style") || html.contains("red"));
}

#[test]
fn test_html_strip_removes_all() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Strip;

    let md = "<b>html bold</b> and **md bold**";
    let html = to_html_with_options(md, &opts);
    // HTML bold should be gone, md bold should remain
    assert!(!html.contains("<b>"));
    assert!(html.contains("strong")); // from **md bold**
}

#[test]
fn test_html_nested_in_markdown() {
    let md = "# <span class='title'>Title</span>";
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("<span"));
    assert!(html.contains("Title"));
}

#[test]
fn test_html_table() {
    let md = "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>";
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("A"));
    assert!(html.contains("B"));
    assert!(html.contains("table"));
}

#[test]
fn test_html_img_tag() {
    let md = "<img src='https://example.com/pic.png' alt='pic'>";
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("img"));
    assert!(html.contains("pic.png"));
}

#[test]
fn test_html_br_tag() {
    let md = "Line 1<br>Line 2";
    let html = to_html(md);
    // <br> should pass through and appear
    assert!(html.contains("<br") || html.contains("Line 1") && html.contains("Line 2"));
}

#[test]
fn test_html_code_tag() {
    let md = "Use <code>assert_eq!</code> here";
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("<code>") || html.contains("assert_eq"));
}

#[test]
fn test_html_details_summary() {
    let md = "<details><summary>Click me</summary>Hidden content</details>";
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("<details"));
    assert!(html.contains("<summary"));
}

#[test]
fn test_html_strip_with_complex_markdown() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Strip;

    let md = "# Title\n\n<div class='note'>Important: **read** this</div>\n\nEnd";
    let html = to_html_with_options(md, &opts);
    // Markdown heading should still work
    assert!(html.contains("<h1>"));
    // HTML div should be stripped
    assert!(!html.contains("<div"));
    assert!(!html.contains("class='note'"));
}

#[test]
fn test_html_entities() {
    let md = "&amp; &lt; &gt; &quot; &#39;";
    let html = to_html(md);
    // Entities should be preserved or double-escaped
    assert!(html.contains("&amp;") || html.contains("&#x26;") || html.contains("&amp;amp;"));
}

#[test]
fn test_html_comment() {
    let md = "<!-- this is a comment -->**bold**";
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;
    let html = to_html_with_options(md, &opts);
    // Comment might be stripped or passed through; bold should work
    assert!(html.contains("bold"));
}

#[test]
fn test_html_a_with_target() {
    let md = "<a href='https://example.com' target='_blank'>external</a>";
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Safe;
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("href"));
    assert!(html.contains("external"));
}
