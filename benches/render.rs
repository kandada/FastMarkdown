// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use criterion::{black_box, Criterion};
use fastmarkdown::{to_html, to_spans, to_plain_text, MarkdownOptions, MathMode, HtmlMode};

fn bench_html_basic(c: &mut Criterion) {
    let md = "# Hello World\n\nThis is a **bold** and *italic* paragraph with `code`.\n\n- list item 1\n- list item 2\n\n```rust\nfn main() {}\n```";
    c.bench_function("html_basic", |b| {
        b.iter(|| to_html(black_box(md)))
    });
}

fn bench_html_table(c: &mut Criterion) {
    let md = "| Col A | Col B | Col C |\n|-------|-------|-------|\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |\n| 7 | 8 | 9 |";
    c.bench_function("html_table", |b| {
        b.iter(|| to_html(black_box(md)))
    });
}

fn bench_html_large(c: &mut Criterion) {
    let md = "# Section 1\n\nLorem ipsum dolor sit amet, consectetur adipiscing elit. **Bold text** and *italic text* and `code spans`.\n\n## Section 2\n\n1. First ordered item\n2. Second ordered item\n3. Third ordered item\n\n> This is a blockquote with **bold** inside it.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n```python\ndef hello():\n    print('Hello, World!')\n```\n\n### Section 3\n\n- [x] Completed task\n- [ ] Pending task\n\nFinal paragraph with a [link](https://example.com) and an image ![alt](https://example.com/img.png).\n".repeat(10);
    c.bench_function("html_large_10x", |b| {
        b.iter(|| to_html(black_box(&md)))
    });
}

fn bench_spans_basic(c: &mut Criterion) {
    let md = "# Hello World\n\nThis is a **bold** and *italic* paragraph with `code`.";
    c.bench_function("spans_basic", |b| {
        b.iter(|| {
            let output = to_spans(black_box(md));
            unsafe { fastmarkdown::free_spans(output); }
        })
    });
}

fn bench_spans_table(c: &mut Criterion) {
    let md = "| Col A | Col B | Col C |\n|-------|-------|-------|\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |\n| 7 | 8 | 9 |";
    c.bench_function("spans_table", |b| {
        b.iter(|| {
            let output = to_spans(black_box(md));
            unsafe { fastmarkdown::free_spans(output); }
        })
    });
}

fn bench_plain_text(c: &mut Criterion) {
    let md = "# Hello\n\n**bold** and *italic* with [links](http://a.com).";
    c.bench_function("plain_text", |b| {
        b.iter(|| to_plain_text(black_box(md)))
    });
}

fn bench_math_detection(c: &mut Criterion) {
    use fastmarkdown::math::detect_math_regions;
    let md = "The equation is $x^2 + y^2 = z^2$ and also $$\\int_0^1 x dx$$. Price is $5.99.";
    c.bench_function("math_detection", |b| {
        b.iter(|| detect_math_regions(black_box(md)))
    });
}

fn bench_html_with_math(c: &mut Criterion) {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;
    let md = "The formula is $x^2 + y^2 = z^2$ and this is **bold**.";
    c.bench_function("html_math_detect", |b| {
        b.iter(|| fastmarkdown::to_html_with_options(black_box(md), &opts))
    });
}

fn bench_streaming(c: &mut Criterion) {
    use fastmarkdown::StreamRenderer;
    let chunks: Vec<&str> = vec![
        "# He", "llo\n\n", "This is **bo", "ld** and *it", "alic*\n\n```", "rust\nfn m", "ain() {}\n```",
    ];
    c.bench_function("stream_append", |b| {
        b.iter(|| {
            let mut stream = StreamRenderer::new(MarkdownOptions::default());
            for chunk in &chunks {
                stream.append(black_box(chunk));
            }
        })
    });
}

criterion::criterion_group!(
    benches,
    bench_html_basic,
    bench_html_table,
    bench_html_large,
    bench_spans_basic,
    bench_spans_table,
    bench_plain_text,
    bench_math_detection,
    bench_html_with_math,
    bench_streaming,
);
criterion::criterion_main!(benches);
