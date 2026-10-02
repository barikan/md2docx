use std::path::Path;

use anyhow::Result;
use docx_rs::*;
use image::GenericImageView;

use crate::config::{Config, PageConfig};
use crate::heading::HeadingManager;
use crate::ir::{Block, Inline, ListItem};
use crate::styles;

const EMU_PER_PIXEL: u64 = 9_525;
const EMU_PER_TWIP: u64 = 635;
const TABLE_WIDTH_PCT: usize = 5_000;
const TABLE_CELL_PADDING_TWIP: usize = 80;

pub fn convert_to_docx(blocks: &[Block], config: &Config, base_path: &Path) -> Result<Docx> {
    let mut ctx = ConvertContext::new(config, base_path);
    let mut docx = Docx::new();

    // sample.docx 準拠のスタイル・番号定義を適用
    docx = styles::setup_document_styles(docx, config);
    docx = docx
        .page_size(config.page.width, config.page.height)
        .page_margin(
            PageMargin::new()
                .top(config.page.margin_top)
                .right(config.page.margin_right)
                .bottom(config.page.margin_bottom)
                .left(config.page.margin_left)
                .header(config.page.margin_header)
                .footer(config.page.margin_footer)
                .gutter(config.page.margin_gutter),
        );

    for block in blocks {
        docx = ctx.convert_block(docx, block);
    }

    Ok(docx)
}

struct ConvertContext<'a> {
    config: &'a Config,
    base_path: &'a Path,
    heading_mgr: HeadingManager,
    /// 現在の H1 章番号（0 = H1 未出現）
    chapter_number: u32,
    /// 章内の図カウンタ
    figure_in_chapter: u32,
    /// 章内の表カウンタ
    table_in_chapter: u32,
    /// グローバル連番（sequential モード用）
    figure_seq: u32,
    table_seq: u32,
}

#[derive(Clone, Copy)]
enum InlineStyle {
    Body,
    TableBody,
    TableHeader,
}

impl<'a> ConvertContext<'a> {
    fn new(config: &'a Config, base_path: &'a Path) -> Self {
        Self {
            config,
            base_path,
            heading_mgr: HeadingManager::new(),
            chapter_number: 0,
            figure_in_chapter: 0,
            table_in_chapter: 0,
            figure_seq: 0,
            table_seq: 0,
        }
    }

    fn convert_block(&mut self, docx: Docx, block: &Block) -> Docx {
        match block {
            Block::Heading { level, content } => self.convert_heading(docx, *level, content),
            Block::PageBreak => self.convert_page_break(docx),
            Block::Paragraph { content } => self.convert_paragraph(docx, content),
            Block::BulletList { items } => self.convert_bullet_list(docx, items, 0),
            Block::OrderedList { items, start } => {
                self.convert_ordered_list(docx, items, *start, 0)
            }
            Block::Table {
                headers,
                rows,
                alignments,
            } => self.convert_table(docx, headers, rows, alignments),
            Block::CodeBlock { lang, code } => self.convert_code_block(docx, lang.as_deref(), code),
            Block::Image { alt, path } => self.convert_image(docx, alt, path),
            Block::BlockQuote { children } => {
                let mut d = docx;
                for child in children {
                    d = self.convert_block(d, child);
                }
                d
            }
            Block::ThematicBreak => {
                // 水平線 → 空段落で代替
                docx.add_paragraph(Paragraph::new())
            }
        }
    }

    fn convert_heading(&mut self, docx: Docx, level: u8, content: &[Inline]) -> Docx {
        // heading_shift モードかつ # (level==1) → 表題として出力
        if self.config.heading.heading_shift && level == 1 {
            return self.convert_title(docx, content);
        }

        // heading_shift モードでは ## 以降を1段下げる (## → H1 相当)
        let effective_level = if self.config.heading.heading_shift {
            level - 1
        } else {
            level
        };

        // heading_mgr のカウンタを進める（番号同期のため）
        let _ = self.heading_mgr.next_heading(effective_level, content);

        // H1 相当出現時: 章番号を更新し、章内カウンタをリセット
        if effective_level == 1 {
            self.chapter_number = self.heading_mgr.current_h1_number();
            self.figure_in_chapter = 0;
            self.table_in_chapter = 0;
        }

        // テキストから既存の番号部分を除去
        let plain_text: String = content.iter().map(|i| i.to_plain_text()).collect();
        let display_text = if self.config.heading.numbering {
            self.heading_mgr
                .strip_number(effective_level, plain_text.trim())
        } else {
            plain_text.trim().to_string()
        };

        // Run はテキストのみ（フォント・サイズ・boldはスタイルが担当）
        let run = Run::new().add_text(&display_text);

        // スタイル ID: 見出し1="1", 見出し2="2", ...
        let style_id = effective_level.to_string();

        // 段落にスタイルと numbering を適用
        let mut para = Paragraph::new()
            .add_run(run)
            .style(&style_id)
            .keep_next(true);
        if self.config.heading.numbering {
            para = para.numbering(
                NumberingId::new(styles::HEADING_NUM_ID),
                IndentLevel::new((effective_level as usize).saturating_sub(1)),
            );
        }

        docx.add_paragraph(para)
    }

    fn convert_title(&self, docx: Docx, content: &[Inline]) -> Docx {
        let plain_text: String = content.iter().map(|i| i.to_plain_text()).collect();
        let run = Run::new().add_text(plain_text.trim());
        let para = Paragraph::new().add_run(run).style(styles::TITLE_STYLE_ID);
        docx.add_paragraph(para)
    }

    fn convert_page_break(&self, docx: Docx) -> Docx {
        let para = Paragraph::new().add_run(Run::new().add_break(BreakType::Page));
        docx.add_paragraph(para)
    }

    /// 図番号文字列を生成（chapter: "1.2", sequential: "2"）
    fn next_figure_number(&mut self) -> String {
        self.figure_seq += 1;
        self.figure_in_chapter += 1;
        match self.config.numbering.figure_format.as_str() {
            "chapter" => {
                let ch = if self.chapter_number == 0 {
                    1
                } else {
                    self.chapter_number
                };
                format!("{}.{}", ch, self.figure_in_chapter)
            }
            _ => format!("{}", self.figure_seq),
        }
    }

