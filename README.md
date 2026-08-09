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
- **Optional code highlighting** — `highlight` feature flag + `MarkdownOptions::highlight = true`. Outputs 15-type token stream in `extra_data` (kind=4). Host maps types to colors. Zero-cost when disabled
- **Streaming API** — native token-level append for real-time AI output
- **Dual output** — HTML string (WebView/export) and structured Span array (native rendering)
- **Block structure for document viewing** — `block_seq` (unique per block, for LazyVStack splitting), `block_depth` (nesting level for list/quote indent), table metadata (column count + per-column alignment) in a self-describing `extra_data` binary format
- **Self-describing extra_data** — `[kind: u8, len: u16 LE, data]` chunks for link URLs, image URLs, code languages, table metadata, and code highlighting tokens. Consumers parse without external indexes
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

## Use Cases

### Chat / AI Messages

fastmarkdown is optimized for chat rendering: parse markdown → flat span array → platform adapter → one `NSAttributedString`/`SpannableString` → one `UITextView`. This eliminates View explosion and gives fast, predictable performance.

### File Preview / Document Viewing

Span output carries `block_seq`, `block_depth`, and self-describing `extra_data`:

| field | size | purpose |
|-------|------|---------|
| `block_seq` | u16 | unique per block; group by seq → N blocks for LazyVStack / lazy loading |
| `block_depth` | u8 | list/quote nesting level; `depth × indentPerLevel` → UI indent |
| `block_type` | u8 | paragraph, heading, code_block, table_cell, list_item, etc. |
| `extra_data` | bytes | `[kind:u8, len:u16 LE, data]` chunks for link URLs, code languages, table metadata (column count + alignment + **per-column max char width**) |

**Table metadata chunk** (kind=3): `[col_count: u8, (align: u8, char_width: u16 LE)*N]` — `char_width` is the maximum character count per column across all rows, useful for host-side column width estimation without measuring text. |

**Consumer guide:**

```rust
// 1. Reconstruct blocks
let mut blocks: Vec<Vec<&Span>> = vec![];
let mut current_seq = spans[0].block_seq;
let mut current: Vec<&Span> = vec![];
for s in spans {
    if s.block_seq != current_seq { blocks.push(current); current = vec![]; current_seq = s.block_seq; }
    current.push(s);
}

// 2. Parse extra_data chunks
let mut pos = 0;
while pos + 3 <= extra.len() {
    let kind = extra[pos];
    let len = u16::from_le_bytes([extra[pos + 1], extra[pos + 2]]) as usize;
    let data = &extra[pos + 3..pos + 3 + len];
    match kind {
        2 => println!("Code language: {}", str::from_utf8(data).unwrap()),
        3 => println!("Table: {} columns, alignments: {:?}", data[0], &data[1..]),
        _ => {}
    }
    pos += 3 + len;
}

// 3. Rebuild table rows
let col_count = table_meta[0] as usize;
let data_cells: Vec<_> = spans.iter().filter(|s| s.block_type == BLOCK_TABLE_CELL).collect();
for row in data_cells.chunks(col_count) { /* render row */ }
```

See `tests/block_structure.rs` and `tests/consumer_verify.rs` for complete working examples.

---

## Code Highlighting (optional)

Enable via `features = ["highlight"]` in Cargo.toml and set `MarkdownOptions::highlight = true`.

**Token types** (15 generic types mapped from syntect scopes):

| id | type | id | type | id | type |
|----|------|----|------|----|------|
| 0 | other | 5 | function | 10 | constant |
| 1 | keyword | 6 | type | 11 | builtin |
| 2 | string | 7 | operator | 12 | entity |
| 3 | comment | 8 | punctuation | 13 | markup |
| 4 | number | 9 | variable | 14 | regex |

**Host consumption** (Swift example):
```swift
// Read extra_data chunk kind=4 after kind=2 (language)
let tokens = parseHighlightChunk(extra)
var pos = 0
for (type, len) in tokens {
    let range = NSRange(location: pos, length: Int(len))
    attrString.addAttribute(.foregroundColor, value: highlightColors[Int(type)], range: range)
    pos += Int(len)
}
```

**Zero-cost when disabled**: `highlight` is a compile-time feature flag. Without it, no syntect dependency, no binary size increase, no runtime overhead.

Supported languages: all in Sublime Text syntax definitions (100+ languages, including rust, python, swift, kotlin, bash, javascript, c, c++, java, go, etc.).

### Integration Checklist

When integrating code highlighting, ensure all three steps are done:

1. **Enable compile-time feature**
   ```
   cargo build --features highlight --release
   ```
   Without `--features highlight`, syntect is not compiled and no highlight chunk is produced. Binary size impact: ~1.2MB (iOS `.a`).

2. **Enable runtime option**
   ```rust
   let mut opts = MarkdownOptions::default();
   opts.highlight = true;  // default is false
   let output = to_spans_with_options(md, &opts);
   ```
   Or via FFI: pass `highlight=1` to `fastmarkdown_to_spans_ex`.

3. **Consume the highlight chunk on the host (Swift/Kotlin)**
   The `extra_data` contains `kind=4` chunks (one per code block, immediately after the `kind=2` language chunk). Parse the chunk format and apply colors to your attributed text. See the Swift example above.

---

## License

Copyright 2025 xiefujin (490021684@qq.com)

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the full text.
