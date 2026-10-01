use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct BulletConfig {
    #[serde(default = "default_bullet_level0")]
    pub level0: String,
    #[serde(default = "default_bullet_level1")]
    pub level1: String,
    #[serde(default = "default_bullet_level2")]
    pub level2: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub fonts: FontConfig,
    #[serde(default)]
    pub sizes: SizeConfig,
    #[serde(default)]
    pub page: PageConfig,
    #[serde(default)]
    pub indent: IndentConfig,
    #[serde(default)]
    pub bullet: BulletConfig,
    #[serde(default)]
    pub numbering: NumberingConfig,
    #[serde(default)]
    pub captions: CaptionConfig,
    #[serde(default)]
    pub table: TableConfig,
    #[serde(default)]
    pub line_numbers: LineNumberConfig,
    #[serde(default)]
    pub code_block: CodeBlockConfig,
    #[serde(default)]
    pub heading: HeadingConfig,
    #[serde(default)]
    pub equal: EqualConfig,
}

#[derive(Debug, Deserialize)]
pub struct FontConfig {
    #[serde(default = "default_body_ja")]
    pub body_ja: String,
    #[serde(default = "default_body_en")]
    pub body_en: String,
    #[serde(default = "default_heading_ja")]
    pub heading_ja: String,
    #[serde(default = "default_heading_en")]
    pub heading_en: String,
    #[serde(default = "default_code_ja")]
    pub code_ja: String,
    #[serde(default = "default_code_en")]
    pub code_en: String,
}

#[derive(Debug, Deserialize)]
pub struct SizeConfig {
    #[serde(default = "default_body_size")]
    pub body: f64,
    #[serde(default = "default_table_body_size")]
    pub table_body: f64,
    #[serde(default = "default_table_header_size")]
    pub table_header: f64,
    #[serde(default = "default_h1_size")]
    pub heading1: f64,
    #[serde(default = "default_h2_size")]
    pub heading2: f64,
    #[serde(default = "default_h3_size")]
    pub heading3: f64,
    #[serde(default = "default_h4_size")]
    pub heading4: f64,
    #[serde(default = "default_h5_size")]
    pub heading5: f64,
}

#[derive(Debug, Deserialize)]
pub struct PageConfig {
    #[serde(default = "default_page_width")]
    pub width: u32,
    #[serde(default = "default_page_height")]
    pub height: u32,
    #[serde(default = "default_page_margin_top")]
    pub margin_top: i32,
    #[serde(default = "default_page_margin_right")]
    pub margin_right: i32,
    #[serde(default = "default_page_margin_bottom")]
    pub margin_bottom: i32,
    #[serde(default = "default_page_margin_left")]
    pub margin_left: i32,
    #[serde(default = "default_page_margin_header")]
    pub margin_header: i32,
    #[serde(default = "default_page_margin_footer")]
    pub margin_footer: i32,
    #[serde(default = "default_page_margin_gutter")]
    pub margin_gutter: i32,
}

#[derive(Debug, Deserialize)]
pub struct IndentConfig {
    #[serde(default = "default_indent_body_left")]
    pub body_left: i32,
    #[serde(default = "default_indent_body_first_line")]
    pub body_first_line: i32,
    #[serde(default = "default_indent_body_right")]
    pub body_right: i32,
    #[serde(default = "default_indent_body_left_chars")]
    pub body_left_chars: i32,
    #[serde(default = "default_indent_heading1_left")]
    pub heading1_left: i32,
    #[serde(default = "default_indent_heading1_hanging")]
    pub heading1_hanging: i32,
    #[serde(default = "default_indent_heading2_left")]
    pub heading2_left: i32,
    #[serde(default = "default_indent_heading2_hanging")]
    pub heading2_hanging: i32,
    #[serde(default = "default_indent_heading3_left")]
    pub heading3_left: i32,
    #[serde(default = "default_indent_heading3_hanging")]
    pub heading3_hanging: i32,
    #[serde(default = "default_indent_heading4_left")]
    pub heading4_left: i32,
    #[serde(default = "default_indent_heading4_hanging")]
    pub heading4_hanging: i32,
    #[serde(default = "default_indent_heading5_left")]
    pub heading5_left: i32,
    #[serde(default = "default_indent_heading5_hanging")]
    pub heading5_hanging: i32,
    #[serde(default = "default_indent_heading6_left")]
    pub heading6_left: i32,
    #[serde(default = "default_indent_heading6_hanging")]
    pub heading6_hanging: i32,
}

