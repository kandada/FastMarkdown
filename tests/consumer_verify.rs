// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

fn get_spans(output: &SpanOutput) -> &[Span] {
    unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) }
}

fn get_extra(output: &SpanOutput) -> &[u8] {
    unsafe { std::slice::from_raw_parts(output.extra_data, output.extra_data_len as usize) }
}

fn get_text(output: &SpanOutput) -> &str {
    unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(
            output.text,
            output.text_len as usize,
        ))
        .unwrap()
    }
}

fn free(output: SpanOutput) {
    unsafe {
        fastmarkdown::free_spans(output);
    }
}

fn parse_extra_chunks(extra: &[u8]) -> Vec<(u8, Vec<u8>)> {
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

// ================================================================
//  extra_data self-describing chunk format
// ================================================================

#[test]
fn test_extra_chunks_are_parseable() {
    let md = "[a](https://a.com) **x** [b](https://b.org) `code` [c](http://c.net) -- ![img](https://i.png)";
    let output = to_spans(md);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    assert!(
        chunks.len() >= 4,
        "Expected at least 4 chunks (3 links + 1 image), got {}",
        chunks.len()
    );
    for (kind, data) in &chunks {
        assert!(!data.is_empty(), "Empty chunk data for kind {}", kind);
        assert!(
            *kind == EXTRA_KIND_LINK_URL || *kind == EXTRA_KIND_IMAGE_URL,
            "Unexpected kind {}",
            kind
        );
    }
    free(output);
}

#[test]
fn test_extra_chunk_header_sizes() {
    let md = "[x](http://a)";
    let output = to_spans(md);
    let extra = get_extra(&output);

    // Chunk: kind(1) + len(2 LE) + data
    assert_eq!(extra[0], EXTRA_KIND_LINK_URL);
    let data_len = u16::from_le_bytes([extra[1], extra[2]]) as usize;
    let data = &extra[3..3 + data_len];
    // The URL data should contain "http://a"
    assert!(std::str::from_utf8(data).unwrap().contains("http"));
    assert!(
        data.len() >= 7,
        "Expected at least 7 bytes for URL, got {}",
        data.len()
    );

    free(output);
}

#[test]
fn test_extra_chunks_in_span_order() {
    let md = "[first](https://one.com) and [second](https://two.org)";
    let output = to_spans(md);
    let spans = get_spans(&output);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    let link_spans: Vec<&Span> = spans.iter().filter(|s| s.is_link()).collect();
    assert_eq!(
        link_spans.len(),
        chunks.len(),
        "Link spans ({}) and chunks ({}) should match",
        link_spans.len(),
        chunks.len()
    );
    assert_eq!(chunks[0].1, b"https://one.com");
    assert_eq!(chunks[1].1, b"https://two.org");

    free(output);
}

// ================================================================
//  Code block language via extra_data
// ================================================================

#[test]
fn test_code_language_in_extra_with_kind() {
    let md = "```python\ndef foo():\n    pass\n```";
    let output = to_spans(md);
    let _spans = get_spans(&output);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    // Should have a CODE_LANGUAGE chunk with "python"
    let lang_chunks: Vec<_> = chunks
        .iter()
        .filter(|(k, _)| *k == EXTRA_KIND_CODE_LANGUAGE)
        .collect();
    assert_eq!(lang_chunks.len(), 1, "Expected exactly 1 language chunk");
    assert_eq!(lang_chunks[0].1, b"python");

    // The chunk is matched positionally by the consumer (one per code block);
    // it is no longer span-linked via `extra_len`.
    free(output);
}

#[test]
fn test_code_block_no_language() {
    let md = "```\nplain code\n```";
    let output = to_spans(md);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    // An unlabeled code block still emits an (empty) language chunk so the
    // positional matching of later code blocks stays aligned.
    let lang_chunks: Vec<_> = chunks
        .iter()
        .filter(|(k, _)| *k == EXTRA_KIND_CODE_LANGUAGE)
        .collect();
    assert_eq!(lang_chunks.len(), 1, "one language chunk per code block");
    assert!(lang_chunks[0].1.is_empty(), "language chunk must be empty");
    free(output);
}

#[test]
fn test_code_block_with_spaces_in_language() {
    let md = "```c++\nint main() { return 0; }\n```";
    let output = to_spans(md);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    let lang = chunks
        .iter()
        .find(|(k, _)| *k == EXTRA_KIND_CODE_LANGUAGE)
        .expect("Language chunk missing");
    assert_eq!(lang.1, b"c++");
    free(output);
}

#[test]
fn test_code_block_bash() {
    let md = "```bash\necho hello\nls -la\n```";
    let output = to_spans(md);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    assert!(chunks
        .iter()
        .any(|(k, d)| *k == EXTRA_KIND_CODE_LANGUAGE && d == b"bash"));
    free(output);
}

// ================================================================
//  Table metadata via extra_data
// ================================================================

#[test]
fn test_table_metadata_parseable_by_consumer() {
    let md = "| A | B | C |\n|:---|:---:|---:|\n| 1 | 2 | 3 |";
    let output = to_spans(md);
    let spans = get_spans(&output);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    // Find table metadata chunk
    let meta = chunks
        .iter()
        .find(|(k, _)| *k == EXTRA_KIND_TABLE_METADATA)
        .expect("Table metadata missing");
    let col_count = meta.1[0] as usize;
    assert_eq!(col_count, 3);

    // Verify alignments (interleaved with u16 char_widths: offset 1, 4, 7)
    assert_eq!(meta.1[1], 1); // Left
    assert_eq!(meta.1[4], 2); // Center
    assert_eq!(meta.1[7], 3); // Right

    // Count cells
    let header_cells: Vec<_> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_TABLE_HEADER_CELL)
        .collect();
    let data_cells: Vec<_> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_TABLE_CELL)
        .collect();

    assert_eq!(header_cells.len(), col_count);
    assert_eq!(data_cells.len(), col_count); // 1 data row × 3 cols

    free(output);
}

