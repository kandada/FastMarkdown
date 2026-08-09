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
- **可选代码高亮** — `highlight` feature flag + `MarkdownOptions::highlight = true`。输出 15 类 token 流至 extra_data (kind=4)，宿主按类型映射颜色。不启用时零成本
- **流式 API** — 原生 token 级增量追加，适配实时 AI 输出
- **双路输出** — HTML 字符串（WebView/导出）+ 结构化 Span 数组（原生渲染）
- **文件预览块结构** — `block_seq`、`block_depth`、表格 metadata，存储于自描述 `extra_data` 二进制格式
- **extradata 自描述格式** — `[kind: u8, len: u16 LE, data]` 分块，支持 link URL、image URL、代码语言、表格 metadata、代码高亮 token
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

### Chat / AI 消息

fastmarkdown 为聊天消息渲染优化：一条消息 → flat span 数组 → 平台适配器 → 一个 `NSAttributedString`/`SpannableString` → 一个 `UITextView`。消除 View 爆炸，渲染性能稳定可预测。

### 文件预览 / 文档查看

Span 输出携带 `block_seq`、`block_depth` 以及自描述 `extra_data`：

| 字段 | 大小 | 用途 |
|------|------|------|
| `block_seq` | u16 | 每块唯一序号；按 seq 分组 → N 个 block，适配 LazyVStack 懒加载 |
| `block_depth` | u8 | 列表/引用嵌套层级；`depth × 每级缩进` → UI 缩进 |
| `block_type` | u8 | 段落、标题、代码块、表格单元格、列表项等 |
| `extra_data` | bytes | `[kind:u8, len:u16 LE, data]` 分块，包含 link URL、代码语言、表格 metadata（列数 + 对齐 + **每列最大字符宽度**） |

**表格 metadata chunk** (kind=3)：`[col_count: u8, (align: u8, char_width: u16 LE)*N]` — `char_width` 为每列所有行中的最大字符数，宿主可直接用于列宽估算，无需逐 cell 测量文本。

**消费者使用指南：**

```rust
// 1. 重建 block 结构
let mut blocks: Vec<Vec<&Span>> = vec![];
let mut current_seq = spans[0].block_seq;
let mut current: Vec<&Span> = vec![];
for s in spans {
    if s.block_seq != current_seq { blocks.push(current); current = vec![]; current_seq = s.block_seq; }
    current.push(s);
}

// 2. 解析 extra_data 分块
let mut pos = 0;
while pos + 3 <= extra.len() {
    let kind = extra[pos];
    let len = u16::from_le_bytes([extra[pos + 1], extra[pos + 2]]) as usize;
    let data = &extra[pos + 3..pos + 3 + len];
    match kind {
        2 => println!("代码语言: {}", str::from_utf8(data).unwrap()),
        3 => println!("表格: {} 列, 对齐: {:?}", data[0], &data[1..]),
        _ => {}
    }
    pos += 3 + len;
}

// 3. 重建表格行
let col_count = table_meta[0] as usize;
let data_cells: Vec<_> = spans.iter().filter(|s| s.block_type == BLOCK_TABLE_CELL).collect();
for row in data_cells.chunks(col_count) { /* 渲染行 */ }
```

参见 `tests/block_structure.rs` 和 `tests/consumer_verify.rs` 中的完整示例。

fastmarkdown 已用于：

- [**AACode**](https://github.com/kandada/aacode) — 移动端 AI 编程助手 (iOS)

---

## 代码高亮（可选）

通过 `features = ["highlight"]` 启用，`MarkdownOptions::highlight = true` 开启。

**Token 类型**（syntect scope 映射为 15 种通用类型）：

| id | 类型 | id | 类型 | id | 类型 |
|----|------|----|------|----|------|
| 0 | 其他 | 5 | 函数 | 10 | 常量 |
| 1 | 关键字 | 6 | 类型 | 11 | 内置 |
| 2 | 字符串 | 7 | 运算符 | 12 | 实体 |
| 3 | 注释 | 8 | 标点 | 13 | 标记 |
| 4 | 数字 | 9 | 变量 | 14 | 正则 |

**宿主消费**（Swift 示例）：
```swift
// 读取 extra_data kind=4 chunk（紧跟 kind=2 语言 chunk）
let tokens = parseHighlightChunk(extra)
var pos = 0
for (type, len) in tokens {
    let range = NSRange(location: pos, length: Int(len))
    attrString.addAttribute(.foregroundColor, value: highlightColors[Int(type)], range: range)
    pos += Int(len)
}
```

**零成本关闭**：`highlight` 是编译期 feature flag。不启用时无 syntect 依赖、无体积膨胀、无运行时开销。

支持语言：Sublime Text 语法定义中的全部语言（100+，包括 rust、python、swift、kotlin、bash、javascript、c、c++、java、go 等）。

### 集成清单

使用代码高亮需要完成三步：

1. **启用编译期 feature**
   ```
   cargo build --features highlight --release
   ```
   不加 `--features highlight` 则不编译 syntect，不产生高亮 chunk。体积影响：iOS `.a` 约 +1.2MB。

2. **启用运行时选项**
   ```rust
   let mut opts = MarkdownOptions::default();
   opts.highlight = true;  // 默认 false
   let output = to_spans_with_options(md, &opts);
   ```
   或通过 FFI：调用 `fastmarkdown_to_spans_ex` 时传入 `highlight=1`。

3. **宿主消费高亮 chunk**
   `extra_data` 中包含 `kind=4` chunk（每个代码块紧跟 `kind=2` 语言 chunk 之后）。解析 chunk 格式并按类型着色。参见上方 Swift 示例。

---

## 开源协议

Copyright 2025 xiefujin (490021684@qq.com)

基于 Apache License, Version 2.0 开源。详见 [LICENSE](LICENSE)。
