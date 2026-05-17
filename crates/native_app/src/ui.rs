use operad::{UiDocument, UiNode, UiNodeId, layout};

const PRE_SHELL_NODE_ID_BUDGET: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiScale {
    factor: f32,
}

impl UiScale {
    pub fn new(factor: f32) -> Self {
        Self {
            factor: factor.clamp(1.0, 3.0),
        }
    }

    pub fn factor(self) -> f32 {
        self.factor
    }

    pub(crate) fn value(self, value: f32) -> f32 {
        value * self.factor
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShellMetrics {
    pub compact_primary_rows: bool,
    pub wide_primary_rows: bool,
    pub nav: NavRailMetrics,
    pub show_secondary_panel: bool,
    pub body_width: f32,
}

impl ShellMetrics {
    pub(crate) fn new(options: ShellMetricsOptions) -> Self {
        let compact_nav = options.viewport_width < options.ui_scale.value(700.0);
        let nav = NavRailMetrics::new(options.ui_scale, compact_nav);
        let show_secondary_panel =
            options.has_secondary_panel && (options.show_layers || options.inline_side_panels);
        let reserved_secondary_width = if show_secondary_panel {
            secondary_panel_width(options.ui_scale)
        } else {
            0.0
        };
        let reserved_inspector_width = if options.show_inspector && options.has_inspector_panel {
            inspector_panel_width(options.ui_scale)
        } else {
            0.0
        };
        let body_width = (options.viewport_width
            - nav.width
            - reserved_secondary_width
            - reserved_inspector_width)
            .max(0.0);

        Self {
            compact_primary_rows: options.viewport_width < options.ui_scale.value(1180.0),
            wide_primary_rows: body_width > options.ui_scale.value(1280.0),
            nav,
            show_secondary_panel,
            body_width,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShellMetricsOptions {
    pub viewport_width: f32,
    pub ui_scale: UiScale,
    pub has_secondary_panel: bool,
    pub show_layers: bool,
    pub inline_side_panels: bool,
    pub show_inspector: bool,
    pub has_inspector_panel: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NavRailMetrics {
    pub width: f32,
    pub gap: f32,
    pub padding: f32,
    pub button_height: f32,
    pub button_text_scale: UiScale,
}

impl NavRailMetrics {
    fn new(ui_scale: UiScale, compact: bool) -> Self {
        let font_size = ui_scale.value(if compact { 12.0 } else { 13.0 });
        Self {
            width: ui_scale.value(if compact { 150.0 } else { 240.0 }),
            gap: ui_scale.value(if compact { 5.0 } else { 8.0 }),
            padding: ui_scale.value(if compact { 8.0 } else { 14.0 }),
            button_height: ui_scale.value(if compact { 26.0 } else { 28.0 }),
            button_text_scale: UiScale::new(font_size / 13.0),
        }
    }
}

pub(crate) fn inspector_panel_width(ui_scale: UiScale) -> f32 {
    ui_scale.value(280.0)
}

pub(crate) fn secondary_panel_width(ui_scale: UiScale) -> f32 {
    ui_scale.value(244.0)
}

pub(crate) fn add_node_marker(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: impl Into<String>,
) -> UiNodeId {
    document.add_child(
        parent,
        UiNode::container(
            name,
            layout::with_absolute_position(
                layout::size(layout::px(0.0), layout::px(0.0)),
                0.0,
                0.0,
            ),
        ),
    )
}

pub(crate) fn pad_pre_shell_node_ids(document: &mut UiDocument, parent: UiNodeId) {
    let start = document.node_count();
    for index in start..PRE_SHELL_NODE_ID_BUDGET {
        add_node_marker(
            document,
            parent,
            format!("fabricad.preshell.id_pad.{index}"),
        );
    }
}