#[test]
fn test_multiple_tables_each_have_metadata() {
    let md = "| A | B |\n|---|---|\n| 1 | 2 |\n\n| X | Y | Z |\n|---|---|---|\n| a | b | c |";
    let output = to_spans(md);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    let meta_chunks: Vec<_> = chunks
        .iter()
        .filter(|(k, _)| *k == EXTRA_KIND_TABLE_METADATA)
        .collect();
    assert_eq!(meta_chunks.len(), 2, "Two tables → two metadata chunks");

    // First table: 2 cols
    assert_eq!(meta_chunks[0].1[0], 2);
    // Second table: 3 cols
    assert_eq!(meta_chunks[1].1[0], 3);

    free(output);
}

#[test]
fn test_table_metadata_with_mixed_content_in_cells() {
    let md = "| Syntax | Example |\n|--------|---------|\n| **bold** | `code` |\n| *italic* | [link](url) |";
    let output = to_spans(md);
    let spans = get_spans(&output);
    let extra = get_extra(&output);
    let chunks = parse_extra_chunks(extra);

    // Metadata exists
    assert!(chunks.iter().any(|(k, _)| *k == EXTRA_KIND_TABLE_METADATA));

    // Inline styles inside table cells should be preserved
    assert!(spans.iter().any(|s| s.is_bold()));
    assert!(spans.iter().any(|s| s.is_inline_code()));
    assert!(spans.iter().any(|s| s.is_italic()));
    assert!(spans.iter().any(|s| s.is_link()));

    // Link inside cell should produce a link chunk
    assert!(chunks.iter().any(|(k, _)| *k == EXTRA_KIND_LINK_URL));

    free(output);
}

// ================================================================
//  Block structure (seq + depth)
// ================================================================

#[test]
fn test_paragraph_has_zero_depth() {
    let output = to_spans("A paragraph.");
    let spans = get_spans(&output);
    for s in spans {
        assert_eq!(s.block_depth, 0);
    }
    free(output);
}

#[test]
fn test_heading_has_zero_depth() {
    let output = to_spans("# Heading");
    let spans = get_spans(&output);
    for s in spans {
        assert_eq!(s.block_depth, 0);
    }
    free(output);
}

#[test]
fn test_blockquote_increases_depth() {
    let output = to_spans("> quoted text");
    let spans = get_spans(&output);
    // Inside blockquote, paragraphs have depth >= 1
    assert!(spans.iter().any(|s| s.block_depth >= 1));
    free(output);
}

#[test]
fn test_nested_blockquote_depth() {
    let md = "> > deeply nested quote";
    let output = to_spans(md);
    let spans = get_spans(&output);
    assert!(
        spans.iter().any(|s| s.block_depth >= 2),
        "Nested blockquotes should have depth >= 2"
    );
    free(output);
}

#[test]
fn test_list_items_share_same_depth_at_same_level() {
    let md = "- item a\n- item b\n- item c";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let depths: Vec<u8> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
        .map(|s| s.block_depth)
        .collect();
    assert!(depths.len() >= 3);
    let first = depths[0];
    assert!(
        depths.iter().all(|&d| d == first),
        "All items at same list level should have same depth: {:?}",
        depths
    );
    free(output);
}

