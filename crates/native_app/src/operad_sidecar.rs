use eframe::egui::{self, Sense, vec2};
use operad::{
    ApproxTextMeasurer, ClipBehavior, ColorRgba, FontWeight, InputBehavior, StrokeStyle, TextStyle,
    TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiSize, UiVisual, layout, root_style,
    widgets,
};

use crate::{operad_egui, ui_chrome::Tone};

const PAD: f32 = 10.0;
const GAP: f32 = 8.0;
const SECTION_TITLE_HEIGHT: f32 = 24.0;
const ROW_HEIGHT: f32 = 44.0;
const EMPTY_HEIGHT: f32 = 36.0;

#[derive(Clone, Debug)]
pub(crate) struct SidecarSection {
    pub(crate) title: String,
    pub(crate) empty: String,
    pub(crate) rows: Vec<SidecarRow>,
}

#[derive(Clone, Debug)]
pub(crate) struct SidecarRow {
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) tone: Tone,
    pub(crate) selected: bool,
    pub(crate) action_name: Option<String>,
}

#[derive(Debug)]
pub(crate) struct SidecarView {
    pub(crate) document: UiDocument,
    pub(crate) size: UiSize,
}

impl SidecarSection {
    pub(crate) fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            empty: "No records".to_string(),
            rows: Vec::new(),
        }
    }

    pub(crate) fn empty(mut self, empty: impl Into<String>) -> Self {
        self.empty = empty.into();
        self
    }

    pub(crate) fn row(mut self, row: SidecarRow) -> Self {
        self.rows.push(row);
        self
    }
}

impl SidecarRow {
    pub(crate) fn new(title: impl Into<String>, detail: impl Into<String>, tone: Tone) -> Self {
        Self {
            title: title.into(),
            detail: detail.into(),
            tone,
            selected: false,
            action_name: None,
        }
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn action(mut self, action_name: impl Into<String>) -> Self {
        self.action_name = Some(action_name.into());
        self
    }
}

pub(crate) fn render_sidecar(
    ui: &mut egui::Ui,
    id: &str,
    sections: &[SidecarSection],
) -> Result<(), String> {
    render_sidecar_interactive(ui, id, sections).map(|_| ())
}

pub(crate) fn render_sidecar_interactive(
    ui: &mut egui::Ui,
    id: &str,
    sections: &[SidecarSection],
) -> Result<Option<String>, String> {
    let width = ui.available_width().max(160.0);
    let mut view = build_sidecar_view(id, width, sections);
    view.document
        .compute_layout(view.size, &mut ApproxTextMeasurer)
        .map_err(|error| error.to_string())?;
    let (rect, response) = ui.allocate_exact_size(vec2(width, view.size.height), Sense::click());
    let mut action = None;
    if response.clicked()
        && let Some(pointer) = response.interact_pointer_pos()
        && let Some(node_name) = operad_egui::hit_test_name(&view.document, rect, pointer)
        && is_sidecar_action(sections, &node_name)
    {
        action = Some(node_name);
    }
    if response.hovered()
        && let Some(pointer) = ui.ctx().pointer_hover_pos()
        && let Some(node_name) = operad_egui::hit_test_name(&view.document, rect, pointer)
        && is_sidecar_action(sections, &node_name)
    {
        ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::PointingHand);
    }
    operad_egui::paint_document_at(ui, &view.document, rect);
    Ok(action)
}

pub(crate) fn build_sidecar_view(id: &str, width: f32, sections: &[SidecarSection]) -> SidecarView {
    let height = sections
        .iter()
        .map(section_height)
        .sum::<f32>()
        .max(EMPTY_HEIGHT)
        + GAP * sections.len().saturating_sub(1) as f32;
    let size = UiSize::new(width, height);
    let mut document = UiDocument::new(root_style(width, height));
    let root = document.root;
    document.set_node_visual(
        root,
        UiVisual::panel(ColorRgba::new(15, 18, 21, 255), None, 0.0),
    );

    for (index, section) in sections.iter().enumerate() {
        add_section(&mut document, root, id, index, width, section);
        if index + 1 < sections.len() {
            add_spacer(&mut document, root, id, index);
        }
    }

    SidecarView { document, size }
}

fn section_height(section: &SidecarSection) -> f32 {
    PAD * 2.0
        + SECTION_TITLE_HEIGHT
        + if section.rows.is_empty() {
            EMPTY_HEIGHT
        } else {
            ROW_HEIGHT * section.rows.len() as f32
        }
}

fn add_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    id: &str,
    index: usize,
    width: f32,
    section: &SidecarSection,
) {
    let section_node = document.add_child(
        parent,
        UiNode::container(
            format!("{id}.section.{index}"),
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(section_height(section)),
                    ),
                    PAD,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(21, 25, 29, 255),
            Some(StrokeStyle::new(ColorRgba::new(43, 50, 58, 255), 1.0)),
            5.0,
        )),
    );
    widgets::label(
        document,
        section_node,
        format!("{id}.section.{index}.title"),
        truncate(&section.title, 42),
        text_style(14.0, FontWeight::BOLD, ColorRgba::new(236, 240, 244, 255)),
        layout::with_size(
            layout::row(),
            layout::percent(1.0),
            layout::px(SECTION_TITLE_HEIGHT),
        ),
    );
    if section.rows.is_empty() {
        add_empty_row(document, section_node, id, index, &section.empty);
    } else {
        for (row_index, row) in section.rows.iter().enumerate() {
            add_row(document, section_node, id, index, row_index, width, row);
        }
    }
}

