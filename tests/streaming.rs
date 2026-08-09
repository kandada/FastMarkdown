// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

#[test]
fn test_stream_single_chunk() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    let output = stream.append("Hello **world**");

    assert!(output.span_count > 0);
    let spans = unsafe { std::slice::from_raw_parts(output.spans as *const Span, output.span_count as usize) };
    assert!(spans.iter().any(|s| s.is_bold()));
}

#[test]
fn test_stream_multiple_chunks() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("# He");
    stream.append("llo\n\n");
    let output = stream.append("**bold text**");

    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("Hello"));
    assert!(text.contains("bold text"));
}

#[test]
fn test_stream_finish() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("Hello");
    let final_output = stream.finish();

    assert!(final_output.is_some());
    let output = final_output.unwrap();
    assert!(output.span_count > 0);
}

#[test]
fn test_stream_reset() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("first message");

    stream.reset();
    assert!(stream.is_empty());

    let output = stream.append("second message");
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("second message"));
    assert!(!text.contains("first message"));
}

#[test]
fn test_stream_empty_chunks() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    let output = stream.append("");
    assert_eq!(output.span_count, 0);

    stream.append("actual text");
    let output = stream.append("");
    assert!(output.span_count > 0); // Previous text should still be there
}

#[test]
fn test_stream_progressive_building() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());

    // Build a markdown document chunk by chunk
    let output1 = stream.append("# Title\n");
    let text1 = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output1.text, output1.text_len as usize))
            .unwrap()
    };
    assert!(text1.contains("Title"));

    let _output2 = stream.append("\nParagraph with ");
    let output3 = stream.append("**bold**");

    let text3 = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output3.text, output3.text_len as usize))
            .unwrap()
    };
    assert!(text3.contains("Title"));
    assert!(text3.contains("bold"));
    assert!(text3.contains("Paragraph"));
}

#[test]
fn test_stream_code_block_progressive() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());

    stream.append("```rust\n");
    stream.append("fn main() {\n");
    stream.append("    println!(\"hi\");\n");
    let output = stream.append("}\n```");

    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("fn main()"));
    assert!(text.contains("println"));

    let spans = unsafe { std::slice::from_raw_parts(output.spans as *const Span, output.span_count as usize) };
    assert!(spans.iter().any(|s| s.block_type == BLOCK_CODE));
}

#[test]
fn test_stream_table_progressive() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());

    stream.append("| Name | Age |\n");
    stream.append("|------|-----|\n");
    let output = stream.append("| Alice | 30 |\n");

    let spans = unsafe { std::slice::from_raw_parts(output.spans as *const Span, output.span_count as usize) };
    assert!(spans.iter().any(|s| s.block_type == BLOCK_TABLE_HEADER_CELL));
}

#[test]
fn test_stream_snapshot_owned() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("Hello **world**");

    let snapshot = stream.snapshot_owned();
    let output = snapshot.as_output();

    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("world"));

    // Snapshot is independent; stream can continue
    stream.append("\nMore text");
    // The snapshot should still be valid because OwnedSpanOutput owns its memory
    let text2 = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    assert_eq!(text2, text);
}

#[test]
fn test_stream_delta_basic() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());

    // Initial append
    stream.append("Hello");

    // Next append with delta
    let delta = stream.delta(" **world**");
    assert!(!delta.deltas.is_empty());
}

#[test]
fn test_stream_delta_new_content() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("Start");

    let delta = stream.delta(" end");
    assert!(!delta.deltas.is_empty());
}

// ================================================================
//  Fast-path streaming tests
// ================================================================

#[test]
fn test_stream_fast_path_avoid_full_reparse() {
    // This test verifies that plain text chunks use the fast path
    // and produce the same result as full parsing
    let mut stream = StreamRenderer::new(MarkdownOptions::default());

    // Build markdown structure first
    stream.append("# Title\n\n");
    let out = stream.append("Paragraph");

    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(out.text, out.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("Title"));
    assert!(text.contains("Paragraph"));
    assert!(out.span_count > 0);

    // Now extend the paragraph with plain text (should use fast path)
    let out2 = stream.append(" with more content");
    let text2 = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(out2.text, out2.text_len as usize))
            .unwrap()
    };
    assert!(text2.contains("with more content"));

    // Build same thing in one shot for comparison
    let expected = to_spans("# Title\n\nParagraph with more content");
    let expected_text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(
            expected.text, expected.text_len as usize,
        )).unwrap()
    };
    assert_eq!(text2, expected_text);
    unsafe { free_spans(expected); }
}

#[test]
fn test_stream_fast_path_with_special_chars_falls_back() {
    // Special characters should trigger full re-parse
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("Hello");

    // * triggers fallback to full parse
    let out = stream.append(" *italic*");
    let spans = unsafe {
        std::slice::from_raw_parts(out.spans as *const Span, out.span_count as usize)
    };
    assert!(spans.iter().any(|s| s.is_italic()), "Should detect italic");
}

#[test]
fn test_stream_fast_path_with_code_block_falls_back() {
    // Code blocks should NOT use fast path (safety)
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("```rust\n");

    // Appending to code block — should use slow path
    let out = stream.append("fn main() {}\n");
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(out.text, out.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("fn main()"));
}

#[test]
fn test_stream_fast_path_many_small_chunks() {
    // Simulate token-level streaming (AI agents)
    let mut stream = StreamRenderer::new(MarkdownOptions::default());

    let tokens: [&str; 11] = ["H","e","l","l","o"," ","W","o","r","l","d"];
    for ch in &tokens {
        stream.append(ch);
    }

    let out = stream.append("!");
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(out.text, out.text_len as usize))
            .unwrap()
    };
    assert_eq!(text, "Hello World!");
}

#[test]
fn test_stream_fast_path_reset_clears_state() {
    let mut stream = StreamRenderer::new(MarkdownOptions::default());
    stream.append("First message");
    stream.reset();

    let out = stream.append("Second");
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(out.text, out.text_len as usize))
            .unwrap()
    };
    assert_eq!(text, "Second");
}
