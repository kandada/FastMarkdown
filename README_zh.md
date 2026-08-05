# fastmarkdown

**极速、跨平台的 Markdown 渲染引擎 —— 一次解析，到处渲染。**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)

fastmarkdown 是一个 Rust 驱动的 Markdown 渲染 SDK，将 Markdown 解析为平台原生可显示对象。专为移动端 AI agent 场景（聊天界面、编程助手）设计，iOS 上输出 `NSAttributedString`，Android 上输出 `SpannableString`，其他平台输出 HTML —— 全部由一个 Rust 核心引擎 + C FFI 驱动。

---

## 为什么需要 fastmarkdown

传统移动端 Markdown 渲染存在三大痛点：

- **View 爆炸**：每个行内样式 span 都拆成一个独立 UI 控件，大量消息压死主线程。
- **二次解析**：HTML 输出需要再 parse 一次才能变成原生对象（iOS 上 WebKit HTML parser 在主线程同步跑）。
- **平台不一致**：iOS 和 Android 用不同的 parser，同一个输入渲染结果不同。

fastmarkdown 的解决方案：

1. **Span 二进制格式**：一个 `(offset, len, flags, block_type)` 的平铺数组替代 N 个逐样式 View，一条消息 → 一个 `NSAttributedString` → 一个 `UITextView`。
2. **无二次解析**：Rust 核心直接产出原生数据结构（Span → NSAttributedString/SpannableString）。
3. **同一套代码**：同一个 parser，同一个测试集，iOS 和 Android 行为完全一致。

---

## 功能

- **完整 GFM 支持** — 标题、粗体、斜体、删除线、行内代码、代码块、链接、图片、有序/无序列表、引用、表格、分割线、任务列表、脚注
- **HTML 标签混排** — `<b>`、`<i>`、`<a>`、`<img>`、`<table>`、`<details>` 等，三种安全模式（Strip / Safe / AllowAll）
- **`$` 符号智能处理** — 字面模式无视 `$HOME`、`$5.99`；可选检测模式识别 LaTeX 公式
- **流式 API** — 原生 token 级增量追加，适配实时 AI 输出
- **双路输出** — HTML 字符串（WebView/导出）+ 结构化 Span 数组（原生渲染）
- **C FFI** — 编译为静态/动态库，可从 Swift、Kotlin、C、Python 调用
- **跨平台** — iOS (aarch64-apple-ios)、Android (aarch64-linux-android)、macOS/Linux/Windows

---

## 快速开始

### Rust

```rust
use fastmarkdown::{to_html, to_spans, to_plain_text, MarkdownOptions, StreamRenderer};

// 渲染为 HTML
let html = to_html("# Hello **world**");
assert!(html.contains("<h1>"));

// 渲染为结构化 Span（供原生渲染）
let output = to_spans("# Hello **world**");
let spans = unsafe {
    std::slice::from_raw_parts(output.spans, output.span_count as usize)
};
assert!(spans.iter().any(|s| s.is_bold()));
unsafe { fastmarkdown::free_spans(output); }

// 提取纯文本（搜索/预览/通知）
let text = to_plain_text("**bold** and *italic*");
assert_eq!(text, "bold and italic");

// 流式渲染（实时 AI 输出）
let mut stream = StreamRenderer::new(MarkdownOptions::default());
stream.append("# He");
stream.append("llo **world**");
```

### C / FFI

```c
#include "fastmarkdown.h"

// HTML 输出
char* html = fastmarkdown_to_html("# Hello **world**");
printf("%s\n", html);
fastmarkdown_free_string(html);

// Span 输出（原生渲染）
FastMarkdownSpanOutput spans = fastmarkdown_to_spans("# Hello **world**");
for (uint32_t i = 0; i < spans.span_count; i++) {
    FastMarkdownSpan s = spans.spans[i];
    // s.offset, s.length, s.flags, s.block_type
}
fastmarkdown_free_spans(&spans);

// 流式
FastMarkdownStream* stream = fastmarkdown_stream_create(1, 0, false);
fastmarkdown_stream_append(stream, "# Hello");
fastmarkdown_stream_append(stream, " **world**");
fastmarkdown_stream_destroy(stream);
```