    /// 表番号文字列を生成（chapter: "1.2", sequential: "2"）
    fn next_table_number(&mut self) -> String {
        self.table_seq += 1;
        self.table_in_chapter += 1;
        match self.config.numbering.table_format.as_str() {
            "chapter" => {
                let ch = if self.chapter_number == 0 {
                    1
                } else {
                    self.chapter_number
                };
                format!("{}.{}", ch, self.table_in_chapter)
            }
            _ => format!("{}", self.table_seq),
        }
    }

    fn convert_paragraph(&self, docx: Docx, content: &[Inline]) -> Docx {
        let para = self
            .build_paragraph(content, false, false, false, InlineStyle::Body)
            .style(styles::BODY_TEXT_STYLE_ID);
        docx.add_paragraph(para)
    }

    fn build_paragraph(
        &self,
        content: &[Inline],
        bold: bool,
        italic: bool,
        equal: bool,
        style: InlineStyle,
    ) -> Paragraph {
        let mut para = Paragraph::new();
        for inline in content {
            para = self.add_inline_to_paragraph(para, inline, bold, italic, equal, style);
        }
        para
    }

    fn add_inline_to_paragraph(
        &self,
        para: Paragraph,
        inline: &Inline,
        bold: bool,
        italic: bool,
        equal: bool,
        style: InlineStyle,
    ) -> Paragraph {
        match inline {
            Inline::Text(text) => {
                let processed = process_text(text);
                let run =
                    self.apply_inline_format(self.make_run(&processed, style), bold, italic, equal);
                para.add_run(run)
            }
            Inline::Code(code) => {
                let display = format!("「{}」", code);
                let run =
                    self.apply_inline_format(self.make_run(&display, style), bold, italic, equal);
                para.add_run(run)
            }
            Inline::Bold(children) => {
                let mut p = para;
                for child in children {
                    p = self.add_inline_to_paragraph(p, child, true, italic, equal, style);
                }
                p
            }
            Inline::Italic(children) => {
                let mut p = para;
                for child in children {
                    p = self.add_inline_to_paragraph(p, child, bold, true, equal, style);
                }
                p
            }
            Inline::Equal(children) => {
                let mut p = para;
                for child in children {
                    p = self.add_inline_to_paragraph(p, child, bold, italic, true, style);
                }
                p
            }
            Inline::Link { text, url } => {
                let label: String = text.iter().map(|child| child.to_plain_text()).collect();
                let display = if label.is_empty() { url.clone() } else { label };
                let processed = process_text(&display);

                let run =
                    self.apply_inline_format(self.make_run(&processed, style), bold, italic, equal);

                let hyperlink = if let Some(anchor) = url.strip_prefix('#') {
                    Hyperlink::new(anchor, HyperlinkType::Anchor).add_run(run)
                } else {
                    Hyperlink::new(url, HyperlinkType::External).add_run(run)
                };

                para.add_hyperlink(hyperlink)
            }
            Inline::SoftBreak => para.add_run(self.apply_inline_format(
                self.make_run(" ", style),
                bold,
                italic,
                equal,
            )),
            Inline::HardBreak => para.add_run(Run::new().add_break(BreakType::TextWrapping)),
        }
    }

    fn make_run(&self, text: &str, style: InlineStyle) -> Run {
        let (fonts, size) = match style {
            InlineStyle::Body => (
                RunFonts::new()
                    .ascii(&self.config.fonts.body_en)
                    .hi_ansi(&self.config.fonts.body_en)
                    .east_asia(&self.config.fonts.body_ja)
                    .cs(&self.config.fonts.body_en),
                self.config.sizes.body,
            ),
            InlineStyle::TableBody => (
                RunFonts::new()
                    .ascii(&self.config.fonts.body_en)
                    .hi_ansi(&self.config.fonts.body_en)
                    .east_asia(&self.config.fonts.body_ja)
                    .cs(&self.config.fonts.body_en),
                self.config.sizes.table_body,
            ),
            InlineStyle::TableHeader => (
                RunFonts::new()
                    .ascii(&self.config.fonts.heading_en)
                    .hi_ansi(&self.config.fonts.heading_en)
                    .east_asia(&self.config.fonts.heading_ja)
                    .cs(&self.config.fonts.heading_en),
                self.config.sizes.table_header,
            ),
        };

        Run::new()
            .add_text(text)
            .size(styles::pt_to_half_point(size))
            .fonts(fonts)
    }

    fn apply_inline_format(&self, mut run: Run, bold: bool, italic: bool, equal: bool) -> Run {
        if bold {
            run = run.bold();
        }
        if italic {
            run = run.italic();
        }
        if equal && self.config.equal.enabled {
            if let Some(font_size) = self.config.equal.font_size
                && font_size.is_finite()
                && font_size > 0.0
            {
                run = run.size(styles::pt_to_half_point(font_size));
            }
            if let Some(background_color) = &self.config.equal.background_color {
                let color = background_color.trim().trim_start_matches('#');
                if !color.is_empty() {
                    run = run.shading(Shading::new().fill(color));
                }
            }
        }
        run
    }

    fn convert_bullet_list(&mut self, docx: Docx, items: &[ListItem], depth: usize) -> Docx {
        let mut d = docx;
        for item in items {
            let level = depth.min(2); // 最大レベル2

            let mut para = Paragraph::new().style(styles::BULLET_STYLE_ID).numbering(
                NumberingId::new(styles::BULLET_NUM_ID),
                IndentLevel::new(level),
            );

            for inline in &item.content {
                para = self.add_inline_to_paragraph(
                    para,
                    inline,
                    false,
                    false,
                    false,
                    InlineStyle::Body,
                );
            }
            d = d.add_paragraph(para);

            // ネストされたブロック（BulletList の場合は depth をインクリメント）
            for child in &item.children {
                match child {
                    Block::BulletList {
                        items: nested_items,
                    } => {
                        d = self.convert_bullet_list(d, nested_items, depth + 1);
                    }
                    Block::OrderedList {
                        items: nested_items,
                        start,
                    } => {
                        d = self.convert_ordered_list(d, nested_items, *start, depth + 1);
                    }
                    _ => {
                        d = self.convert_block(d, child);
                    }
                }
            }
        }
        d
    }

