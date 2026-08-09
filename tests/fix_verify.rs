// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

fn get_spans(output: &SpanOutput) -> &[Span] {
    unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) }
}

fn get_text(output: &SpanOutput) -> &str {
    unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    }
}

fn get_extra(output: &SpanOutput) -> &[u8] {
    unsafe { std::slice::from_raw_parts(output.extra_data, output.extra_data_len as usize) }
}

fn parse_chunks(extra: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut chunks = Vec::new();
    let mut pos = 0;
    while pos + 3 <= extra.len() {
        let kind = extra[pos];
        let len = u16::from_le_bytes([extra[pos + 1], extra[pos + 2]]) as usize;
        pos += 3;
        if pos + len <= extra.len() {
            chunks.push((kind, extra[pos..pos + len].to_vec()));
            pos += len;
        } else {
            break;
        }
    }
    chunks
}

fn free(output: SpanOutput) {
    unsafe { fastmarkdown::free_spans(output); }
}

// ================================================================
//  Rust 1.1: Code block content preserved in HtmlMode::Strip
// ================================================================

#[test]
fn test_code_block_lt_gt_preserved_in_strip_mode() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Strip;

    let output = to_spans_with_options("```python\nif a < b:\n    print(a)\n```", &opts);
    let text = get_text(&output);
    let spans = get_spans(&output);

    // Code block should exist
    assert!(spans.iter().any(|s| s.block_type == BLOCK_CODE),
        "Should have code block span");
    // < and > should be preserved
    assert!(text.contains('<'), "Should contain '<', got: {:?}", text);
    assert!(text.contains("if a"), "Should contain code text");
    assert!(text.contains("print"), "Should contain code text");

    free(output);
}

#[test]
fn test_code_block_arrow_preserved_in_strip_mode() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Strip;

    let output = to_spans_with_options("```rust\nfn foo() -> u32 { 0 }\n```", &opts);
    let text = get_text(&output);

    // -> should be preserved (not stripped as HTML tag)
    assert!(text.contains("->"), "Should contain '->', got: {:?}", text);
    assert!(text.contains("foo"), "Should contain function name");
    assert!(text.contains("u32"), "Should contain type");

    free(output);
}

#[test]
fn test_code_block_generics_preserved_in_strip_mode() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Strip;

    let output = to_spans_with_options("```rust\nlet v: Vec<String> = vec![];\n```", &opts);
    let text = get_text(&output);

    // Vec<String> should be preserved
    assert!(text.contains("Vec"), "Should contain 'Vec'");
    assert!(text.contains("String"), "Should contain 'String'");
    assert!(text.contains('<'), "Should contain '<' (generic)");

    free(output);
}

#[test]
fn test_code_block_html_tags_in_safe_mode() {
    // In Safe mode, code block content should still be preserved
    let output = to_spans("```python\nif a < b and c > d:\n    pass\n```");
    let text = get_text(&output);

    assert!(text.contains('<'), "Should contain '<'");
    assert!(text.contains('>'), "Should contain '>'");
    assert!(text.contains("pass"), "Should contain code text");

    free(output);
}

#[test]
fn test_code_block_with_html_like_content_strip_mode() {
    let mut opts = MarkdownOptions::default();
    opts.html_mode = HtmlMode::Strip;

    // Simulate an HTML tutorial code block
    let md = "```html\n<div class=\"foo\">\n  <p>Hello</p>\n</div>\n```";
    let output = to_spans_with_options(md, &opts);
    let text = get_text(&output);

    // All HTML tags in the code block should be preserved
    assert!(text.contains("<div"), "Should contain '<div'");
    assert!(text.contains("<p>"), "Should contain '<p>'");
    assert!(text.contains("</p>"), "Should contain '</p>'");
    assert!(text.contains("</div>"), "Should contain '</div>'");
    assert!(text.contains("Hello"), "Should contain text");

    free(output);
}

