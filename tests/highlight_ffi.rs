// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

#[cfg(feature = "highlight")]
use fastmarkdown::ffi::*;
#[cfg(feature = "highlight")]
use fastmarkdown::highlight::highlight_code;
#[cfg(feature = "highlight")]
use fastmarkdown::*;

// ============================================================
//  Helper: safe highlight via Rust API (used for cross-validation)
// ============================================================

#[cfg(feature = "highlight")]
fn safe_highlight(code: &str, lang: &str) -> Option<(Vec<u8>, u32)> {
    let raw = highlight_code_serialized(code, lang)?;
    let data_len = raw.len() as u32;
    Some((raw, data_len))
}

// ============================================================
//  Helper: call FFI function and wrap with auto-free guard
// ============================================================

#[cfg(feature = "highlight")]
fn call_highlight(code: &str, lang: &str) -> (Vec<u8>, u32) {
    use std::ffi::CString;
    let c_code = CString::new(code).unwrap();
    let c_lang = CString::new(lang).unwrap();
    let output = fastmarkdown_highlight(c_code.as_ptr(), c_lang.as_ptr());

    assert!(
        !output.data.is_null(),
        "FFI highlight should succeed for lang={}, code={:?}",
        lang,
        code
    );
    assert!(
        output.data_len >= 2,
        "data_len should be at least 2 bytes (count field)"
    );

    let data =
        unsafe { std::slice::from_raw_parts(output.data, output.data_len as usize).to_vec() };
    let data_len = output.data_len;

    // Free via FFI
    let mut out_copy = output;
    fastmarkdown_free_highlight(&mut out_copy);

    (data, data_len)
}

// ============================================================
//  Helper: decode token binary data
// ============================================================