    fn convert_ordered_list(
        &mut self,
        docx: Docx,
        items: &[ListItem],
        start: u64,
        depth: usize,
    ) -> Docx {
        let mut d = docx;
        for (i, item) in items.iter().enumerate() {
            let num = start + i as u64;
            let indent_twip = (depth as i32 + 1) * styles::pt_to_twip(18.0);

            let mut para = Paragraph::new().indent(Some(indent_twip), None, None, None);
            let prefix = format!("{}. ", num);
            let prefix_run = self.make_run(&prefix, InlineStyle::Body);
            para = para.add_run(prefix_run);

            for inline in &item.content {
                para = self.add_inline_to_paragraph(
                    para,
                    inline,
                    false,
                    false,
                    false,
                    InlineStyle::Body,
                );
            }
            d = d.add_paragraph(para);

            // ネストされたブロック
            for child in &item.children {
                match child {
                    Block::OrderedList {
                        items: nested_items,
                        start,
                    } => {
                        d = self.convert_ordered_list(d, nested_items, *start, depth + 1);
                    }
                    Block::BulletList {
                        items: nested_items,
                    } => {
                        d = self.convert_bullet_list(d, nested_items, depth + 1);
                    }
                    _ => {
                        d = self.convert_block(d, child);
                    }
                }
            }
        }
        d
    }

    fn convert_table(
        &mut self,
        docx: Docx,
        headers: &[Vec<Inline>],
        rows: &[Vec<Vec<Inline>>],
        alignments: &[crate::ir::Alignment],
    ) -> Docx {
        let column_count = headers
            .len()
            .max(rows.iter().map(|row| row.len()).max().unwrap_or(0));
        if column_count == 0 {
            return docx;
        }
        let margins = &self.config.table;
        let width = (i64::from(body_width_twip(&self.config.page))
            - i64::from(margins.margin_left)
            - i64::from(margins.margin_right))
        .max(column_count as i64) as usize;
        let docx = if margins.margin_top > 0 {
            docx.add_paragraph(outer_margin_spacer(margins.margin_top).keep_next(true))
        } else {
            docx
        };
        let docx = if self.config.captions.table {
            // 表番号キャプション
            let caption_fonts = RunFonts::new()
                .ascii(&self.config.fonts.heading_en)
                .hi_ansi(&self.config.fonts.heading_en)
                .east_asia(&self.config.fonts.heading_ja)
                .cs(&self.config.fonts.heading_en);
            let body_size = styles::pt_to_half_point(self.config.sizes.body);

            let table_number = self.next_table_number();

            let caption_para = match self.config.numbering.table_format.as_str() {
                "chapter" => {
                    // 章番号モード: "表X.Y" をプレーンテキストで生成
                    let label_run = Run::new()
                        .add_text(format!("表{}", table_number))
                        .size(body_size)
                        .bold()
                        .fonts(caption_fonts);
                    Paragraph::new()
                        .add_run(label_run)
                        .align(AlignmentType::Center)
                }
                _ => {
                    // 連番モード: Word SEQ フィールドを使用
                    let label_run = Run::new()
                        .add_text("表")
                        .size(body_size)
                        .bold()
                        .fonts(caption_fonts.clone());
                    let seq_run = Run::new()
                        .add_field_char(FieldCharType::Begin, true)
                        .add_instr_text(InstrText::Unsupported(
                            " SEQ Table \\* ARABIC ".to_string(),
                        ))
                        .add_field_char(FieldCharType::Separate, false)
                        .add_text(&table_number)
                        .add_field_char(FieldCharType::End, false)
                        .size(body_size)
                        .bold()
                        .fonts(caption_fonts);
                    Paragraph::new()
                        .add_run(label_run)
                        .add_run(seq_run)
                        .align(AlignmentType::Center)
                }
            };

            docx.add_paragraph(
                caption_para
                    .indent(
                        Some(self.config.table.margin_left),
                        None,
                        Some(self.config.table.margin_right),
                        None,
                    )
                    .keep_next(true),
            )
        } else {
            docx
        };
        let column_widths = build_table_grid(column_count, width);

        // ヘッダー行
        let header_cells: Vec<TableCell> = headers
            .iter()
            .enumerate()
            .map(|(index, cell_content)| {
                let mut para = Paragraph::new().align(AlignmentType::Center);
                for inline in cell_content {
                    para = self.add_inline_to_paragraph(
                        para,
                        inline,
                        true,
                        false,
                        false,
                        InlineStyle::TableHeader,
                    );
                }
                TableCell::new()
                    .width(column_widths[index], WidthType::Dxa)
                    .vertical_align(VAlignType::Center)
                    .add_paragraph(para)
            })
            .collect();

        let header_row = TableRow::new(header_cells).cant_split();

        // データ行
        let mut table_rows = vec![header_row];
        for row in rows {
            let cells: Vec<TableCell> = row
                .iter()
                .enumerate()
                .map(|(index, cell_content)| {
                    let mut para = Paragraph::new().align(table_alignment_to_paragraph_alignment(
                        alignments.get(index),
                    ));
                    for inline in cell_content {
                        para = self.add_inline_to_paragraph(
                            para,
                            inline,
                            false,
                            false,
                            false,
                            InlineStyle::TableBody,
                        );
                    }
                    TableCell::new()
                        .width(column_widths[index], WidthType::Dxa)
                        .vertical_align(VAlignType::Center)
                        .add_paragraph(para)
                })
                .collect();
            table_rows.push(TableRow::new(cells).cant_split());
        }

        let mut table = Table::new(table_rows)
            .align(TableAlignmentType::Center)
            .layout(TableLayoutType::Fixed)
            .width(TABLE_WIDTH_PCT, WidthType::Pct)
            .set_grid(column_widths)
            .margins(TableCellMargins::new().margin(
                TABLE_CELL_PADDING_TWIP,
                TABLE_CELL_PADDING_TWIP,
                TABLE_CELL_PADDING_TWIP,
                TABLE_CELL_PADDING_TWIP,
            ));
        let margins = &self.config.table;
        if margins.margin_left != 0 || margins.margin_right != 0 {
            table = table
                .align(TableAlignmentType::Left)
                .indent(margins.margin_left)
                .width(width, WidthType::Dxa);
        }
        let docx = docx.add_table(table);
        if margins.margin_bottom > 0 {
            docx.add_paragraph(outer_margin_spacer(margins.margin_bottom))
        } else {
            docx
        }
    }

