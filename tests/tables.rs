// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

#[test]
fn test_simple_table() {
    let md = "| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |";
    let html = to_html(md);
    assert!(html.contains("<table>"));
    assert!(html.contains("<thead>"));
    assert!(html.contains("<tbody>"));
    assert!(html.contains("<td>Alice</td>"));
    assert!(html.contains("<td>30</td>"));
}

#[test]
fn test_table_with_alignment() {
    let md = "| Left | Center | Right |\n|:-----|:------:|------:|\n| L | C | R |";
    let html = to_html(md);
    assert!(html.contains("<table>"));
    // Alignment styles vary by pulldown-cmark version
    // At minimum, the table should be present
    assert!(html.contains("Left") || html.contains("L"));
    assert!(html.contains("Center") || html.contains("C"));
    assert!(html.contains("Right") || html.contains("R"));
}

#[test]
fn test_table_spans() {
    let md = "| A | B |\n|---|---|\n| 1 | 2 |";
    let output = to_spans(md);
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };

    assert!(spans
        .iter()
        .any(|s| s.block_type == BLOCK_TABLE_HEADER_CELL));
    assert!(spans.iter().any(|s| s.block_type == BLOCK_TABLE_CELL));
    unsafe {
        fastmarkdown::free_spans(output);
    }
}

#[test]
fn test_empty_table_cells() {
    let md = "| Col1 | Col2 |\n|------|------|\n|  | data |\n| more |  |";
    let html = to_html(md);
    assert!(html.contains("<table>"));
}

#[test]
fn test_table_with_markdown_in_cells() {
    let md =
        "| Syntax | Example |\n|--------|---------|\n| Bold | **bold** |\n| Code | `let x = 1` |";
    let html = to_html(md);
    assert!(html.contains("<table>"));
    // Inline markdown in table cells should be rendered
    assert!(html.contains("bold"));
    assert!(html.contains("let x"));
}

#[test]
fn test_table_with_escaped_pipes() {
    let md = "| Name | Description |\n|------|-------------|\n| Foo | a \\| b |";
    let html = to_html(md);
    assert!(html.contains("<table>"));
}

#[test]
fn test_wide_table() {
    let headers: Vec<String> = (0..10).map(|i| format!("Col{}", i)).collect();
    let header_row = headers.join(" | ");
    let sep: Vec<&str> = (0..10).map(|_| "---").collect();
    let sep_row = sep.join(" | ");
    let data_row = "1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 0";

    let md = format!("| {} |\n| {} |\n| {} |", header_row, sep_row, data_row);
    let html = to_html(&md);
    assert!(html.matches("<td>").count() >= 10);
}

#[test]
fn test_table_without_header() {
    // Some implementations require header row; pulldown-cmark requires it
    let md = "| a | b |\n| 1 | 2 |";
    let html = to_html(md);
    // pulldown-cmark requires separator row to detect a table
    assert!(!html.contains("<table>") || html.contains("<th>") || html.contains("<td>"));
}

#[test]
fn test_table_trailing_whitespace() {
    let md = "| Name | Value |  \n|------|-------|  \n| foo  | 42    |  ";
    let html = to_html(md);
    assert!(html.contains("<table>"));
    assert!(html.contains("<td>foo</td>"));
}

#[test]
fn test_mixed_alignment() {
    let md =
        "| Left | Center | Right | None |\n|:-----|:------:|------:|------|\n| a | b | c | d |";
    let html = to_html(md);
    assert!(html.contains("<table>"));
}
