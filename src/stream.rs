// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use crate::options::MarkdownOptions;
use crate::span::*;
use crate::spans_renderer::OwnedSpanOutput;

/// 流式渲染器
///
/// 维护内部文本 buffer，每次调用 `append()` 追加新文本并返回完整 SpanOutput。
/// 返回的 SpanOutput 指针在下一次 `append()` 或 `drop` 后失效。
///
/// 内置 fast-path 优化：当新的 chunk 为纯文本（不包含任何 markdown 结构字符）时，
/// 直接在上一轮输出上扩展，避免对全文重新解析。这使流式 AI 输出的常见场景（token 级追加）
/// 从 O(n²) 降为近似 O(n)。
pub struct StreamRenderer {
    buffer: String,
    options: MarkdownOptions,
    current_output: Option<OwnedSpanOutput>,
    previous_span_snapshots: Vec<SpanSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpanSnapshot {
    offset: u32,
    length: u32,
    flags: u32,
    block_type: u8,
}

impl StreamRenderer {
    pub fn new(options: MarkdownOptions) -> Self {
        Self {
            buffer: String::new(),
            options,
            current_output: None,
            previous_span_snapshots: Vec::new(),
        }
    }

    /// 追加文本块，返回当前完整 SpanOutput。
    ///
    /// 返回值的指针有效期：直到下一次调用 `append()` 或 StreamRenderer 被销毁。
    /// 调用方不应调用 `free_spans()` 释放此输出。
    pub fn append(&mut self, chunk: &str) -> SpanOutput {
        self.buffer.push_str(chunk);

        // Fast path: if this chunk is pure continuation text (no markdown structure chars),
        // extend the previous output in-place rather than re-parsing everything.
        if let Some(ref prev) = self.current_output {
            if is_plain_continuation(chunk) && !prev.spans.is_empty() {
                if let Some(output) = try_extend_output(prev, chunk) {
                    self.current_output = Some(output);
                    let spans = unsafe {
                        std::slice::from_raw_parts(
                            self.current_output.as_ref().unwrap().spans.as_ptr(),
                            self.current_output.as_ref().unwrap().spans.len(),
                        )
                    };
                    self.previous_span_snapshots = snapshots(spans);
                    return self.current_output.as_ref().unwrap().as_output();
                }
            }
        }

        // Slow path: full re-parse
        self.current_output = None;

        let output = crate::spans_renderer::markdown_to_spans_owned(&self.buffer, &self.options);
        let result = output.as_output();

        let spans = unsafe { std::slice::from_raw_parts(result.spans, result.span_count as usize) };
        self.previous_span_snapshots = snapshots(spans);

        self.current_output = Some(output);
        result
    }

    /// 标记流结束，flush 最后一个未闭合块。
    /// 返回最终的 SpanOutput（或空，如果没有新内容）。
    /// 这之后不应再调用 append。
    pub fn finish(&mut self) -> Option<SpanOutput> {
        if self.buffer.is_empty() {
            return None;
        }

        // Re-parse to ensure final state (close any open blocks)
        self.current_output = None;
        let output = crate::spans_renderer::markdown_to_spans_owned(&self.buffer, &self.options);
        let result = output.as_output();
        self.current_output = Some(output);
        Some(result)
    }

    /// 计算自上次 append 以来的增量 delta。
    /// 返回新增或变更的 SpanDelta。
    pub fn delta(&mut self, chunk: &str) -> SpanDeltaList {
        self.buffer.push_str(chunk);

        // Try fast path first (same as append)
        if let Some(ref prev) = self.current_output {
            if is_plain_continuation(chunk) && !prev.spans.is_empty() {
                if let Some(output) = try_extend_output(prev, chunk) {
                    let current_spans = unsafe {
                        std::slice::from_raw_parts(output.spans.as_ptr(), output.spans.len())
                    };
                    let delta = compute_delta(&self.previous_span_snapshots, current_spans);
                    self.previous_span_snapshots = snapshots(current_spans);
                    self.current_output = Some(output);
                    return delta;
                }
            }
        }

        // Slow path: full re-parse
        self.current_output = None;
        let output = crate::spans_renderer::markdown_to_spans_owned(&self.buffer, &self.options);

        let current_spans =
            unsafe { std::slice::from_raw_parts(output.spans.as_ptr(), output.spans.len()) };

        let delta = compute_delta(&self.previous_span_snapshots, current_spans);

        self.previous_span_snapshots = snapshots(current_spans);
        self.current_output = Some(output);

        delta
    }