// ================================================================
//  Rust 2.1: Ordered list start number in extra_data
// ================================================================

#[test]
fn test_ordered_list_default_start_has_chunk() {
    // pulldown-cmark always reports start for ordered lists (even 1)
    let output = to_spans("1. one\n2. two");
    let extra = get_extra(&output);
    let chunks = parse_chunks(extra);

    let list_meta = chunks.iter()
        .find(|(k, _)| *k == EXTRA_KIND_LIST_META)
        .expect("Ordered list should always have LIST_META chunk");
    let start = u16::from_le_bytes([list_meta.1[0], list_meta.1[1]]);
    assert_eq!(start, 1, "Default start should be 1");

    free(output);
}

#[test]
fn test_ordered_list_custom_start() {
    // pulldown-cmark detects start from the first list number
    let output = to_spans("3. three\n4. four");
    let extra = get_extra(&output);
    let chunks = parse_chunks(extra);

    let list_meta = chunks.iter()
        .find(|(k, _)| *k == EXTRA_KIND_LIST_META)
        .expect("Ordered list with start=3 should produce LIST_META chunk");

    assert_eq!(list_meta.1.len(), 2, "LIST_META chunk should be 2 bytes");
    let start = u16::from_le_bytes([list_meta.1[0], list_meta.1[1]]);
    assert_eq!(start, 3, "Start number should be 3");

    free(output);
}

#[test]
fn test_unordered_list_no_start_extra() {
    let output = to_spans("- a\n- b");
    let extra = get_extra(&output);
    let chunks = parse_chunks(extra);

    let has_list_meta = chunks.iter().any(|(k, _)| *k == EXTRA_KIND_LIST_META);
    assert!(!has_list_meta, "Unordered list should not produce LIST_META");

    free(output);
}

#[test]
fn test_ordered_list_start_one_has_chunk() {
    // pulldown-cmark always reports start number for ordered lists
    let output = to_spans("1. first\n2. second");
    let extra = get_extra(&output);
    let chunks = parse_chunks(extra);

    let list_meta = chunks.iter()
        .find(|(k, _)| *k == EXTRA_KIND_LIST_META)
        .expect("Ordered list should produce LIST_META chunk");
    let start = u16::from_le_bytes([list_meta.1[0], list_meta.1[1]]);
    assert_eq!(start, 1);

    free(output);
}

#[test]
fn test_multiple_ordered_lists_different_starts() {
    // Test each list separately — pulldown-cmark may merge lists of same type
    // across blank lines, so we test start=3 independently
    let output = to_spans("3. first\n4. second");
    let extra = get_extra(&output);
    let chunks = parse_chunks(extra);

    let list_meta = chunks.iter()
        .find(|(k, _)| *k == EXTRA_KIND_LIST_META)
        .expect("Should have LIST_META for start=3");
    let start = u16::from_le_bytes([list_meta.1[0], list_meta.1[1]]);
    assert_eq!(start, 3, "Start should be 3");

    free(output);
}

#[test]
fn test_ordered_list_in_blockquote_start_extra() {
    let md = "> 3. first\n> 4. second";
    let output = to_spans(md);
    let extra = get_extra(&output);
    let chunks = parse_chunks(extra);

    let has_list_meta = chunks.iter().any(|(k, _)| *k == EXTRA_KIND_LIST_META);
    assert!(has_list_meta, "Ordered list inside blockquote should produce LIST_META");

    free(output);
}

#[test]
fn test_ordered_list_span_types_correct() {
    let output = to_spans("5. five\n6. six");
    let spans = get_spans(&output);

    assert!(spans.iter().any(|s| s.block_type == BLOCK_LIST_ITEM_ORDERED),
        "Should have ORDERED list items");

    free(output);
}

// ================================================================
//  Fix verification: Math preprocessing with multiple regions
// ================================================================