    fn convert_code_block(&self, docx: Docx, lang: Option<&str>, code: &str) -> Docx {
        let _ = lang;
        // コードブロックは設定されたフォントで表示
        let fonts = RunFonts::new()
            .ascii(&self.config.fonts.code_en)
            .hi_ansi(&self.config.fonts.code_en)
            .east_asia(&self.config.fonts.code_ja)
            .cs(&self.config.fonts.code_en);

        let mut d = docx;
        let use_border = self.config.code_block.border;
        let margins = &self.config.code_block;
        let mut lines: Vec<&str> = code.lines().collect();
        // Wordの表セルには少なくとも一つの段落が必要。
        if use_border && lines.is_empty() {
            lines.push("");
        }
        let width = (i64::from(body_width_twip(&self.config.page))
            - i64::from(margins.margin_left)
            - i64::from(margins.margin_right))
        .max(1) as usize;
        let mut cell = TableCell::new().width(width, WidthType::Dxa);
        let n = lines.len();
        for (i, line) in lines.iter().enumerate() {
            let run = Run::new()
                .add_text(*line)
                .size(styles::pt_to_half_point(9.0))
                .fonts(fonts.clone());

            // 上下の余白は各行ではなくブロック全体の前後に適用する。
            let spacing = LineSpacing::new()
                .before(if !use_border && i == 0 {
                    margins.margin_top
                } else {
                    0
                })
                .after(if !use_border && i == n - 1 {
                    margins.margin_bottom
                } else {
                    0
                });
            let para = Paragraph::new()
                .add_run(run)
                .indent(
                    Some(if use_border { 0 } else { margins.margin_left }),
                    None,
                    Some(if use_border { 0 } else { margins.margin_right }),
                    None,
                )
                .line_spacing(spacing);
            if use_border {
                cell = cell.add_paragraph(para);
            } else {
                d = d.add_paragraph(para);
            }
        }
        if use_border {
            if margins.margin_top > 0 {
                d = d.add_paragraph(outer_margin_spacer(margins.margin_top).keep_next(true));
            }
            let table = Table::new(vec![TableRow::new(vec![cell])])
                .align(TableAlignmentType::Left)
                .indent(margins.margin_left)
                .layout(TableLayoutType::Fixed)
                .width(width, WidthType::Dxa)
                .set_grid(vec![width])
                .margins(TableCellMargins::new().margin(
                    TABLE_CELL_PADDING_TWIP,
                    TABLE_CELL_PADDING_TWIP,
                    TABLE_CELL_PADDING_TWIP,
                    TABLE_CELL_PADDING_TWIP,
                ))
                .set_borders(code_block_borders());
            d = d.add_table(table);
            if margins.margin_bottom > 0 {
                d = d.add_paragraph(outer_margin_spacer(margins.margin_bottom));
            }
        }
        d
    }

    fn convert_image(&mut self, docx: Docx, alt: &str, path: &str) -> Docx {
        let image_path = self.base_path.join(path);

        let buf = match std::fs::read(&image_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "警告: 画像ファイルを読み込めません: {} ({})",
                    image_path.display(),
                    e
                );
                // 画像が見つからない場合はaltテキストのみ表示
                let run = self.make_run(&format!("[画像: {}]", alt), InlineStyle::Body);
                return docx.add_paragraph(Paragraph::new().add_run(run));
            }
        };

        // 画像をPNGに変換しつつ寸法を取得する
        let (png_buf, width_px, height_px) = match convert_to_png_with_dimensions(&buf) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("警告: 画像の変換に失敗しました: {} ({})", path, e);
                let run = self.make_run(&format!("[画像: {}]", alt), InlineStyle::Body);
                return docx.add_paragraph(Paragraph::new().add_run(run));
            }
        };

        let (width_emu, height_emu) =
            fit_image_to_body_width(width_px, height_px, &self.config.page);
        let pic = Pic::new(&png_buf).size(width_emu, height_emu);

        let image_para = Paragraph::new()
            .add_run(Run::new().add_image(pic))
            .align(AlignmentType::Center);

        let docx = docx.add_paragraph(image_para);

        if !self.config.captions.figure {
            return docx;
        }

        // 図番号キャプション
        let caption_fonts = RunFonts::new()
            .ascii(&self.config.fonts.body_en)
            .hi_ansi(&self.config.fonts.body_en)
            .east_asia(&self.config.fonts.body_ja)
            .cs(&self.config.fonts.body_en);
        let body_size = styles::pt_to_half_point(self.config.sizes.body);

        let figure_number = self.next_figure_number();

        let mut caption_para = match self.config.numbering.figure_format.as_str() {
            "chapter" => {
                // 章番号モード: "図X.Y" をプレーンテキストで生成
                let label_run = Run::new()
                    .add_text(format!("図{}", figure_number))
                    .size(body_size)
                    .fonts(caption_fonts.clone());
                Paragraph::new()
                    .add_run(label_run)
                    .align(AlignmentType::Center)
            }
            _ => {
                // 連番モード: Word SEQ フィールドを使用
                let label_run = Run::new()
                    .add_text("図")
                    .size(body_size)
                    .fonts(caption_fonts.clone());
                let seq_run = Run::new()
                    .add_field_char(FieldCharType::Begin, true)
                    .add_instr_text(InstrText::Unsupported(
                        " SEQ Figure \\* ARABIC ".to_string(),
                    ))
                    .add_field_char(FieldCharType::Separate, false)
                    .add_text(&figure_number)
                    .add_field_char(FieldCharType::End, false)
                    .size(body_size)
                    .fonts(caption_fonts.clone());
                Paragraph::new()
                    .add_run(label_run)
                    .add_run(seq_run)
                    .align(AlignmentType::Center)
            }
        };

        if !alt.is_empty() {
            let alt_run = Run::new()
                .add_text(format!(" {}", alt))
                .size(body_size)
                .fonts(caption_fonts);
            caption_para = caption_para.add_run(alt_run);
        }

        docx.add_paragraph(caption_para)
    }
}