#[cfg(feature = "highlight")]
fn decode_tokens(data: &[u8]) -> Vec<(u8, u16)> {
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

// ============================================================
//  Tests
// ============================================================

#[cfg(feature = "highlight")]
mod ffi_tests {
    use super::*;

    // --- Basic functionality ---

    #[test]
    fn test_highlight_rust_basic() {
        let code = "fn main() {\n    println!(\"Hello\");\n}\n";
        let (data, data_len) = call_highlight(code, "rust");

        assert_eq!(data.len() as u32, data_len);
        let tokens = decode_tokens(&data);
        assert!(!tokens.is_empty(), "Should produce tokens for Rust code");

        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len(), "Token lengths must sum to code length");
    }

    #[test]
    fn test_highlight_python() {
        let code = "def hello():\n    return 'world'\n";
        let (data, _) = call_highlight(code, "python");
        let tokens = decode_tokens(&data);
        assert!(!tokens.is_empty(), "Should produce tokens for Python code");
        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len());
    }

    #[test]
    fn test_highlight_bash() {
        let code = "echo \"hello world\"\nls -la\n";
        let (data, _) = call_highlight(code, "bash");
        let tokens = decode_tokens(&data);
        assert!(!tokens.is_empty(), "Should produce tokens for Bash code");
        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len());
    }

    #[test]
    fn test_highlight_javascript() {
        let code = "const x = 42;\nfunction foo() { return x; }\n";
        let (data, _) = call_highlight(code, "javascript");
        let tokens = decode_tokens(&data);
        assert!(!tokens.is_empty(), "Should produce tokens for JS code");
    }

    #[test]
    fn test_highlight_c() {
        let code = "#include <stdio.h>\nint main() { return 0; }\n";
        let (data, _) = call_highlight(code, "c");
        let tokens = decode_tokens(&data);
        assert!(!tokens.is_empty(), "Should produce tokens for C code");
    }

    #[test]
    fn test_highlight_go() {
        let code = "package main\nimport \"fmt\"\nfunc main() { fmt.Println(\"hi\") }\n";
        let (data, _) = call_highlight(code, "go");
        let tokens = decode_tokens(&data);
        assert!(!tokens.is_empty(), "Should produce tokens for Go code");
    }

    // --- Token length verification ---

    #[test]
    fn test_token_length_sum_equals_code() {
        let cases = vec![
            ("rust", "let x = 42;"),
            ("python", "x = 42"),
            ("javascript", "var x = 42;"),
            ("bash", "x=42"),
            ("c", "int x = 42;"),
        ];
        for (lang, code) in cases {
            let (data, _) = call_highlight(code, lang);
            let tokens = decode_tokens(&data);
            let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
            assert_eq!(
                total,
                code.len(),
                "Length mismatch for lang={}, code={:?}",
                lang,
                code
            );
        }
    }

    // --- Multiple token types ---

    #[test]
    fn test_multiple_token_types() {
        let code = "// comment\nlet x = \"hello\";\n";
        let (data, _) = call_highlight(code, "rust");
        let tokens = decode_tokens(&data);
        let types: std::collections::HashSet<u8> = tokens.iter().map(|(t, _)| *t).collect();
        assert!(
            types.len() >= 3,
            "Expected >=3 distinct token types, got {:?}",
            types
        );
    }

    // --- Empty / edge cases ---

    #[test]
    fn test_highlight_empty_code() {
        let (data, data_len) = call_highlight("", "rust");
        // Empty code should return count=0 (only 2 bytes for count field)
        assert_eq!(data_len, 2);
        let count = u16::from_le_bytes([data[0], data[1]]);
        assert_eq!(count, 0, "Empty code should have 0 tokens");
    }

    #[test]
    fn test_highlight_empty_code_multiple_langs() {
        for lang in &["rust", "python", "javascript", "c", "go"] {
            let (data, data_len) = call_highlight("", lang);
            assert_eq!(
                data_len, 2,
                "Lang {}: empty code should have data_len=2",
                lang
            );
            let count = u16::from_le_bytes([data[0], data[1]]);
            assert_eq!(count, 0);
        }
    }

    // --- Error cases ---

    #[test]
    fn test_highlight_unknown_language() {
        use std::ffi::CString;
        let code = CString::new("some code").unwrap();
        let lang = CString::new("nonexistent_lang_xyz123").unwrap();
        let output = fastmarkdown_highlight(code.as_ptr(), lang.as_ptr());

        assert!(
            output.data.is_null(),
            "data should be null for unknown language"
        );
        assert_eq!(
            output.data_len, 0,
            "data_len should be 0 for unknown language"
        );

        // free on null data should be safe
        let mut out_copy = output;
        fastmarkdown_free_highlight(&mut out_copy);
    }

    #[test]
    fn test_highlight_unknown_language_rust() {
        use std::ffi::CString;
        let code = CString::new("let x = 1;").unwrap();
        let lang = CString::new("zzz_not_a_language").unwrap();
        let output = fastmarkdown_highlight(code.as_ptr(), lang.as_ptr());
        assert!(output.data.is_null());
        assert_eq!(output.data_len, 0);
        let mut out_copy = output;
        fastmarkdown_free_highlight(&mut out_copy);
    }

    // --- Safety: free on null pointer ---

    #[test]
    fn test_free_null_pointer() {
        fastmarkdown_free_highlight(std::ptr::null_mut());
    }

    // --- Safety: free on zero data ---

    #[test]
    fn test_free_zero_data() {
        let mut output = FastMarkdownHighlightOutput {
            data: std::ptr::null_mut(),
            data_len: 0,
        };
        fastmarkdown_free_highlight(&mut output);
        // No crash = pass
    }

    // --- Data format validation ---

    #[test]
    fn test_format_valid_count_field() {
        let code = "let a = 1;";
        let (data, _) = call_highlight(code, "rust");
        assert!(data.len() >= 2);
        let count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let expected_size = 2 + count * 3;
        assert_eq!(
            data.len(),
            expected_size,
            "Data length {} != 2 + {}*3 = {}",
            data.len(),
            count,
            expected_size
        );
    }

    #[test]
    fn test_format_token_count_positive() {
        let code = "fn hello() {}";
        let (data, _) = call_highlight(code, "rust");
        let count = u16::from_le_bytes([data[0], data[1]]);
        assert!(count > 0, "Non-empty code should have >0 tokens");
    }

    #[test]
    fn test_format_all_token_lengths_nonzero() {
        let code = "let x: i32 = 42;\n";
        let (data, _) = call_highlight(code, "rust");
        let tokens = decode_tokens(&data);
        for (i, (tt, len)) in tokens.iter().enumerate() {
            assert!(*len > 0, "Token {} (type={}) has zero length", i, tt);
        }
    }

    // --- Cross-validation: FFI == Rust API ---

    #[test]
    fn test_ffi_equals_rust_api() {
        let cases = vec![
            ("rust", "fn main() {}"),
            ("python", "def foo(): pass"),
            ("javascript", "const a = 1;"),
            ("bash", "echo hi"),
            ("c", "int main() { return 0; }"),
        ];
        for (lang, code) in cases {
            let (ffi_data, _) = call_highlight(code, lang);
            let (rust_data, _) = safe_highlight(code, lang).expect("Rust API should succeed");
            assert_eq!(
                ffi_data, rust_data,
                "FFI and Rust API should return identical data for lang={}",
                lang
            );
        }
    }

    // --- Multi-line code ---

    #[test]
    fn test_multi_line_code() {
        let code = "fn foo() {\n    let x = 1;\n    let y = 2;\n    x + y\n}\n";
        let (data, _) = call_highlight(code, "rust");
        let tokens = decode_tokens(&data);
        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len());
    }

    // --- Code with trailing newline ---

    #[test]
    fn test_code_with_trailing_newline() {
        let code = "let x = 42;\n";
        let (data, _) = call_highlight(code, "rust");
        let tokens = decode_tokens(&data);
        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len());
    }

    #[test]
    fn test_code_without_trailing_newline() {
        let code = "let x = 42;";
        let (data, _) = call_highlight(code, "rust");
        let tokens = decode_tokens(&data);
        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len());
    }

    // --- UTF-8 / Unicode ---

    #[test]
    fn test_utf8_comments() {
        let code = "// 你好世界\nlet msg = \"こんにちは\";\n";
        let (data, _) = call_highlight(code, "rust");
        let tokens = decode_tokens(&data);
        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len());
    }

    #[test]
    fn test_unicode_identifiers() {
        // Some languages support Unicode identifiers (e.g., Go)
        let code = "package main\nvar 变量 = 42\n";
        let (data, _) = call_highlight(code, "go");
        let tokens = decode_tokens(&data);
        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len());
    }

    // --- Whitespace-only code edge case ---

    #[test]
    fn test_whitespace_only() {
        let code = "   \n  \n ";
        let (data, _) = call_highlight(code, "rust");
        let tokens = decode_tokens(&data);
        let total: usize = tokens.iter().map(|(_, l)| *l as usize).sum();
        assert_eq!(total, code.len());
    }

    // --- Multiple consecutive calls (no leaks) ---

    #[test]
    fn test_multiple_consecutive_calls() {
        for i in 0..20 {
            let code = format!("let x{} = {};", i, i);
            let (data, data_len) = call_highlight(&code, "rust");
            assert!(data_len > 0);
            let tokens = decode_tokens(&data);
            assert!(!tokens.is_empty());
        }
    }

    // --- Unknown language via Rust API ---

    #[test]
    fn test_rust_api_unknown_lang_returns_none() {
        let result = highlight_code_serialized("code", "zzz_not_a_lang");
        assert_eq!(result, None);
    }

    // --- Rust API empty code ---

    #[test]
    fn test_rust_api_empty_code() {
        let result = highlight_code_serialized("", "rust");
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.len(), 2);
        let count = u16::from_le_bytes([data[0], data[1]]);
        assert_eq!(count, 0);
    }

    // --- Rust API vs highlight_code comparison ---

    #[test]
    fn test_serialized_vs_raw_highlight_code() {
        let code = "fn test(x: i32) -> bool { x > 0 }\n";
        let raw_tokens = highlight_code(code, "rust").unwrap();
        let serialized = highlight_code_serialized(code, "rust").unwrap();
        let decoded = decode_tokens(&serialized);
        assert_eq!(raw_tokens.len(), decoded.len());
        for (i, (raw, dec)) in raw_tokens.iter().zip(decoded.iter()).enumerate() {
            assert_eq!(raw.token_type, dec.0, "Token {} type mismatch", i);
            assert_eq!(raw.len, dec.1, "Token {} length mismatch", i);
        }
    }
}
