// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::options::{HtmlMode, MarkdownOptions, MathMode};
use crate::span::*;
use crate::theme::ThemeConfig;
use crate::stream::StreamRenderer;

// ============================================================
//  Memory management helpers
// ============================================================

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

unsafe fn free_c_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

fn ptr_to_str(ptr: *const c_char) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    // Use to_string_lossy via a small helper: we need to return a &str but
    // CStr::to_str can fail for non-UTF-8. We use to_str().unwrap_or() to
    // return "" on invalid UTF-8, which is acceptable for empty/default behavior.
    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => s,
        Err(_) => "",
    }
}

fn html_mode_from_u32(mode: u32) -> HtmlMode {
    match mode {
        0 => HtmlMode::Strip,
        2 => HtmlMode::AllowAll,
        _ => HtmlMode::Safe,
    }
}

fn math_mode_from_u32(mode: u32) -> MathMode {
    match mode {
        1 => MathMode::Detect,
        _ => MathMode::Literal,
    }
}

// ============================================================
//  SpanOwned → raw thin pointers (for FFI transfer)
// ============================================================

/// Extract thin pointers from OwnedSpanOutput, transferring ownership.
/// The returned (text, text_len, spans, span_count, extra, extra_len) must be
/// freed via `free_span_raw_parts`.
fn owned_to_raw_parts(
    owned: crate::spans_renderer::OwnedSpanOutput,
) -> (*const u8, u32, *const Span, u32, *const u8, u32) {
    let text_len = owned.text.len() as u32;
    let span_count = owned.spans.len() as u32;
    let extra_len = owned.extra_data.len() as u32;

    // Convert Box<[T]> to fat raw pointer, then extract data pointer
    let text_fat: *mut [u8] = Box::into_raw(owned.text);
    let text_ptr = text_fat as *const u8;

    let spans_fat: *mut [Span] = Box::into_raw(owned.spans);
    let spans_ptr = spans_fat as *const Span;

    let extra_fat: *mut [u8] = Box::into_raw(owned.extra_data);
    let extra_ptr = extra_fat as *const u8;

    (text_ptr, text_len, spans_ptr, span_count, extra_ptr, extra_len)
}

/// Free raw span parts allocated by `owned_to_raw_parts`.
unsafe fn free_span_raw_parts(
    text_ptr: *mut u8,
    text_len: u32,
    spans_ptr: *mut Span,
    span_count: u32,
    extra_ptr: *mut u8,
    extra_len: u32,
) {
    if !text_ptr.is_null() && text_len > 0 {
        let fat = std::ptr::slice_from_raw_parts_mut(text_ptr, text_len as usize);
        drop(Box::from_raw(fat));
    }
    if !spans_ptr.is_null() && span_count > 0 {
        let fat = std::ptr::slice_from_raw_parts_mut(spans_ptr, span_count as usize);
        drop(Box::from_raw(fat));
    }
    if !extra_ptr.is_null() && extra_len > 0 {
        let fat = std::ptr::slice_from_raw_parts_mut(extra_ptr, extra_len as usize);
        drop(Box::from_raw(fat));
    }
}

// ============================================================
//  FFI type definitions (C-compatible, must match Span layout)
// ============================================================

/// C-compatible Span (layout-identical to crate::span::Span)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FastMarkdownSpan {
    pub offset: u32,
    pub length: u32,
    pub flags: u32,
    pub extra_len: u32,
    pub block_type: u8,
    pub _padding: [u8; 3],
}

/// C-compatible SpanOutput (ownership of pointers transfers to caller)
/// NOTE: Not Copy/Clone — prevents accidental double-free of owned pointers.
#[repr(C)]
#[derive(Debug)]
pub struct FastMarkdownSpanOutput {
    pub text: *const u8,
    pub text_len: u32,
    pub spans: *const FastMarkdownSpan,
    pub span_count: u32,
    pub extra_data: *const u8,
    pub extra_data_len: u32,
}

// Compile-time layout check
const _LAYOUT_CHECK: () = {
    assert!(std::mem::size_of::<Span>() == std::mem::size_of::<FastMarkdownSpan>());
    assert!(std::mem::align_of::<Span>() == std::mem::align_of::<FastMarkdownSpan>());
};

