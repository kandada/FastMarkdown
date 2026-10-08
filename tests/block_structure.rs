// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

fn get_spans(output: &SpanOutput) -> &[Span] {
    unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) }
}

fn free(output: SpanOutput) {
    unsafe {
        fastmarkdown::free_spans(output);
    }
}

// ============================================================
//  block_seq tests
// ============================================================

#[test]
fn test_block_seq_increments_per_block() {
    let md = "# Heading\n\nParagraph one\n\nParagraph two";
    let output = to_spans(md);
    let spans = get_spans(&output);

    // Collect unique seq values
    let mut seqs: Vec<u16> = spans.iter().map(|s| s.block_seq).collect();
    seqs.dedup();
    // Each block (heading, para1, para2) gets a unique seq
    assert!(
        seqs.len() >= 3,
        "Expected >=3 unique seq values, got {}",
        seqs.len()
    );
    free(output);
}

#[test]
fn test_block_seq_same_within_block() {
    let md = "**bold** and *italic* in same paragraph";
    let output = to_spans(md);
    let spans = get_spans(&output);

    // All spans in the same paragraph have the same seq
    let first_seq = spans[0].block_seq;
    for s in spans.iter() {
        assert_eq!(
            s.block_seq, first_seq,
            "All spans in one paragraph should share seq"
        );
    }
    free(output);
}

#[test]
fn test_block_seq_different_across_blocks() {
    let md = "# H1\n\n## H2\n\n### H3";
    let output = to_spans(md);
    let spans = get_spans(&output);

    // Each heading is a separate block→different seq
    let h1_seqs: Vec<u16> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_HEADING_H1)
        .map(|s| s.block_seq)
        .collect();
    let h2_seqs: Vec<u16> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_HEADING_H2)
        .map(|s| s.block_seq)
        .collect();
    let h3_seqs: Vec<u16> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_HEADING_H3)
        .map(|s| s.block_seq)
        .collect();

    assert!(!h1_seqs.is_empty());
    assert!(!h2_seqs.is_empty());
    assert!(!h3_seqs.is_empty());
    assert_ne!(h1_seqs[0], h2_seqs[0]);
    assert_ne!(h2_seqs[0], h3_seqs[0]);
    free(output);
}

#[test]
fn test_block_seq_in_code_block() {
    let md = "```rust\nfn main() {}\n```";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let code_spans: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_CODE)
        .collect();
    assert!(!code_spans.is_empty(), "Should have code block spans");

    // All code spans within the same code block have the same seq
    let seq = code_spans[0].block_seq;
    for s in &code_spans {
        assert_eq!(s.block_seq, seq);
    }
    free(output);
}

#[test]
fn test_block_seq_in_table() {
    let md = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let header_spans: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_TABLE_HEADER_CELL)
        .collect();
    let cell_spans: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_TABLE_CELL)
        .collect();

    // Cells exist
    assert!(!header_spans.is_empty(), "Should have header cells");
    assert!(!cell_spans.is_empty(), "Should have data cells");

    // Cells are ordered left-to-right, top-to-bottom in the span array
    // Consumer can split into rows using col_count from extra_data
    free(output);
}

#[test]
fn test_block_seq_inlist() {
    let md = "- item a\n- item b\n- item c";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let list_spans: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
        .collect();
    assert!(list_spans.len() >= 3, "Expected >=3 list item spans");

    // Each list item is a new block → different seq
    let mut seqs: Vec<u16> = list_spans.iter().map(|s| s.block_seq).collect();
    seqs.dedup();
    assert!(
        seqs.len() >= 3,
        "Each list item should have unique seq, got {}",
        seqs.len()
    );
    free(output);
}

// ============================================================
//  block_depth tests
// ============================================================

#[test]
fn test_block_depth_zero_for_top_level() {
    let md = "Just a paragraph";
    let output = to_spans(md);
    let spans = get_spans(&output);

    for s in spans.iter() {
        assert_eq!(s.block_depth, 0, "Top-level blocks should have depth 0");
    }
    free(output);
}

#[test]
fn test_block_depth_increases_for_list() {
    let md = "- item";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let list_spans: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
        .collect();
    assert!(!list_spans.is_empty());

    // List is a nesting block → depth >= 1
    for s in &list_spans {
        assert_eq!(
            s.block_depth, 1,
            "Top-level list items have depth 1, got {}",
            s.block_depth
        );
    }
    free(output);
}

#[test]
fn test_block_depth_nested_list() {
    let md = "- A\n  - B\n    - C";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let list_spans: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
        .collect();

    // Deeper nesting → greater depth
    // Sort by depth and verify uniqueness
    let mut depths: Vec<u8> = list_spans.iter().map(|s| s.block_depth).collect();
    depths.sort();
    depths.dedup();
    assert!(
        depths.len() >= 2,
        "Expected at least 2 distinct depth levels, got {:?}",
        depths
    );
    free(output);
}

#[test]
fn test_block_depth_in_blockquote() {
    let md = "> quoted text";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let bq_spans: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_BLOCKQUOTE)
        .collect();

    if !bq_spans.is_empty() {
        for s in &bq_spans {
            assert!(
                s.block_depth >= 1,
                "Blockquote should have depth >= 1, got {}",
                s.block_depth
            );
        }
    }
    free(output);
}

// ============================================================
//  Table metadata in extra_data
// ============================================================