### iOS (Swift)

```swift
let output = fastmarkdown_to_spans(md)
defer { fastmarkdown_free_spans(&output) }

let text = String(bytesNoCopy: ...)
let attrString = NSMutableAttributedString(string: text)
let spans = UnsafeBufferPointer(start: output.spans, count: Int(output.span_count))
for span in spans {
    let range = NSRange(location: Int(span.offset), length: Int(span.length))
    if span.flags & FLAG_BOLD != 0 {
        attrString.addAttribute(.font, value: boldFont, range: range)
    }
}
textView.attributedText = attrString
```

### Android (Kotlin / JNI)

```kotlin
external fun fastmarkdown_to_spans(markdown: String): FastMarkdownSpanOutput
external fun fastmarkdown_free_spans(output: FastMarkdownSpanOutput)

val output = fastmarkdown_to_spans(md)
val spannable = SpannableString(String(output.text, 0, output.text_len.toInt()))
for (i in 0 until output.span_count.toInt()) {
    val s = output.spans[i]
    if ((s.flags and FLAG_BOLD) != 0u) {
        spannable.setSpan(StyleSpan(Typeface.BOLD), s.offset.toInt(), ...)
    }
}
fastmarkdown_free_spans(output)
textView.text = spannable
```

---

## API 参考

### 配置选项

```rust
pub struct MarkdownOptions {
    pub html_mode: HtmlMode,        // Strip | Safe (默认) | AllowAll
    pub math_mode: MathMode,        // Literal (默认) | Detect
    pub smart_punctuation: bool,
    pub code_class_prefix: Option<String>,
    pub base_url: Option<String>,
}
```

### 主题配置

```rust
pub struct ThemeConfig {
    pub body_font_name: *const c_char,  // null = 系统默认
    pub body_font_size: f32,            // 默认 14.0
    pub monospace_font_name: *const c_char,
    pub monospace_font_size: f32,       // 默认 13.0
    pub heading_font_sizes: [f32; 6],
    pub text_color: u32,                // 0xRRGGBBAA
    pub link_color: u32,
    pub code_bg_color: u32,
    pub code_text_color: u32,
    pub blockquote_bar_color: u32,
    pub paragraph_spacing: f32,
    pub line_spacing: f32,
    pub list_indent: f32,
    pub blockquote_indent: f32,
}
```

`ThemeConfig` 实现了 `Default`，不传参数也有完整的预设值（14pt 正文、13pt 等宽、黑色文本、蓝色链接等）。

### C FFI 函数列表

详见 [design.md](design.md) 中的完整 API 参考和 Span 二进制格式定义。

---

## 架构

```
Markdown 字符串
      │
      ▼
┌─────────────┐    ┌──────────┐    ┌──────────────────┐
│  Parser     │───▶│   AST    │───▶│  Renderer        │
│ pulldown    │    │ (内部IR) │    │                  │
│ -cmark      │    └──────────┘    │ ① HTML String    │
└─────────────┘                    │ ② Span Array     │
                                   │ ③ Plain Text     │
                                   └────────┬─────────┘
                                            │
                                     C FFI Layer
                                            │
                          ┌─────────────────┼─────────────────┐
                          │                 │                 │
                     ┌────┴────┐      ┌─────┴─────┐     ┌────┴────┐
                     │   iOS   │      │  Android  │     │ Desktop │
                     │  Swift  │      │   Kotlin  │     │ Rust/C  │
                     └─────────┘      └───────────┘     └─────────┘
```

---

## 编译

```bash
git clone https://github.com/kandada/fastshell
cd fastshell/fastmarkdown

cargo build --release        # 库文件
cargo test                   # 运行测试
cargo bench                  # 性能基准

# iOS 交叉编译
cargo build --release --target aarch64-apple-ios

# Android 交叉编译
cargo build --release --target aarch64-linux-android
```

---

## 使用场景

fastmarkdown 已用于：

- [**AACode**](https://github.com/kandada/aacode) — 移动端 AI 编程助手 (iOS)

---

## 开源协议

Copyright 2025 xiefujin (490021684@qq.com)

基于 Apache License, Version 2.0 开源。详见 [LICENSE](LICENSE)。