fn empty_output() -> FastMarkdownSpanOutput {
    FastMarkdownSpanOutput {
        text: std::ptr::null(),
        text_len: 0,
        spans: std::ptr::null(),
        span_count: 0,
        extra_data: std::ptr::null(),
        extra_data_len: 0,
    }
}

// ============================================================
//  Public FFI exported functions
// ============================================================

// --- HTML output ---

#[no_mangle]
pub extern "C" fn fastmarkdown_to_html(markdown: *const c_char) -> *mut c_char {
    let md = ptr_to_str(markdown);
    let options = MarkdownOptions::default();
    let html = crate::html::markdown_to_html(md, &options);
    to_c_string(&html)
}

#[no_mangle]
pub extern "C" fn fastmarkdown_to_html_ex(
    markdown: *const c_char,
    html_mode: u32,
    math_mode: u32,
    smart_punct: bool,
) -> *mut c_char {
    let md = ptr_to_str(markdown);
    let options = MarkdownOptions {
        html_mode: html_mode_from_u32(html_mode),
        math_mode: math_mode_from_u32(math_mode),
        smart_punctuation: smart_punct,
        ..Default::default()
    };
    let html = crate::html::markdown_to_html(md, &options);
    to_c_string(&html)
}

#[no_mangle]
pub extern "C" fn fastmarkdown_free_string(s: *mut c_char) {
    unsafe { free_c_string(s); }
}

// --- Span output ---

#[no_mangle]
pub extern "C" fn fastmarkdown_to_spans(markdown: *const c_char) -> FastMarkdownSpanOutput {
    let md = ptr_to_str(markdown);
    let options = MarkdownOptions::default();
    let owned = crate::spans_renderer::markdown_to_spans_owned(md, &options);
    let (text, text_len, spans, span_count, extra, extra_len) = owned_to_raw_parts(owned);
    FastMarkdownSpanOutput {
        text,
        text_len,
        spans: spans as *const FastMarkdownSpan,
        span_count,
        extra_data: extra,
        extra_data_len: extra_len,
    }
}

#[no_mangle]
pub extern "C" fn fastmarkdown_to_spans_ex(
    markdown: *const c_char,
    html_mode: u32,
    math_mode: u32,
    smart_punct: bool,
) -> FastMarkdownSpanOutput {
    let md = ptr_to_str(markdown);
    let options = MarkdownOptions {
        html_mode: html_mode_from_u32(html_mode),
        math_mode: math_mode_from_u32(math_mode),
        smart_punctuation: smart_punct,
        ..Default::default()
    };
    let owned = crate::spans_renderer::markdown_to_spans_owned(md, &options);
    let (text, text_len, spans, span_count, extra, extra_len) = owned_to_raw_parts(owned);
    FastMarkdownSpanOutput {
        text,
        text_len,
        spans: spans as *const FastMarkdownSpan,
        span_count,
        extra_data: extra,
        extra_data_len: extra_len,
    }
}

#[no_mangle]
pub extern "C" fn fastmarkdown_to_plain_text(markdown: *const c_char) -> *mut c_char {
    let md = ptr_to_str(markdown);
    let options = MarkdownOptions::default();
    let text = crate::plain::markdown_to_plain_text(md, &options);
    to_c_string(&text)
}

#[no_mangle]
pub extern "C" fn fastmarkdown_free_spans(output: *mut FastMarkdownSpanOutput) {
    if output.is_null() {
        return;
    }
    let o = unsafe { &*output };
    unsafe {
        free_span_raw_parts(
            o.text as *mut u8,
            o.text_len,
            o.spans as *mut Span,
            o.span_count,
            o.extra_data as *mut u8,
            o.extra_data_len,
        );
    }
}