#[test]
fn test_mixed_list_and_blockquote_nesting() {
    let md = "> - item inside blockquote";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let list_spans: Vec<_> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
        .collect();
    assert!(!list_spans.is_empty());
    // Inside blockquote → depth >= 2 (blockquote + list wrapping)
    assert!(list_spans.iter().any(|s| s.block_depth >= 1));
    free(output);
}

#[test]
fn test_each_paragraph_has_unique_seq() {
    let md = "Para one.\n\nPara two.\n\nPara three.";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let para_spans: Vec<_> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_PARAGRAPH)
        .collect();
    let mut seqs: Vec<u16> = para_spans.iter().map(|s| s.block_seq).collect();
    seqs.sort();
    seqs.dedup();
    assert_eq!(seqs.len(), 3, "3 paragraphs → 3 unique seq values");
    free(output);
}

#[test]
fn test_code_block_has_its_own_seq() {
    let md = "Before.\n\n```\ncode\n```\n\nAfter.";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let code_seqs: Vec<u16> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_CODE)
        .map(|s| s.block_seq)
        .collect();
    let para_seqs: Vec<u16> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_PARAGRAPH)
        .map(|s| s.block_seq)
        .collect();

    assert!(!code_seqs.is_empty());
    assert!(!para_seqs.is_empty());
    // Code block should have a different seq from adjacent paragraphs
    assert!(
        !code_seqs.iter().any(|&c| para_seqs.contains(&c)),
        "Code block seq should not overlap with paragraph seqs"
    );
    free(output);
}

#[test]
fn test_table_has_its_own_seq() {
    let md = "Before.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\nAfter.";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let table_seqs: Vec<u16> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_TABLE_CELL || s.block_type == BLOCK_TABLE_HEADER_CELL)
        .map(|s| s.block_seq)
        .collect();
    assert!(!table_seqs.is_empty());

    // Count unique seqs in table = num cells (each cell is a boundary)
    let mut unique: Vec<u16> = table_seqs.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 4, "4 cells → 4 unique seqs in table");
    free(output);
}

#[test]
fn test_horizontal_rule_is_separate_block() {
    let md = "Before.\n\n---\n\nAfter.";
    let output = to_spans(md);
    let spans = get_spans(&output);

    assert!(spans.iter().any(|s| s.block_type == BLOCK_HORIZONTAL_RULE));

    // HR span should exist with correct block type
    let hr_spans: Vec<_> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_HORIZONTAL_RULE)
        .collect();
    assert_eq!(hr_spans.len(), 1, "Should have exactly 1 HR span");
    free(output);
}

// ================================================================
//  Text buffer separators
// ================================================================

#[test]
fn test_text_buffer_has_block_separators() {
    let md = "# H1\n\nPara1\n\n- item";
    let output = to_spans(md);
    let text = get_text(&output);
    // Blocks should be separated by newlines
    assert!(
        text.contains("\n"),
        "Text buffer should have newline separators"
    );
    free(output);
}

#[test]
fn test_text_buffer_table_cell_tabs() {
    let md = "| A | B | C |\n|---|---|---|\n| 1 | 2 | 3 |";
    let output = to_spans(md);
    let text = get_text(&output);

    // Cells within a row should be tab-separated
    let tab_count = text.matches('\t').count();
    // Header row: 2 tabs between 3 cells; Data row: 2 tabs between 3 cells = 4 tabs
    assert!(
        tab_count >= 2,
        "Expected tabs between table cells, got {} tabs in: {:?}",
        tab_count,
        text
    );
    free(output);
}

#[test]
fn test_text_buffer_table_row_newlines() {
    let md = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
    let output = to_spans(md);
    let text = get_text(&output);

    // Should have newlines between rows
    let newline_count = text.matches('\n').count();
    assert!(
        newline_count >= 2,
        "Expected newlines between table rows, got {}",
        newline_count
    );
    free(output);
}

// ================================================================
//  Math display/inline detection via pulldown-cmark ENABLE_MATH
// ================================================================

#[test]
fn test_display_math_gets_block_type() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let output = to_spans_with_options("$$\nx^2 + y^2\n$$", &opts);
    let spans = get_spans(&output);

    // Display math should produce BLOCK_MATH_BLOCK type
    assert!(
        spans.iter().any(|s| s.block_type == BLOCK_MATH_BLOCK),
        "Display math should use BLOCK_MATH_BLOCK, spans: {:?}",
        spans
            .iter()
            .map(|s| (s.block_type, s.flags))
            .collect::<Vec<_>>()
    );
    free(output);
}