fn default_body_ja() -> String {
    "游明朝".to_string()
}
fn default_body_en() -> String {
    "Century".to_string()
}
fn default_heading_ja() -> String {
    "游ゴシック".to_string()
}
fn default_heading_en() -> String {
    "Century".to_string()
}
fn default_body_size() -> f64 {
    10.5
}
fn default_code_ja() -> String {
    "ＭＳ ゴシック".to_string()
}
fn default_code_en() -> String {
    "Courier New".to_string()
}
fn default_table_body_size() -> f64 {
    9.5
}
fn default_table_header_size() -> f64 {
    9.5
}
fn default_h1_size() -> f64 {
    14.0
}
fn default_h2_size() -> f64 {
    12.0
}
fn default_h3_size() -> f64 {
    11.0
}
fn default_h4_size() -> f64 {
    11.0
}
fn default_h5_size() -> f64 {
    10.5
}

fn default_page_width() -> u32 {
    11_906
}
fn default_page_height() -> u32 {
    16_838
}
fn default_page_margin_top() -> i32 {
    1_985
}
fn default_page_margin_right() -> i32 {
    1_701
}
fn default_page_margin_bottom() -> i32 {
    1_701
}
fn default_page_margin_left() -> i32 {
    1_701
}
fn default_page_margin_header() -> i32 {
    851
}
fn default_page_margin_footer() -> i32 {
    992
}
fn default_page_margin_gutter() -> i32 {
    0
}

fn default_indent_body_left() -> i32 {
    210
}
fn default_indent_body_first_line() -> i32 {
    210
}
fn default_indent_body_right() -> i32 {
    210
}
fn default_indent_body_left_chars() -> i32 {
    100
}
fn default_indent_heading1_left() -> i32 {
    420
}
fn default_indent_heading1_hanging() -> i32 {
    420
}
fn default_indent_heading2_left() -> i32 {
    612
}
fn default_indent_heading2_hanging() -> i32 {
    612
}
fn default_indent_heading3_left() -> i32 {
    783
}
fn default_indent_heading3_hanging() -> i32 {
    783
}
fn default_indent_heading4_left() -> i32 {
    709
}
fn default_indent_heading4_hanging() -> i32 {
    709
}
fn default_indent_heading5_left() -> i32 {
    709
}
fn default_indent_heading5_hanging() -> i32 {
    709
}
fn default_indent_heading6_left() -> i32 {
    709
}
fn default_indent_heading6_hanging() -> i32 {
    709
}

#[derive(Debug, Deserialize)]
pub struct CaptionConfig {
    #[serde(default = "default_caption_visible")]
    pub table: bool,
    #[serde(default = "default_caption_visible")]
    pub figure: bool,
}

/// 通常の表の外側余白（twip）。
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct TableConfig {
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub margin_left: i32,
    pub margin_right: i32,
}

fn default_caption_visible() -> bool {
    true
}

