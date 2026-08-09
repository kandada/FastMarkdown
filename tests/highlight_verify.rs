// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

#[cfg(feature = "highlight")]
mod tests {
    use fastmarkdown::*;

    fn get_extra(output: &SpanOutput) -> &[u8] {
        unsafe { std::slice::from_raw_parts(output.extra_data, output.extra_data_len as usize) }
    }

    fn parse_chunks(extra: &[u8]) -> Vec<(u8, Vec<u8>)> {
        let mut chunks = Vec::new();
        let mut pos = 0;
        while pos + 3 <= extra.len() {
            let kind = extra[pos];
            let len = u16::from_le_bytes([extra[pos + 1], extra[pos + 2]]) as usize;
            pos += 3;
            if pos + len <= extra.len() {
                chunks.push((kind, extra[pos..pos + len].to_vec()));
                pos += len;
            } else {
                break;
            }
        }
        chunks
    }

    fn parse_highlight_data(data: &[u8]) -> Vec<(u8, u16)> {
        if data.len() < 2 {
            return vec![];
        }
        let count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let mut tokens = Vec::with_capacity(count);
        let mut off = 2;
        for _ in 0..count {
            if off + 3 > data.len() {
                break;
            }
            let tt = data[off];
            let len = u16::from_le_bytes([data[off + 1], data[off + 2]]);
            tokens.push((tt, len));
            off += 3;
        }
        tokens
    }

    fn free(output: SpanOutput) {
        unsafe { fastmarkdown::free_spans(output); }
    }

    #[test]
    fn test_highlight_produces_chunk() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```rust\nfn main() {}\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        let has_highlight = chunks.iter().any(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT);
        assert!(has_highlight, "Should have highlight chunk for Rust code block");
        free(output);
    }

    #[test]
    fn test_highlight_disabled_no_chunk() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = false;

        let output = to_spans_with_options("```rust\nfn main() {}\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        let has_highlight = chunks.iter().any(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT);
        assert!(!has_highlight, "Should NOT have highlight chunk when disabled");
        free(output);
    }

    #[test]
    fn test_highlight_token_length_sum_matches_code() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```rust\nlet x = 42;\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        let hl = chunks.iter().find(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT)
            .expect("Should have highlight chunk");
        let tokens = parse_highlight_data(&hl.1);
        let total: u32 = tokens.iter().map(|(_, len)| *len as u32).sum();
        let code_text: &str = "let x = 42;\n";
        assert_eq!(total as usize, code_text.len());
        free(output);
    }

    #[test]
    fn test_highlight_multiple_types() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```rust\n// comment\nlet x = \"hello\";\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        let hl = chunks.iter().find(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT)
            .expect("Should have highlight chunk");
        let tokens = parse_highlight_data(&hl.1);
        let types: std::collections::HashSet<u8> = tokens.iter().map(|(t, _)| *t).collect();

        assert!(types.len() >= 3, "Expected >=3 distinct types, got {:?}", types);
        free(output);
    }

    #[test]
    fn test_highlight_python() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```python\ndef foo():\n    return 42\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        assert!(chunks.iter().any(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT),
            "Python should produce highlight tokens");
        free(output);
    }

    #[test]
    fn test_highlight_bash() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```bash\necho hello\nls -la\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        assert!(chunks.iter().any(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT),
            "Bash should produce highlight tokens");
        free(output);
    }

    #[test]
    fn test_highlight_unknown_language_no_chunk() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```xyz_notalang\nsome text\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        let has_highlight = chunks.iter().any(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT);
        assert!(!has_highlight, "Unknown language should not produce highlight chunk");
        free(output);
    }

    #[test]
    fn test_highlight_no_language_block() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```\nplain text block\n```", &opts);
        let extra = get_extra(&output);
        let _chunks = parse_chunks(extra);
        // Should not crash
        free(output);
    }

    #[test]
    fn test_highlight_chunk_after_language_chunk() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```rust\nlet x = 1;\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        let lang_idx = chunks.iter().position(|(k, _)| *k == EXTRA_KIND_CODE_LANGUAGE);
        let hl_idx = chunks.iter().position(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT);

        if let (Some(li), Some(hi)) = (lang_idx, hl_idx) {
            assert_eq!(hi, li + 1, "Highlight chunk should immediately follow language chunk");
        }
        free(output);
    }

    #[test]
    fn test_highlight_empty_code_block() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```rust\n```", &opts);
        // Empty code block should not crash
        free(output);
    }

    #[test]
    fn test_highlight_code_with_multiple_same_type() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```rust\nlet a = 1;\nlet b = 2;\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        let hl = chunks.iter().find(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT);
        if let Some(hl) = hl {
            let tokens = parse_highlight_data(&hl.1);
            let total: u32 = tokens.iter().map(|(_, len)| *len as u32).sum();
            assert_eq!(total as usize, "let a = 1;\nlet b = 2;\n".len());
        }
        free(output);
    }

    #[test]
    fn test_highlight_chunk_format_valid() {
        let mut opts = MarkdownOptions::default();
        opts.highlight = true;

        let output = to_spans_with_options("```rust\n42\n```", &opts);
        let extra = get_extra(&output);
        let chunks = parse_chunks(extra);

        let hl = chunks.iter().find(|(k, _)| *k == EXTRA_KIND_CODE_HIGHLIGHT);
        if let Some(hl) = hl {
            // Verify token format: [count:u16][(type:u8, len:u16)*]
            assert!(hl.1.len() >= 2, "Chunk data must have at least count field");
            let tokens = parse_highlight_data(&hl.1);
            assert!(!tokens.is_empty(), "Should have tokens");
        }
        free(output);
    }
}