#[test]
fn test_inline_math_has_flag() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let output = to_spans_with_options("Formula $x^2$ here", &opts);
    let spans = get_spans(&output);

    assert!(
        spans.iter().any(|s| s.is_math()),
        "Inline math should have FLAG_MATH"
    );
    free(output);
}

#[test]
fn test_dollar_literal_no_math_in_spans() {
    // Default Literal mode: $ should not trigger math
    let output = to_spans("Price: $5.99 for $HOME");
    let spans = get_spans(&output);

    assert!(
        spans.iter().all(|s| !s.is_math()),
        "Literal mode: no span should have math flag"
    );
    free(output);
}

// ================================================================
//  Consumer simulation: full file preview reconstruction
// ================================================================

#[test]
fn test_consumer_reconstruct_full_document() {
    let md = "\
# Document Title

This is a **bold** introduction.

## Section A

Some paragraph with `code` inside.

```rust
fn main() {
    println!(\"Hello\");
}
```

### Subsection

| Name  | Value |
|-------|-------|
| Alice | 30    |
| Bob   | 25    |

> A blockquote with *italic* text.

- Item one
- Item two
  - Nested item
- Item three
";

    let output = to_spans(md);
    let spans = get_spans(&output);
    let extra = get_extra(&output);
    let text = get_text(&output);
    let chunks = parse_extra_chunks(extra);

    // ---- 1. Reconstruct blocks by seq ----
    let mut blocks: Vec<Vec<&Span>> = vec![];
    let mut current_seq = spans[0].block_seq;
    let mut current: Vec<&Span> = vec![];
    for s in spans.iter() {
        if s.block_seq != current_seq {
            blocks.push(std::mem::take(&mut current));
            current_seq = s.block_seq;
        }
        current.push(s);
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    assert!(
        blocks.len() >= 10,
        "Expected >=10 blocks, got {}",
        blocks.len()
    );

    // ---- 2. Verify block types ----
    let block_types: Vec<u8> = blocks.iter().map(|b| b[0].block_type).collect();
    assert!(block_types.contains(&BLOCK_HEADING_H1));
    assert!(block_types.contains(&BLOCK_HEADING_H2));
    assert!(block_types.contains(&BLOCK_HEADING_H3));
    assert!(block_types.contains(&BLOCK_PARAGRAPH));
    assert!(block_types.contains(&BLOCK_CODE));
    assert!(block_types.contains(&BLOCK_TABLE_HEADER_CELL));
    assert!(block_types.contains(&BLOCK_TABLE_CELL));
    assert!(block_types.contains(&BLOCK_LIST_ITEM_UNORDERED));

    // ---- 3. Extract code language ----
    let code_lang = chunks.iter().find(|(k, _)| *k == EXTRA_KIND_CODE_LANGUAGE);
    assert!(code_lang.is_some(), "Should have code language chunk");
    assert_eq!(code_lang.unwrap().1, b"rust");

    // ---- 4. Extract table metadata ----
    let table_meta = chunks.iter().find(|(k, _)| *k == EXTRA_KIND_TABLE_METADATA);
    assert!(table_meta.is_some(), "Should have table metadata chunk");
    let col_count = table_meta.unwrap().1[0] as usize;
    assert_eq!(col_count, 2);

    // ---- 5. Count table rows ----
    let data_cells: Vec<_> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_TABLE_CELL)
        .collect();
    let rows = data_cells.len() / col_count;
    assert_eq!(rows, 2);

    // ---- 6. Verify list depths ----
    let list_depths: Vec<u8> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
        .map(|s| s.block_depth)
        .collect();
    // Top-level items: depth 1 (List is a nesting container)
    assert!(
        list_depths.contains(&1),
        "Should have top-level items at depth 1, got {:?}",
        list_depths
    );
    // Nested item has depth >= 2
    assert!(
        list_depths.iter().any(|&d| d >= 2),
        "Should have nested items"
    );

    // ---- 7. Verify inline styles are preserved ----
    assert!(spans.iter().any(|s| s.is_bold()));
    assert!(spans.iter().any(|s| s.is_inline_code()));
    assert!(spans.iter().any(|s| s.is_italic()));

    // ---- 8. Verify link extraction ----
    let link_count = chunks
        .iter()
        .filter(|(k, _)| *k == EXTRA_KIND_LINK_URL)
        .count();
    assert_eq!(link_count, 0, "No links in this document");

    // ---- 9. Text buffer is non-trivial ----
    assert!(
        text.len() > 50,
        "Text buffer should contain the full document"
    );

    free(output);
}