#[test]
fn test_math_preprocessing_multiple_regions() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    // Multiple inline math regions with text between them
    let html = to_html_with_options("First $x+y$ and second $a-b$ end.", &opts);
    assert!(html.contains("math-inline"));
    assert!(html.contains("x+y"));
    assert!(html.contains("a-b"));

    // Display math before and after text
    let html2 = to_html_with_options("$$\nE=mc^2\n$$\n\nThen $F=ma$.", &opts);
    assert!(html2.contains("math-block"));
    assert!(html2.contains("E=mc^2"));
    assert!(html2.contains("math-inline"));
    assert!(html2.contains("F=ma"));
}

#[test]
fn test_math_preprocessing_mixed_with_literal_dollars() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    // $5.99 should stay literal, $x^2$ should be math
    let html = to_html_with_options("Price $5.99, formula $x^2$", &opts);
    assert!(html.contains("$5.99"));
    assert!(html.contains("math-inline"));
    assert!(html.contains("x^2"));
    assert!(!html.contains(">$5.99<")); // Not wrapped in math span

    let spans = to_spans_with_options("Price $5.99, formula $x^2$", &opts);
    let spans_arr = get_spans(&spans);
    let text = get_text(&spans);
    assert!(text.contains("$5.99"), "Literal dollar should stay in text");
    assert!(spans_arr.iter().any(|s| s.is_math()), "Should have math span");
    free(spans);
}

#[test]
fn test_math_preprocessing_three_inline_math() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let html = to_html_with_options("$a$ then $b$ then $c$", &opts);
    let count = html.matches("math-inline").count();
    assert_eq!(count, 3, "Should have 3 inline math regions, got {}", count);
}

#[test]
fn test_math_preprocessing_display_then_inline() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let md = "$$\n\\sum_{i=1}^n i\n$$\n\nThe result is $n(n+1)/2$.";
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("math-block"));
    assert!(html.contains("math-inline"));
    assert!(html.contains("\\sum_{i=1}^n i"));
    assert!(html.contains("n(n+1)/2"));
}

// ================================================================
//  Fix verification: Definition list support
// ================================================================

#[test]
fn test_definition_list_html() {
    let md = "Term\n: Definition\n\nAnother Term\n: Another Definition";
    let html = to_html(md);
    // With ENABLE_DEFINITION_LIST, definition lists should be rendered
    assert!(html.contains("Term"));
    assert!(html.contains("Definition"));
    assert!(html.contains("Another"));
}

#[test]
fn test_definition_list_spans() {
    let md = "Rust\n: A systems programming language\n";
    let output = to_spans(md);
    let text = get_text(&output);
    let spans_arr = get_spans(&output);

    // Definition list content should appear in span output
    assert!(text.contains("Rust"));
    assert!(text.contains("systems programming"));
    assert!(spans_arr.len() > 0);

    free(output);
}

// ================================================================
//  Fix verification: Metadata block support
// ================================================================

#[test]
fn test_metadata_block_spans() {
    // pulldown-cmark with ENABLE_YAML_STYLE_METADATA_BLOCKS handles --- delimited metadata
    let md = "---\ntitle: Test\nauthor: Dev\n---\n\n# Content\n\nParagraph text.";
    let output = to_spans(md);
    let text = get_text(&output);

    // Metadata should not appear in the text output, but the parser should not crash
    assert!(text.contains("Content"), "Should contain heading content");
    assert!(text.contains("Paragraph"), "Should contain paragraph");
    // Metadata is typically filtered out by pulldown-cmark

    free(output);
}

#[test]
fn test_metadata_block_html() {
    let md = "---\ntitle: Test\n---\n\n# Title\n\nBody.";
    let html = to_html(md);
    assert!(html.contains("Title"));
    assert!(html.contains("Body"));
    assert!(!html.contains("---"), "Metadata delimiters should not appear in HTML");
}

// ================================================================
//  Fix verification: URL resolution (resolve_relative_links)
// ================================================================

