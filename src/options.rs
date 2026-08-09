// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlMode {
    Strip,
    Safe,
    AllowAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathMode {
    Literal,
    Detect,
}

#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    pub html_mode: HtmlMode,
    pub math_mode: MathMode,
    pub smart_punctuation: bool,
    pub code_class_prefix: Option<String>,
    pub base_url: Option<String>,
    /// 代码高亮（默认关闭，需启用 `highlight` feature）
    #[cfg(feature = "highlight")]
    pub highlight: bool,
    /// 是否计算 UTF-16 偏移（为 iOS/Android 等 UTF-16 平台优化）
    /// 启用后在 SpanOutput 中输出与 span_count 一一对应的 UTF-16 偏移数组。
    /// 对于纯 ASCII 文本（占 AI 输出 95%+），UTF-8 == UTF-16，几乎零成本。
    pub compute_utf16_offsets: bool,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            html_mode: HtmlMode::Safe,
            math_mode: MathMode::Literal,
            smart_punctuation: false,
            code_class_prefix: Some("language-".into()),
            base_url: None,
            #[cfg(feature = "highlight")]
            highlight: false,
            compute_utf16_offsets: false,
        }
    }
}

impl MarkdownOptions {
    pub fn to_pulldown_options(&self) -> pulldown_cmark::Options {
        use pulldown_cmark::Options;
        let mut opts = Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_HEADING_ATTRIBUTES
            | Options::ENABLE_SUBSCRIPT
            | Options::ENABLE_SUPERSCRIPT
            | Options::ENABLE_DEFINITION_LIST
            | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
        if self.smart_punctuation {
            opts |= Options::ENABLE_SMART_PUNCTUATION;
        }
        if self.math_mode == MathMode::Detect {
            opts |= Options::ENABLE_MATH;
        }
        opts
    }
}

#[derive(Debug, Error)]
pub enum MarkdownError {
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Input too large: {0} bytes exceeds maximum {1} bytes")]
    InputTooLarge(usize, usize),
}
