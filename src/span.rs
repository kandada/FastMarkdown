// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

/// 样式标志位
pub const FLAG_BOLD: u32 = 1 << 0;
pub const FLAG_ITALIC: u32 = 1 << 1;
pub const FLAG_STRIKETHROUGH: u32 = 1 << 2;
pub const FLAG_INLINE_CODE: u32 = 1 << 3;
pub const FLAG_LINK: u32 = 1 << 4;
pub const FLAG_IMAGE: u32 = 1 << 5;
pub const FLAG_MATH: u32 = 1 << 6;
pub const FLAG_SUBSCRIPT: u32 = 1 << 7;
pub const FLAG_SUPERSCRIPT: u32 = 1 << 8;

/// 块类型
pub const BLOCK_PARAGRAPH: u8 = 0;
pub const BLOCK_HEADING_H1: u8 = 1;
pub const BLOCK_HEADING_H2: u8 = 2;
pub const BLOCK_HEADING_H3: u8 = 3;
pub const BLOCK_HEADING_H4: u8 = 4;
pub const BLOCK_HEADING_H5: u8 = 5;
pub const BLOCK_HEADING_H6: u8 = 6;
pub const BLOCK_CODE: u8 = 7;
pub const BLOCK_BLOCKQUOTE: u8 = 8;
pub const BLOCK_LIST_ITEM_ORDERED: u8 = 9;
pub const BLOCK_LIST_ITEM_UNORDERED: u8 = 10;
pub const BLOCK_TABLE_CELL: u8 = 11;
pub const BLOCK_TABLE_HEADER_CELL: u8 = 12;
pub const BLOCK_HORIZONTAL_RULE: u8 = 13;
pub const BLOCK_MATH_BLOCK: u8 = 14;
pub const BLOCK_IMAGE: u8 = 15;

/// extra_data chunk kind identifiers
pub const EXTRA_KIND_LINK_URL: u8 = 0;
pub const EXTRA_KIND_IMAGE_URL: u8 = 1;
pub const EXTRA_KIND_CODE_LANGUAGE: u8 = 2;
pub const EXTRA_KIND_TABLE_METADATA: u8 = 3;
pub const EXTRA_KIND_CODE_HIGHLIGHT: u8 = 4;
pub const EXTRA_KIND_LIST_META: u8 = 5;

/// 单个样式 span（连续同属性的文本片段）
/// #[repr(C)] 确保 FFI 兼容，16 字节对齐
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// 相对于全文起始位置的字节偏移
    pub offset: u32,
    /// 字节长度
    pub length: u32,
    /// 样式标志位（按位组合）
    pub flags: u32,
    /// 附加数据长度（在 extra_data 中的连续字节数，0 表示无）
    pub extra_len: u32,
    /// 块类型
    pub block_type: u8,
    /// 块嵌套深度（列表/引用的层级，0=根层级）
    pub block_depth: u8,
    /// 块序号（每个新的 block 级元素 +1，同 block 内 spans 共享同一序号）
    pub block_seq: u16,
}

/// Span 输出（包含文本、span 数组和额外数据）
/// 注意：不实现 Clone / Copy，防止双份指针导致 double-free
#[repr(C)]
#[derive(Debug)]
pub struct SpanOutput {
    /// 全文纯文本（所有 span 的 offset/len 索引此 buffer 的子串）
    pub text: *const u8,
    pub text_len: u32,
    /// Span 数组
    pub spans: *const Span,
    pub span_count: u32,
    /// 额外数据区（链接 URL、图片 URL 等）
    pub extra_data: *const u8,
    pub extra_data_len: u32,
    /// 可选：与 spans 一一对应的 UTF-16 偏移（指针为 null 表示未启用）
    /// 用于 iOS(NSString)/Android(SpannableString) 等 UTF-16 平台的零拷贝偏移转换
    pub utf16_offsets: *const u32,
    pub utf16_offsets_len: u32,
}

/// 流式增量 delta
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanDeltaKind {
    /// 追加到末尾
    Append,
    /// 替换最后 N 个 span
    ReplaceLast { count: u32 },
}

#[derive(Debug, Clone)]
pub struct SpanDelta {
    pub kind: SpanDeltaKind,
    pub spans: Vec<Span>,
}

impl Span {
    pub fn new(
        offset: u32,
        length: u32,
        flags: u32,
        block_type: u8,
        extra_len: u32,
        block_depth: u8,
        block_seq: u16,
    ) -> Self {
        Self {
            offset,
            length,
            flags,
            block_type,
            extra_len,
            block_depth,
            block_seq,
        }
    }

    pub fn is_bold(&self) -> bool {
        self.flags & FLAG_BOLD != 0
    }

    pub fn is_italic(&self) -> bool {
        self.flags & FLAG_ITALIC != 0
    }

    pub fn is_strikethrough(&self) -> bool {
        self.flags & FLAG_STRIKETHROUGH != 0
    }

    pub fn is_inline_code(&self) -> bool {
        self.flags & FLAG_INLINE_CODE != 0
    }

    pub fn is_link(&self) -> bool {
        self.flags & FLAG_LINK != 0
    }

    pub fn is_image(&self) -> bool {
        self.flags & FLAG_IMAGE != 0
    }

    pub fn is_math(&self) -> bool {
        self.flags & FLAG_MATH != 0
    }

    pub fn has_any_style(&self) -> bool {
        self.flags
            & (FLAG_BOLD
                | FLAG_ITALIC
                | FLAG_STRIKETHROUGH
                | FLAG_INLINE_CODE
                | FLAG_LINK
                | FLAG_IMAGE
                | FLAG_MATH
                | FLAG_SUBSCRIPT
                | FLAG_SUPERSCRIPT)
            != 0
    }

    pub fn is_subscript(&self) -> bool {
        self.flags & FLAG_SUBSCRIPT != 0
    }

    pub fn is_superscript(&self) -> bool {
        self.flags & FLAG_SUPERSCRIPT != 0
    }
}

impl SpanOutput {
    /// 从拥有的数据创建 SpanOutput（用于测试和内部使用）
    /// 调用者必须保持这些 buffer 存活
    pub unsafe fn from_buffers(text: &[u8], spans: &[Span], extra: &[u8]) -> Self {
        Self {
            text: text.as_ptr(),
            text_len: text.len() as u32,
            spans: spans.as_ptr(),
            span_count: spans.len() as u32,
            extra_data: extra.as_ptr(),
            extra_data_len: extra.len() as u32,
            utf16_offsets: std::ptr::null(),
            utf16_offsets_len: 0,
        }
    }
}