    /// 获取当前完整 SpanOutput 的一份拷贝（安全版本，拥有所有权）
    /// 调用方负责使用 `free_spans()` 或 `drop` 释放。
    pub fn snapshot_owned(&self) -> OwnedSpanOutput {
        crate::spans_renderer::markdown_to_spans_owned(&self.buffer, &self.options)
    }

    /// 重置渲染器用于新的流
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.current_output = None;
        self.previous_span_snapshots.clear();
    }

    /// 获取当前累积的原始 Markdown 文本
    pub fn raw_text(&self) -> &str {
        &self.buffer
    }

    /// 获取当前累积文本的长度
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// Checks whether a chunk is safe for fast-path extension:
/// no markdown structure characters, no newlines.
fn is_plain_continuation(chunk: &str) -> bool {
    for b in chunk.bytes() {
        match b {
            b'\n' | b'\r' | b'#' | b'*' | b'_' | b'`' | b'[' | b']' | b'<' | b'>' | b'|' | b'!'
            | b'~' | b'$' | b'-' | b'+' | b'=' | b'\\' | b'^' => return false,
            _ => {}
        }
    }
    !chunk.is_empty()
}

/// Try to extend the previous output with new plain text, avoiding full re-parse.
/// Returns None if the last block type makes extension unsafe (e.g., inside a code block).
fn try_extend_output(prev: &OwnedSpanOutput, chunk: &str) -> Option<OwnedSpanOutput> {
    let chunk_bytes = chunk.as_bytes();
    if chunk_bytes.is_empty() {
        return None;
    }

    // Don't fast-path if the last span is in a special block that may need re-interpretation
    if let Some(last_span) = prev.spans.last() {
        if last_span.block_type == BLOCK_CODE {
            return None;
        }
        if (last_span.flags & FLAG_IMAGE) != 0 {
            return None;
        }
    }

    // Extend text buffer
    let mut new_text: Vec<u8> = Vec::with_capacity(prev.text.len() + chunk_bytes.len());
    new_text.extend_from_slice(&prev.text);
    new_text.extend_from_slice(chunk_bytes);

    // Clone spans, extending the last span's length
    let mut new_spans: Vec<Span> = prev.spans.to_vec();
    if let Some(ref mut last) = new_spans.last_mut() {
        last.length += chunk.len() as u32;
    }

    // Extend utf16_offsets: the last span's UTF-16 offset stays the same;
    // no new spans are added, so the array length remains unchanged.
    let new_utf16 = prev.utf16_offsets.clone();

    Some(OwnedSpanOutput {
        text: new_text.into_boxed_slice(),
        spans: new_spans.into_boxed_slice(),
        extra_data: prev.extra_data.clone(),
        utf16_offsets: new_utf16,
    })
}

fn snapshots(spans: &[Span]) -> Vec<SpanSnapshot> {
    spans
        .iter()
        .map(|s| SpanSnapshot {
            offset: s.offset,
            length: s.length,
            flags: s.flags,
            block_type: s.block_type,
        })
        .collect()
}

/// 增量 Span 列表
#[derive(Debug, Clone)]
pub struct SpanDeltaList {
    pub deltas: Vec<SpanDelta>,
    /// 在输出 text buffer 中，之前已锁定的字节数
    pub locked_text_len: u32,
}

fn compute_delta(previous: &[SpanSnapshot], current: &[Span]) -> SpanDeltaList {
    if previous.is_empty() {
        let all_spans: Vec<Span> = current.to_vec();
        let locked_len = 0u32;
        return SpanDeltaList {
            deltas: vec![SpanDelta {
                kind: SpanDeltaKind::Append,
                spans: all_spans,
            }],
            locked_text_len: locked_len,
        };
    }

    // Find common prefix of matching spans
    let mut common = 0usize;
    let min_len = previous.len().min(current.len());

    for i in 0..min_len {
        let p = &previous[i];
        let c = &current[i];
        if p.offset == c.offset
            && p.length == c.length
            && p.flags == c.flags
            && p.block_type == c.block_type
        {
            common = i + 1;
        } else {
            break;
        }
    }

    // Spans before 'common' are locked/stable
    let locked_len = if common > 0 && common <= current.len() {
        current[common - 1].offset + current[common - 1].length
    } else {
        0
    };

    let mut deltas = Vec::new();

    if common < previous.len() && common < current.len() {
        // Changed spans
        let replaced_count = (previous.len() - common) as u32;
        let new_spans: Vec<Span> = current[common..].to_vec();
        if !new_spans.is_empty() {
            deltas.push(SpanDelta {
                kind: SpanDeltaKind::ReplaceLast {
                    count: replaced_count,
                },
                spans: new_spans,
            });
        }
    } else if common < current.len() {
        // New spans appended
        let new_spans: Vec<Span> = current[common..].to_vec();
        deltas.push(SpanDelta {
            kind: SpanDeltaKind::Append,
            spans: new_spans,
        });
    }

    SpanDeltaList {
        deltas,
        locked_text_len: locked_len,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_basic() {
        let mut stream = StreamRenderer::new(MarkdownOptions::default());
        let output = stream.append("# Hello");
        assert!(output.span_count > 0);

        let output = stream.append("\nWorld");
        assert!(output.span_count > 0);
    }

    #[test]
    fn test_stream_delta() {
        let mut stream = StreamRenderer::new(MarkdownOptions::default());
        stream.append("# Hello");

        let delta = stream.delta(" **world**");
        assert!(!delta.deltas.is_empty());
    }

    #[test]
    fn test_stream_reset() {
        let mut stream = StreamRenderer::new(MarkdownOptions::default());
        stream.append("hello");
        assert!(!stream.is_empty());

        stream.reset();
        assert!(stream.is_empty());

        let output = stream.append("new");
        assert!(output.span_count > 0);
    }

    #[test]
    fn test_stream_finish() {
        let mut stream = StreamRenderer::new(MarkdownOptions::default());
        stream.append("hello world");
        let final_output = stream.finish();
        assert!(final_output.is_some());
    }

    #[test]
    fn test_stream_snapshot() {
        let mut stream = StreamRenderer::new(MarkdownOptions::default());
        stream.append("hello");
        let snapshot = stream.snapshot_owned();
        let output = snapshot.as_output();
        assert!(output.span_count > 0);
    }

    #[test]
    fn test_is_plain_continuation() {
        assert!(is_plain_continuation("hello"));
        assert!(is_plain_continuation("World 123"));
        assert!(is_plain_continuation("foo.bar(baz)"));
        assert!(is_plain_continuation("text, with; punctuation: etc"));
        assert!(!is_plain_continuation(""));
        assert!(!is_plain_continuation("hello\nworld"));
        assert!(!is_plain_continuation("hello**bold**"));
        assert!(!is_plain_continuation("hello*world*"));
        assert!(!is_plain_continuation("code`here`"));
        assert!(!is_plain_continuation("link[text](url)"));
        assert!(!is_plain_continuation("has#tag"));
        assert!(!is_plain_continuation("pipe|table"));
        assert!(!is_plain_continuation("$math$"));
    }

    #[test]
    fn test_fast_path_extends_last_span() {
        let mut stream = StreamRenderer::new(MarkdownOptions::default());
        let out1 = stream.append("Hello");
        let first_span_count = out1.span_count;
        let first_text_len = out1.text_len;

        // Append plain text — should use fast path
        let out2 = stream.append(" World");
        // Text should be extended
        assert_eq!(out2.text_len, first_text_len + 6);
        assert_eq!(out2.span_count, first_span_count);
    }

    #[test]
    fn test_fast_path_falls_back_on_markdown() {
        let mut stream = StreamRenderer::new(MarkdownOptions::default());
        stream.append("Hello");
        // This contains ** which triggers slow path
        let out = stream.append(" **bold**");
        // Should still be correct
        assert!(out.span_count > 0);
    }

    #[test]
    fn test_fast_path_correctness() {
        // Fast path output should be consistent with full parse output
        let mut stream1 = StreamRenderer::new(MarkdownOptions::default());
        stream1.append("Hello");

        let mut stream2 = StreamRenderer::new(MarkdownOptions::default());
        stream2.append("Hello World");

        let out1 = stream1.append(" World");
        let out1_text = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(
                out1.text,
                out1.text_len as usize,
            ))
            .unwrap()
        };

        let snapshot = stream2.snapshot_owned();
        let out2_full = snapshot.as_output();
        let out2_full_text = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(
                out2_full.text,
                out2_full.text_len as usize,
            ))
            .unwrap()
        };

        assert_eq!(out1.text_len, out2_full.text_len);
        assert_eq!(out1_text, out2_full_text);
    }

    #[test]
    fn test_fast_path_with_punctuation() {
        let mut stream = StreamRenderer::new(MarkdownOptions::default());
        stream.append("The ");
        let out = stream.append("quick, brown. fox! (test)");
        assert!(out.text_len > 0);
        assert!(out.span_count > 0);

        // Verify text content
        let text = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(out.text, out.text_len as usize))
                .unwrap()
        };
        assert!(text.contains("quick, brown. fox! (test)"));
    }
}