impl Default for CaptionConfig {
    fn default() -> Self {
        Self {
            table: default_caption_visible(),
            figure: default_caption_visible(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NumberingConfig {
    #[serde(default = "default_figure_format")]
    pub figure_format: String,
    #[serde(default = "default_table_format")]
    pub table_format: String,
}

fn default_figure_format() -> String {
    "sequential".to_string()
}
fn default_table_format() -> String {
    "sequential".to_string()
}

impl Default for NumberingConfig {
    fn default() -> Self {
        Self {
            figure_format: default_figure_format(),
            table_format: default_table_format(),
        }
    }
}

fn default_bullet_level0() -> String {
    "●".to_string()
}
fn default_bullet_level1() -> String {
    "■".to_string()
}
fn default_bullet_level2() -> String {
    "▲".to_string()
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            body_ja: default_body_ja(),
            body_en: default_body_en(),
            heading_ja: default_heading_ja(),
            heading_en: default_heading_en(),
            code_ja: default_code_ja(),
            code_en: default_code_en(),
        }
    }
}

impl Default for SizeConfig {
    fn default() -> Self {
        Self {
            body: default_body_size(),
            table_body: default_table_body_size(),
            table_header: default_table_header_size(),
            heading1: default_h1_size(),
            heading2: default_h2_size(),
            heading3: default_h3_size(),
            heading4: default_h4_size(),
            heading5: default_h5_size(),
        }
    }
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            width: default_page_width(),
            height: default_page_height(),
            margin_top: default_page_margin_top(),
            margin_right: default_page_margin_right(),
            margin_bottom: default_page_margin_bottom(),
            margin_left: default_page_margin_left(),
            margin_header: default_page_margin_header(),
            margin_footer: default_page_margin_footer(),
            margin_gutter: default_page_margin_gutter(),
        }
    }
}

impl Default for IndentConfig {
    fn default() -> Self {
        Self {
            body_left: default_indent_body_left(),
            body_first_line: default_indent_body_first_line(),
            body_right: default_indent_body_right(),
            body_left_chars: default_indent_body_left_chars(),
            heading1_left: default_indent_heading1_left(),
            heading1_hanging: default_indent_heading1_hanging(),
            heading2_left: default_indent_heading2_left(),
            heading2_hanging: default_indent_heading2_hanging(),
            heading3_left: default_indent_heading3_left(),
            heading3_hanging: default_indent_heading3_hanging(),
            heading4_left: default_indent_heading4_left(),
            heading4_hanging: default_indent_heading4_hanging(),
            heading5_left: default_indent_heading5_left(),
            heading5_hanging: default_indent_heading5_hanging(),
            heading6_left: default_indent_heading6_left(),
            heading6_hanging: default_indent_heading6_hanging(),
        }
    }
}

impl Default for BulletConfig {
    fn default() -> Self {
        Self {
            level0: default_bullet_level0(),
            level1: default_bullet_level1(),
            level2: default_bullet_level2(),
        }
    }
}

/// 行番号設定
#[derive(Debug, Deserialize)]
pub struct LineNumberConfig {
    /// 行番号を有効にするか（デフォルト: false）
    #[serde(default)]
    pub enabled: bool,
    /// N行ごとに番号を表示（デフォルト: 1）
    #[serde(default = "default_ln_count_by")]
    pub count_by: u32,
    /// 開始番号（デフォルト: 1）
    #[serde(default = "default_ln_start")]
    pub start: u32,
    /// 採番リセットのタイミング: "newPage" / "newSection" / "continuous"（デフォルト: "newPage"）
    #[serde(default = "default_ln_restart")]
    pub restart: String,
}

fn default_ln_count_by() -> u32 {
    1
}
fn default_ln_start() -> u32 {
    1
}
fn default_ln_restart() -> String {
    "newPage".to_string()
}

impl Default for LineNumberConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            count_by: default_ln_count_by(),
            start: default_ln_start(),
            restart: default_ln_restart(),
        }
    }
}

/// コードブロック設定
#[derive(Debug, Deserialize, Default)]
pub struct CodeBlockConfig {
    /// コードブロックを1行1列の罫線付き表で囲む（デフォルト: false）
    #[serde(default)]
    pub border: bool,
    /// ブロックの外側の余白（単位: twip）。罫線ありの場合は表の外側に適用。
    #[serde(default)]
    pub margin_top: u32,
    #[serde(default)]
    pub margin_bottom: u32,
    #[serde(default)]
    pub margin_right: i32,
    #[serde(default)]
    pub margin_left: i32,
}

/// `==text==` による文字装飾設定
#[derive(Debug, Deserialize, Default)]
pub struct EqualConfig {
    /// `==text==` の解析と装飾を有効にする（デフォルト: false）
    #[serde(default)]
    pub enabled: bool,
    /// 装飾部分の文字サイズ (pt)。省略時は周囲と同じサイズ
    #[serde(default)]
    pub font_size: Option<f64>,
    /// 装飾部分の背景色 (`#RRGGBB` または `RRGGBB`)。省略時は背景色なし
    #[serde(default)]
    pub background_color: Option<String>,
}

