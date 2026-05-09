use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FabObjectKind {
    Lot,
    Wafer,
    Tool,
    Recipe,
    MaterialLot,
    ToolRun,
    ProcessRoute,
    ProcessStep,
    Measurement,
    NotebookEntry,
    LayoutShape,
    SafetyIncident,
    SafetySensor,
}

impl FabObjectKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lot => "Lot",
            Self::Wafer => "Wafer",
            Self::Tool => "Tool",
            Self::Recipe => "Recipe",
            Self::MaterialLot => "Material lot",
            Self::ToolRun => "Tool run",
            Self::ProcessRoute => "Process route",
            Self::ProcessStep => "Process step",
            Self::Measurement => "Measurement",
            Self::NotebookEntry => "Notebook entry",
            Self::LayoutShape => "Layout shape",
            Self::SafetyIncident => "Safety incident",
            Self::SafetySensor => "Safety sensor",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Lot => "lot",
            Self::Wafer => "wafer",
            Self::Tool => "tool",
            Self::Recipe => "recipe",
            Self::MaterialLot => "material_lot",
            Self::ToolRun => "tool_run",
            Self::ProcessRoute => "process_route",
            Self::ProcessStep => "process_step",
            Self::Measurement => "measurement",
            Self::NotebookEntry => "notebook_entry",
            Self::LayoutShape => "layout_shape",
            Self::SafetyIncident => "safety_incident",
            Self::SafetySensor => "safety_sensor",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FabObjectRef {
    pub kind: FabObjectKind,
    pub id: String,
}

impl FabObjectRef {
    pub fn new(kind: FabObjectKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }

    pub fn lot(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::Lot, id)
    }

    pub fn wafer(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::Wafer, id)
    }

    pub fn tool(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::Tool, id)
    }

    pub fn recipe(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::Recipe, id)
    }

    pub fn material_lot(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::MaterialLot, id)
    }

    pub fn tool_run(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::ToolRun, id)
    }

    pub fn process_route(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::ProcessRoute, id)
    }

    pub fn process_step(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::ProcessStep, id)
    }

    pub fn measurement(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::Measurement, id)
    }

    pub fn notebook_entry(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::NotebookEntry, id)
    }

    pub fn layout_shape(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::LayoutShape, id)
    }

    pub fn safety_incident(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::SafetyIncident, id)
    }

    pub fn safety_sensor(id: impl Into<String>) -> Self {
        Self::new(FabObjectKind::SafetySensor, id)
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.kind.key(), self.id)
    }

    pub fn label(&self) -> String {
        format!("{} {}", self.kind.label(), self.id)
    }
}

impl fmt::Display for FabObjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind.label(), self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::{FabObjectKind, FabObjectRef};

    #[test]
    fn fab_object_ref_has_stable_kind_and_id() {
        let lot = FabObjectRef::lot("L-00042");
        assert_eq!(lot.kind, FabObjectKind::Lot);
        assert_eq!(lot.id, "L-00042");
        assert_eq!(lot.key(), "lot:L-00042");
        assert_eq!(lot.label(), "Lot L-00042");
    }
}