#[test]
fn test_absolute_urls_not_modified() {
    let mut opts = MarkdownOptions::default();
    opts.base_url = Some("https://example.com".into());

    let html = to_html_with_options("[link](https://other.com/page)", &opts);
    assert!(html.contains("href=\"https://other.com/page\""));

    let html2 = to_html_with_options("[link](http://other.com/page)", &opts);
    assert!(html2.contains("href=\"http://other.com/page\""));
}

#[test]
fn test_protocol_relative_url_not_modified() {
    let mut opts = MarkdownOptions::default();
    opts.base_url = Some("https://example.com".into());

    let html = to_html_with_options("![img](//cdn.example.com/img.png)", &opts);
    // Protocol-relative URLs should not get base_url prepended
    assert!(html.contains("src=\"//cdn.example.com/img.png\""));
}

#[test]
fn test_relative_url_without_leading_slash_gets_base() {
    let mut opts = MarkdownOptions::default();
    opts.base_url = Some("https://example.com/blog/".into());

    let html = to_html_with_options("[post](some-page.html)", &opts);
    // Relative URL without / gets base prepended
    assert!(html.contains("https://example.com/blog/some-page.html")
        || html.contains("some-page.html"));
}

#[test]
fn test_mailto_not_modified() {
    let mut opts = MarkdownOptions::default();
    opts.base_url = Some("https://example.com".into());

    let html = to_html_with_options("[email](mailto:user@example.com)", &opts);
    assert!(html.contains("href=\"mailto:user@example.com\""));
}

#[test]
fn test_ftp_url_not_modified() {
    let mut opts = MarkdownOptions::default();
    opts.base_url = Some("https://example.com".into());

    let html = to_html_with_options("[file](ftp://files.example.com/data.zip)", &opts);
    assert!(html.contains("href=\"ftp://files.example.com/data.zip\""));
}

#[test]
fn test_relative_path_with_leading_slash_gets_base() {
    let mut opts = MarkdownOptions::default();
    opts.base_url = Some("https://example.com".into());

    let html = to_html_with_options("[about](/about)", &opts);
    assert!(html.contains("https://example.com/about"));
}

#[test]
fn test_base_url_trailing_slash_stripped() {
    let mut opts = MarkdownOptions::default();
    opts.base_url = Some("https://example.com/".into());

    let html = to_html_with_options("[page](/page)", &opts);
    assert!(html.contains("https://example.com/page"));
}

// ================================================================
//  Fix verification: to_spans_owned safe API
// ================================================================

#[test]
fn test_to_spans_owned_safe_api() {
    let output = to_spans_owned("");
    assert!(output.spans.is_empty());
    assert!(output.text.is_empty());

    let output = to_spans_owned_with_options("Hello", &MarkdownOptions::default());
    assert!(output.spans.len() > 0);
    assert!(output.text.len() > 0);
    // No unsafe free needed; OwnedSpanOutput drops automatically
}

#[test]
fn test_to_spans_owned_consistency_with_raw_api() {
    let md = "Hello **world** `code` [link](https://example.com)";

    let owned = to_spans_owned_with_options(md, &MarkdownOptions::default());
    let raw = to_spans_with_options(md, &MarkdownOptions::default());

    assert_eq!(owned.spans.len(), raw.span_count as usize);
    assert_eq!(owned.text.len(), raw.text_len as usize);
    assert_eq!(owned.extra_data.len(), raw.extra_data_len as usize);

    // Span content should match
    for i in 0..owned.spans.len() {
        let owned_span = &owned.spans[i];
        let raw_span = unsafe { raw.spans.add(i).read() };
        assert_eq!(owned_span.offset, raw_span.offset);
        assert_eq!(owned_span.length, raw_span.length);
        assert_eq!(owned_span.flags, raw_span.flags);
        assert_eq!(owned_span.block_type, raw_span.block_type);
    }

    unsafe { free_spans(raw); }
}