fn add_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    id: &str,
    section_index: usize,
    row_index: usize,
    width: f32,
    row: &SidecarRow,
) {
    let fill = if row.selected {
        ColorRgba::new(27, 42, 56, 255)
    } else {
        ColorRgba::new(26, 31, 36, 255)
    };
    let stroke = if row.selected {
        tone_color(Tone::Info)
    } else {
        ColorRgba::new(41, 48, 56, 255)
    };
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{id}.section.{section_index}.row.{row_index}"));
    let mut row_node = UiNode::container(
        row_name,
        UiNodeStyle {
            layout: layout::with_padding_all(
                layout::with_size(layout::row(), layout::percent(1.0), layout::px(ROW_HEIGHT)),
                5.0,
            ),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(UiVisual::panel(
        fill,
        Some(StrokeStyle::new(stroke, 1.0)),
        4.0,
    ));
    if row.action_name.is_some() {
        row_node = row_node.with_input(InputBehavior::BUTTON);
    }
    let row_node = document.add_child(parent, row_node);
    document.add_child(
        row_node,
        UiNode::container(
            format!("{id}.section.{section_index}.row.{row_index}.tone"),
            UiNodeStyle {
                layout: layout::fixed(4.0, ROW_HEIGHT - 10.0),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(tone_color(row.tone), None, 2.0)),
    );
    let text_width = (width - PAD * 2.0 - 20.0).max(96.0);
    let text = document.add_child(
        row_node,
        UiNode::container(
            format!("{id}.section.{section_index}.row.{row_index}.text"),
            UiNodeStyle {
                layout: layout::with_size(
                    layout::column(),
                    layout::px(text_width),
                    layout::px(ROW_HEIGHT - 10.0),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    widgets::label(
        document,
        text,
        format!("{id}.section.{section_index}.row.{row_index}.title"),
        truncate(&row.title, 56),
        text_style(13.0, FontWeight::BOLD, ColorRgba::new(228, 234, 240, 255)),
        layout::with_size(layout::row(), layout::percent(1.0), layout::px(18.0)),
    );
    widgets::label(
        document,
        text,
        format!("{id}.section.{section_index}.row.{row_index}.detail"),
        truncate(&row.detail, 76),
        text_style(11.0, FontWeight::NORMAL, ColorRgba::new(156, 164, 172, 255)),
        layout::with_size(layout::row(), layout::percent(1.0), layout::px(16.0)),
    );
}

fn add_empty_row(document: &mut UiDocument, parent: UiNodeId, id: &str, index: usize, empty: &str) {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{id}.section.{index}.empty"),
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(EMPTY_HEIGHT),
                    ),
                    7.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(26, 31, 36, 255),
            Some(StrokeStyle::new(ColorRgba::new(41, 48, 56, 255), 1.0)),
            4.0,
        )),
    );
    widgets::label(
        document,
        row,
        format!("{id}.section.{index}.empty.label"),
        truncate(empty, 68),
        text_style(12.0, FontWeight::NORMAL, ColorRgba::new(150, 158, 166, 255)),
        layout::with_size(layout::row(), layout::percent(1.0), layout::px(18.0)),
    );
}

fn add_spacer(document: &mut UiDocument, parent: UiNodeId, id: &str, index: usize) {
    document.add_child(
        parent,
        UiNode::container(
            format!("{id}.spacer.{index}"),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(GAP)),
                ..Default::default()
            },
        ),
    );
}

fn text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn is_sidecar_action(sections: &[SidecarSection], node_name: &str) -> bool {
    sections
        .iter()
        .flat_map(|section| section.rows.iter())
        .any(|row| row.action_name.as_deref() == Some(node_name))
}

fn truncate(text: impl AsRef<str>, max_chars: usize) -> String {
    let text = text.as_ref();
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let head = keep / 2;
    let tail = keep - head;
    let start = text.chars().take(head).collect::<String>();
    let end = text
        .chars()
        .rev()
        .take(tail)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{start}...{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_layout_audits_common_widths() {
        let sections = vec![
            SidecarSection::new("Context").row(SidecarRow::new(
                "Selected item",
                "Compact detail text for the side panel",
                Tone::Info,
            )),
            SidecarSection::new("Status").empty("Nothing pending"),
        ];
        for width in [180.0, 240.0, 320.0] {
            let mut view = build_sidecar_view("test_sidecar", width, &sections);
            view.document
                .compute_layout(view.size, &mut ApproxTextMeasurer)
                .expect("sidecar layout should compute");
            let warnings = view.document.audit_layout();
            assert!(warnings.is_empty(), "{width}: {warnings:?}");
            assert!(view.document.paint_list().items.len() > 0);
        }
    }
}