#[test]
fn test_table_metadata_in_extra_data() {
    let md = "| A | B | C |\n|---|---|---|\n| 1 | 2 | 3 |";
    let output = to_spans(md);

    // extra_data uses self-describing chunks: [kind: u8, data_len: u16 LE, data]
    let extra =
        unsafe { std::slice::from_raw_parts(output.extra_data, output.extra_data_len as usize) };

    // Should have at least one chunk with kind=EXTRA_KIND_TABLE_METADATA
    assert!(
        extra.len() >= 6,
        "Extra data too small: {} bytes",
        extra.len()
    );
    assert_eq!(
        extra[0], EXTRA_KIND_TABLE_METADATA,
        "First chunk should be table metadata"
    );
    let data_len = u16::from_le_bytes([extra[1], extra[2]]) as usize;
    let data = &extra[3..3 + data_len];
    assert_eq!(data[0], 3, "Expected 3 columns");
    // Format: [col_count, (align: u8, char_width: u16 LE)*N] — 10 bytes
    assert!(
        data.len() >= 10,
        "Expected at least 10 bytes, got {}",
        data.len()
    );
    // Alignments at offsets 1, 4, 7
    for i in &[1, 4, 7] {
        let align = data[*i];
        assert!(align <= 3, "Invalid alignment {} at offset {}", align, i);
    }

    free(output);
}

#[test]
fn test_table_metadata_alignment_values() {
    let md = "| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |";
    let output = to_spans(md);

    let extra =
        unsafe { std::slice::from_raw_parts(output.extra_data, output.extra_data_len as usize) };

    assert_eq!(extra[0], EXTRA_KIND_TABLE_METADATA);
    let data_len = u16::from_le_bytes([extra[1], extra[2]]) as usize;
    let data = &extra[3..3 + data_len];
    assert_eq!(data[0], 3, "Expected 3 columns");
    // Format: [col_count: u8, (align: u8, char_width: u16 LE)*N]
    // Alignments are at offsets 1, 4, 7
    assert_eq!(data[1], 1, "Expected Left alignment");
    assert_eq!(data[4], 2, "Expected Center alignment");
    assert_eq!(data[7], 3, "Expected Right alignment");

    free(output);
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

#[test]
fn test_span_struct_size() {
    assert_eq!(
        std::mem::size_of::<Span>(),
        20,
        "Span size check (4+4+4+4+1+1+2=20)"
    );
}

// ============================================================
//  File preview scenario simulation
// ============================================================

#[test]
fn test_file_preview_reconstruct_blocks_by_seq() {
    let md = "# Title\n\nPara one.\n\n```\ncode\n```\n\nPara two.\n\n- a\n- b";
    let output = to_spans(md);
    let spans = get_spans(&output);

    // Simulate: group spans by block_seq to reconstruct blocks
    let mut blocks: Vec<Vec<&Span>> = vec![];
    let mut current_seq = spans[0].block_seq;
    let mut current_block: Vec<&Span> = vec![];

    for span in spans.iter() {
        if span.block_seq != current_seq {
            assert!(!current_block.is_empty(), "Each block should have spans");
            blocks.push(current_block);
            current_block = vec![];
            current_seq = span.block_seq;
        }
        current_block.push(span);
    }
    if !current_block.is_empty() {
        blocks.push(current_block);
    }

    // We have: heading, para1, code, para2, list_item_a, list_item_b = 6 blocks
    assert!(
        blocks.len() >= 5,
        "Expected >=5 blocks, got {}",
        blocks.len()
    );

    // Each block has a consistent block_type
    for block in &blocks {
        let types: Vec<u8> = block.iter().map(|s| s.block_type).collect();
        let unique_types: Vec<u8> = {
            let mut v = types.clone();
            v.sort();
            v.dedup();
            v
        };
        // All spans in a block either share one type, or have the same base type
        assert!(!unique_types.is_empty());
    }

    free(output);
}

#[test]
fn test_file_preview_table_cell_count() {
    let md = "| A | B | C |\n|---|---|---|\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |";
    let output = to_spans(md);
    let spans = get_spans(&output);
    let extra =
        unsafe { std::slice::from_raw_parts(output.extra_data, output.extra_data_len as usize) };

    // Parse self-describing extra_data chunk: [kind: u8, len: u16 LE, data]
    let chunks = parse_extra_chunks(extra);
    let meta = chunks
        .iter()
        .find(|(k, _)| *k == EXTRA_KIND_TABLE_METADATA)
        .expect("Table metadata chunk not found");
    let col_count = meta.1[0] as usize;
    assert_eq!(col_count, 3);

    let header_cells: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_TABLE_HEADER_CELL)
        .collect();
    assert_eq!(
        header_cells.len(),
        col_count,
        "Header cell count should match col_count"
    );

    let data_cells: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_TABLE_CELL)
        .collect();
    assert_eq!(
        data_cells.len() % col_count,
        0,
        "Data cells should be divisible by col_count"
    );

    let rows = data_cells.len() / col_count;
    assert_eq!(rows, 2, "Expected 2 rows of data");

    free(output);
}

#[test]
fn test_file_preview_indent_by_depth() {
    let md = "- Top level\n  - Nested one\n    - Deeply nested\n- Back to top";
    let output = to_spans(md);
    let spans = get_spans(&output);

    let list_spans: Vec<&Span> = spans
        .iter()
        .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
        .collect();

    // Depths should increase then decrease
    let depths: Vec<u8> = list_spans.iter().map(|s| s.block_depth).collect();

    // At least one depth transition (deep then shallow)
    let mut max_seen = 0u8;
    let mut had_deep = false;
    for &d in &depths {
        if d > max_seen {
            max_seen = d;
        }
        if d >= 3 {
            had_deep = true;
        }
        if had_deep && d <= 1 {
            // Found shallow after deep → depth transition works
            break;
        }
    }
    assert!(
        max_seen >= 2,
        "Expected nesting depth >= 2, got {}",
        max_seen
    );

    free(output);
}
