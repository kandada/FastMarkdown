// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::options::{MarkdownOptions, MathMode};
use crate::span::*;

pub struct OwnedSpanOutput {
    pub text: Box<[u8]>,
    pub spans: Box<[Span]>,
    pub extra_data: Box<[u8]>,
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
        }
    }
}

pub fn markdown_to_spans_owned(md: &str, options: &MarkdownOptions) -> OwnedSpanOutput {
    let md = if options.html_mode == crate::options::HtmlMode::Strip {
        strip_html_tags(md)
    } else {
        preprocess_math_spans(md, options)
    };
    build_spans(&md, options)
}

pub fn markdown_to_spans(md: &str, options: &MarkdownOptions) -> SpanOutput {
    let owned = markdown_to_spans_owned(md, options);
    owned_to_span_output(owned)
}

fn owned_to_span_output(owned: OwnedSpanOutput) -> SpanOutput {
    let text_len = owned.text.len() as u32;
    let span_count = owned.spans.len() as u32;
    let extra_len = owned.extra_data.len() as u32;

    let text_fat: *mut [u8] = Box::into_raw(owned.text);
    let text_ptr = text_fat as *const u8;

    let spans_fat: *mut [Span] = Box::into_raw(owned.spans);
    let spans_ptr = spans_fat as *const Span;

    let extra_fat: *mut [u8] = Box::into_raw(owned.extra_data);
    let extra_ptr = extra_fat as *const u8;

    SpanOutput {
        text: text_ptr,
        text_len,
        spans: spans_ptr,
        span_count,
        extra_data: extra_ptr,
        extra_data_len: extra_len,
    }
}

fn preprocess_math_spans(md: &str, options: &MarkdownOptions) -> String {
    if options.math_mode != MathMode::Detect {
        return md.to_string();
    }
    let regions = crate::math::detect_math_regions(md);
    if regions.is_empty() {
        return md.to_string();
    }
    // Replace math regions with HTML spans for span mode
    let mut sorted: Vec<_> = regions.iter().collect();
    sorted.sort_by(|a, b| b.start.cmp(&a.start));
    let mut result = md.to_string();
    for region in &sorted {
        let content = &md[region.start..region.end];
        if region.is_display {
            let replacement = format!(
                "\n<div data-math=\"display\">{}</div>\n",
                &content[2..content.len() - 2]
            );
            result.replace_range(region.start..region.end, &replacement);
        } else {
            let replacement = format!(
                "<span data-math=\"inline\">{}</span>",
                &content[1..content.len() - 1]
            );
            result.replace_range(region.start..region.end, &replacement);
        }
    }
    result
}

fn build_spans(md: &str, options: &MarkdownOptions) -> OwnedSpanOutput {
    let parser = Parser::new_ext(md, options.to_pulldown_options());

    let mut state = SpanBuildState::new();
    for event in parser {
        state.process_event(event);
    }
    state.    finish();
    state.into_output()
}

struct SpanBuildState {
    text_buf: String,
    spans: Vec<Span>,
    extra_data: Vec<u8>,

    current_flags: u32,
    current_block: u8,
    block_stack: Vec<u8>,

    // Link URL tracking
    link_urls: Vec<String>,

    // Table state
    table_alignments: Vec<Alignment>,
    in_table_head: bool,
    table_cell_started: bool,
}

