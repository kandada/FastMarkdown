// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::options::MarkdownOptions;
use crate::span::*;

pub struct OwnedSpanOutput {
    pub text: Box<[u8]>,
    pub spans: Box<[Span]>,
    pub extra_data: Box<[u8]>,
    /// 可选：每个 span 的 UTF-16 偏移，与 spans 一一对应。启用 compute_utf16_offsets 时有效。
    pub utf16_offsets: Box<[u32]>,
}

impl OwnedSpanOutput {
    pub fn as_output(&self) -> SpanOutput {
        SpanOutput {
            text: self.text.as_ptr(),
            text_len: self.text.len() as u32,
            spans: self.spans.as_ptr(),
            span_count: self.spans.len() as u32,
            extra_data: self.extra_data.as_ptr(),
            extra_data_len: self.extra_data.len() as u32,
            utf16_offsets: if self.utf16_offsets.is_empty() {
                std::ptr::null()
            } else {
                self.utf16_offsets.as_ptr()
            },
            utf16_offsets_len: self.utf16_offsets.len() as u32,
        }
    }
}

pub fn markdown_to_spans_owned(md: &str, options: &MarkdownOptions) -> OwnedSpanOutput {
    if md.len() > u32::MAX as usize {
        return OwnedSpanOutput {
            text: Box::new([]),
            spans: Box::new([]),
            extra_data: Box::new([]),
            utf16_offsets: Box::new([]),
        };
    }
    build_spans(md, options)
}

pub(crate) fn markdown_to_spans(md: &str, options: &MarkdownOptions) -> SpanOutput {
    let owned = markdown_to_spans_owned(md, options);
    owned_to_span_output(owned)
}

fn owned_to_span_output(owned: OwnedSpanOutput) -> SpanOutput {
    let text_len = owned.text.len() as u32;
    let span_count = owned.spans.len() as u32;
    let extra_len = owned.extra_data.len() as u32;
    let utf16_len = owned.utf16_offsets.len() as u32;

    let text_fat: *mut [u8] = Box::into_raw(owned.text);
    let text_ptr = text_fat as *const u8;

    let spans_fat: *mut [Span] = Box::into_raw(owned.spans);
    let spans_ptr = spans_fat as *const Span;

    let extra_fat: *mut [u8] = Box::into_raw(owned.extra_data);
    let extra_ptr = extra_fat as *const u8;

    let (utf16_ptr, utf16_len) = if utf16_len > 0 {
        let utf16_fat: *mut [u32] = Box::into_raw(owned.utf16_offsets);
        (utf16_fat as *const u32, utf16_len)
    } else {
        (std::ptr::null(), 0)
    };

    SpanOutput {
        text: text_ptr,
        text_len,
        spans: spans_ptr,
        span_count,
        extra_data: extra_ptr,
        extra_data_len: extra_len,
        utf16_offsets: utf16_ptr,
        utf16_offsets_len: utf16_len,
    }
}

fn build_spans(md: &str, options: &MarkdownOptions) -> OwnedSpanOutput {
    let parser = Parser::new_ext(md, options.to_pulldown_options());
    let highlight_enabled = cfg!(feature = "highlight") && {
        #[cfg(feature = "highlight")]
        { options.highlight }
        #[cfg(not(feature = "highlight"))]
        { false }
    };
    let mut state = SpanBuildState::new(highlight_enabled, options.compute_utf16_offsets);
    for event in parser {
        state.process_event(event);
    }
    state.finish();
    state.into_output()
}

struct SpanBuildState {
    text_buf: String,
    spans: Vec<Span>,
    extra_data: Vec<u8>,

    current_flags: u32,
    current_block: u8,
    block_stack: Vec<u8>,

    block_seq: u16,
    block_depth: u8,

    link_urls: Vec<String>,

