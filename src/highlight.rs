// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

/// Token type constants for code highlighting output.
/// These are language-agnostic: the highlighter maps syntect scopes
/// into these 15 generic types, and the host maps them to colors.
pub const TOKEN_OTHER: u8 = 0;
pub const TOKEN_KEYWORD: u8 = 1;
pub const TOKEN_STRING: u8 = 2;
pub const TOKEN_COMMENT: u8 = 3;
pub const TOKEN_NUMBER: u8 = 4;
pub const TOKEN_FUNCTION: u8 = 5;
pub const TOKEN_TYPE: u8 = 6;
pub const TOKEN_OPERATOR: u8 = 7;
pub const TOKEN_PUNCTUATION: u8 = 8;
pub const TOKEN_VARIABLE: u8 = 9;
pub const TOKEN_CONSTANT: u8 = 10;
pub const TOKEN_BUILTIN: u8 = 11;
pub const TOKEN_ENTITY: u8 = 12;
pub const TOKEN_MARKUP: u8 = 13;
pub const TOKEN_REGEX: u8 = 14;

pub const TOKEN_TYPE_COUNT: u8 = 15;

/// Token name lookup (for debugging / display)
pub static TOKEN_NAMES: &[&str; 15] = &[
    "other",
    "keyword",
    "string",
    "comment",
    "number",
    "function",
    "type",
    "operator",
    "punctuation",
    "variable",
    "constant",
    "builtin",
    "entity",
    "markup",
    "regex",
];

/// A single highlighted token: (token_type, byte_length_in_source)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightToken {
    pub token_type: u8,
    pub len: u16,
}

/// Highlight a code block, returning flat token list.
/// The concatenation of token lengths equals the source text length.
/// Returns None if the language is not recognized or highlighting is unavailable.
#[cfg(feature = "highlight")]
pub fn highlight_code(code: &str, language: &str) -> Option<Vec<HighlightToken>> {
    use std::sync::OnceLock;
    use syntect::parsing::{ParseState, ScopeStack, ScopeStackOp, SyntaxSet};

    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    let ss = SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines());

    let syntax = ss.find_syntax_by_token(language)?;

    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let mut tokens: Vec<HighlightToken> = Vec::new();
    let mut pos: usize;

    let lines: Vec<&str> = code.lines().collect();
    if lines.is_empty() {
        return Some(Vec::new());
    }
    let line_count = lines.len();

    for (line_idx, line) in lines.iter().enumerate() {
        let ops = match parse_state.parse_line(line, ss) {
            Ok(ops) => ops,
            Err(_) => return None,
        };
        pos = 0;

        for (i, op) in &ops {
            let i = *i;
            if i > pos {
                let text_len = i - pos;
                let tt = map_scopes(&scope_stack);
                merge_or_push(&mut tokens, tt, text_len as u32);
            }
            match op {
                ScopeStackOp::Push(scope) => scope_stack.push(*scope),
                ScopeStackOp::Pop(n) => {
                    for _ in 0..*n {
                        scope_stack.pop();
                    }
                }
                _ => {}
            }
            pos = i;
        }

        if pos < line.len() {
            let text_len = line.len() - pos;
            let tt = map_scopes(&scope_stack);
            merge_or_push(&mut tokens, tt, text_len as u32);
        }

        // Add newline separator between lines (not after the last)
        if line_idx + 1 < line_count {
            tokens.push(HighlightToken {
                token_type: TOKEN_OTHER,
                len: 1,
            });
        }
    }

    // Add trailing newline if present in source
    if code.ends_with('\n') && !tokens.is_empty() {
        tokens.push(HighlightToken {
            token_type: TOKEN_OTHER,
            len: 1,
        });
    }

    // Verify total length matches
    let total: usize = tokens.iter().map(|t| t.len as usize).sum();
    if total != code.len() {
        return None;
    }

    Some(tokens)
}

#[cfg(not(feature = "highlight"))]
pub fn highlight_code(_code: &str, _language: &str) -> Option<Vec<HighlightToken>> {
    None
}

