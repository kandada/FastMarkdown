# fastmarkdown

**Blazing fast, cross-platform Markdown renderer — parse once, display everywhere.**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)

fastmarkdown is a Rust-powered Markdown rendering SDK that parses Markdown into platform-native display objects. Designed for mobile AI agent applications (chat interfaces, code assistants), it outputs `NSAttributedString` on iOS, `SpannableString` on Android, and HTML everywhere else — all from a single Rust core with a C FFI.

---

## Why

Traditional Markdown rendering on mobile suffers from:
- **View explosion** — each inline style span becomes a separate UI widget, drowning the main thread.
- **Double parsing** — HTML output requires a second parse pass (WebKit HTML parser on iOS) before native display.
- **Platform divergence** — iOS and Android use different parsers with different behaviors.

fastmarkdown solves all three:
1. **Span binary format** — one flat array of `(offset, len, flags, block_type)` replaces N per-span widgets with one `NSAttributedString` → one `UITextView`.
2. **No double parsing** — the Rust core produces platform-native objects directly (Span → NSAttributedString/SpannableString).
3. **One codebase** — the same Rust parser, the same test suite, the same behavior on every platform.

---

## Features

- **Full GFM support** — headings, bold, italic, strikethrough, inline code, code blocks, links, images, ordered/unordered lists, blockquotes, tables, horizontal rules, task lists, footnotes
- **HTML tag passthrough** — `<b>`, `<i>`, `<a>`, `<img>`, `<table>`, `<details>`, etc., with three safety modes (Strip / Safe / AllowAll)
- **`$` sign pitfalls avoided** — literal mode treats `$HOME`, `$5.99` as text; optional detect mode for LaTeX math
- **Streaming API** — native token-level append for real-time AI output
- **Dual output** — HTML string (WebView/export) and structured Span array (native rendering)
- **C FFI** — static/dynamic library compiled from Rust, consumable from Swift, Kotlin, C, Python
- **Cross-platform** — iOS (aarch64-apple-ios), Android (aarch64-linux-android), macOS/Linux/Windows

---

## Quick Start

### Rust

```rust
use fastmarkdown::{to_html, to_spans, to_plain_text, MarkdownOptions, StreamRenderer};

// Render to HTML
let html = to_html("# Hello **world**");
assert!(html.contains("<h1>"));

// Render to structured spans (for native display)
let output = to_spans("# Hello **world**");
let spans = unsafe {
    std::slice::from_raw_parts(output.spans, output.span_count as usize)
};
assert!(spans.iter().any(|s| s.is_bold()));
unsafe { fastmarkdown::free_spans(output); }

// Extract plain text (search/preview/notifications)
let text = to_plain_text("**bold** and *italic*");
assert_eq!(text, "bold and italic");

// Streaming for real-time AI output
let mut stream = StreamRenderer::new(MarkdownOptions::default());
stream.append("# He");
stream.append("llo **world**");
```

### C / FFI

```c
#include "fastmarkdown.h"

// HTML output
char* html = fastmarkdown_to_html("# Hello **world**");
printf("%s\n", html);
fastmarkdown_free_string(html);

// Span output for native rendering
FastMarkdownSpanOutput spans = fastmarkdown_to_spans("# Hello **world**");
for (uint32_t i = 0; i < spans.span_count; i++) {
    FastMarkdownSpan s = spans.spans[i];
    // s.offset, s.length, s.flags, s.block_type
}
fastmarkdown_free_spans(&spans);

// Streaming
FastMarkdownStream* stream = fastmarkdown_stream_create(1, 0, false);  // Safe HTML, Literal math
fastmarkdown_stream_append(stream, "# Hello");
fastmarkdown_stream_append(stream, " **world**");
fastmarkdown_stream_destroy(stream);
```

### iOS (Swift)

```swift
// Using the FFI → spans → NSAttributedString pattern
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
val text = String(output.text, 0, output.text_len.toInt())
val spannable = SpannableString(text)
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

## API Reference

### Rust API

```rust
// One-shot rendering
pub fn to_html(md: &str) -> String;
pub fn to_html_with_options(md: &str, options: &MarkdownOptions) -> String;
pub fn to_spans(md: &str) -> SpanOutput;
pub fn to_spans_with_options(md: &str, options: &MarkdownOptions) -> SpanOutput;
pub fn to_plain_text(md: &str) -> String;
pub fn version() -> &'static str;
pub unsafe fn free_spans(output: SpanOutput);

// Streaming
pub struct StreamRenderer { ... }
impl StreamRenderer {
    pub fn new(options: MarkdownOptions) -> Self;
    pub fn append(&mut self, chunk: &str) -> SpanOutput;
    pub fn finish(&mut self) -> Option<SpanOutput>;
    pub fn reset(&mut self);
}

// Configuration
pub struct MarkdownOptions {
    pub html_mode: HtmlMode,        // Strip | Safe (default) | AllowAll
    pub math_mode: MathMode,        // Literal (default) | Detect
    pub smart_punctuation: bool,
    pub code_class_prefix: Option<String>,
    pub base_url: Option<String>,
}

pub struct ThemeConfig {            // Platform-agnostic styling
    pub body_font_name: *const c_char,
    pub body_font_size: f32,
    pub monospace_font_name: *const c_char,
    pub monospace_font_size: f32,
    pub heading_font_sizes: [f32; 6],
    pub text_color: u32,            // 0xRRGGBBAA
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

### C FFI

```c
// Core
char* fastmarkdown_to_html(const char* markdown);
char* fastmarkdown_to_html_ex(const char* markdown, uint32_t html_mode, uint32_t math_mode, bool smart_punct);
FastMarkdownSpanOutput fastmarkdown_to_spans(const char* markdown);
FastMarkdownSpanOutput fastmarkdown_to_spans_ex(const char* markdown, uint32_t html_mode, uint32_t math_mode, bool smart_punct);
char* fastmarkdown_to_plain_text(const char* markdown);
void  fastmarkdown_free_string(char* s);
void  fastmarkdown_free_spans(FastMarkdownSpanOutput* output);
const char* fastmarkdown_version(void);

// Streaming
FastMarkdownStream* fastmarkdown_stream_create(uint32_t html_mode, uint32_t math_mode, bool smart_punct);
FastMarkdownSpanOutput fastmarkdown_stream_append(FastMarkdownStream* stream, const char* chunk);
FastMarkdownSpanOutput fastmarkdown_stream_finish(FastMarkdownStream* stream);
FastMarkdownSpanDeltaList fastmarkdown_stream_get_delta(FastMarkdownStream* stream, const char* chunk);
void fastmarkdown_free_deltas(FastMarkdownSpanDeltaList* list);
void fastmarkdown_stream_reset(FastMarkdownStream* stream);
void fastmarkdown_stream_destroy(FastMarkdownStream* stream);

// Theme
FastMarkdownTheme fastmarkdown_theme_default(void);
```

See the [design document](design.md) for the full architecture, Span binary format specification, and platform integration guides.

---

## Architecture

```
Markdown String
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

## Build from Source

```bash
# Clone
git clone https://github.com/kandada/fastshell
cd fastshell/fastmarkdown

# Build (library)
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench

# Cross-compile for iOS
cargo build --release --target aarch64-apple-ios

# Cross-compile for Android
cargo build --release --target aarch64-linux-android
```

---

## In the Wild

fastmarkdown powers markdown rendering in:

- [**AACode**](https://github.com/kandada/aacode) — the mobile AI coding agent for iOS

---

## License

Copyright 2025 xiefujin (490021684@qq.com)

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the full text.