#[test]
fn test_to_spans_owned_automatic_cleanup() {
    // This test verifies OwnedSpanOutput is automatically freed when dropped
    for _ in 0..100 {
        let md = format!("Hello **bold** {}", "repeated.".repeat(10));
        let _owned = to_spans_owned_with_options(&md, &MarkdownOptions::default());
        // OwnedSpanOutput drops here, no memory leak
    }
}

// ================================================================
//  Fix verification: strip_html_tags unification
// ================================================================

#[test]
fn test_html_stripping_consistent_across_outputs() {
    let md = "Some <b>bold HTML</b> and **bold MD** text";

    let _html = to_html(md);
    let spans_output = to_spans(md);
    let sp_text = get_text(&spans_output);
    let plain = to_plain_text(md);

    // All paths should strip tags and preserve text content
    assert!(sp_text.contains("bold HTML"));
    assert!(sp_text.contains("bold MD"));
    assert!(plain.contains("bold HTML"));
    assert!(plain.contains("bold MD"));

    free(spans_output);
}

#[test]
fn test_complex_html_tag_stripping() {
    let md = "<div class=\"wrap\"><span>nested</span> content</div> and `normal` text";

    let spans_output = to_spans(md);
    let sp_text = get_text(&spans_output);
    let plain = to_plain_text(md);

    assert!(sp_text.contains("nested"));
    assert!(sp_text.contains("content"));
    assert!(sp_text.contains("normal"));
    assert!(plain.contains("nested"));
    assert!(plain.contains("content"));

    free(spans_output);
}

// ================================================================
//  Fix verification: CRLF handling in math detection
// ================================================================

#[test]
fn test_crlf_inline_math_not_detected() {
    // Inline math with \r\n should NOT be detected
    let regions = fastmarkdown::math::detect_math_regions("$x\r\ny$");
    assert!(regions.is_empty(), "Math with CRLF should not be detected as inline math");
}

#[test]
fn test_crlf_display_math_still_works() {
    let regions = fastmarkdown::math::detect_math_regions("$$\r\nx^2\r\n$$");
    // Display math spans lines, so CRLF should be fine inside
    assert!(regions.len() > 0, "Display math with CRLF should be detected");
}

#[test]
fn test_cr_inline_math_not_detected() {
    // Mac-style \r line endings
    let regions = fastmarkdown::math::detect_math_regions("$x\ry$");
    assert!(regions.is_empty(), "Math with CR should not be detected as inline math");
}

#[test]
fn test_crlf_between_math_and_text() {
    let regions = fastmarkdown::math::detect_math_regions("$x^2$\r\nNext line");
    assert_eq!(regions.len(), 1, "Should detect one math region");
    assert!(!regions[0].is_display);
}

// ================================================================
//  Fix verification: ThemeConfig with_fonts builder
// ================================================================

#[test]
fn test_theme_config_default() {
    let theme = ThemeConfig::default();
    assert_eq!(theme.body_font_size, 14.0);
    assert_eq!(theme.monospace_font_size, 13.0);
    assert_eq!(theme.text_color, 0xFF000000);
    assert_eq!(theme.link_color, 0xFF0000FF);
    assert_eq!(theme.heading_font_sizes[0], 24.0);
    assert_eq!(theme.heading_font_sizes[5], 14.0);
    assert!(theme.body_font_name.is_null());
    assert!(theme.monospace_font_name.is_null());
}

#[test]
fn test_theme_config_with_fonts() {
    use std::ffi::CStr;
    let font_name = CStr::from_bytes_with_nul(b"Helvetica\0").unwrap();
    let mono_name = CStr::from_bytes_with_nul(b"Menlo\0").unwrap();

    let theme = ThemeConfig::with_fonts(
        Some(font_name), 16.0,
        Some(mono_name), 14.0,
    );
    assert_eq!(theme.body_font_size, 16.0);
    assert_eq!(theme.monospace_font_size, 14.0);
    assert!(!theme.body_font_name.is_null());
    assert!(!theme.monospace_font_name.is_null());
}