#[no_mangle]
pub extern "C" fn fastmarkdown_version() -> *const c_char {
    // Return a pointer to a static string (no need to free)
    static VERSION: &str = concat!("fastmarkdown ", env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

// --- Streaming ---

#[no_mangle]
pub extern "C" fn fastmarkdown_stream_create(
    html_mode: u32,
    math_mode: u32,
    smart_punct: bool,
) -> *mut std::ffi::c_void {
    let options = MarkdownOptions {
        html_mode: html_mode_from_u32(html_mode),
        math_mode: math_mode_from_u32(math_mode),
        smart_punctuation: smart_punct,
        ..Default::default()
    };
    let stream = Box::new(StreamRenderer::new(options));
    Box::into_raw(stream) as *mut std::ffi::c_void
}

/// Append a chunk to the stream.
/// Returns a SpanOutput whose pointers are valid until the next
/// `fastmarkdown_stream_append`, `fastmarkdown_stream_finish`, or
/// `fastmarkdown_stream_destroy` call.
/// The caller must NOT free this SpanOutput individually.
#[no_mangle]
pub extern "C" fn fastmarkdown_stream_append(
    stream_ptr: *mut std::ffi::c_void,
    chunk: *const c_char,
) -> FastMarkdownSpanOutput {
    if stream_ptr.is_null() {
        return empty_output();
    }
    let stream = unsafe { &mut *(stream_ptr as *mut StreamRenderer) };
    let text = ptr_to_str(chunk);
    let output = stream.append(text);

    FastMarkdownSpanOutput {
        text: output.text,
        text_len: output.text_len,
        spans: output.spans as *const FastMarkdownSpan,
        span_count: output.span_count,
        extra_data: output.extra_data,
        extra_data_len: output.extra_data_len,
    }
}

/// Finalize the stream, returning the last SpanOutput (or empty if no content).
#[no_mangle]
pub extern "C" fn fastmarkdown_stream_finish(
    stream_ptr: *mut std::ffi::c_void,
) -> FastMarkdownSpanOutput {
    if stream_ptr.is_null() {
        return empty_output();
    }
    let stream = unsafe { &mut *(stream_ptr as *mut StreamRenderer) };
    match stream.finish() {
        Some(output) => FastMarkdownSpanOutput {
            text: output.text,
            text_len: output.text_len,
            spans: output.spans as *const FastMarkdownSpan,
            span_count: output.span_count,
            extra_data: output.extra_data,
            extra_data_len: output.extra_data_len,
        },
        None => empty_output(),
    }
}

/// Destroy the stream and free all internals.
/// Any SpanOutput pointers from this stream are invalidated.
#[no_mangle]
pub extern "C" fn fastmarkdown_stream_destroy(stream_ptr: *mut std::ffi::c_void) {
    if !stream_ptr.is_null() {
        unsafe { drop(Box::from_raw(stream_ptr as *mut StreamRenderer)); }
    }
}

/// Reset the stream for reuse with a new markdown document.
#[no_mangle]
pub extern "C" fn fastmarkdown_stream_reset(stream_ptr: *mut std::ffi::c_void) {
    if !stream_ptr.is_null() {
        let stream = unsafe { &mut *(stream_ptr as *mut StreamRenderer) };
        stream.reset();
    }
}

// ============================================================
//  ThemeConfig FFI
// ============================================================

/// C-compatible ThemeConfig (identical layout to crate::theme::ThemeConfig)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FastMarkdownTheme {
    pub body_font_name: *const std::os::raw::c_char,
    pub body_font_size: f32,
    pub monospace_font_name: *const std::os::raw::c_char,
    pub monospace_font_size: f32,
    pub heading_font_sizes: [f32; 6],
    pub text_color: u32,
    pub link_color: u32,
    pub code_bg_color: u32,
    pub code_text_color: u32,
    pub blockquote_bar_color: u32,
    pub paragraph_spacing: f32,
    pub line_spacing: f32,
    pub list_indent: f32,
    pub blockquote_indent: f32,
}

/// Create a default ThemeConfig. Returns a stack-allocated struct (no free needed).
#[no_mangle]
pub extern "C" fn fastmarkdown_theme_default() -> FastMarkdownTheme {
    let t = ThemeConfig::default();
    FastMarkdownTheme {
        body_font_name: t.body_font_name,
        body_font_size: t.body_font_size,
        monospace_font_name: t.monospace_font_name,
        monospace_font_size: t.monospace_font_size,
        heading_font_sizes: t.heading_font_sizes,
        text_color: t.text_color,
        link_color: t.link_color,
        code_bg_color: t.code_bg_color,
        code_text_color: t.code_text_color,
        blockquote_bar_color: t.blockquote_bar_color,
        paragraph_spacing: t.paragraph_spacing,
        line_spacing: t.line_spacing,
        list_indent: t.list_indent,
        blockquote_indent: t.blockquote_indent,
    }
}

// ============================================================
//  Stream Delta FFI
// ============================================================

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum FastMarkdownDeltaKind {
    Append = 0,
    ReplaceLast = 1,
}

#[repr(C)]
#[derive(Debug)]
pub struct FastMarkdownSpanDelta {
    pub kind: FastMarkdownDeltaKind,
    pub replaced_count: u32,
    pub spans: *const FastMarkdownSpan,
    pub span_count: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct FastMarkdownSpanDeltaList {
    pub deltas: *const FastMarkdownSpanDelta,
    pub delta_count: u32,
    pub locked_text_len: u32,
}

/// Compute incremental delta since last state.
/// The returned delta list must be freed via `fastmarkdown_free_deltas`.
#[no_mangle]
pub extern "C" fn fastmarkdown_stream_get_delta(
    stream_ptr: *mut std::ffi::c_void,
    chunk: *const c_char,
) -> FastMarkdownSpanDeltaList {
    if stream_ptr.is_null() {
        return empty_delta_list();
    }
    let stream = unsafe { &mut *(stream_ptr as *mut StreamRenderer) };
    let text = ptr_to_str(chunk);
    let delta = stream.delta(text);

    if delta.deltas.is_empty() {
        return FastMarkdownSpanDeltaList {
            deltas: std::ptr::null(),
            delta_count: 0,
            locked_text_len: delta.locked_text_len,
        };
    }

    let count = delta.deltas.len();
    let mut ff_deltas: Vec<FastMarkdownSpanDelta> = Vec::with_capacity(count);
    for d in &delta.deltas {
        let kind = match d.kind {
            SpanDeltaKind::Append => FastMarkdownDeltaKind::Append,
            SpanDeltaKind::ReplaceLast { .. } => FastMarkdownDeltaKind::ReplaceLast,
        };
        let replaced_count = match &d.kind {
            SpanDeltaKind::ReplaceLast { count } => *count,
            _ => 0,
        };
        let span_box: Box<[Span]> = d.spans.clone().into_boxed_slice();
        let span_count = span_box.len() as u32;
        let spans_ptr = Box::into_raw(span_box) as *const FastMarkdownSpan;

        ff_deltas.push(FastMarkdownSpanDelta {
            kind,
            replaced_count,
            spans: spans_ptr,
            span_count,
        });
    }

    let deltas_box = ff_deltas.into_boxed_slice();
    let delta_count = deltas_box.len() as u32;
    let deltas_ptr = Box::into_raw(deltas_box) as *const FastMarkdownSpanDelta;

    FastMarkdownSpanDeltaList {
        deltas: deltas_ptr,
        delta_count,
        locked_text_len: delta.locked_text_len,
    }
}

/// Free a delta list allocated by fastmarkdown_stream_get_delta.
#[no_mangle]
pub extern "C" fn fastmarkdown_free_deltas(list: *mut FastMarkdownSpanDeltaList) {
    if list.is_null() {
        return;
    }
    let l = unsafe { &*list };
    if !l.deltas.is_null() && l.delta_count > 0 {
        let deltas = unsafe {
            std::slice::from_raw_parts(l.deltas, l.delta_count as usize)
        };
        for d in deltas {
            if !d.spans.is_null() && d.span_count > 0 {
                unsafe {
                    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        d.spans as *mut Span,
                        d.span_count as usize,
                    )));
                }
            }
        }
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                l.deltas as *mut FastMarkdownSpanDelta,
                l.delta_count as usize,
            )));
        }
    }
}

fn empty_delta_list() -> FastMarkdownSpanDeltaList {
    FastMarkdownSpanDeltaList {
        deltas: std::ptr::null(),
        delta_count: 0,
        locked_text_len: 0,
    }
}
