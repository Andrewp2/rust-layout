#![allow(dead_code)]

use std::ops::Range;

use operad::{
    ApproxTextMeasurer, AuditWarning, ClipBehavior, ColorRgba, FontWeight, ScrollState,
    StrokeStyle, TextStyle, TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiSize, UiVisual,
    layout, root_style, widgets,
};

const PANEL_PADDING: f32 = 12.0;
const TOOLBAR_HEIGHT: f32 = 36.0;
const DIVIDER_HEIGHT: f32 = 1.0;
const ROW_HEIGHT: f32 = 30.0;
const MIN_LIST_HEIGHT: f32 = 96.0;

pub(crate) const SIDEBAR_MODULES_DEFAULT_BUTTON: &str = "sidebar_modules.toolbar.default";
pub(crate) const SIDEBAR_MODULES_ALL_BUTTON: &str = "sidebar_modules.toolbar.all";
pub(crate) const SIDEBAR_MODULES_NONE_BUTTON: &str = "sidebar_modules.toolbar.none";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SidebarModuleRowKind {
    Group,
    Module { shown: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SidebarModuleRow {
    pub(crate) label: &'static str,
    pub(crate) kind: SidebarModuleRowKind,
}

impl SidebarModuleRow {
    pub(crate) fn group(label: &'static str) -> Self {
        Self {
            label,
            kind: SidebarModuleRowKind::Group,
        }
    }

    pub(crate) fn module(label: &'static str, shown: bool) -> Self {
        Self {
            label,
            kind: SidebarModuleRowKind::Module { shown },
        }
    }
}

#[derive(Debug)]
pub(crate) struct SidebarModulesOperadAudit {
    pub(crate) warnings: Vec<AuditWarning>,
    pub(crate) node_count: usize,
    pub(crate) paint_items: usize,
    pub(crate) row_count: usize,
    pub(crate) module_count: usize,
    pub(crate) visible_range: Range<usize>,
    pub(crate) modeled_content_height: f32,
    pub(crate) scroll: ScrollState,
}

pub(crate) struct SidebarModulesOperadView {
    pub(crate) document: UiDocument,
    pub(crate) list_id: UiNodeId,
    pub(crate) visible_range: Range<usize>,
    pub(crate) modeled_content_height: f32,
}

pub(crate) fn audit_sidebar_modules_layout(
    rows: &[SidebarModuleRow],
    viewport: UiSize,
    scroll_offset: f32,
) -> Result<SidebarModulesOperadAudit, String> {
    let mut view = build_sidebar_modules_view(rows, viewport, scroll_offset);
    view.document
        .compute_layout(viewport, &mut ApproxTextMeasurer)
        .map_err(|error| error.to_string())?;
    let scroll = view
        .document
        .scroll_state(view.list_id)
        .ok_or_else(|| "sidebar modules list did not create a scroll node".to_string())?;
    Ok(SidebarModulesOperadAudit {
        warnings: view.document.audit_layout(),
        node_count: view.document.node_count(),
        paint_items: view.document.paint_list().items.len(),
        row_count: rows.len(),
        module_count: rows
            .iter()
            .filter(|row| matches!(row.kind, SidebarModuleRowKind::Module { .. }))
            .count(),
        visible_range: view.visible_range,
        modeled_content_height: view.modeled_content_height,
        scroll,
    })
}

pub(crate) fn build_sidebar_modules_view(
    rows: &[SidebarModuleRow],
    viewport: UiSize,
    scroll_offset: f32,
) -> SidebarModulesOperadView {
    let mut document = UiDocument::new(root_style(viewport.width, viewport.height));
    let root = document.root;
    let panel = document.add_child(
        root,
        UiNode::container(
            "sidebar_modules.panel",
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::percent(1.0)),
                    PANEL_PADDING,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(17, 22, 27, 255),
            Some(StrokeStyle::new(ColorRgba::new(58, 66, 76, 255), 1.0)),
            6.0,
        )),
    );

    add_toolbar(&mut document, panel);
    document.add_child(
        panel,
        UiNode::container(
            "sidebar_modules.divider",
            UiNodeStyle {
                layout: layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(DIVIDER_HEIGHT),
                ),
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(ColorRgba::new(58, 66, 76, 255), None, 0.0)),
    );

    let list_height = (viewport.height - PANEL_PADDING * 2.0 - TOOLBAR_HEIGHT - DIVIDER_HEIGHT)
        .max(MIN_LIST_HEIGHT);
    let spec = widgets::VirtualListSpec {
        row_count: rows.len(),
        row_height: ROW_HEIGHT,
        viewport_height: list_height,
        scroll_offset,
        overscan: 0,
    };
    let visible_range = spec.visible_range();
    let modeled_content_height = spec.total_height();
    let list = widgets::virtual_list(
        &mut document,
        panel,
        "sidebar_modules.list",
        spec,
        |document, parent, row_index| {
            add_sidebar_modules_row(document, parent, row_index, rows[row_index]);
        },
    );

    SidebarModulesOperadView {
        document,
        list_id: list,
        visible_range,
        modeled_content_height,
    }
}

