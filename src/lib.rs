// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

pub mod ffi;
pub mod highlight;
pub mod html;
pub mod math;
pub mod options;
pub mod plain;
pub mod span;
pub mod spans_renderer;
pub mod stream;
pub mod theme;

pub use highlight::{
    HighlightToken, TOKEN_BUILTIN, TOKEN_COMMENT, TOKEN_CONSTANT, TOKEN_ENTITY, TOKEN_FUNCTION,
    TOKEN_KEYWORD, TOKEN_MARKUP, TOKEN_NUMBER, TOKEN_OPERATOR, TOKEN_OTHER, TOKEN_PUNCTUATION,
    TOKEN_REGEX, TOKEN_STRING, TOKEN_TYPE, TOKEN_TYPE_COUNT, TOKEN_VARIABLE,
};
pub use options::{HtmlMode, MarkdownOptions, MathMode};
pub use span::{
    Span, SpanDelta, SpanDeltaKind, SpanOutput, BLOCK_BLOCKQUOTE, BLOCK_CODE, BLOCK_HEADING_H1,
    BLOCK_HEADING_H2, BLOCK_HEADING_H3, BLOCK_HEADING_H4, BLOCK_HEADING_H5, BLOCK_HEADING_H6,
    BLOCK_HORIZONTAL_RULE, BLOCK_IMAGE, BLOCK_LIST_ITEM_ORDERED, BLOCK_LIST_ITEM_UNORDERED,
    BLOCK_MATH_BLOCK, BLOCK_PARAGRAPH, BLOCK_TABLE_CELL, BLOCK_TABLE_HEADER_CELL,
    EXTRA_KIND_CODE_HIGHLIGHT, EXTRA_KIND_CODE_LANGUAGE, EXTRA_KIND_IMAGE_URL, EXTRA_KIND_LINK_URL,
    EXTRA_KIND_LIST_META, EXTRA_KIND_TABLE_METADATA, FLAG_BOLD, FLAG_IMAGE, FLAG_INLINE_CODE,
    FLAG_ITALIC, FLAG_LINK, FLAG_MATH, FLAG_STRIKETHROUGH, FLAG_SUBSCRIPT, FLAG_SUPERSCRIPT,
};
pub use stream::StreamRenderer;
pub use theme::ThemeConfig;

// Re-export OwnedSpanOutput for safe Rust consumers
pub use spans_renderer::OwnedSpanOutput;

/// Strip HTML tags from a string, keeping text content only.
/// Shared across spans_renderer, html, and plain text paths.
pub(crate) fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}

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

/// Safe version of to_spans: returns owned buffers, no unsafe free required.
pub fn to_spans_owned(md: &str) -> OwnedSpanOutput {
    spans_renderer::markdown_to_spans_owned(md, &MarkdownOptions::default())
}

/// Safe version of to_spans_with_options: returns owned buffers.
pub fn to_spans_owned_with_options(md: &str, options: &MarkdownOptions) -> OwnedSpanOutput {
    spans_renderer::markdown_to_spans_owned(md, options)
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
        let fat =
            std::ptr::slice_from_raw_parts_mut(output.text as *mut u8, output.text_len as usize);
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
    if output.utf16_offsets_len > 0 {
        let fat = std::ptr::slice_from_raw_parts_mut(
            output.utf16_offsets as *mut u32,
            output.utf16_offsets_len as usize,
        );
        drop(Box::from_raw(fat));
    }
}

/// Highlight code with syntect, returning serialized binary token data.
/// Format: [count: u16 LE][(token_type: u8, len: u16 LE) * N]
/// Returns `None` if the language is not recognized or highlighting is unavailable.
#[cfg(feature = "highlight")]
pub fn highlight_code_serialized(code: &str, language: &str) -> Option<Vec<u8>> {
    let tokens = highlight::highlight_code(code, language)?;
    let count = tokens.len() as u16;
    let mut data: Vec<u8> = Vec::with_capacity(2 + count as usize * 3);
    data.extend_from_slice(&count.to_le_bytes());
    for t in &tokens {
        data.push(t.token_type);
        data.extend_from_slice(&t.len.to_le_bytes());
    }
    Some(data)
}

#[cfg(not(feature = "highlight"))]
pub fn highlight_code_serialized(_code: &str, _language: &str) -> Option<Vec<u8>> {
    None
}

/// Return the SDK version string
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