#[cfg(feature = "highlight")]
fn map_scopes(scopes: &syntect::parsing::ScopeStack) -> u8 {
    use std::fmt::Write;
    thread_local! {
        // Reused buffer: the combined scope string is built per token position.
        static SCRATCH: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
    }
    let slice = scopes.as_slice();
    if slice.is_empty() {
        return TOKEN_OTHER;
    }
    SCRATCH.with(|cell| {
        let mut combined = cell.borrow_mut();
        combined.clear();
        for scope in slice {
            let _ = write!(&mut *combined, "{} ", scope);
        }
        combined.pop(); // trailing space
        let combined = combined.as_str();

        if combined.contains("comment") {
            return TOKEN_COMMENT;
        }
        // Must check string.regexp before string (substring order)
        if combined.contains("string.regexp") {
            return TOKEN_REGEX;
        }
        if combined.contains("string") {
            return TOKEN_STRING;
        }
        if combined.contains("keyword") && !combined.contains("operator") {
            return TOKEN_KEYWORD;
        }
        if combined.contains("constant.numeric") {
            return TOKEN_NUMBER;
        }
        if combined.contains("entity.name.function")
            || combined.contains("support.function")
            || combined.contains("meta.function")
        {
            return TOKEN_FUNCTION;
        }
        if combined.contains("entity.name.type")
            || combined.contains("support.type")
            || combined.contains("storage.type")
        {
            return TOKEN_TYPE;
        }
        if combined.contains("keyword.operator") || combined.contains("punctuation.definition") {
            return TOKEN_OPERATOR;
        }
        if combined.contains("punctuation")
            || combined.contains("meta.brace")
            || combined.contains("meta.delimiter")
            || combined.contains("meta.group")
        {
            return TOKEN_PUNCTUATION;
        }
        if combined.contains("variable") || combined.contains("meta") {
            return TOKEN_VARIABLE;
        }
        if combined.contains("constant") {
            return TOKEN_CONSTANT;
        }
        if combined.contains("support") {
            return TOKEN_BUILTIN;
        }
        if combined.contains("entity") {
            return TOKEN_ENTITY;
        }
        if combined.contains("markup") {
            return TOKEN_MARKUP;
        }

        TOKEN_OTHER
    })
}

#[cfg(feature = "highlight")]
fn merge_or_push(tokens: &mut Vec<HighlightToken>, tt: u8, mut len: u32) {
    // Token lengths are u16 in the wire format; split runs longer than 65535
    // into multiple tokens instead of truncating (which used to make the total
    // mismatch and silently disable highlighting for the whole block).
    while len > 0 {
        if let Some(last) = tokens.last_mut() {
            if last.token_type == tt && last.len < u16::MAX {
                let room = (u16::MAX - last.len) as u32;
                let add = room.min(len);
                last.len += add as u16;
                len -= add;
                continue;
            }
        }
        let take = len.min(u16::MAX as u32) as u16;
        tokens.push(HighlightToken {
            token_type: tt,
            len: take,
        });
        len -= take as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let code = "fn main() {\n    println!(\"Hello\");\n}\n";
        let tokens = highlight_code(code, "rust");
        if let Some(ref tokens) = tokens {
            let total: usize = tokens.iter().map(|t| t.len as usize).sum();
            assert_eq!(total, code.len(), "Token lengths must sum to code length");
            let types: std::collections::HashSet<u8> =
                tokens.iter().map(|t| t.token_type).collect();
            assert!(
                types.len() >= 2,
                "Should have multiple token types, got {:?}",
                tokens
            );
        }
    }

    #[test]
    fn test_highlight_python() {
        let code = "def hello():\n    return 'world'\n";
        let tokens = highlight_code(code, "python");
        if let Some(tokens) = tokens {
            let total: usize = tokens.iter().map(|t| t.len as usize).sum();
            assert_eq!(total, code.len());
        }
    }

    #[test]
    fn test_highlight_unknown_language() {
        let tokens = highlight_code("text", "nonexistent_lang_xyz");
        assert_eq!(tokens, None);
    }

    #[test]
    fn test_highlight_plain_text() {
        let tokens = highlight_code("just plain text", "text");
        if let Some(tokens) = tokens {
            let total: usize = tokens.iter().map(|t| t.len as usize).sum();
            assert_eq!(total, "just plain text".len());
        }
    }

    #[test]
    fn test_empty_code() {
        let tokens = highlight_code("", "rust");
        if let Some(tokens) = tokens {
            assert!(tokens.is_empty());
        }
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn merge_or_push_splits_long_runs() {
        // Long same-type runs must be split into <=65535 tokens, not truncated,
        // so the total still equals the source length.
        let mut tokens = Vec::new();
        merge_or_push(&mut tokens, TOKEN_STRING, 200_000);
        let total: usize = tokens.iter().map(|t| t.len as usize).sum();
        assert_eq!(total, 200_000);
        assert!(tokens.len() >= 4, "long run must split: {}", tokens.len());
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn big_code_highlight_total_matches() {
        let code = format!("// {}\n", "x".repeat(100_000));
        if let Some(tokens) = highlight_code(&code, "rust") {
            let total: usize = tokens.iter().map(|t| t.len as usize).sum();
            assert_eq!(total, code.len(), "token lengths must sum to code length");
        }
    }
}
