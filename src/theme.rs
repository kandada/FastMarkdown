// Copyright (c) 2025 xiefujin <490021684@qq.com>
// Licensed under Apache-2.0, see LICENSE file for full license terms.

/// 跨平台主题配置（C FFI 兼容）
///
/// 通过此结构将宿主平台的字体、颜色、间距偏好传递给渲染层。
/// 所有字段均为 C 兼容的标量类型，可通过 FFI 无痛传递。
///
/// # 使用方式
///
/// ## Rust 侧
/// ```ignore
/// use fastmarkdown::ThemeConfig;
/// let theme = ThemeConfig::default(); // 使用默认值
/// ```
///
/// ## iOS (Swift)
/// ```swift
/// import FastMarkdown
/// var theme = FastMarkdownTheme.default()
/// theme.body_font_name = "Helvetica".cString(using: .utf8)
/// theme.body_font_size = 14.0
/// theme.text_color = 0xFF000000  // black
/// theme.link_color = 0xFF0000FF  // blue
/// ```
///
/// ## Android (Kotlin/JNI)
/// ```kotlin
/// val theme = FastMarkdownTheme()
/// theme.body_font_name = null    // uses system default
/// theme.body_font_size = 14.0f
/// theme.text_color = 0xFF000000  // black
/// ```
///
/// ## 颜色格式
/// RGBA 打包为 u32：0xRRGGBBAA, 各 8 bits
/// 例如：红色=0xFF0000FF, 蓝色=0x0000FFFF, 黑色=0x000000FF, 半透黑=0x00000080
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ThemeConfig {
    // -- 字体 --
    /// 正文字体名称（null-terminated C string，null 表示系统默认）
    pub body_font_name: *const std::os::raw::c_char,
    pub body_font_size: f32,

    /// 等宽字体名称（用于代码块和行内代码）
    pub monospace_font_name: *const std::os::raw::c_char,
    pub monospace_font_size: f32,

    /// 标题字号数组 [h1, h2, h3, h4, h5, h6]
    pub heading_font_sizes: [f32; 6],

    // -- 颜色（RGBA 各 8 bits，0xAABBGGRR little-endian 或按需定义） --
    /// 正文颜色
    pub text_color: u32,
    /// 链接颜色
    pub link_color: u32,
    /// 代码块背景色
    pub code_bg_color: u32,
    /// 代码块文字色
    pub code_text_color: u32,
    /// 引用块左边竖线颜色
    pub blockquote_bar_color: u32,

    // -- 布局 --
    /// 段落间距（points）
    pub paragraph_spacing: f32,
    /// 行间距（倍数）
    pub line_spacing: f32,
    /// 列表缩进（points）
    pub list_indent: f32,
    /// 引用缩进（points）
    pub blockquote_indent: f32,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            body_font_name: std::ptr::null(),
            body_font_size: 14.0,
            monospace_font_name: std::ptr::null(),
            monospace_font_size: 13.0,
            heading_font_sizes: [24.0, 20.0, 18.0, 16.0, 15.0, 14.0],
            text_color: 0xFF000000,
            link_color: 0xFF0000FF,
            code_bg_color: 0xFFF0F0F0,
            code_text_color: 0xFF000000,
            blockquote_bar_color: 0xFF808080,
            paragraph_spacing: 6.0,
            line_spacing: 1.2,
            list_indent: 20.0,
            blockquote_indent: 12.0,
        }
    }
}