/// 画像データをPNG形式に変換し、元のピクセル寸法も返す
fn convert_to_png_with_dimensions(buf: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(buf)?;
    let (width, height) = img.dimensions();
    let mut png_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png)?;
    Ok((png_buf.into_inner(), width, height))
}

fn fit_image_to_body_width(width_px: u32, height_px: u32, page: &PageConfig) -> (u32, u32) {
    let width_emu = width_px as u64 * EMU_PER_PIXEL;
    let height_emu = height_px as u64 * EMU_PER_PIXEL;
    let body_width_twip = page
        .width
        .saturating_sub(page.margin_left.max(0) as u32)
        .saturating_sub(page.margin_right.max(0) as u32) as u64;
    let max_width_emu = body_width_twip * EMU_PER_TWIP;

    if width_emu <= max_width_emu {
        return (width_emu as u32, height_emu as u32);
    }

    let scaled_height_emu = height_emu * max_width_emu / width_emu;
    (max_width_emu as u32, scaled_height_emu as u32)
}

fn body_width_twip(page: &PageConfig) -> u32 {
    page.width
        .saturating_sub(page.margin_left.max(0) as u32)
        .saturating_sub(page.margin_right.max(0) as u32)
}

fn build_table_grid(column_count: usize, width: usize) -> Vec<usize> {
    let width = width.max(column_count);
    let base = width / column_count;
    let remainder = width % column_count;

    (0..column_count)
        .map(|index| base + usize::from(index < remainder))
        .collect()
}

fn table_alignment_to_paragraph_alignment(
    alignment: Option<&crate::ir::Alignment>,
) -> AlignmentType {
    match alignment {
        Some(crate::ir::Alignment::Center) => AlignmentType::Center,
        Some(crate::ir::Alignment::Right) => AlignmentType::Right,
        _ => AlignmentType::Left,
    }
}

