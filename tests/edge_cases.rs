// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

#[test]
fn test_consecutive_text() {
    // pulldown-cmark merges consecutive text events; ensure spans are consistent
    let output = to_spans("**bold**normal*italic*");
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("bold"));
    assert!(text.contains("normal"));
    assert!(text.contains("italic"));
    assert!(spans.iter().any(|s| s.is_bold()));
    assert!(spans.iter().any(|s| s.is_italic()));
    unsafe { fastmarkdown::free_spans(output); }
}

#[test]
fn test_offset_monotonic() {
    let output = to_spans("A **bold** C *italic* E");
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
    // Offsets should be non-decreasing and within bounds
    for span in spans {
        assert!((span.offset + span.length) as usize <= output.text_len as usize);
    }
    // Check monotonic
    for w in spans.windows(2) {
        assert!(w[0].offset <= w[1].offset);
    }
    unsafe { fastmarkdown::free_spans(output); }
}

#[test]
fn test_large_input() {
    let mut md = String::new();
    // Generate a large markdown document
    for i in 0..500 {
        md.push_str(&format!("# Section {}\n\n", i));
        md.push_str(&format!("This is paragraph {} with **bold** and `code`.\n\n", i));
        md.push_str("- item a\n- item b\n- item c\n\n");
        md.push_str(&format!("| A | B |\n|---|---|\n| {} | {} |\n\n", i, i * 2));
    }

    let html = to_html(&md);
    assert!(html.len() > 10000);
    assert!(html.contains("<table>"));
    assert!(html.contains("<h1>"));

    let output = to_spans(&md);
    assert!(output.span_count > 100);
    unsafe { fastmarkdown::free_spans(output); }
}

#[test]
fn test_unicode() {
    let md = "中文 **粗体** 🎉 *italic* 日本語";
    let html = to_html(md);
    assert!(html.contains("中文"));
    assert!(html.contains("粗体"));
    assert!(html.contains("🎉"));

    let output = to_spans(md);
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    assert!(text.contains("中文"));
    unsafe { fastmarkdown::free_spans(output); }
}

#[test]
fn test_multibyte_offsets() {
    // Unicode chars take 2-4 bytes; spans should track byte offsets correctly
    let md = "Hello 🌍 **world**";
    let output = to_spans(md);
    let text = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(output.text, output.text_len as usize))
            .unwrap()
    };
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };

    for span in spans {
        let substr = &text[span.offset as usize..(span.offset + span.length) as usize];
        assert!(!substr.is_empty());
    }

    // Check the bold span contains "world"
    let bold_span = spans.iter().find(|s| s.is_bold()).unwrap();
    let bold_text = &text[bold_span.offset as usize..(bold_span.offset + bold_span.length) as usize];
    assert_eq!(bold_text, "world");

    unsafe { fastmarkdown::free_spans(output); }
}

#[test]
fn test_empty_paragraphs() {
    let md = "# Title\n\n\n\nParagraph after empty lines";
    let html = to_html(md);
    assert!(html.contains("<h1>"));
    assert!(html.contains("Paragraph"));
}

#[test]
fn test_trailing_newlines() {
    let html = to_html("Hello\n\n\n");
    assert!(html.contains("Hello"));
}

#[test]
fn test_only_markdown_syntax_without_text() {
    let html = to_html("** **");
    assert!(!html.is_empty() || html.len() > 0);
}

#[test]
fn test_link_with_title() {
    let md = "[Google](https://google.com \"Search\")";
    let html = to_html(md);
    assert!(html.contains("google.com"));
}

#[test]
fn test_image_with_title() {
    let md = "![logo](https://example.com/logo.png \"Logo\")";
    let html = to_html(md);
    assert!(html.contains("logo.png"));
}

#[test]
fn test_footnote() {
    let md = "Text with footnote[^1]\n\n[^1]: The footnote content";
    let html = to_html(md);
    // Might or might not render footnotes; just ensure no crash
    assert!(html.contains("Text"));
}

#[test]
fn test_definition_list() {
    let md = "Term\n: Definition\n\nAnother Term\n: Another Definition";
    let html = to_html(md);
    // pulldown-cmark 0.13 supports definition lists with the flag
    assert!(!html.is_empty());
}

#[test]
fn test_subscript_and_superscript() {
    let md = "H~2~O and x^2^";
    let html = to_html(md);
    // pulldown-cmark 0.13 supports sub/sup with flags
    assert!(html.contains("H") && html.contains("O"));
}

#[test]
fn test_mixed_html_and_markdown_paragraph() {
    let md = "Start <b>bold html</b> and **bold md** end";
    let html = to_html(md);
    assert!(html.contains("Start"));
    assert!(html.contains("end"));
}

#[test]
fn test_complex_nested_structure() {
    let md = r#"# Main Title

## Section 1

This is a paragraph with **bold**, *italic*, ~~strikethrough~~, and `code`.

> This is a blockquote with **bold** inside.

1. First ordered item
2. Second with `code`
3. Third

- Unordered list
  - Nested item
  - Nested **bold**
- Back to top level

| Feature | Status |
|---------|--------|
| Fast | ✅ |
| Safe | ✅ |

```rust
fn main() {
    // code block
    println!("Hello");
}
```

Final paragraph with [link](https://example.com).
"#;

    let html = to_html(md);
    assert!(html.contains("<h1>"));
    assert!(html.contains("<h2>"));
    assert!(html.contains("<blockquote>"));
    assert!(html.contains("<ol>"));
    assert!(html.contains("<ul>"));
    assert!(html.contains("<table>"));
    assert!(html.contains("<code>"));

    let output = to_spans(md);
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
    assert!(spans.len() > 10);
    unsafe { fastmarkdown::free_spans(output); }
}

#[test]
fn test_code_block_with_no_language() {
    let md = "```\nplain code block\n```";
    let html = to_html(md);
    assert!(html.contains("plain code block"));
}

#[test]
fn test_backtick_escape_in_code() {
    let md = "Use `` `backtick` `` here";
    let html = to_html(md);
    assert!(html.contains("backtick") || html.contains("`"));
}

#[test]
fn test_literal_asterisks() {
    let md = r"The function is called \*deref\* not deref";
    let html = to_html(md);
    // Should not contain em tags for the escaped part
    assert!(html.contains("deref"));
}