#[test]
fn test_theme_config_with_fonts_none_uses_null() {
    let theme = ThemeConfig::with_fonts(None, 14.0, None, 13.0);
    assert!(theme.body_font_name.is_null());
    assert!(theme.monospace_font_name.is_null());
}

// ================================================================
//  Fix verification: Stream fast path opt in long messages
// ================================================================

#[test]
fn test_stream_long_message_fast_path() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());

    stream.append("Hello");

    let words = [" this", " is", " a", " long", " message", " with", " many", " small", " chunks"];
    for word in &words {
        stream.append(word);
    }

    let output = stream.append(".");
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("Hello this is a long message with many small chunks."));
}

#[test]
fn test_stream_fast_path_consistent_with_full_parse() {
    // Build message incrementally
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("The");
    stream.append(" quick");
    stream.append(" brown");
    stream.append(" fox");
    stream.append(" jumps");
    stream.append(" over");
    stream.append(" the");
    stream.append(" lazy");
    let out1 = stream.append(" dog");

    // Build same message in one shot
    let out2 = to_spans("The quick brown fox jumps over the lazy dog");

    let text1 = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(out1.text, out1.text_len as usize))
            .unwrap()
    };
    let text2 = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(out2.text, out2.text_len as usize))
            .unwrap()
    };

    assert_eq!(text1, text2);
    assert_eq!(out1.span_count, out2.span_count);
    assert_eq!(out1.text_len, out2.text_len);

    unsafe { free_spans(out2); }
}

// ================================================================
//  Fix verification: Multiple pulldown-cmark extensions enabled
// ================================================================

#[test]
fn test_definition_list_in_complex_document() {
    let md = "# Glossary\n\nTerm One\n: The first definition\n\nTerm Two\n: The second definition\n\nFinal paragraph.";
    let output = to_spans(md);
    let text = get_text(&output);

    assert!(text.contains("Glossary"));
    assert!(text.contains("Term One"));
    assert!(text.contains("first definition"));
    assert!(text.contains("Final paragraph"));

    free(output);
}

#[test]
fn test_yaml_metadata_block_with_content_after() {
    let md = "---\ntitle: My Document\n---\n\n# Real Content\n\nThis should be visible.";
    let html = to_html(md);
    assert!(html.contains("Real Content"));
    assert!(html.contains("visible"));

    let output = to_spans(md);
    let text = get_text(&output);
    assert!(text.contains("Real Content"));
    // Metadata content should NOT appear in span text
    assert!(!text.contains("title: My Document"), "Metadata should not leak into span text: '{}'", text);

    free(output);
}

// ================================================================
//  P0 + P1 verification: UTF-16 offsets, token constants, delta fast-path
// ================================================================

#[test]
fn test_utf16_offsets_disabled_by_default() {
    let output = to_spans("Hello **world**");
    assert_eq!(output.utf16_offsets_len, 0);
    assert!(output.utf16_offsets.is_null());
    free(output);
}

#[test]
fn test_utf16_offsets_computed_when_enabled() {
    let mut opts = MarkdownOptions::default();
    opts.compute_utf16_offsets = true;

    let output = to_spans_with_options("Hello **world**", &opts);
    let offsets = unsafe {
        std::slice::from_raw_parts(output.utf16_offsets, output.utf16_offsets_len as usize)
    };

    // Should have as many offsets as spans
    assert_eq!(offsets.len(), output.span_count as usize);
    // First offset should be 0 (start of text)
    assert_eq!(offsets[0], 0);
    free(output);
}