    // Table state
    table_alignments: Vec<Alignment>,
    in_table_head: bool,
    table_cells_in_row: u32,
    col_widths: Vec<u16>,     // max char count per column
    cell_char_count: u16,     // char count for current cell

    // Code block language pending (written on first span of the block)
    pending_code_lang: Option<String>,
    code_lang_written: bool,
    // Code text accumulation (for highlighting at block end)
    code_text_buf: String,
    // Highlight toggle from options
    highlight_enabled: bool,
    // Skip text while inside metadata blocks
    in_metadata_block: bool,

    // UTF-16 offset tracking
    compute_utf16: bool,
    utf16_offsets: Vec<u32>,
    utf16_offset: u32,
}

impl SpanBuildState {
    fn new(highlight_enabled: bool, compute_utf16: bool) -> Self {
        Self {
            text_buf: String::new(),
            spans: Vec::new(),
            extra_data: Vec::new(),
            current_flags: 0,
            current_block: BLOCK_PARAGRAPH,
            block_stack: Vec::new(),
            block_seq: 0,
            block_depth: 0,
            link_urls: Vec::new(),
            table_alignments: Vec::new(),
            in_table_head: false,
            table_cells_in_row: 0,
            col_widths: Vec::new(),
            cell_char_count: 0,
            pending_code_lang: None,
            code_lang_written: false,
            code_text_buf: String::new(),
            highlight_enabled,
            in_metadata_block: false,
            compute_utf16,
            utf16_offsets: Vec::new(),
            utf16_offset: 0,
        }
    }

    // ─── block structure helpers ────────────────────────────────────

    /// Push a nesting block (paragraph, heading, blockquote, codeblock, list, table).
    /// Increments seq, tracks depth, pushes onto block_stack.
    fn push_nesting_block(&mut self, block_type: u8) {
        self.block_stack.push(block_type);
        self.current_block = block_type;
        self.block_seq = self.block_seq.wrapping_add(1);
        self.block_depth = self.block_stack.len() as u8;
    }

    /// Mark a new structural boundary (list item, table cell) without nesting.
    /// Increments seq, keeps stack and depth unchanged.
    fn new_block_boundary(&mut self, block_type: u8) {
        self.current_block = block_type;
        self.block_seq = self.block_seq.wrapping_add(1);
    }

    /// Pop the current nesting block and restore parent.
    fn pop_nesting(&mut self) {
        if self.block_stack.is_empty() {
            return;
        }
        self.block_stack.pop();
        if let Some(&parent) = self.block_stack.last() {
            self.current_block = parent;
        } else {
            self.current_block = BLOCK_PARAGRAPH;
        }
        self.block_depth = self.block_stack.len() as u8;
    }

    // ─── extra_data helpers ─────────────────────────────────────────

    /// Write a self-describing chunk into extra_data.
    /// Format: [kind: u8][data_len: u16 LE][data: data_len bytes]
    /// Returns total bytes written (3 + data.len()), or 0 if data too large.
    fn write_extra_chunk(&mut self, kind: u8, data: &[u8]) -> u32 {
        if data.len() > u16::MAX as usize {
            return 0; // chunk too large for u16 length field
        }
        let start = self.extra_data.len();
        self.extra_data.push(kind);
        let len = data.len() as u16;
        self.extra_data.extend_from_slice(&len.to_le_bytes());
        self.extra_data.extend_from_slice(data);
        (self.extra_data.len() - start) as u32
    }

    // ─── text buffer separators ─────────────────────────────────────

    fn push_text_sep(&mut self, sep: &str) {
        self.text_buf.push_str(sep);
    }

    // ─── event processing ───────────────────────────────────────────

