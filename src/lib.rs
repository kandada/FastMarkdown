// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

pub mod options;
pub mod span;
pub mod theme;
pub mod math;
pub mod html;
pub mod plain;
pub mod spans_renderer;
pub mod stream;
pub mod ffi;

pub use options::{HtmlMode, MarkdownOptions, MathMode};
pub use theme::ThemeConfig;
pub use span::{
    Span, SpanOutput, SpanDelta, SpanDeltaKind,
    FLAG_BOLD, FLAG_ITALIC, FLAG_STRIKETHROUGH, FLAG_INLINE_CODE,
    FLAG_LINK, FLAG_IMAGE, FLAG_MATH,
    BLOCK_PARAGRAPH, BLOCK_HEADING_H1, BLOCK_HEADING_H2, BLOCK_HEADING_H3,
    BLOCK_HEADING_H4, BLOCK_HEADING_H5, BLOCK_HEADING_H6,
    BLOCK_CODE, BLOCK_BLOCKQUOTE, BLOCK_LIST_ITEM_ORDERED,
    BLOCK_LIST_ITEM_UNORDERED, BLOCK_TABLE_CELL, BLOCK_TABLE_HEADER_CELL,
    BLOCK_HORIZONTAL_RULE, BLOCK_MATH_BLOCK, BLOCK_IMAGE,
};
pub use stream::StreamRenderer;

/// Render markdown to HTML with default options
pub fn to_html(md: &str) -> String {
    html::markdown_to_html(md, &MarkdownOptions::default())
}

/// Render markdown to HTML with custom options
pub fn to_html_with_options(md: &str, options: &MarkdownOptions) -> String {
    html::markdown_to_html(md, options)
}

/// Render markdown to structured Span output with default options
pub fn to_spans(md: &str) -> SpanOutput {
    spans_renderer::markdown_to_spans(md, &MarkdownOptions::default())
}

/// Render markdown to structured Span output with custom options
pub fn to_spans_with_options(md: &str, options: &MarkdownOptions) -> SpanOutput {
    spans_renderer::markdown_to_spans(md, options)
}

/// Extract plain text from markdown
pub fn to_plain_text(md: &str) -> String {
    plain::markdown_to_plain_text(md, &MarkdownOptions::default())
}

/// Free SpanOutput memory (for Rust API consumers)
///
/// # Safety
/// The SpanOutput must have been created by `to_spans` or `to_spans_with_options`
/// and must not have been freed already.
pub unsafe fn free_spans(output: SpanOutput) {
    if output.text.is_null() {
        return;
    }
    if output.text_len > 0 {
        let fat = std::ptr::slice_from_raw_parts_mut(
            output.text as *mut u8,
            output.text_len as usize,
        );
        drop(Box::from_raw(fat));
    }
    if output.span_count > 0 {
        let fat = std::ptr::slice_from_raw_parts_mut(
            output.spans as *mut Span,
            output.span_count as usize,
        );
        drop(Box::from_raw(fat));
    }
    if output.extra_data_len > 0 {
        let fat = std::ptr::slice_from_raw_parts_mut(
            output.extra_data as *mut u8,
            output.extra_data_len as usize,
        );
        drop(Box::from_raw(fat));
    }
}

/// Return the SDK version string
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