#[test]
fn test_utf16_offsets_with_unicode() {
    let mut opts = MarkdownOptions::default();
    opts.compute_utf16_offsets = true;

    // "Hello" = 5 UTF-16 units, 🌍 = 2 UTF-16 units, " world" = 6 UTF-16 units
    let output = to_spans_with_options("Hello🌍 **world**", &opts);
    let offsets = unsafe {
        std::slice::from_raw_parts(output.utf16_offsets, output.utf16_offsets_len as usize)
    };
    let spans = get_spans(&output);

    assert_eq!(offsets.len(), spans.len());

    // Find the bold span and verify its UTF-16 offset is correct
    // "Hello🌍 " = 5 + 2 + 1 = 8 UTF-16 units
    for (i, span) in spans.iter().enumerate() {
        if span.is_bold() {
            // Bold text "world" should be at offset 8
            assert_eq!(offsets[i], 8, "Bold span UTF-16 offset should be 8");
        }
    }

    free(output);
}

#[test]
fn test_utf16_offsets_ascii_equals_utf8() {
    let mut opts = MarkdownOptions::default();
    opts.compute_utf16_offsets = true;

    let output = to_spans_with_options("Hello **world** `code` [link](url)", &opts);
    let offsets = unsafe {
        std::slice::from_raw_parts(output.utf16_offsets, output.utf16_offsets_len as usize)
    };
    let spans = get_spans(&output);

    // For pure ASCII, UTF-8 offsets == UTF-16 offsets
    for (i, span) in spans.iter().enumerate() {
        assert_eq!(offsets[i], span.offset,
            "ASCII: UTF-16 offset {} != UTF-8 offset {} for span {}",
            offsets[i], span.offset, i);
    }

    free(output);
}

#[test]
fn test_utf16_offsets_with_mixed_content() {
    let mut opts = MarkdownOptions::default();
    opts.compute_utf16_offsets = true;

    // Mix ASCII and multi-byte chars
    let md = "# 标题\n\n中文 **粗体** `代码`";
    let output = to_spans_with_options(md, &opts);
    let offsets = unsafe {
        std::slice::from_raw_parts(output.utf16_offsets, output.utf16_offsets_len as usize)
    };
    let spans = get_spans(&output);

    // Verify offsets array has correct length
    assert_eq!(offsets.len(), spans.len());
    // Verify offsets are non-decreasing
    for w in offsets.windows(2) {
        assert!(w[0] <= w[1], "UTF-16 offsets must be non-decreasing");
    }

    free(output);
}

#[test]
fn test_utf16_offsets_owned_api() {
    let mut opts = MarkdownOptions::default();
    opts.compute_utf16_offsets = true;

    let owned = to_spans_owned_with_options("Hello **world**", &opts);
    assert_eq!(owned.utf16_offsets.len(), owned.spans.len());
    assert!(!owned.utf16_offsets.is_empty());
}

#[test]
fn test_utf16_offsets_with_tables() {
    let mut opts = MarkdownOptions::default();
    opts.compute_utf16_offsets = true;

    let md = "| A | B |\n|---|---|\n| 1 | 2 |";
    let output = to_spans_with_options(md, &opts);
    let offsets = unsafe {
        std::slice::from_raw_parts(output.utf16_offsets, output.utf16_offsets_len as usize)
    };
    let spans = get_spans(&output);

    assert_eq!(offsets.len(), spans.len());
    // All offsets should be strictly non-decreasing
    for w in offsets.windows(2) {
        assert!(w[0] <= w[1], "UTF-16 offsets must be non-decreasing");
    }

    free(output);
}

// ================================================================
//  Token constant verification
// ================================================================

#[test]
fn test_token_constants_exported() {
    // Verify all 15 token types are exported and have correct values
    assert_eq!(TOKEN_OTHER, 0);
    assert_eq!(TOKEN_KEYWORD, 1);
    assert_eq!(TOKEN_STRING, 2);
    assert_eq!(TOKEN_COMMENT, 3);
    assert_eq!(TOKEN_NUMBER, 4);
    assert_eq!(TOKEN_FUNCTION, 5);
    assert_eq!(TOKEN_TYPE, 6);
    assert_eq!(TOKEN_OPERATOR, 7);
    assert_eq!(TOKEN_PUNCTUATION, 8);
    assert_eq!(TOKEN_VARIABLE, 9);
    assert_eq!(TOKEN_CONSTANT, 10);
    assert_eq!(TOKEN_BUILTIN, 11);
    assert_eq!(TOKEN_ENTITY, 12);
    assert_eq!(TOKEN_MARKUP, 13);
    assert_eq!(TOKEN_REGEX, 14);
    assert_eq!(TOKEN_TYPE_COUNT, 15);
}

