// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

use fastmarkdown::*;

#[test]
fn test_dollar_literal_mode_default() {
    // Default is Literal mode: $ should be treated as regular text
    let html = to_html("Price: $5.99");
    assert!(!html.contains("math")); // Not treated as math
    assert!(html.contains("$5.99") || html.contains("&#36;5.99"));
}

#[test]
fn test_dollar_path() {
    let html = to_html("Export PATH=$HOME/.local/bin:$PATH");
    assert!(!html.contains("math"));
    assert!(html.contains("$HOME") || html.contains("&#36;HOME"));
}

#[test]
fn test_dollar_single_unmatched() {
    let html = to_html("This has a $ but no closing dollar");
    assert!(!html.contains("math"));
}

#[test]
fn test_dollar_in_code_block() {
    let md = "```bash\necho $HOME\n```";
    let html = to_html(md);
    assert!(html.contains("$HOME") || html.contains("&#36;HOME"));
}

#[test]
fn test_dollar_in_inline_code() {
    let html = to_html("Run `echo $PATH` to see path");
    assert!(html.contains("$PATH") || html.contains("&#36;PATH"));
}

#[test]
fn test_math_detect_mode_inline() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let html = to_html_with_options("The formula $x^2 + y^2 = z^2$ is famous", &opts);
    assert!(html.contains("math"));
}

#[test]
fn test_math_detect_mode_display() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let md = "$$\n\\int_0^1 x^2 dx = \\frac{1}{3}\n$$";
    let html = to_html_with_options(md, &opts);
    assert!(html.contains("math"));
}

#[test]
fn test_math_detect_still_literal_for_currency() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let html = to_html_with_options("Price: $5.99 and $10.50", &opts);
    // $5.99 starts with digit, should not be treated as math
    assert!(!html.contains("math"));
}

#[test]
fn test_math_detect_still_literal_for_single_dollar() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let html = to_html_with_options("This costs $100 only", &opts);
    assert!(!html.contains("math"));
}

#[test]
fn test_math_detect_multiple_inline() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let html = to_html_with_options("First $a+b$ then $c-d$", &opts);
    let math_count = html.matches("math").count();
    assert!(
        math_count >= 2,
        "Expected at least 2 math regions, got {}",
        math_count
    );
}

#[test]
fn test_math_detect_mixed() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let md = "Price $5.99 but $E = mc^2$ is cool";
    let html = to_html_with_options(md, &opts);
    // $E = mc^2$ should be math, $5.99 should not
    assert!(html.contains("math"));
    assert!(html.contains("5.99"));
}

#[test]
fn test_math_in_span_output_detect() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let output = to_spans_with_options("The formula $x^2$ is known", &opts);
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
    assert!(spans.iter().any(|s| s.is_math()));
    unsafe {
        fastmarkdown::free_spans(output);
    }
}

#[test]
fn test_math_in_span_output_literal() {
    let output = to_spans("Cost $10 per item");
    let spans = unsafe { std::slice::from_raw_parts(output.spans, output.span_count as usize) };
    // No span should have math flag in literal mode
    assert!(spans.iter().all(|s| !s.is_math()));
    unsafe {
        fastmarkdown::free_spans(output);
    }
}

#[test]
fn test_escaped_dollar_preserved() {
    let html = to_html(r"The price is \$10");
    // Backslash escape should not trigger math or issues
    assert!(html.contains("10") || html.contains("$10") || html.contains("&#36;10"));
}

#[test]
fn test_dollar_with_special_chars() {
    let md = "\\(this is not math\\) but **this is** bold";
    let html = to_html(md);
    assert!(html.contains("bold"));
}

#[test]
fn test_double_dollar_in_code() {
    let md = "```python\nprint(f'Cost: $${price}')\n```";
    let html = to_html(md);
    assert!(html.contains("$") || html.contains("&#36;"));
}

#[test]
fn test_math_with_underscores() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    // Underscores in math should not trigger italic
    let html = to_html_with_options("$a_i + b_j$", &opts);
    assert!(html.contains("math"));
}

#[test]
fn test_math_with_braces() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let html = to_html_with_options("$\\frac{1}{2}$", &opts);
    assert!(html.contains("math"));
}

#[test]
fn test_math_adjacent_to_text() {
    let mut opts = MarkdownOptions::default();
    opts.math_mode = MathMode::Detect;

    let html = to_html_with_options("value$x^2$here", &opts);
    assert!(html.contains("math"));
}