/// テキスト処理: 英日間スペースの削除
fn process_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ' ' && i > 0 && i + 1 < chars.len() {
            let prev = chars[i - 1];
            let next = chars[i + 1];
            // 英語→スペース→日本語 or 日本語→スペース→英語 のスペースを削除
            if (is_ascii_char(prev) && is_japanese_char(next))
                || (is_japanese_char(prev) && is_ascii_char(next))
            {
                i += 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

fn is_ascii_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c.is_ascii_punctuation()
}

fn is_japanese_char(c: char) -> bool {
    matches!(c,
        '\u{3040}'..='\u{309F}' | // ひらがな
        '\u{30A0}'..='\u{30FF}' | // カタカナ
        '\u{4E00}'..='\u{9FFF}' | // CJK統合漢字
        '\u{3400}'..='\u{4DBF}' | // CJK統合漢字拡張A
        '\u{FF00}'..='\u{FFEF}' | // 全角文字
        '\u{3000}'..='\u{303F}'   // CJK記号
    )
}

/// 表の外側に指定した高さの空白を設ける。段落の既定行高は使用しない。
fn outer_margin_spacer(height: u32) -> Paragraph {
    Paragraph::new()
        .add_run(Run::new().size(2))
        .snap_to_grid(false)
        .line_spacing(
            LineSpacing::new()
                .before(0)
                .after(0)
                .line_rule(LineSpacingType::Exact)
                .line(height.min(i32::MAX as u32) as i32),
        )
}

/// コードブロック全体を囲む1×1の表の外枠（0.5pt）。
fn code_block_borders() -> TableBorders {
    let make = |pos| TableBorder::new(pos).size(4);
    TableBorders::with_empty()
        .set(make(TableBorderPosition::Top))
        .set(make(TableBorderPosition::Bottom))
        .set(make(TableBorderPosition::Left))
        .set(make(TableBorderPosition::Right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use docx_rs::{DocumentChild, HyperlinkData, ParagraphChild, RunChild};

    #[test]
    fn heading_numbering_can_be_disabled_with_or_without_shift() {
        for shift in [false, true] {
            for numbering in [false, true] {
                let mut config = Config::default();
                config.heading.heading_shift = shift;
                config.heading.numbering = numbering;
                config.numbering.table_format = "chapter".into();
                let mut markdown = String::new();
                if shift {
                    markdown.push_str("# Title\n\n");
                }
                for level in 1..=5 {
                    markdown.push_str(&format!(
                        "{} {} Heading{}\n\n",
                        "#".repeat(level + usize::from(shift)),
                        ["8", "8.1", "8.1.1", "(1)", "①"][level - 1],
                        level
                    ));
                }
                markdown.push_str("| A |\n|---|\n| B |\n");
                let blocks = crate::parser::parse_markdown(&markdown, false).unwrap();
                let built = convert_to_docx(&blocks, &config, Path::new("."))
                    .unwrap()
                    .build();
                let document = String::from_utf8(built.document).unwrap();
                let styles = String::from_utf8(built.styles).unwrap();
                assert_eq!(document.contains("<w:numPr>"), numbering);
                assert_eq!(styles.contains("<w:numPr>"), numbering);
                for (level, prefix) in ["8", "8.1", "8.1.1", "(1)", "①"].iter().enumerate() {
                    let text = if numbering {
                        format!("Heading{}", level + 1)
                    } else {
                        format!("{prefix} Heading{}", level + 1)
                    };
                    assert!(document.contains(&format!(">{text}</w:t>")));
                    assert!(document.contains(&format!("w:pStyle w:val=\"{}\"", level + 1)));
                }
                assert!(document.contains("表8.1"));
            }
        }
    }

    #[test]
    fn applies_outer_table_margins_with_or_without_caption() {
        let mut config: Config = toml::from_str(
            r#"
            [table]
            margin_top = 240
            margin_bottom = 120
            margin_left = 300
            margin_right = 500
            [page]
            width = 8001
            margin_left = 1000
            margin_right = 1000
        "#,
        )
        .unwrap();
        let blocks =
            crate::parser::parse_markdown("| A | B |\n|---|---|\n| C | D |", false).unwrap();
        for caption in [false, true] {
            config.captions.table = caption;
            let docx = convert_to_docx(&blocks, &config, Path::new(".")).unwrap();
            let children = &docx.document.children;
            assert_eq!(children.len(), 3 + usize::from(caption));
            for (index, height) in [(0, 240), (children.len() - 1, 120)] {
                let DocumentChild::Paragraph(p) = &children[index] else {
                    panic!("expected spacer")
                };
                let xml = String::from_utf8(p.build()).unwrap();
                assert!(xml.contains(&format!(r#"w:line="{height}""#)));
                assert!(xml.contains(r#"w:lineRule="exact""#));
            }
            if caption {
                let DocumentChild::Paragraph(p) = &children[1] else {
                    panic!("expected caption")
                };
                assert_eq!(p.property.indent.as_ref().unwrap().start, Some(300));
                assert_eq!(p.property.indent.as_ref().unwrap().end, Some(500));
            }
            let DocumentChild::Table(table) = &children[1 + usize::from(caption)] else {
                panic!("expected table")
            };
            let xml = String::from_utf8(table.build()).unwrap();
            assert!(xml.contains(r#"<w:tblW w:w="5201" w:type="dxa" />"#));
            assert!(xml.contains(r#"<w:tblInd w:w="300" w:type="dxa" />"#));
            for width in [2601, 2600] {
                assert!(xml.contains(&format!(r#"<w:gridCol w:w="{width}""#)));
                assert!(xml.contains(&format!(r#"<w:tcW w:w="{width}""#)));
            }
            assert!(xml.contains(r#"<w:tblCellMar><w:top w:w="80""#));
        }
    }

    #[test]
    fn toggles_table_and_figure_captions_without_removing_content() {
        let blocks = vec![
            Block::Table {
                headers: vec![vec![Inline::Text("Header".into())]],
                rows: vec![vec![vec![Inline::Text("Body".into())]]],
                alignments: vec![],
            },
            Block::Image {
                alt: "Figure description".into(),
                path: "examples/test.png".into(),
            },
        ];
        for format in ["sequential", "chapter"] {
            for (table, figure) in [(true, true), (false, true), (true, false), (false, false)] {
                let mut config = Config::default();
                config.numbering.table_format = format.into();
                config.numbering.figure_format = format.into();
                config.captions.table = table;
                config.captions.figure = figure;
                let docx = convert_to_docx(&blocks, &config, Path::new(env!("CARGO_MANIFEST_DIR")))
                    .unwrap();
                assert_eq!(
                    docx.document.children.len(),
                    2 + usize::from(table) + usize::from(figure)
                );
                let xml = String::from_utf8(docx.document.build()).unwrap();
                assert!(xml.contains("<w:tbl>"));
                assert!(xml.contains(">Header</w:t>"));
                assert!(xml.contains(">Body</w:t>"));
                assert!(xml.contains("<w:drawing>"));
                assert_eq!(xml.contains(">表"), table);
                assert_eq!(xml.contains(">図"), figure);
                assert_eq!(xml.contains("Figure description"), figure);
                assert_eq!(xml.contains("SEQ Table"), table && format == "sequential");
                assert_eq!(xml.contains("SEQ Figure"), figure && format == "sequential");
            }
        }
    }

    #[test]
    fn applies_code_block_fonts_and_margins_with_or_without_borders() {
        let mut config: Config = toml::from_str(
            r#"
            [fonts]
            code_ja = "BIZ UDゴシック"
            code_en = "Consolas"
            [code_block]
            margin_top = 240
            margin_bottom = 120
            margin_right = 360
            margin_left = 420
            "#,
        )
        .unwrap();

        for border in [false, true] {
            config.code_block.border = border;
            for (markdown, expected_spacing) in [
                (
                    "```rust\n先頭\n\n  last\n```\n",
                    vec![(240, 0), (0, 0), (0, 120)],
                ),
                ("    single\n", vec![(240, 120)]),
                ("```\n```\n", if border { vec![(240, 120)] } else { vec![] }),
            ] {
                let blocks = crate::parser::parse_markdown(markdown, false).unwrap();
                let docx = convert_to_docx(&blocks, &config, Path::new(".")).unwrap();
                let paragraphs: Vec<&Paragraph> = if border {
                    assert_eq!(docx.document.children.len(), 3);
                    for (index, height) in [(0, 240), (2, 120)] {
                        let DocumentChild::Paragraph(spacer) = &docx.document.children[index]
                        else {
                            panic!("expected outer spacer");
                        };
                        let xml = String::from_utf8(spacer.build()).unwrap();
                        assert!(xml.contains(&format!(r#"w:line="{height}""#)));
                        assert!(xml.contains(r#"w:lineRule="exact""#));
                        assert!(xml.contains(r#"w:before="0""#));
                        assert!(xml.contains(r#"w:after="0""#));
                    }
                    let DocumentChild::Table(table) = &docx.document.children[1] else {
                        panic!("expected code table");
                    };
                    assert_eq!(table.rows.len(), 1);
                    let TableChild::TableRow(row) = &table.rows[0];
                    assert_eq!(row.cells.len(), 1);
                    let TableRowChild::TableCell(cell) = &row.cells[0];
                    let xml = String::from_utf8(table.build()).unwrap();
                    assert!(xml.contains("<w:tblBorders>"));
                    for side in ["top", "bottom", "left", "right"] {
                        assert!(xml.contains(&format!(r#"<w:{side} w:val="single" w:sz="4""#)));
                    }
                    assert!(xml.contains(r#"<w:tblLayout w:type="fixed" />"#));
                    let width = body_width_twip(&config.page) - 420 - 360;
                    assert!(xml.contains(r#"<w:tblInd w:w="420" w:type="dxa" />"#));
                    assert!(xml.contains(&format!(r#"<w:gridCol w:w="{width}""#)));
                    assert!(xml.contains(&format!(r#"<w:tcW w:w="{width}""#)));
                    assert!(xml.contains(&format!(r#"<w:tblW w:w="{width}" w:type="dxa" />"#)));
                    assert!(!xml.contains("SEQ Table"));
                    cell.children
                        .iter()
                        .map(|child| match child {
                            TableCellContent::Paragraph(p) => p,
                            other => panic!("unexpected cell content: {other:?}"),
                        })
                        .collect()
                } else {
                    docx.document
                        .children
                        .iter()
                        .map(|child| match child {
                            DocumentChild::Paragraph(p) => p.as_ref(),
                            other => panic!("unexpected document content: {other:?}"),
                        })
                        .collect()
                };
                assert_eq!(paragraphs.len(), expected_spacing.len());
                for (p, (before, after)) in paragraphs.into_iter().zip(expected_spacing) {
                    let (before, after) = if border { (0, 0) } else { (before, after) };
                    let xml = String::from_utf8(p.build()).unwrap();
                    for attribute in ["ascii", "hAnsi", "cs"] {
                        assert!(xml.contains(&format!(r#"w:{attribute}="Consolas""#)));
                    }
                    assert!(xml.contains(r#"w:eastAsia="BIZ UDゴシック""#));
                    assert!(xml.contains(r#"<w:sz w:val="18" />"#));
                    assert!(xml.contains(&format!(r#"w:before="{before}""#)));
                    assert!(xml.contains(&format!(r#"w:after="{after}""#)));
                    assert_eq!(
                        p.property.indent.as_ref().unwrap().end,
                        Some(if border { 0 } else { 360 })
                    );
                    assert_eq!(
                        p.property.indent.as_ref().unwrap().start,
                        Some(if border { 0 } else { 420 })
                    );
                    assert!(!xml.contains("<w:pBdr>"));
                }
                let xml = String::from_utf8(docx.document.build()).unwrap();
                if markdown.contains("last") {
                    assert!(xml.contains(r#"<w:t xml:space="preserve">  last</w:t>"#));
                }
            }
        }
    }

    #[test]
    fn bordered_code_adds_spacers_only_for_nonzero_outer_margins() {
        let mut config = Config::default();
        config.code_block.border = true;
        config.page.width = 8_000;
        config.page.margin_left = 1_000;
        config.page.margin_right = 1_000;
        config.code_block.margin_left = 300;
        config.code_block.margin_right = 500;
        let blocks = crate::parser::parse_markdown("```\ncode\n```", false).unwrap();
        for (top, bottom) in [(0, 0), (100, 0), (0, 200)] {
            config.code_block.margin_top = top;
            config.code_block.margin_bottom = bottom;
            let docx = convert_to_docx(&blocks, &config, Path::new(".")).unwrap();
            assert_eq!(
                docx.document.children.len(),
                1 + usize::from(top > 0) + usize::from(bottom > 0)
            );
            let DocumentChild::Table(table) = &docx.document.children[usize::from(top > 0)] else {
                panic!("expected table between outer spacers");
            };
            let xml = String::from_utf8(table.build()).unwrap();
            assert!(xml.contains(r#"<w:tblW w:w="5200" w:type="dxa" />"#));
            assert!(xml.contains(r#"<w:tblInd w:w="300" w:type="dxa" />"#));
        }
    }

    #[test]
    fn applies_markdown_bold_and_italic_to_word_runs() {
        let blocks = vec![Block::Paragraph {
            content: vec![
                Inline::Text("normal".to_string()),
                Inline::Bold(vec![Inline::Text("bold".to_string())]),
                Inline::Italic(vec![Inline::Text("italic".to_string())]),
                Inline::Bold(vec![Inline::Italic(vec![Inline::Text("both".to_string())])]),
            ],
        }];

        let docx = convert_to_docx(&blocks, &Config::default(), Path::new(".")).unwrap();
        let para = match &docx.document.children[0] {
            DocumentChild::Paragraph(p) => p,
            other => panic!("unexpected child: {other:?}"),
        };
        let runs: Vec<_> = para
            .children
            .iter()
            .filter_map(|child| match child {
                ParagraphChild::Run(run) => Some(run),
                _ => None,
            })
            .collect();

        assert_eq!(runs.len(), 4);
        assert!(runs[0].run_property.bold.is_none());
        assert!(runs[0].run_property.italic.is_none());
        assert!(runs[1].run_property.bold.is_some());
        assert!(runs[1].run_property.italic.is_none());
        assert!(runs[2].run_property.bold.is_none());
        assert!(runs[2].run_property.italic.is_some());
        assert!(runs[3].run_property.bold.is_some());
        assert!(runs[3].run_property.italic.is_some());
    }

    #[test]
    fn applies_configured_equal_markup_size_and_background() {
        let mut config = Config::default();
        config.equal.enabled = true;
        config.equal.font_size = Some(18.0);
        config.equal.background_color = Some("#FFFF00".to_string());
        let blocks = crate::parser::parse_markdown("==marked==", config.equal.enabled).unwrap();

        let docx = convert_to_docx(&blocks, &config, Path::new(".")).unwrap();
        let xml = String::from_utf8(docx.document.build()).unwrap();

        assert!(xml.contains(r#"<w:sz w:val="36" />"#));
        assert!(xml.contains(r#"<w:shd w:val="clear" w:color="auto" w:fill="FFFF00" />"#));
        assert!(!xml.contains("==marked=="));
    }

    #[test]
    fn converts_inline_link_to_word_hyperlink() {
        let blocks = vec![Block::Paragraph {
            content: vec![Inline::Link {
                text: vec![Inline::Text("Rust".to_string())],
                url: "https://www.rust-lang.org/".to_string(),
            }],
        }];

        let docx = convert_to_docx(&blocks, &Config::default(), Path::new(".")).unwrap();
        let para = match &docx.document.children[0] {
            DocumentChild::Paragraph(p) => p,
            other => panic!("unexpected child: {other:?}"),
        };

        let hyperlink = para
            .children
            .iter()
            .find_map(|child| match child {
                ParagraphChild::Hyperlink(link) => Some(link),
                _ => None,
            })
            .expect("hyperlink should exist");

        match &hyperlink.link {
            HyperlinkData::External { path, .. } => {
                assert_eq!(path, "https://www.rust-lang.org/");
            }
            other => panic!("unexpected hyperlink type: {other:?}"),
        }

        let link_text = hyperlink
            .children
            .iter()
            .find_map(|child| match child {
                ParagraphChild::Run(run) => {
                    run.children.iter().find_map(|run_child| match run_child {
                        RunChild::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                }
                _ => None,
            })
            .expect("hyperlink text should exist");
        assert_eq!(link_text, "Rust");
    }

    #[test]
    fn inserts_space_for_soft_break() {
        let blocks = vec![Block::Paragraph {
            content: vec![
                Inline::Text("foo".to_string()),
                Inline::SoftBreak,
                Inline::Text("bar".to_string()),
            ],
        }];

        let docx = convert_to_docx(&blocks, &Config::default(), Path::new(".")).unwrap();
        let para = match &docx.document.children[0] {
            DocumentChild::Paragraph(p) => p,
            other => panic!("unexpected child: {other:?}"),
        };

        let mut joined = String::new();
        for child in &para.children {
            if let ParagraphChild::Run(run) = child {
                for run_child in &run.children {
                    if let RunChild::Text(t) = run_child {
                        joined.push_str(&t.text);
                    }
                }
            }
        }

        assert_eq!(joined, "foo bar");
    }

    #[test]
    fn converts_page_break_block_to_word_page_break() {
        let docx =
            convert_to_docx(&[Block::PageBreak], &Config::default(), Path::new(".")).unwrap();
        let xml = String::from_utf8(docx.document.build()).unwrap();
        assert!(xml.contains(r#"<w:br w:type="page" />"#));
    }

    #[test]
    fn indents_nested_ordered_lists_by_depth() {
        let nested = Block::OrderedList {
            start: 1,
            items: vec![ListItem {
                content: vec![Inline::Text("outer".to_string())],
                children: vec![Block::OrderedList {
                    start: 1,
                    items: vec![ListItem {
                        content: vec![Inline::Text("inner".to_string())],
                        children: vec![],
                    }],
                }],
            }],
        };

        let docx = convert_to_docx(&[nested], &Config::default(), Path::new(".")).unwrap();
        let indents: Vec<Option<i32>> = docx
            .document
            .children
            .iter()
            .filter_map(|child| match child {
                DocumentChild::Paragraph(p) => {
                    Some(p.property.indent.as_ref().and_then(|i| i.start))
                }
                _ => None,
            })
            .collect();

        assert_eq!(indents.len(), 2);
        assert_eq!(indents[0], Some(360));
        assert_eq!(indents[1], Some(720));
    }

    #[test]
    fn shrinks_wide_images_to_body_width() {
        let (width_emu, height_emu) = fit_image_to_body_width(2532, 729, &Config::default().page);
        assert_eq!(width_emu, 5_400_040);
        assert!(height_emu < width_emu);
    }

    #[test]
    fn keeps_small_images_original_size() {
        let (width_emu, height_emu) = fit_image_to_body_width(382, 376, &Config::default().page);
        assert_eq!(width_emu, 3_638_550);
        assert_eq!(height_emu, 3_581_400);
    }

    #[test]
    fn uses_configured_page_width_for_image_scaling() {
        let mut config = Config::default();
        config.page.width = 8_000;
        config.page.margin_left = 1_000;
        config.page.margin_right = 1_000;

        let (width_emu, height_emu) = fit_image_to_body_width(2532, 729, &config.page);
        assert_eq!(width_emu, 3_810_000);
        assert_eq!(height_emu, 1_096_954);
    }

    #[test]
    fn makes_table_full_width_with_padding_and_centered_headers() {
        let blocks = vec![Block::Table {
            headers: vec![
                vec![Inline::Text("H1".to_string())],
                vec![Inline::Text("H2".to_string())],
            ],
            rows: vec![vec![
                vec![Inline::Text("L".to_string())],
                vec![Inline::Text("R".to_string())],
            ]],
            alignments: vec![crate::ir::Alignment::Left, crate::ir::Alignment::Right],
        }];

        let docx = convert_to_docx(&blocks, &Config::default(), Path::new(".")).unwrap();
        let xml = String::from_utf8(docx.document.build()).unwrap();

        assert!(xml.contains(r#"<w:tblW w:w="5000" w:type="pct" />"#));
        assert!(xml.contains(r#"<w:tblLayout w:type="fixed" />"#));
        assert!(xml.contains(r#"<w:tblCellMar><w:top w:w="80" w:type="dxa" /><w:left w:w="80" w:type="dxa" /><w:bottom w:w="80" w:type="dxa" /><w:right w:w="80" w:type="dxa" /></w:tblCellMar>"#));
        assert!(xml.contains(
            r#"<w:gridCol w:w="4252" w:type="dxa" /><w:gridCol w:w="4252" w:type="dxa" />"#
        ));
        assert!(xml.contains(r#"<w:jc w:val="center" />"#));
        assert!(xml.contains(r#"<w:jc w:val="right" />"#));
        assert!(xml.contains(r#"<w:sz w:val="19" />"#));
    }

    #[test]
    fn uses_configured_table_font_sizes() {
        let blocks = vec![Block::Table {
            headers: vec![vec![Inline::Text("Header".to_string())]],
            rows: vec![vec![vec![Inline::Text("Body".to_string())]]],
            alignments: vec![crate::ir::Alignment::Left],
        }];

        let mut config = Config::default();
        config.sizes.table_header = 8.5;
        config.sizes.table_body = 8.0;

        let docx = convert_to_docx(&blocks, &config, Path::new(".")).unwrap();
        let xml = String::from_utf8(docx.document.build()).unwrap();

        assert!(xml.contains(r#"<w:t xml:space="preserve">Header</w:t>"#));
        assert!(xml.contains(r#"<w:t xml:space="preserve">Body</w:t>"#));
        assert!(xml.contains(r#"<w:sz w:val="17" />"#));
        assert!(xml.contains(r#"<w:sz w:val="16" />"#));
    }
}
