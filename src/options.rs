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
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            html_mode: HtmlMode::Safe,
            math_mode: MathMode::Literal,
            smart_punctuation: false,
            code_class_prefix: Some("language-".into()),
            base_url: None,
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
            | Options::ENABLE_HEADING_ATTRIBUTES;
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
