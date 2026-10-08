// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

/// Tests that run quickly but verify performance characteristics
/// (no actual microbenchmarks; use `cargo bench` for that)

#[test]
fn test_html_generation_is_fast() {
    let md = "# Hello\n\n**bold** and `code` in paragraph.\n\n- list item\n\n```\ncode block\n```"
        .repeat(100);

    let start = std::time::Instant::now();
    let html = to_html(&md);
    let elapsed = start.elapsed();

    // Should process 100x repetitions of a typical message in well under 100ms
    assert!(html.len() > 10000);
    assert!(
        elapsed.as_millis() < 500,
        "HTML generation of {} chars took {:?}",
        md.len(),
        elapsed
    );
}

#[test]
fn test_spans_generation_is_fast() {
    let md = "# Hello\n\n**bold** and *italic* with `code`.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n"
        .repeat(50);

    let start = std::time::Instant::now();
    let output = to_spans(&md);
    let elapsed = start.elapsed();

    assert!(output.span_count > 10);
    assert!(
        elapsed.as_millis() < 500,
        "Span generation of {} chars took {:?}",
        md.len(),
        elapsed
    );

    unsafe {
        fastmarkdown::free_spans(output);
    }
}

#[test]
fn test_plain_text_is_fast() {
    let md = "# Title\n\n**bold** and *italic* with [links](https://example.com).\n\n- item\n"
        .repeat(200);

    let start = std::time::Instant::now();
    let text = to_plain_text(&md);
    let elapsed = start.elapsed();

    assert!(!text.is_empty());
    assert!(
        elapsed.as_millis() < 500,
        "Plain text extraction of {} chars took {:?}",
        md.len(),
        elapsed
    );
}

#[test]
fn test_streaming_is_fast() {
    let chunks: Vec<&str> = vec![
        "# Hello\n\n",
        "This is a ",
        "**bold** ",
        "and *italic* ",
        "paragraph.\n\n",
        "- item 1\n",
        "- item 2\n",
        "- item 3\n\n",
        "```\ncode\n```\n",
    ];

    let start = std::time::Instant::now();
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    for chunk in &chunks {
        stream.append(chunk);
    }
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() < 100, "Streaming took {:?}", elapsed);
}

#[test]
fn test_math_detection_is_fast() {
    let md = "Price $5.99 but formula $x^2 + y^2 = z^2$ and $$\\frac{1}{2}$$".repeat(100);

    let start = std::time::Instant::now();
    let regions = fastmarkdown::math::detect_math_regions(&md);
    let elapsed = start.elapsed();

    assert!(!regions.is_empty());
    assert!(
        elapsed.as_millis() < 100,
        "Math detection of {} chars took {:?}",
        md.len(),
        elapsed
    );
}

#[test]
fn test_large_table_performance() {
    let mut md = String::from(
        "| Col1 | Col2 | Col3 | Col4 | Col5 |\n|------|------|------|------|------|\n",
    );
    for i in 0..200 {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            i,
            i * 2,
            i * 3,
            i * 4,
            i * 5
        ));
    }

    let start = std::time::Instant::now();
    let html = to_html(&md);
    let elapsed = start.elapsed();

    assert!(html.contains("<table>"));
    assert!(
        elapsed.as_millis() < 500,
        "Large table render took {:?}",
        elapsed
    );
}

#[test]
fn test_no_leak_with_many_calls() {
    // Make many calls to ensure no memory leak
    let md = "Hello **world** `code` [link](https://x.com)";

    for _ in 0..100 {
        let output = to_spans(md);
        assert!(output.span_count > 0);
        unsafe {
            fastmarkdown::free_spans(output);
        }

        let html = to_html(md);
        assert!(!html.is_empty());

        let text = to_plain_text(md);
        assert!(!text.is_empty());
    }
}

#[test]
fn test_html_output_size_roughly_proportional() {
    let short = to_html("**x**");
    let long = to_html(&"**x** ".repeat(100));

    let short_len = short.len();
    let long_len = long.len();

    // Long output should be roughly proportional (within factor of 3)
    assert!(long_len > short_len * 10);
    assert!(long_len < short_len * 300);
}

#[test]
fn test_zero_allocation_for_empty() {
    let output = to_spans("");
    assert_eq!(output.span_count, 0);
    assert_eq!(output.text_len, 0);
    unsafe {
        fastmarkdown::free_spans(output);
    }
}