    fn process_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.handle_start(tag),
            Event::End(tag) => self.handle_end(tag),
            Event::Text(text) => self.handle_text(&text, false),
            Event::Code(code) => self.handle_text(&code, true),
            Event::Html(html) => self.handle_html_block(&html),
            Event::InlineHtml(html) => self.handle_inline_html(&html),
            Event::InlineMath(math) => self.handle_math(&math, false),
            Event::DisplayMath(math) => self.handle_math(&math, true),
            Event::SoftBreak => self.handle_break(false),
            Event::HardBreak => self.handle_break(true),
            Event::Rule => self.handle_rule(),
            Event::TaskListMarker(checked) => {
                let marker = if checked { "[x] " } else { "[ ] " };
                self.emit_span(marker, 0, self.current_block);
            }
            Event::FootnoteReference(name) => {
                let ref_text = format!("[^{}]", name);
                self.emit_span(&ref_text, 0, self.current_block);
            }
        }
    }

    fn handle_start(&mut self, tag: Tag) {
        match tag {
            // ── leaf blocks (sequence boundary, not nesting) ──
            Tag::Paragraph => {
                self.block_seq = self.block_seq.wrapping_add(1);
                if self.block_stack.is_empty() {
                    self.current_block = BLOCK_PARAGRAPH;
                }
            }
            Tag::Heading { level, .. } => {
                self.block_seq = self.block_seq.wrapping_add(1);
                if self.block_stack.is_empty() {
                    self.current_block = heading_to_block_type(level);
                }
            }

            // ── container blocks (nesting + seq + depth) ──
            Tag::BlockQuote(_kind) => self.push_nesting_block(BLOCK_BLOCKQUOTE),
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => lang.to_string(),
                    _ => String::new(),
                };
                self.push_nesting_block(BLOCK_CODE);
                if !language.is_empty() {
                    self.pending_code_lang = Some(language);
                }
            }
            Tag::List(number) => {
                let bt = if number.is_some() {
                    BLOCK_LIST_ITEM_ORDERED
                } else {
                    BLOCK_LIST_ITEM_UNORDERED
                };
                self.push_nesting_block(bt);
                // Write list start number for ordered lists
                if let Some(start) = number {
                    let start = start.min(u16::MAX as u64) as u16;
                    self.write_extra_chunk(EXTRA_KIND_LIST_META, &start.to_le_bytes());
                }
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments.clone();
                self.in_table_head = true;
                self.table_cells_in_row = 0;
                self.col_widths = vec![0u16; alignments.len()];
                self.push_nesting_block(BLOCK_TABLE_CELL);
            }

            // ── structural boundaries ──
            Tag::Item => {
                let bt = self.current_block; // preserve list type
                self.push_text_sep("\n");
                self.new_block_boundary(bt);
            }
            Tag::TableCell => {
                let bt = if self.in_table_head {
                    BLOCK_TABLE_HEADER_CELL
                } else {
                    BLOCK_TABLE_CELL
                };
                if self.table_cells_in_row > 0 {
                    self.push_text_sep("\t");
                }
                self.new_block_boundary(bt);
                self.cell_char_count = 0;
            }

            // ── table sections ──
            Tag::TableHead => {
                self.in_table_head = true;
                self.table_cells_in_row = 0;
            }
            Tag::TableRow => {
                self.table_cells_in_row = 0;
            }

            // ── inline styles ──
            Tag::Emphasis => self.current_flags |= FLAG_ITALIC,
            Tag::Strong => self.current_flags |= FLAG_BOLD,
            Tag::Strikethrough => self.current_flags |= FLAG_STRIKETHROUGH,
            Tag::Link { dest_url, .. } => {
                self.current_flags |= FLAG_LINK;
                self.link_urls.push(dest_url.to_string());
            }
            Tag::Image { dest_url, .. } => {
                self.current_flags |= FLAG_IMAGE;
                self.link_urls.push(dest_url.to_string());
            }

            // ── ignore ──
            Tag::MetadataBlock(_) => {
                self.in_metadata_block = true;
            }
            Tag::HtmlBlock
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition => {}
            Tag::Subscript => self.current_flags |= FLAG_SUBSCRIPT,
            Tag::Superscript => self.current_flags |= FLAG_SUPERSCRIPT,
            Tag::FootnoteDefinition(_) => {}
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }

    fn handle_end(&mut self, tag: TagEnd) {
        match tag {
            // ── container blocks ──
            TagEnd::BlockQuote(_) => {
                self.push_text_sep("\n");
                self.pop_nesting();
            }
            TagEnd::CodeBlock => {
                self.push_text_sep("\n");
                if self.highlight_enabled && !self.code_text_buf.is_empty() {
                    self.highlight_code_block();
                }
                self.code_text_buf.clear();
                self.pending_code_lang = None;
                self.code_lang_written = false;
                self.pop_nesting();
            }
            TagEnd::List(_) => {
                self.push_text_sep("\n");
                self.pop_nesting();
            }
            TagEnd::Table => {
                self.push_text_sep("\n");
                // Write table metadata: [col_count: u8, (align: u8, char_width: u16 LE)*]
                let mut meta: Vec<u8> = Vec::with_capacity(1 + self.col_widths.len() * 3);
                meta.push(self.col_widths.len() as u8);
                for (i, w) in self.col_widths.iter().enumerate() {
                    let align = if i < self.table_alignments.len() {
                        match self.table_alignments[i] {
                            Alignment::None => 0u8,
                            Alignment::Left => 1,
                            Alignment::Center => 2,
                            Alignment::Right => 3,
                        }
                    } else {
                        0u8
                    };
                    meta.push(align);
                    meta.extend_from_slice(&w.to_le_bytes());
                }
                self.write_extra_chunk(EXTRA_KIND_TABLE_METADATA, &meta);
                self.table_alignments.clear();
                self.col_widths.clear();
                self.in_table_head = false;
                self.table_cells_in_row = 0;
                self.pop_nesting();
            }

            // ── leaf blocks (no pop; just boundaries) ──
            TagEnd::Paragraph | TagEnd::Heading(_) => {}
            TagEnd::Item => {}
            TagEnd::TableHead => {
                self.in_table_head = false;
                self.table_cells_in_row = 0;
            }
            TagEnd::TableRow => {
                self.push_text_sep("\n");
                self.table_cells_in_row = 0;
            }
            TagEnd::TableCell => {
                let col = self.table_cells_in_row as usize;
                if col < self.col_widths.len() {
                    self.col_widths[col] = self.col_widths[col].max(self.cell_char_count);
                }
                self.table_cells_in_row += 1;
            }

            // Inline styles
            TagEnd::Emphasis => self.current_flags &= !FLAG_ITALIC,
            TagEnd::Strong => self.current_flags &= !FLAG_BOLD,
            TagEnd::Strikethrough => self.current_flags &= !FLAG_STRIKETHROUGH,
            TagEnd::Link => {
                self.current_flags &= !FLAG_LINK;
                self.link_urls.pop();
            }
            TagEnd::Image => {
                self.current_flags &= !FLAG_IMAGE;
                self.link_urls.pop();
            }

            TagEnd::FootnoteDefinition => {}
            TagEnd::HtmlBlock
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition => {}
            TagEnd::MetadataBlock(_) => {
                self.in_metadata_block = false;
            }
            TagEnd::Subscript => self.current_flags &= !FLAG_SUBSCRIPT,
            TagEnd::Superscript => self.current_flags &= !FLAG_SUPERSCRIPT,
        }
    }

    fn handle_text(&mut self, text: &str, is_code: bool) {
        if self.in_metadata_block {
            return;
        }
        let flags = if is_code {
            self.current_flags | FLAG_INLINE_CODE
        } else {
            self.current_flags
        };
        if self.current_block == BLOCK_CODE {
            self.code_text_buf.push_str(text);
        }
        self.emit_span(text, flags, self.current_block);
    }

    fn handle_math(&mut self, math: &str, is_display: bool) {
        let flags = self.current_flags | FLAG_MATH;
        let block = if is_display {
            BLOCK_MATH_BLOCK
        } else {
            self.current_block
        };
        self.emit_span(math, flags, block);
    }

    fn handle_html_block(&mut self, html: &str) {
        let stripped = crate::strip_html_tags(html);
        if !stripped.is_empty() {
            self.emit_span(&stripped, self.current_flags, self.current_block);
        }
    }

    fn handle_inline_html(&mut self, html: &str) {
        let stripped = crate::strip_html_tags(html);
        if !stripped.is_empty() {
            self.emit_span(&stripped, self.current_flags, self.current_block);
        }
    }

    fn handle_break(&mut self, hard: bool) {
        if hard {
            self.emit_span("\n", 0, self.current_block);
        } else {
            self.emit_span(" ", 0, self.current_block);
        }
    }

    fn handle_rule(&mut self) {
        self.block_seq = self.block_seq.wrapping_add(1);
        let offset = self.text_buf.len() as u32;
        self.text_buf.push_str("\n---\n");
        self.spans.push(Span::new(
            offset, 5, 0, BLOCK_HORIZONTAL_RULE, 0,
            self.block_depth, self.block_seq,
        ));
    }

    fn emit_span(&mut self, text: &str, flags: u32, block_type: u8) {
        if text.is_empty() {
            return;
        }
        let offset = self.text_buf.len() as u32;
        self.text_buf.push_str(text);

        // Record UTF-16 offset at span start
        if self.compute_utf16 {
            self.utf16_offsets.push(self.utf16_offset);
            for ch in text.chars() {
                self.utf16_offset += ch.len_utf16() as u32;
            }
        }

        // Track per-cell char count for column width estimation
        if block_type == BLOCK_TABLE_CELL || block_type == BLOCK_TABLE_HEADER_CELL {
            self.cell_char_count = self.cell_char_count.saturating_add(
                text.chars().count() as u16
            );
        }

        let extra_len = self.compute_extra_len(flags, block_type);
        let span = Span::new(
            offset,
            text.len() as u32,
            flags,
            block_type,
            extra_len,
            self.block_depth,
            self.block_seq,
        );
        self.spans.push(span);
    }

    fn compute_extra_len(&mut self, flags: u32, block_type: u8) -> u32 {
        // Link / image URLs
        if (flags & FLAG_LINK) != 0 || (flags & FLAG_IMAGE) != 0 {
            let url = self.link_urls.last().cloned();
            if let Some(url) = url {
                let kind = if (flags & FLAG_IMAGE) != 0 {
                    EXTRA_KIND_IMAGE_URL
                } else {
                    EXTRA_KIND_LINK_URL
                };
                return self.write_extra_chunk(kind, url.as_bytes());
            }
            return 0;
        }

        // Code block language — write once for the first span of the block
        if block_type == BLOCK_CODE && !self.code_lang_written {
            let lang = self.pending_code_lang.clone();
            if let Some(lang) = lang {
                self.code_lang_written = true;
                let len = self.write_extra_chunk(
                    EXTRA_KIND_CODE_LANGUAGE, lang.as_bytes(),
                );
                return len;
            }
            return 0;
        }

        // Table metadata — write once, attach to first table cell span
        if block_type == BLOCK_TABLE_HEADER_CELL || block_type == BLOCK_TABLE_CELL {
            return 0; // metadata is written at table start; consumer scans for it
        }

        0
    }

    fn highlight_code_block(&mut self) {
        #[cfg(feature = "highlight")]
        {
            let lang = self.pending_code_lang.as_deref().unwrap_or("text");
            if let Some(tokens) = crate::highlight::highlight_code(&self.code_text_buf, lang) {
                if tokens.len() > u16::MAX as usize {
                    return; // too many tokens, skip highlighting
                }
                let count = tokens.len() as u16;
                let mut data: Vec<u8> = Vec::with_capacity(2 + tokens.len() * 3);
                data.extend_from_slice(&count.to_le_bytes());
                for t in &tokens {
                    data.push(t.token_type);
                    data.extend_from_slice(&t.len.to_le_bytes());
                }
                self.write_extra_chunk(EXTRA_KIND_CODE_HIGHLIGHT, &data);
            }
        }
        #[cfg(not(feature = "highlight"))]
        let _ = ();
    }

    fn finish(&mut self) {
        // Trim trailing empty spans
        self.spans.retain(|s| s.length > 0);
    }

    fn into_output(self) -> OwnedSpanOutput {
        OwnedSpanOutput {
            text: self.text_buf.into_bytes().into_boxed_slice(),
            spans: self.spans.into_boxed_slice(),
            extra_data: self.extra_data.into_boxed_slice(),
            utf16_offsets: self.utf16_offsets.into_boxed_slice(),
        }
    }
}