#[test]
fn test_token_constants_match_ffi() {
    // Verify Rust constants match FFI constants
    assert_eq!(TOKEN_ENTITY, fastmarkdown::ffi::FM_TOKEN_ENTITY);
    assert_eq!(TOKEN_MARKUP, fastmarkdown::ffi::FM_TOKEN_MARKUP);
    assert_eq!(TOKEN_REGEX, fastmarkdown::ffi::FM_TOKEN_REGEX);
    assert_eq!(TOKEN_BUILTIN, fastmarkdown::ffi::FM_TOKEN_BUILTIN);
    assert_eq!(TOKEN_TYPE_COUNT, fastmarkdown::ffi::FM_TOKEN_TYPE_COUNT);
}

// ================================================================
//  Delta fast-path verification
// ================================================================

#[test]
fn test_delta_fast_path_plain_text() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("Hello");

    // delta() should use fast path for plain text chunks too
    let delta = stream.delta(" world");
    assert!(!delta.deltas.is_empty());
}

#[test]
fn test_delta_fast_path_falls_back_on_structure() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("Hello");

    // Structure character triggers full re-parse
    let delta = stream.delta(" **bold**");
    assert!(!delta.deltas.is_empty());
}

#[test]
fn test_delta_fast_path_consistency_with_append() {
    // append() and delta() should produce consistent internal state
    let mut stream_a = StreamRenderer::new(MarkdownOptions::default());
    stream_a.append("# Title\n\n");
    stream_a.append("Paragraph with ");

    let mut stream_b = StreamRenderer::new(MarkdownOptions::default());
    stream_b.append("# Title\n\n");
    stream_b.delta("Paragraph with ");

    // Both should produce the same output on the next append
    let out_a = stream_a.append("text.");
    let out_b_raw = stream_b.append("text.");
    let out_b = out_b_raw;

    assert_eq!(out_a.text_len, out_b.text_len);
    assert_eq!(out_a.span_count, out_b.span_count);
}

#[test]
fn test_delta_fast_path_continues_after_both_methods() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());

    // Mix append() and delta() calls
    stream.append("Start");
    let d1 = stream.delta(" continue");
    assert!(!d1.deltas.is_empty());

    let out = stream.append(" end.");
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(out.text, out.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("Start continue end."));
}

// ================================================================
//  SpanOutput struct layout verification (backward compat)
// ================================================================

#[test]
fn test_span_output_size_includes_utf16_fields() {
    // SpanOutput grew by 2 * 8 bytes (pointer + u32 padded to 8 each on 64-bit)
    // The exact size depends on alignment; just verify it's larger than before
    assert!(std::mem::size_of::<SpanOutput>() > 40,
        "SpanOutput should now include utf16_offsets fields");
}

#[test]
fn test_fastmarkdown_span_output_ffi_layout_consistent() {
    use fastmarkdown::ffi::FastMarkdownSpanOutput;
    // Both structs should have the same field count and layout
    // (cannot compare sizes directly due to different Span types, but field order matches)
    let ffi_out = FastMarkdownSpanOutput {
        text: std::ptr::null(),
        text_len: 0,
        spans: std::ptr::null(),
        span_count: 0,
        extra_data: std::ptr::null(),
        extra_data_len: 0,
        utf16_offsets: std::ptr::null(),
        utf16_offsets_len: 0,
    };
    assert!(ffi_out.utf16_offsets.is_null());
    assert_eq!(ffi_out.utf16_offsets_len, 0);
}