pub(crate) fn sidebar_module_row_index(node_name: &str) -> Option<usize> {
    let rest = node_name.strip_prefix("sidebar_modules.row.")?;
    let (index, suffix) = rest.split_once('.')?;
    (suffix == "module")
        .then(|| index.parse::<usize>().ok())
        .flatten()
}

fn add_toolbar(document: &mut UiDocument, parent: UiNodeId) {
    let toolbar = document.add_child(
        parent,
        UiNode::container(
            "sidebar_modules.toolbar",
            UiNodeStyle {
                layout: layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(TOOLBAR_HEIGHT),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );

    for (name, label) in [
        (SIDEBAR_MODULES_DEFAULT_BUTTON, "Default"),
        (SIDEBAR_MODULES_ALL_BUTTON, "All"),
        (SIDEBAR_MODULES_NONE_BUTTON, "None"),
    ] {
        let mut options =
            widgets::ButtonOptions::new(layout::with_margin_all(layout::fixed(78.0, 30.0), 2.0));
        options.text_style = body_text(ColorRgba::new(232, 236, 240, 255));
        options.visual = UiVisual::panel(
            ColorRgba::new(43, 48, 55, 255),
            Some(StrokeStyle::new(ColorRgba::new(74, 84, 96, 255), 1.0)),
            5.0,
        );
        widgets::button(document, toolbar, name, label, options);
    }
}

fn add_sidebar_modules_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    row_index: usize,
    row: SidebarModuleRow,
) {
    match row.kind {
        SidebarModuleRowKind::Group => add_group_row(document, parent, row_index, row.label),
        SidebarModuleRowKind::Module { shown } => {
            let mut options = widgets::CheckboxOptions::default();
            options.layout =
                layout::with_size(layout::row(), layout::percent(1.0), layout::px(ROW_HEIGHT));
            options.box_visual = if shown {
                UiVisual::panel(
                    ColorRgba::new(65, 116, 157, 255),
                    Some(StrokeStyle::new(ColorRgba::new(111, 174, 224, 255), 1.0)),
                    4.0,
                )
            } else {
                UiVisual::panel(
                    ColorRgba::new(55, 59, 64, 255),
                    Some(StrokeStyle::new(ColorRgba::new(72, 78, 86, 255), 1.0)),
                    4.0,
                )
            };
            options.text_style = body_text(if shown {
                ColorRgba::new(230, 235, 240, 255)
            } else {
                ColorRgba::new(166, 172, 178, 255)
            });
            widgets::checkbox(
                document,
                parent,
                format!("sidebar_modules.row.{row_index}.module"),
                row.label,
                shown,
                options,
            );
        }
    }
}

fn add_group_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    row_index: usize,
    label: &'static str,
) {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("sidebar_modules.row.{row_index}.group"),
            UiNodeStyle {
                layout: layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ROW_HEIGHT),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    widgets::label(
        document,
        row,
        format!("sidebar_modules.row.{row_index}.group.label"),
        label,
        group_text(),
        layout::with_margin_all(
            layout::with_size(layout::row(), layout::percent(1.0), layout::px(ROW_HEIGHT)),
            2.0,
        ),
    );
}

fn body_text(color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size: 15.0,
        line_height: 20.0,
        wrap: TextWrap::None,
        color,
        ..Default::default()
    }
}

fn group_text() -> TextStyle {
    TextStyle {
        font_size: 14.0,
        line_height: 20.0,
        weight: FontWeight::BOLD,
        wrap: TextWrap::None,
        color: ColorRgba::new(245, 247, 250, 255),
        ..Default::default()
    }
}