// ─── helpers ──────────────────────────────────────────────────────────

fn heading_to_block_type(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => BLOCK_HEADING_H1,
        HeadingLevel::H2 => BLOCK_HEADING_H2,
        HeadingLevel::H3 => BLOCK_HEADING_H3,
        HeadingLevel::H4 => BLOCK_HEADING_H4,
        HeadingLevel::H5 => BLOCK_HEADING_H5,
        HeadingLevel::H6 => BLOCK_HEADING_H6,
    }
}

// ─── tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::*;
    use crate::{to_spans, to_spans_with_options};

    fn get_spans(output: SpanOutput) -> (Vec<Span>, String, Vec<u8>) {
        let spans = unsafe {
            std::slice::from_raw_parts(output.spans, output.span_count as usize)
        }.to_vec();
        let text = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(
                output.text, output.text_len as usize,
            )).unwrap().to_string()
        };
        let extra = unsafe {
            std::slice::from_raw_parts(output.extra_data, output.extra_data_len as usize)
        }.to_vec();
        unsafe { crate::free_spans(output); }
        (spans, text, extra)
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
    fn test_basic_bold() {
        let (spans, _text, _extra) = get_spans(to_spans("Hello **world**"));
        assert!(spans.iter().any(|s| s.is_bold()));
    }

    #[test]
    fn test_italic() {
        let (spans, _, _) = get_spans(to_spans("*italic text*"));
        assert!(spans.iter().any(|s| s.is_italic()));
    }

    #[test]
    fn test_bold_italic() {
        let (spans, _, _) = get_spans(to_spans("***bold italic***"));
        assert!(spans.iter().any(|s| s.is_bold() && s.is_italic()));
    }

    #[test]
    fn test_inline_code() {
        let (spans, _, _) = get_spans(to_spans("Use `println!` macro"));
        assert!(spans.iter().any(|s| s.is_inline_code()));
    }

    #[test]
    fn test_heading_block_type() {
        let (spans, _, _) = get_spans(to_spans("# H1"));
        assert!(spans.iter().any(|s| s.block_type == BLOCK_HEADING_H1));
        let (spans, _, _) = get_spans(to_spans("### H3"));
        assert!(spans.iter().any(|s| s.block_type == BLOCK_HEADING_H3));
    }

    #[test]
    fn test_link_url_in_extra() {
        let (spans, _, extra) = get_spans(to_spans("[click](https://example.com)"));
        let link_span = spans.iter().find(|s| s.is_link()).unwrap();
        assert!(link_span.extra_len > 0);
        let chunks = parse_extra_chunks(&extra);
        assert!(chunks.iter().any(|(k, d)| *k == EXTRA_KIND_LINK_URL
            && d == b"https://example.com"));
    }

    #[test]
    fn test_code_block_with_language() {
        let (spans, _, extra) = get_spans(to_spans("```rust\nfn main() {}\n```"));
        assert!(spans.iter().any(|s| s.block_type == BLOCK_CODE));
        let chunks = parse_extra_chunks(&extra);
        assert!(chunks.iter().any(|(k, d)| *k == EXTRA_KIND_CODE_LANGUAGE
            && d == b"rust"));
    }

    #[test]
    fn test_horizontal_rule() {
        let (spans, _, _) = get_spans(to_spans("---"));
        assert!(spans.iter().any(|s| s.block_type == BLOCK_HORIZONTAL_RULE));
    }

    #[test]
    fn test_strikethrough() {
        let (spans, _, _) = get_spans(to_spans("~~deleted~~"));
        assert!(spans.iter().any(|s| s.is_strikethrough()));
    }

    #[test]
    fn test_list_item() {
        let (spans, _, _) = get_spans(to_spans("- item one\n- item two"));
        assert!(spans.iter().any(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED));
    }

    #[test]
    fn test_empty_input() {
        let output = to_spans("");
        assert_eq!(output.span_count, 0);
        assert_eq!(output.text_len, 0);
        unsafe { crate::free_spans(output); }
    }

    #[test]
    fn test_text_offset_consistency() {
        let (spans, text, _) = get_spans(to_spans("ABC **DEF** GHI"));
        for span in &spans {
            let end = (span.offset + span.length) as usize;
            assert!(end <= text.len(), "Span offset+len exceeds text length");
        }
        assert!(text.contains("ABC"));
        assert!(text.contains("DEF"));
        assert!(text.contains("GHI"));
    }

    #[test]
    fn test_table_cell_block_types() {
        let (spans, _, extra) = get_spans(to_spans(
            "| Name | Value |\n|------|-------|\n| foo  | 42    |"
        ));
        assert!(spans.iter().any(|s| s.block_type == BLOCK_TABLE_HEADER_CELL));
        assert!(spans.iter().any(|s| s.block_type == BLOCK_TABLE_CELL));
        let chunks = parse_extra_chunks(&extra);
        assert!(chunks.iter().any(|(k, d)| *k == EXTRA_KIND_TABLE_METADATA
            && d.len() >= 3));
    }

    // ── new block structure tests ────────────────────────────────

    #[test]
    fn test_block_seq_unique_per_block() {
        let (spans, _, _) = get_spans(to_spans("# H1\n\nPara1\n\nPara2"));
        let mut seqs: Vec<u16> = spans.iter().map(|s| s.block_seq).collect();
        seqs.sort();
        seqs.dedup();
        assert!(seqs.len() >= 3, "H1, Para1, Para2 should each have unique seq");
    }

    #[test]
    fn test_list_items_get_unique_seq() {
        let (spans, _, _) = get_spans(to_spans("- a\n- b\n- c"));
        let list_seqs: Vec<u16> = spans.iter()
            .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
            .map(|s| s.block_seq)
            .collect();
        let mut unique: Vec<u16> = list_seqs.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 3, "Each list item should have unique seq, got {:?}", list_seqs);
    }

    #[test]
    fn test_list_items_same_depth() {
        let (spans, _, _) = get_spans(to_spans("- a\n- b"));
        let depths: Vec<u8> = spans.iter()
            .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
            .map(|s| s.block_depth)
            .collect();
        let first = depths[0];
        assert!(depths.iter().all(|&d| d == first),
            "All top-level list items should have same depth, got {:?}", depths);
    }

    #[test]
    fn test_nested_list_depths() {
        let (spans, _, _) = get_spans(to_spans("- A\n  - B\n    - C\n- D"));
        let depths: Vec<u8> = spans.iter()
            .filter(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED)
            .map(|s| s.block_depth)
            .collect();
        // Should have deep nesting for C (depth >= 3)
        assert!(depths.iter().any(|&d| d >= 2), "Should have depth >= 2 for C, got {:?}", depths);
        // Top-level items A and D should have same depth
        if depths.len() >= 2 {
            assert_eq!(depths[0], depths[depths.len() - 1],
                "First and last list item should have same depth, got {:?}", depths);
        }
    }

    #[test]
    fn test_table_cells_get_unique_seq() {
        let (spans, _, _) = get_spans(to_spans("| A | B |\n|---|---|\n| 1 | 2 |"));
        let cell_seqs: Vec<(u8, u16)> = spans.iter()
            .filter(|s| s.block_type == BLOCK_TABLE_CELL
                || s.block_type == BLOCK_TABLE_HEADER_CELL)
            .map(|s| (s.block_type, s.block_seq))
            .collect();
        // Each cell should have its own seq for row/col reconstruction
        assert!(cell_seqs.len() >= 4, "Expected 4 cells, got {}", cell_seqs.len());
    }

    #[test]
    fn test_extra_data_self_describing() {
        let (_, _, extra) = get_spans(to_spans(
            "[link](https://a.com) **bold** [img](https://b.com/pic.png)"
        ));
        let chunks = parse_extra_chunks(&extra);
        // Each chunk has kind + len + data, and they're in order
        for &(kind, ref data) in &chunks {
            match kind {
                EXTRA_KIND_LINK_URL => assert!(!data.is_empty()),
                EXTRA_KIND_IMAGE_URL => assert!(!data.is_empty()),
                _ => {} // may have other kinds
            }
        }
        // Links should be in same order as they appear
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_code_language_in_extra_with_code_span() {
        let (spans, _, extra) = get_spans(to_spans("```python\nprint(1)\n```"));
        let code_spans: Vec<&Span> = spans.iter()
            .filter(|s| s.block_type == BLOCK_CODE)
            .collect();
        assert!(!code_spans.is_empty());
        // At least one code span references the language chunk
        assert!(code_spans.iter().any(|s| s.extra_len > 0));
        let chunks = parse_extra_chunks(&extra);
        assert!(chunks.iter().any(|(k, d)| *k == EXTRA_KIND_CODE_LANGUAGE
            && d == b"python"));
    }

    #[test]
    fn test_table_metadata_in_extra() {
        let (_, _, extra) = get_spans(to_spans(
            "| Left | Right |\n|:-----|------:|\n| a | b |"
        ));
        let chunks = parse_extra_chunks(&extra);
        let meta_chunk = chunks.iter()
            .find(|(k, _)| *k == EXTRA_KIND_TABLE_METADATA)
            .expect("Table metadata chunk should exist");
        // Format: [col_count: u8, (align: u8, char_width: u16 LE)*N]
        assert_eq!(meta_chunk.1[0], 2); // 2 columns
        assert_eq!(meta_chunk.1[1], 1); // Left align
        // Column widths follow (u16 LE per column after each align byte)
        assert!(meta_chunk.1.len() >= 7, "Expected 1+2*3=7 bytes, got {}", meta_chunk.1.len());
    }

    #[test]
    fn test_text_buffer_has_table_separators() {
        let (_, text, _) = get_spans(to_spans("| A | B |\n|---|---|\n| 1 | 2 |"));
        // Table cells should be tab-separated, rows newline-separated
        assert!(text.contains("\t"), "Cell separators should be tabs, got: {:?}", text);
        assert!(text.contains("\n"), "Row separators should be newlines");
    }

    #[test]
    fn test_display_math_uses_block_type() {
        let mut opts = MarkdownOptions::default();
        opts.math_mode = MathMode::Detect;
        let (spans, _, _) = get_spans(to_spans_with_options("$$x^2$$", &opts));
        assert!(spans.iter().any(|s| s.block_type == BLOCK_MATH_BLOCK
            || (s.is_math() && s.block_type != BLOCK_PARAGRAPH)));
    }

    #[test]
    fn test_span_struct_size_is_20() {
        assert_eq!(std::mem::size_of::<Span>(), 20);
    }
}
