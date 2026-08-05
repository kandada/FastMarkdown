// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use crate::options::MarkdownOptions;
use crate::span::*;
use crate::spans_renderer::OwnedSpanOutput;

/// 流式渲染器
///
/// 维护内部文本 buffer，每次调用 `append()` 追加新文本并返回完整 SpanOutput。
/// 返回的 SpanOutput 指针在下一次 `append()` 或 `drop` 后失效。
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

        self.current_output = None;

        let output = crate::spans_renderer::markdown_to_spans_owned(&self.buffer, &self.options);
        let result = output.as_output();

        // Update snapshots for future delta calculation
        let spans = unsafe {
            std::slice::from_raw_parts(result.spans, result.span_count as usize)
        };
        self.previous_span_snapshots = snapshots(spans);

        self.current_output = Some(output);
        result
    }

    /// 标记流结束，flush 最后一个未闭合块。
    /// 返回最终的 SpanOutput（或空，如果没有新内容）。
    /// 这之后不应再调用 append。
    pub fn finish(&mut self) -> Option<SpanOutput> {
        // In our current design, append already gives the complete output.
        // finish ensures the last state is flushed.
        // If there's nothing in the buffer, return None.
        if self.buffer.is_empty() {
            return None;
        }

        // Re-parse to ensure final state
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

        self.current_output = None;
        let output = crate::spans_renderer::markdown_to_spans_owned(&self.buffer, &self.options);

        let current_spans = unsafe {
            std::slice::from_raw_parts(output.spans.as_ptr(), output.spans.len())
        };

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
}