/// 見出し関連設定
#[derive(Debug, Deserialize)]
pub struct HeadingConfig {
    /// # → 表題、## → 見出し1、### → 見出し2 … とレベルをシフトする
    #[serde(default)]
    pub heading_shift: bool,
    /// 表題スタイルのフォントサイズ (pt)  ※ heading_shift = true 時に使用
    #[serde(default = "default_title_size")]
    pub title_size: f64,
}

fn default_title_size() -> f64 {
    18.0
}

impl Default for HeadingConfig {
    fn default() -> Self {
        Self {
            heading_shift: false,
            title_size: default_title_size(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_margins_default_to_zero() {
        for source in ["", "[table]\nmargin_left = 300"] {
            let config: Config = toml::from_str(source).unwrap();
            assert_eq!(config.table.margin_top, 0);
            assert_eq!(config.table.margin_bottom, 0);
            assert_eq!(config.table.margin_right, 0);
            assert_eq!(
                config.table.margin_left,
                if source.is_empty() { 0 } else { 300 }
            );
        }
    }

    #[test]
    fn captions_default_to_visible_and_can_be_disabled_independently() {
        assert!(Config::default().captions.table);
        assert!(Config::default().captions.figure);
        for (source, table, figure) in [
            ("", true, true),
            ("[captions]\ntable = false", false, true),
            ("[captions]\nfigure = false", true, false),
            ("[captions]\ntable = false\nfigure = false", false, false),
        ] {
            let config: Config = toml::from_str(source).unwrap();
            assert_eq!(config.captions.table, table);
            assert_eq!(config.captions.figure, figure);
        }
    }

    #[test]
    fn code_block_options_are_optional() {
        for source in [
            "",
            "[fonts]\nbody_en = 'Arial'",
            "[code_block]\nborder = true",
        ] {
            let config: Config = toml::from_str(source).unwrap();
            assert_eq!(config.fonts.code_ja, "ＭＳ ゴシック");
            assert_eq!(config.fonts.code_en, "Courier New");
            assert_eq!(config.code_block.margin_top, 0);
            assert_eq!(config.code_block.margin_bottom, 0);
            assert_eq!(config.code_block.margin_right, 0);
            assert_eq!(config.code_block.margin_left, 0);
            assert_eq!(config.code_block.border, source.contains("border = true"));
        }
        let partial: Config = toml::from_str("[code_block]\nmargin_top = 240").unwrap();
        assert_eq!(partial.code_block.margin_top, 240);
        assert_eq!(partial.code_block.margin_bottom, 0);
        assert!(!partial.code_block.border);
        let defaults = Config::default();
        assert_eq!(defaults.fonts.code_ja, "ＭＳ ゴシック");
        assert_eq!(defaults.fonts.code_en, "Courier New");
        assert_eq!(defaults.code_block.margin_top, 0);
        assert_eq!(defaults.code_block.margin_bottom, 0);
        assert_eq!(defaults.code_block.margin_right, 0);
        assert_eq!(defaults.code_block.margin_left, 0);
    }

    #[test]
    fn reads_equal_markup_options_and_allows_optional_styles() {
        let configured: Config = toml::from_str(
            r##"
                [equal]
                enabled = true
                font_size = 18.0
                background_color = "#FFFF00"
            "##,
        )
        .unwrap();
        assert!(configured.equal.enabled);
        assert_eq!(configured.equal.font_size, Some(18.0));
        assert_eq!(
            configured.equal.background_color.as_deref(),
            Some("#FFFF00")
        );

        let minimal: Config = toml::from_str("[equal]\nenabled = true").unwrap();
        assert!(minimal.equal.enabled);
        assert_eq!(minimal.equal.font_size, None);
        assert_eq!(minimal.equal.background_color, None);

        let defaults: Config = toml::from_str("").unwrap();
        assert!(!defaults.equal.enabled);
    }
}