impl SpanBuildState {
    fn new() -> Self {
        Self {
            text_buf: String::new(),
            spans: Vec::new(),
            extra_data: Vec::new(),
            current_flags: 0,
            current_block: BLOCK_PARAGRAPH,
            block_stack: vec![BLOCK_PARAGRAPH],
            link_urls: Vec::new(),
            table_alignments: Vec::new(),
            in_table_head: false,
            table_cell_started: false,
        }
    }

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
                self.emit_span(marker, 0, BLOCK_PARAGRAPH);
            }
            Event::FootnoteReference(name) => {
                let ref_text = format!("[^{}]", name);
                self.emit_span(&ref_text, 0, self.current_block);
            }
        }
    }

    fn handle_start(&mut self, tag: Tag) {
        match tag {
            // Block-level tags
            Tag::Paragraph => {
                self.push_block(BLOCK_PARAGRAPH);
            }
            Tag::Heading { level, .. } => {
                let bt = heading_to_block_type(level);
                self.push_block(bt);
            }
            Tag::BlockQuote(_kind) => {
                self.push_block(BLOCK_BLOCKQUOTE);
            }
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => lang.to_string(),
                    _ => String::new(),
                };
                self.push_block(BLOCK_CODE);
                // Store language in extra_data if present
                if !language.is_empty() {
                    self.store_extra(&language);
                }
            }
            Tag::List(number) => {
                let bt = if number.is_some() {
                    BLOCK_LIST_ITEM_ORDERED
                } else {
                    BLOCK_LIST_ITEM_UNORDERED
                };
                self.push_block(bt);
            }
            Tag::Item => {
                // Keep parent list type, but mark as new item
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments;
                self.in_table_head = true;
            }
            Tag::TableHead => {
                self.in_table_head = true;
            }
            Tag::TableRow => {
                // Row groups cells
            }
            Tag::TableCell => {
                self.table_cell_started = true;
                let bt = if self.in_table_head {
                    BLOCK_TABLE_HEADER_CELL
                } else {
                    BLOCK_TABLE_CELL
                };
                self.push_block(bt);
            }
            Tag::FootnoteDefinition(_) => {
                // Skip footnote definitions in output
            }

            // Inline style tags
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

            Tag::MetadataBlock(_) => {} // ignore for rendering

            // Additional tags from pulldown-cmark 0.13
            Tag::HtmlBlock => {}
            Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition => {}
            Tag::Subscript | Tag::Superscript => {}
        }
    }

    fn handle_end(&mut self, tag: TagEnd) {
        match tag {
            // Block-level tags
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock => {
                self.pop_block();
            }
            TagEnd::List(_) => {
                self.pop_block();
            }
            TagEnd::Item => {
                // Add a line break after each list item
                self.emit_break_span();
            }
            TagEnd::Table => {
                self.table_alignments.clear();
                self.in_table_head = false;
            }
            TagEnd::TableHead => {
                self.in_table_head = false;
            }
            TagEnd::TableRow => {
                // Emit a row separator
                self.emit_break_span();
            }
            TagEnd::TableCell => {
                self.table_cell_started = false;
                self.pop_block();
            }
            TagEnd::FootnoteDefinition => {}

            // Inline style tags
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

            // Additional tags from pulldown-cmark 0.13
            TagEnd::HtmlBlock
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition => {}
            TagEnd::Subscript | TagEnd::Superscript => {}
            _ => {}
        }
    }

    fn handle_text(&mut self, text: &str, is_code: bool) {
        let flags = if is_code {
            self.current_flags | FLAG_INLINE_CODE
        } else {
            self.current_flags
        };
        self.emit_span(text, flags, self.current_block);
    }

    fn handle_math(&mut self, math: &str, _is_display: bool) {
        let flags = self.current_flags | FLAG_MATH;
        let block = self.current_block;
        self.emit_span(math, flags, block);
    }

    fn handle_html_block(&mut self, html: &str) {
        if html.contains("data-math=\"display\"") || html.contains("data-math=\"inline\"") {
            // Math wrappers from preprocessing; math flag set via handle_inline_html
        } else {
            let stripped = strip_html_tags(html);
            if !stripped.is_empty() {
                self.emit_span(&stripped, self.current_flags, self.current_block);
            }
        }
    }

    fn handle_inline_html(&mut self, html: &str) {
        // Check for math span markers from preprocessing
        if html.contains("data-math=\"inline\"") {
            // Math content will come as HTML body text via Text events
            self.current_flags |= FLAG_MATH;
        } else if html.contains("/span") && (self.current_flags & FLAG_MATH) != 0 {
            self.current_flags &= !FLAG_MATH;
        } else {
            // For non-math inline HTML, strip tags and emit text
            let stripped = strip_html_tags(html);
            if !stripped.is_empty() {
                self.emit_span(&stripped, self.current_flags, self.current_block);
            }
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
        // Emit a placeholder for horizontal rule
        let offset = self.text_buf.len() as u32;
        // Using \n---\n as separator
        self.text_buf.push_str("\n---\n");
        self.spans.push(Span::new(
            offset,
            5,
            0,
            BLOCK_HORIZONTAL_RULE,
            0,
        ));
    }

    fn emit_span(&mut self, text: &str, flags: u32, block_type: u8) {
        if text.is_empty() {
            return;
        }
        let offset = self.text_buf.len() as u32;
        let len = text.len() as u32;
        self.text_buf.push_str(text);

        // Check if we need to store a link URL in extra_data
        let extra_len = if (flags & FLAG_LINK) != 0 || (flags & FLAG_IMAGE) != 0 {
            if let Some(url) = self.link_urls.last() {
                let url_bytes = url.as_bytes();
                let start = self.extra_data.len();
                self.extra_data.extend_from_slice(url_bytes);
                (self.extra_data.len() - start) as u32
            } else {
                0
            }
        } else {
            0
        };

        self.spans.push(Span::new(offset, len, flags, block_type, extra_len));
    }

    fn emit_break_span(&mut self) {
        // Utility to add a break
        self.text_buf.push('\n');
    }

    fn push_block(&mut self, block_type: u8) {
        self.block_stack.push(block_type);
        self.current_block = block_type;
    }

    fn pop_block(&mut self) {
        if self.block_stack.len() > 1 {
            self.block_stack.pop();
            self.current_block = *self.block_stack.last().unwrap();
        }
    }

    fn store_extra(&mut self, data: &str) {
        self.extra_data.extend_from_slice(data.as_bytes());
    }

    fn finish(&mut self) {
        // Trim trailing whitespace spans
        while let Some(last) = self.spans.last() {
            if last.length == 0 || self.text_buf.as_bytes()
                .get(last.offset as usize..(last.offset + last.length) as usize)
                .map(|s| s.iter().all(|b| b.is_ascii_whitespace()))
                .unwrap_or(false)
            {
                self.spans.pop();
            } else {
                break;
            }
        }
    }

    fn into_output(self) -> OwnedSpanOutput {
        OwnedSpanOutput {
            text: self.text_buf.into_bytes().into_boxed_slice(),
            spans: self.spans.into_boxed_slice(),
            extra_data: self.extra_data.into_boxed_slice(),
        }
    }
}

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

fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn get_spans(md: &str) -> (OwnedSpanOutput, SpanOutput) {
        let owned = markdown_to_spans_owned(md, &Default::default());
        let output = owned.as_output();
        (owned, output)
    }

    #[test]
    fn test_basic_bold() {
        let (_owned, output) = get_spans("Hello **world**");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        let text = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
                .unwrap()
        };

        assert!(spans.iter().any(|s| s.is_bold() && &text[s.offset as usize..(s.offset + s.length) as usize] == "world"));
    }

    #[test]
    fn test_italic() {
        let (_owned, output) = get_spans("*italic text*");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.is_italic()));
    }

    #[test]
    fn test_bold_italic() {
        let (_owned, output) = get_spans("***bold italic***");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.is_bold() && s.is_italic()));
    }

    #[test]
    fn test_inline_code() {
        let (_owned, output) = get_spans("Use `println!` macro");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.is_inline_code()));
    }

    #[test]
    fn test_heading_block_type() {
        let (_owned, output) = get_spans("# Heading 1");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.block_type == BLOCK_HEADING_H1));

        let (_owned, output) = get_spans("### Heading 3");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.block_type == BLOCK_HEADING_H3));
    }

    #[test]
    fn test_link_url_in_extra() {
        let (_owned, output) = get_spans("[click here](https://example.com)");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        let link_span = spans.iter().find(|s| s.is_link()).unwrap();
        assert!(link_span.extra_len > 0);
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let (_owned, output) = get_spans(md);
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.block_type == BLOCK_CODE));
    }

    #[test]
    fn test_horizontal_rule() {
        let (_owned, output) = get_spans("---");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.block_type == BLOCK_HORIZONTAL_RULE));
    }

    #[test]
    fn test_strikethrough() {
        let (_owned, output) = get_spans("~~deleted~~");
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.is_strikethrough()));
    }

    #[test]
    fn test_list_item() {
        let md = "- item one\n- item two";
        let (_owned, output) = get_spans(md);
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        assert!(spans.iter().any(|s| s.block_type == BLOCK_LIST_ITEM_UNORDERED));
    }

    #[test]
    fn test_empty_input() {
        let (_owned, output) = get_spans("");
        assert_eq!(output.span_count, 0);
        assert_eq!(output.text_len, 0);
    }

    #[test]
    fn test_text_offset_consistency() {
        let (_owned, output) = get_spans("ABC **DEF** GHI");
        let text = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
                .unwrap()
        };
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };

        for span in spans {
            let end = (span.offset + span.length) as usize;
            assert!(end <= text.len(), "Span offset+len exceeds text length");
        }

        // Check that text contains expected content
        assert!(text.contains("ABC"));
        assert!(text.contains("DEF"));
        assert!(text.contains("GHI"));
    }

    #[test]
    fn test_table_cell_block_types() {
        let md = "| Name | Value |\n|------|-------|\n| foo  | 42    |";
        let (_owned, output) = get_spans(md);
        let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
        let has_header = spans.iter().any(|s| s.block_type == BLOCK_TABLE_HEADER_CELL);
        let has_cell = spans.iter().any(|s| s.block_type == BLOCK_TABLE_CELL);
        assert!(has_header, "Should have table header cells");
        assert!(has_cell, "Should have table data cells");
    }
}
