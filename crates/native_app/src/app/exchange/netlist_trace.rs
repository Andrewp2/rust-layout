#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutExtractedNetlistExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) layout_revision: u64,
    pub(crate) technology_name: String,
    pub(crate) components: Vec<LayoutExtractedNet>,
    #[serde(default)]
    pub(crate) devices: Vec<LayoutExtractedDevice>,
    pub(crate) shorts: Vec<LayoutExtractedNetShort>,
    pub(crate) opens: Vec<LayoutExtractedNetOpen>,
    pub(crate) skipped: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutExtractedNet {
    pub(crate) id: usize,
    pub(crate) shapes: Vec<ShapeOccurrenceId>,
    pub(crate) bounds: Rect,
    pub(crate) labels: Vec<LayoutExtractedNetLabel>,
    pub(crate) explicit_nets: Vec<NetId>,
    pub(crate) net_name: Option<String>,
    pub(crate) net_id: Option<NetId>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutExtractedNetLabel {
    pub(crate) occurrence: ShapeOccurrenceId,
    pub(crate) text: String,
    pub(crate) position: Point,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutExtractedDevice {
    pub(crate) id: usize,
    pub(crate) kind: String,
    pub(crate) model: String,
    pub(crate) terminals: Vec<LayoutExtractedDeviceTerminal>,
    pub(crate) bounds: Rect,
    pub(crate) width: Coord,
    pub(crate) length: Coord,
    pub(crate) occurrences: Vec<ShapeOccurrenceId>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutExtractedDeviceTerminal {
    pub(crate) name: String,
    pub(crate) component: Option<usize>,
    pub(crate) net_name: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutExtractedNetShort {
    pub(crate) component: usize,
    pub(crate) names: Vec<String>,
    pub(crate) bounds: Rect,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutExtractedNetOpen {
    pub(crate) name: String,
    pub(crate) components: Vec<usize>,
    pub(crate) bounds: Rect,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutTraceStateExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) layout_revision: u64,
    pub(crate) selected_component: Option<usize>,
    pub(crate) history: Vec<usize>,
    pub(crate) route_points: Vec<Point>,
    #[serde(default = "default_layout_trace_highlight_mode_slug")]
    pub(crate) highlight_mode: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutL2nDatabaseExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) layout_revision: u64,
    pub(crate) netlist: LayoutExtractedNetlistExchange,
    pub(crate) trace_state: LayoutTraceStateExchange,
}

impl LayoutExtractedNetlistExchange {
    pub(crate) fn from_report(
        document: &Document,
        layout_revision: u64,
        technology_name: String,
        report: ConnectivityReport,
    ) -> Self {
        Self {
            schema_version: LAYOUT_EXTRACTED_NETLIST_EXCHANGE_SCHEMA_VERSION,
            document_id: document.id.to_string(),
            document_name: document.name.clone(),
            layout_revision,
            technology_name,
            components: report
                .components
                .into_iter()
                .map(LayoutExtractedNet::from)
                .collect(),
            devices: report
                .devices
                .into_iter()
                .map(LayoutExtractedDevice::from)
                .collect(),
            shorts: report
                .shorts
                .into_iter()
                .map(LayoutExtractedNetShort::from)
                .collect(),
            opens: report
                .opens
                .into_iter()
                .map(LayoutExtractedNetOpen::from)
                .collect(),
            skipped: report.skipped,
        }
    }

    pub(crate) fn into_report(self) -> ConnectivityReport {
        let components = self
            .components
            .into_iter()
            .map(LayoutExtractedNet::into_component)
            .collect::<Vec<_>>();
        let shape_to_component = components
            .iter()
            .flat_map(|component| {
                component
                    .shapes
                    .iter()
                    .cloned()
                    .map(|shape| (shape, component.id))
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeMap<_, _>>();
        ConnectivityReport {
            components,
            shape_to_component,
            devices: self
                .devices
                .into_iter()
                .map(LayoutExtractedDevice::into_device)
                .collect::<Vec<_>>(),
            shorts: self
                .shorts
                .into_iter()
                .map(LayoutExtractedNetShort::into_short)
                .collect::<Vec<_>>(),
            opens: self
                .opens
                .into_iter()
                .map(LayoutExtractedNetOpen::into_open)
                .collect::<Vec<_>>(),
            skipped: self.skipped,
        }
    }
}

impl From<NetComponent> for LayoutExtractedNet {
    fn from(component: NetComponent) -> Self {
        Self {
            id: component.id,
            shapes: component.shapes,
            bounds: component.bounds,
            labels: component
                .labels
                .into_iter()
                .map(LayoutExtractedNetLabel::from)
                .collect(),
            explicit_nets: component.explicit_nets,
            net_name: component.net_name,
            net_id: component.net_id,
        }
    }
}

impl LayoutExtractedNet {
    pub(crate) fn into_component(self) -> NetComponent {
        NetComponent {
            id: self.id,
            shapes: self.shapes,
            bounds: self.bounds,
            labels: self
                .labels
                .into_iter()
                .map(LayoutExtractedNetLabel::into_label)
                .collect(),
            explicit_nets: self.explicit_nets,
            net_name: self.net_name,
            net_id: self.net_id,
        }
    }
}

impl From<NetLabel> for LayoutExtractedNetLabel {
    fn from(label: NetLabel) -> Self {
        Self {
            occurrence: label.occurrence,
            text: label.text,
            position: label.position,
        }
    }
}

impl LayoutExtractedNetLabel {
    pub(crate) fn into_label(self) -> NetLabel {
        NetLabel {
            occurrence: self.occurrence,
            text: self.text,
            position: self.position,
        }
    }
}

impl From<ExtractedDevice> for LayoutExtractedDevice {
    fn from(device: ExtractedDevice) -> Self {
        Self {
            id: device.id,
            kind: device.kind,
            model: device.model,
            terminals: device
                .terminals
                .into_iter()
                .map(LayoutExtractedDeviceTerminal::from)
                .collect(),
            bounds: device.bounds,
            width: device.width,
            length: device.length,
            occurrences: device.occurrences,
        }
    }
}

impl LayoutExtractedDevice {
    pub(crate) fn into_device(self) -> ExtractedDevice {
        ExtractedDevice {
            id: self.id,
            kind: self.kind,
            model: self.model,
            terminals: self
                .terminals
                .into_iter()
                .map(LayoutExtractedDeviceTerminal::into_terminal)
                .collect(),
            bounds: self.bounds,
            width: self.width,
            length: self.length,
            occurrences: self.occurrences,
        }
    }
}

impl From<ExtractedDeviceTerminal> for LayoutExtractedDeviceTerminal {
    fn from(terminal: ExtractedDeviceTerminal) -> Self {
        Self {
            name: terminal.name,
            component: terminal.component,
            net_name: terminal.net_name,
        }
    }
}

impl LayoutExtractedDeviceTerminal {
    pub(crate) fn into_terminal(self) -> ExtractedDeviceTerminal {
        ExtractedDeviceTerminal {
            name: self.name,
            component: self.component,
            net_name: self.net_name,
        }
    }
}

impl From<NetShort> for LayoutExtractedNetShort {
    fn from(short: NetShort) -> Self {
        Self {
            component: short.component,
            names: short.names,
            bounds: short.bounds,
        }
    }
}

impl LayoutExtractedNetShort {
    pub(crate) fn into_short(self) -> NetShort {
        NetShort {
            component: self.component,
            names: self.names,
            bounds: self.bounds,
        }
    }
}

impl From<NetOpen> for LayoutExtractedNetOpen {
    fn from(open: NetOpen) -> Self {
        Self {
            name: open.name,
            components: open.components,
            bounds: open.bounds,
        }
    }
}

impl LayoutExtractedNetOpen {
    pub(crate) fn into_open(self) -> NetOpen {
        NetOpen {
            name: self.name,
            components: self.components,
            bounds: self.bounds,
        }
    }
}

impl LayoutTraceStateExchange {
    pub(crate) fn from_app(app: &GlassworksApp) -> Self {
        let selected_component = app
            .connectivity_report()
            .ok()
            .and_then(|report| selected_layout_net_component_id(app, &report));
        let mut history = Vec::new();
        for component_id in &app.layout_trace_history {
            if !history.contains(component_id) {
                history.push(*component_id);
            }
            if history.len() >= MAX_LAYOUT_TRACE_HISTORY {
                break;
            }
        }
        Self {
            schema_version: LAYOUT_TRACE_STATE_EXCHANGE_SCHEMA_VERSION,
            document_id: app.workspace.document.id.to_string(),
            document_name: app.workspace.document.name.clone(),
            layout_revision: app.layout_revision,
            selected_component,
            history,
            route_points: app.route_points.clone(),
            highlight_mode: app.layout_trace_highlight_mode.slug().to_string(),
        }
    }
}

fn default_layout_trace_highlight_mode_slug() -> String {
    LayoutTraceHighlightMode::Selected.slug().to_string()
}
