use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    mes,
    recipe::{RecipeBinding, ToolClass},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessFlowRouteId(pub String);

impl ProcessFlowRouteId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProcessFlowRouteId {
    fn default() -> Self {
        Self::new("FLOW-UNTITLED")
    }
}

impl From<&str> for ProcessFlowRouteId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ProcessFlowRouteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessFlowNodeId(pub String);

impl ProcessFlowNodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ProcessFlowNodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ProcessFlowNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessFlowEdgeId(pub String);

impl ProcessFlowEdgeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ProcessFlowEdgeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ProcessFlowEdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessFlowNodeKind {
    Start,
    Operation,
    Measurement,
    Hold,
    End,
}

impl ProcessFlowNodeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Operation => "operation",
            Self::Measurement => "measurement",
            Self::Hold => "hold",
            Self::End => "end",
        }
    }

    pub fn requires_recipe(self) -> bool {
        matches!(self, Self::Operation | Self::Measurement)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessFlowEdgeKind {
    Sequence,
    Branch,
    Rework,
}

impl ProcessFlowEdgeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sequence => "sequence",
            Self::Branch => "branch",
            Self::Rework => "rework",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementCheckpoint {
    pub measurement_name: String,
    pub target: String,
    pub sample_plan: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessFlowNode {
    pub id: ProcessFlowNodeId,
    pub name: String,
    pub kind: ProcessFlowNodeKind,
    pub area: String,
    #[serde(default)]
    pub allowed_tool_classes: Vec<ToolClass>,
    #[serde(default)]
    pub eligible_tools: Vec<String>,
    #[serde(default)]
    pub recipe: Option<RecipeBinding>,
    #[serde(default)]
    pub expected_inputs: Vec<String>,
    #[serde(default)]
    pub expected_outputs: Vec<String>,
    #[serde(default)]
    pub hold_point: bool,
    #[serde(default)]
    pub measurement_checkpoint: Option<MeasurementCheckpoint>,
    #[serde(default)]
    pub notes: String,
}

impl ProcessFlowNode {
    pub fn has_tool_coverage(&self) -> bool {
        !self.allowed_tool_classes.is_empty() && !self.eligible_tools.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessFlowEdge {
    pub id: ProcessFlowEdgeId,
    pub from: ProcessFlowNodeId,
    pub to: ProcessFlowNodeId,
    pub kind: ProcessFlowEdgeKind,
    #[serde(default)]
    pub condition: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessFlowRoute {
    pub id: ProcessFlowRouteId,
    pub version: u32,
    pub name: String,
    pub owner: String,
    pub mes_route_id: String,
    pub mask_design_id: String,
    pub layout_revision: String,
    #[serde(default)]
    pub nodes: Vec<ProcessFlowNode>,
    #[serde(default)]
    pub edges: Vec<ProcessFlowEdge>,
}

impl ProcessFlowRoute {
    pub fn node(&self, id: &ProcessFlowNodeId) -> Option<&ProcessFlowNode> {
        self.nodes.iter().find(|node| &node.id == id)
    }

    pub fn successors(&self, id: &ProcessFlowNodeId) -> Vec<&ProcessFlowNode> {
        self.edges
            .iter()
            .filter(|edge| &edge.from == id)
            .filter_map(|edge| self.node(&edge.to))
            .collect()
    }

    pub fn validation_findings(&self) -> Vec<ProcessFlowFinding> {
        validate_process_flow(self)
    }

    pub fn export_to_mes_route(&self) -> mes::ProcessRoute {
        let mut sequence = 10;
        let steps = self
            .nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.kind,
                    ProcessFlowNodeKind::Operation | ProcessFlowNodeKind::Measurement
                )
            })
            .map(|node| {
                let required_tool_class = node
                    .allowed_tool_classes
                    .first()
                    .copied()
                    .map(map_tool_class_to_mes)
                    .unwrap_or(mes::ToolClass::LithographyTrack);
                let step = mes::ProcessStep {
                    id: mes::ProcessStepId::new(node.id.as_str()),
                    sequence,
                    name: node.name.clone(),
                    area: node.area.clone(),
                    required_tool_class,
                    required_recipe: mes::RecipeId::new(
                        node.recipe
                            .as_ref()
                            .map(|recipe| recipe.recipe_id.as_str())
                            .unwrap_or("UNASSIGNED_RECIPE"),
                    ),
                    eligible_tools: node
                        .eligible_tools
                        .iter()
                        .map(|tool| mes::ToolId::new(tool.clone()))
                        .collect(),
                    signoff_required: node.hold_point,
                    rework_allowed: self.edges.iter().any(|edge| {
                        edge.from == node.id && edge.kind == ProcessFlowEdgeKind::Rework
                    }),
                };
                sequence += 10;
                step
            })
            .collect();

        mes::ProcessRoute {
            id: mes::ProcessRouteId::new(&self.mes_route_id),
            name: self.name.clone(),
            revision: format!("v{}", self.version),
            mask_design_id: self.mask_design_id.clone(),
            layout_revision: self.layout_revision.clone(),
            steps,
        }
    }
}

impl Default for ProcessFlowRoute {
    fn default() -> Self {
        Self {
            id: ProcessFlowRouteId::default(),
            version: 1,
            name: "Untitled process flow".to_string(),
            owner: String::new(),
            mes_route_id: String::new(),
            mask_design_id: String::new(),
            layout_revision: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessFlowVersion {
    pub version: u32,
    pub author: String,
    pub timestamp: String,
    pub change_note: String,
    pub node_count: usize,
    pub edge_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessFlowFindingSeverity {
    Info,
    Warning,
    Error,
}

impl ProcessFlowFindingSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessFlowFinding {
    pub severity: ProcessFlowFindingSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub node_id: Option<ProcessFlowNodeId>,
    #[serde(default)]
    pub edge_id: Option<ProcessFlowEdgeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProcessFlowValidationContext {
    pub mes_route_ids: BTreeSet<String>,
    pub recipe_ids: BTreeSet<String>,
}

impl ProcessFlowValidationContext {
    pub fn from_recipe_ids_and_mes<I>(recipe_ids: I, mes: &mes::FabMesData) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        Self {
            mes_route_ids: mes
                .routes
                .keys()
                .map(|route_id| route_id.as_str().to_string())
                .collect(),
            recipe_ids: recipe_ids.into_iter().collect(),
        }
    }

    fn contains_mes_route(&self, route_id: &str) -> bool {
        self.mes_route_ids.contains(route_id)
    }

    fn contains_recipe(&self, recipe_id: &RecipeBinding) -> bool {
        self.recipe_ids.contains(recipe_id.recipe_id.as_str())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessFlowModel {
    pub route: ProcessFlowRoute,
    #[serde(default)]
    pub versions: Vec<ProcessFlowVersion>,
}

impl ProcessFlowModel {
    pub fn sample() -> Self {
        sample_process_flow_model()
    }

    pub fn findings(&self) -> Vec<ProcessFlowFinding> {
        validate_process_flow_model(self)
    }

    pub fn validate_with_context(
        &self,
        context: &ProcessFlowValidationContext,
    ) -> Vec<ProcessFlowFinding> {
        let mut findings = validate_process_flow_model(self);
        validate_process_flow_context_links(&self.route, context, &mut findings);
        findings
    }

    pub fn export_to_mes_route(&self) -> mes::ProcessRoute {
        self.route.export_to_mes_route()
    }
}

pub fn validate_process_flow(route: &ProcessFlowRoute) -> Vec<ProcessFlowFinding> {
    let mut findings = Vec::new();
    if route.id.as_str().trim().is_empty() {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Error,
            code: "empty_route_id".to_string(),
            message: "process flow route id cannot be empty".to_string(),
            node_id: None,
            edge_id: None,
        });
    }
    if route.version == 0 {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Error,
            code: "invalid_route_version".to_string(),
            message: format!("process flow route {} has version 0", route.id),
            node_id: None,
            edge_id: None,
        });
    }

    let mut node_ids = BTreeSet::new();
    let mut duplicate_node_ids = BTreeSet::new();
    for node in &route.nodes {
        if node.id.as_str().trim().is_empty() {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Error,
                code: "empty_node_id".to_string(),
                message: "process flow node id cannot be empty".to_string(),
                node_id: Some(node.id.clone()),
                edge_id: None,
            });
            continue;
        }
        if !node_ids.insert(node.id.clone()) {
            duplicate_node_ids.insert(node.id.clone());
        }
    }

    for node_id in duplicate_node_ids {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Error,
            code: "duplicate_node".to_string(),
            message: format!("node id {node_id} is used more than once"),
            node_id: Some(node_id),
            edge_id: None,
        });
    }

    let mut edge_ids = BTreeSet::new();
    let mut duplicate_edge_ids = BTreeSet::new();
    for edge in &route.edges {
        if edge.id.as_str().trim().is_empty() {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Error,
                code: "empty_edge_id".to_string(),
                message: "process flow edge id cannot be empty".to_string(),
                node_id: None,
                edge_id: Some(edge.id.clone()),
            });
            continue;
        }
        if !edge_ids.insert(edge.id.clone()) {
            duplicate_edge_ids.insert(edge.id.clone());
        }
    }

    for edge_id in duplicate_edge_ids {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Error,
            code: "duplicate_edge".to_string(),
            message: format!("edge id {edge_id} is used more than once"),
            node_id: None,
            edge_id: Some(edge_id),
        });
    }

    let start_count = route
        .nodes
        .iter()
        .filter(|node| node.kind == ProcessFlowNodeKind::Start)
        .count();
    if start_count != 1 {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Error,
            code: "start_count".to_string(),
            message: format!("route must have exactly one start node; found {start_count}"),
            node_id: None,
            edge_id: None,
        });
    }

    let end_count = route
        .nodes
        .iter()
        .filter(|node| node.kind == ProcessFlowNodeKind::End)
        .count();
    if end_count != 1 {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Error,
            code: "end_count".to_string(),
            message: format!("route must have exactly one end node; found {end_count}"),
            node_id: None,
            edge_id: None,
        });
    }

    for edge in &route.edges {
        if !node_ids.contains(&edge.from) {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Error,
                code: "edge_from_missing".to_string(),
                message: format!("edge {} starts at missing node {}", edge.id, edge.from),
                node_id: Some(edge.from.clone()),
                edge_id: Some(edge.id.clone()),
            });
        }
        if !node_ids.contains(&edge.to) {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Error,
                code: "edge_to_missing".to_string(),
                message: format!("edge {} ends at missing node {}", edge.id, edge.to),
                node_id: Some(edge.to.clone()),
                edge_id: Some(edge.id.clone()),
            });
        }
    }

    for node in &route.nodes {
        if node.kind.requires_recipe() {
            if node.recipe.is_none() {
                findings.push(ProcessFlowFinding {
                    severity: ProcessFlowFindingSeverity::Error,
                    code: "missing_recipe".to_string(),
                    message: format!("{} must have a recipe binding", node.name),
                    node_id: Some(node.id.clone()),
                    edge_id: None,
                });
            }
            if !node.has_tool_coverage() {
                findings.push(ProcessFlowFinding {
                    severity: ProcessFlowFindingSeverity::Error,
                    code: "missing_tool_coverage".to_string(),
                    message: format!("{} must name an allowed class and eligible tool", node.name),
                    node_id: Some(node.id.clone()),
                    edge_id: None,
                });
            }
        }

        if node.kind == ProcessFlowNodeKind::Measurement && node.measurement_checkpoint.is_none() {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Warning,
                code: "missing_checkpoint".to_string(),
                message: format!("{} should define a measurement checkpoint", node.name),
                node_id: Some(node.id.clone()),
                edge_id: None,
            });
        }
    }

    if !route
        .edges
        .iter()
        .any(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
    {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Info,
            code: "no_rework_path".to_string(),
            message: "route has no explicit rework path".to_string(),
            node_id: None,
            edge_id: None,
        });
    }

    findings
}

pub fn validate_process_flow_model(model: &ProcessFlowModel) -> Vec<ProcessFlowFinding> {
    let mut findings = validate_process_flow(&model.route);
    validate_process_flow_versions(model, &mut findings);
    findings
}

pub fn validate_process_flow_with_context(
    route: &ProcessFlowRoute,
    context: &ProcessFlowValidationContext,
) -> Vec<ProcessFlowFinding> {
    let mut findings = validate_process_flow(route);
    validate_process_flow_context_links(route, context, &mut findings);
    findings
}

fn validate_process_flow_context_links(
    route: &ProcessFlowRoute,
    context: &ProcessFlowValidationContext,
    findings: &mut Vec<ProcessFlowFinding>,
) {
    let mes_route_id = route.mes_route_id.trim();
    if !mes_route_id.is_empty() && !context.contains_mes_route(mes_route_id) {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Error,
            code: "missing_mes_route".to_string(),
            message: format!(
                "process flow route {} references missing MES route {mes_route_id}",
                route.id
            ),
            node_id: None,
            edge_id: None,
        });
    }

    for node in &route.nodes {
        if let Some(binding) = &node.recipe
            && !context.contains_recipe(binding)
        {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Error,
                code: "missing_recipe_reference".to_string(),
                message: format!(
                    "process flow node {} references missing recipe {}",
                    node.id, binding.recipe_id
                ),
                node_id: Some(node.id.clone()),
                edge_id: None,
            });
        }
    }
}

fn validate_process_flow_versions(
    model: &ProcessFlowModel,
    findings: &mut Vec<ProcessFlowFinding>,
) {
    let mut versions = BTreeSet::new();
    let mut has_current_version = false;
    for version in &model.versions {
        if version.version == 0 {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Error,
                code: "invalid_version_record".to_string(),
                message: "process flow version history contains version 0".to_string(),
                node_id: None,
                edge_id: None,
            });
            continue;
        }
        if !versions.insert(version.version) {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Error,
                code: "duplicate_version_record".to_string(),
                message: format!(
                    "process flow version history repeats version {}",
                    version.version
                ),
                node_id: None,
                edge_id: None,
            });
        }
        if version.author.trim().is_empty() {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Warning,
                code: "empty_version_author".to_string(),
                message: format!("process flow version {} has no author", version.version),
                node_id: None,
                edge_id: None,
            });
        }
        if version.timestamp.trim().is_empty() {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Warning,
                code: "empty_version_timestamp".to_string(),
                message: format!("process flow version {} has no timestamp", version.version),
                node_id: None,
                edge_id: None,
            });
        }
        if version.change_note.trim().is_empty() {
            findings.push(ProcessFlowFinding {
                severity: ProcessFlowFindingSeverity::Warning,
                code: "empty_version_change_note".to_string(),
                message: format!(
                    "process flow version {} has no change note",
                    version.version
                ),
                node_id: None,
                edge_id: None,
            });
        }
        if version.version == model.route.version {
            has_current_version = true;
            if version.node_count != model.route.nodes.len()
                || version.edge_count != model.route.edges.len()
            {
                findings.push(ProcessFlowFinding {
                    severity: ProcessFlowFindingSeverity::Error,
                    code: "stale_current_version_counts".to_string(),
                    message: format!(
                        "process flow current version {} records {} nodes / {} edges but route has {} nodes / {} edges",
                        version.version,
                        version.node_count,
                        version.edge_count,
                        model.route.nodes.len(),
                        model.route.edges.len()
                    ),
                    node_id: None,
                    edge_id: None,
                });
            }
        }
    }

    if !model.versions.is_empty() && !has_current_version && model.route.version != 0 {
        findings.push(ProcessFlowFinding {
            severity: ProcessFlowFindingSeverity::Error,
            code: "missing_current_version_record".to_string(),
            message: format!(
                "process flow version history is missing current route version {}",
                model.route.version
            ),
            node_id: None,
            edge_id: None,
        });
    }
}

fn map_tool_class_to_mes(tool_class: ToolClass) -> mes::ToolClass {
    match tool_class {
        ToolClass::SpinCoater => mes::ToolClass::LithographyTrack,
        ToolClass::PlasmaEtcher => mes::ToolClass::PlasmaEtcher,
        ToolClass::FurnaceHotplate => mes::ToolClass::BakeOven,
        ToolClass::LithographyExposure => mes::ToolClass::MaskAligner,
    }
}

pub fn sample_process_flow_model() -> ProcessFlowModel {
    let route = ProcessFlowRoute {
        id: ProcessFlowRouteId::new("FLOW-DEMO-INVERTER-POLY-A"),
        version: 3,
        name: "Demo inverter poly module".to_string(),
        owner: "process.engineer".to_string(),
        mes_route_id: "ROUTE-DEMO-INVERTER-POLY-A".to_string(),
        mask_design_id: "FABRICAD-DEMO-INVERTER".to_string(),
        layout_revision: "layout_model:demo-inverter@rev-5".to_string(),
        nodes: vec![
            ProcessFlowNode {
                id: ProcessFlowNodeId::new("START"),
                name: "Lot released".to_string(),
                kind: ProcessFlowNodeKind::Start,
                area: "MES".to_string(),
                allowed_tool_classes: Vec::new(),
                eligible_tools: Vec::new(),
                recipe: None,
                expected_inputs: vec!["25 wafer lot".to_string()],
                expected_outputs: vec!["traveler active".to_string()],
                hold_point: false,
                measurement_checkpoint: None,
                notes: "Route starts after mask revision check".to_string(),
            },
            ProcessFlowNode {
                id: ProcessFlowNodeId::new("COAT"),
                name: "Coat PR and soft bake".to_string(),
                kind: ProcessFlowNodeKind::Operation,
                area: "lithography".to_string(),
                allowed_tool_classes: vec![ToolClass::SpinCoater, ToolClass::FurnaceHotplate],
                eligible_tools: vec!["TRACK-01".to_string(), "TRACK-02".to_string()],
                recipe: Some(RecipeBinding::new("SPIN_PR_3000", 1)),
                expected_inputs: vec!["clean wafer".to_string()],
                expected_outputs: vec!["1.2 um photoresist".to_string()],
                hold_point: false,
                measurement_checkpoint: None,
                notes: "Soft bake runs inline on the track".to_string(),
            },
            ProcessFlowNode {
                id: ProcessFlowNodeId::new("EXPOSE"),
                name: "Align and expose poly mask".to_string(),
                kind: ProcessFlowNodeKind::Operation,
                area: "lithography".to_string(),
                allowed_tool_classes: vec![ToolClass::LithographyExposure],
                eligible_tools: vec!["ALIGNER-01".to_string()],
                recipe: Some(RecipeBinding::new("LITHO_POLY_EXPOSE_001", 1)),
                expected_inputs: vec!["coated wafer".to_string(), "poly reticle".to_string()],
                expected_outputs: vec!["latent poly image".to_string()],
                hold_point: true,
                measurement_checkpoint: None,
                notes: "Engineer signoff required before expose".to_string(),
            },
            ProcessFlowNode {
                id: ProcessFlowNodeId::new("DEVELOP"),
                name: "Develop and inspect resist".to_string(),
                kind: ProcessFlowNodeKind::Measurement,
                area: "lithography".to_string(),
                allowed_tool_classes: vec![ToolClass::SpinCoater],
                eligible_tools: vec!["TRACK-01".to_string(), "TRACK-02".to_string()],
                recipe: Some(RecipeBinding::new("LITHO_DEVELOP_001", 1)),
                expected_inputs: vec!["exposed resist".to_string()],
                expected_outputs: vec!["developed resist image".to_string()],
                hold_point: false,
                measurement_checkpoint: Some(MeasurementCheckpoint {
                    measurement_name: "post-develop inspection".to_string(),
                    target: "no scum, no bridge defects".to_string(),
                    sample_plan: "5 die per wafer on first and last wafer".to_string(),
                }),
                notes: "Failing inspection branches to strip and recoat".to_string(),
            },
            ProcessFlowNode {
                id: ProcessFlowNodeId::new("ETCH"),
                name: "Poly plasma etch".to_string(),
                kind: ProcessFlowNodeKind::Operation,
                area: "etch".to_string(),
                allowed_tool_classes: vec![ToolClass::PlasmaEtcher],
                eligible_tools: vec!["ETCH-01".to_string()],
                recipe: Some(RecipeBinding::new("ETCH_CF4_POLY_001", 1)),
                expected_inputs: vec!["developed resist image".to_string()],
                expected_outputs: vec!["etched poly gate".to_string()],
                hold_point: true,
                measurement_checkpoint: None,
                notes: "Critical etch step; no rework after completion".to_string(),
            },
            ProcessFlowNode {
                id: ProcessFlowNodeId::new("CD_METRO"),
                name: "Poly CD metrology".to_string(),
                kind: ProcessFlowNodeKind::Measurement,
                area: "metrology".to_string(),
                allowed_tool_classes: vec![ToolClass::LithographyExposure],
                eligible_tools: vec!["CDSEM-01".to_string()],
                recipe: Some(RecipeBinding::new("METRO_POLY_CD_001", 1)),
                expected_inputs: vec!["etched poly gate".to_string()],
                expected_outputs: vec!["CD distribution".to_string()],
                hold_point: false,
                measurement_checkpoint: Some(MeasurementCheckpoint {
                    measurement_name: "poly gate CD".to_string(),
                    target: "45 nm +/- 4 nm".to_string(),
                    sample_plan: "9 site wafer map on slots 1, 13, 25".to_string(),
                }),
                notes: "Feeds run-to-run etch endpoint tuning".to_string(),
            },
            ProcessFlowNode {
                id: ProcessFlowNodeId::new("END"),
                name: "Module complete".to_string(),
                kind: ProcessFlowNodeKind::End,
                area: "MES".to_string(),
                allowed_tool_classes: Vec::new(),
                eligible_tools: Vec::new(),
                recipe: None,
                expected_inputs: vec!["accepted CD data".to_string()],
                expected_outputs: vec!["lot released to next module".to_string()],
                hold_point: false,
                measurement_checkpoint: None,
                notes: String::new(),
            },
        ],
        edges: vec![
            ProcessFlowEdge {
                id: ProcessFlowEdgeId::new("E010"),
                from: ProcessFlowNodeId::new("START"),
                to: ProcessFlowNodeId::new("COAT"),
                kind: ProcessFlowEdgeKind::Sequence,
                condition: String::new(),
            },
            ProcessFlowEdge {
                id: ProcessFlowEdgeId::new("E020"),
                from: ProcessFlowNodeId::new("COAT"),
                to: ProcessFlowNodeId::new("EXPOSE"),
                kind: ProcessFlowEdgeKind::Sequence,
                condition: String::new(),
            },
            ProcessFlowEdge {
                id: ProcessFlowEdgeId::new("E030"),
                from: ProcessFlowNodeId::new("EXPOSE"),
                to: ProcessFlowNodeId::new("DEVELOP"),
                kind: ProcessFlowEdgeKind::Sequence,
                condition: String::new(),
            },
            ProcessFlowEdge {
                id: ProcessFlowEdgeId::new("E040"),
                from: ProcessFlowNodeId::new("DEVELOP"),
                to: ProcessFlowNodeId::new("ETCH"),
                kind: ProcessFlowEdgeKind::Branch,
                condition: "inspection passes".to_string(),
            },
            ProcessFlowEdge {
                id: ProcessFlowEdgeId::new("E041"),
                from: ProcessFlowNodeId::new("DEVELOP"),
                to: ProcessFlowNodeId::new("COAT"),
                kind: ProcessFlowEdgeKind::Rework,
                condition: "strip and recoat if post-develop inspection fails".to_string(),
            },
            ProcessFlowEdge {
                id: ProcessFlowEdgeId::new("E050"),
                from: ProcessFlowNodeId::new("ETCH"),
                to: ProcessFlowNodeId::new("CD_METRO"),
                kind: ProcessFlowEdgeKind::Sequence,
                condition: String::new(),
            },
            ProcessFlowEdge {
                id: ProcessFlowEdgeId::new("E060"),
                from: ProcessFlowNodeId::new("CD_METRO"),
                to: ProcessFlowNodeId::new("END"),
                kind: ProcessFlowEdgeKind::Sequence,
                condition: "CD in spec".to_string(),
            },
        ],
    };

    ProcessFlowModel {
        versions: vec![
            ProcessFlowVersion {
                version: 1,
                author: "process.engineer".to_string(),
                timestamp: "2026-05-01T15:20:00Z".to_string(),
                change_note: "initial lithography and etch flow".to_string(),
                node_count: 5,
                edge_count: 4,
            },
            ProcessFlowVersion {
                version: 2,
                author: "integration.owner".to_string(),
                timestamp: "2026-05-03T18:10:00Z".to_string(),
                change_note: "added post-develop inspection branch".to_string(),
                node_count: 6,
                edge_count: 6,
            },
            ProcessFlowVersion {
                version: 3,
                author: "process.engineer".to_string(),
                timestamp: "2026-05-06T13:30:00Z".to_string(),
                change_note: "linked CD metrology to R2R control".to_string(),
                node_count: route.nodes.len(),
                edge_count: route.edges.len(),
            },
        ],
        route,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validation_context() -> ProcessFlowValidationContext {
        ProcessFlowValidationContext::from_recipe_ids_and_mes(
            crate::recipe::RecipeCatalog::sample()
                .recipes
                .keys()
                .map(|recipe_id| recipe_id.as_str().to_string()),
            &crate::mes::FabMesData::sample(),
        )
    }

    #[test]
    fn sample_process_flow_is_valid_for_mvp_export() {
        let model = ProcessFlowModel::sample();
        let findings = model.findings();
        assert!(
            findings
                .iter()
                .all(|finding| finding.severity != ProcessFlowFindingSeverity::Error),
            "{findings:?}"
        );
        let mes_route = model.export_to_mes_route();
        assert_eq!(mes_route.id.as_str(), "ROUTE-DEMO-INVERTER-POLY-A");
        assert_eq!(mes_route.steps.len(), 5);
        assert!(
            mes_route
                .steps
                .iter()
                .any(|step| step.signoff_required && step.name == "Poly plasma etch")
        );
    }

    #[test]
    fn context_validation_accepts_sample_references() {
        let model = ProcessFlowModel::sample();
        let findings = model.validate_with_context(&validation_context());

        assert!(
            findings
                .iter()
                .all(|finding| finding.severity != ProcessFlowFindingSeverity::Error),
            "{findings:?}"
        );
    }

    #[test]
    fn context_validation_reports_missing_mes_route_and_recipe() {
        let mut model = ProcessFlowModel::sample();
        model.route.mes_route_id = "ROUTE-MISSING".to_string();
        let etch = model
            .route
            .nodes
            .iter_mut()
            .find(|node| node.id.as_str() == "ETCH")
            .unwrap();
        etch.recipe = Some(RecipeBinding::new("RECIPE-MISSING", 1));

        let findings = model.validate_with_context(&validation_context());

        assert!(findings.iter().any(|finding| {
            finding.code == "missing_mes_route"
                && finding.severity == ProcessFlowFindingSeverity::Error
        }));
        assert!(findings.iter().any(|finding| {
            finding.code == "missing_recipe_reference"
                && finding.node_id == Some(ProcessFlowNodeId::new("ETCH"))
        }));
    }

    #[test]
    fn validation_reports_missing_recipe_and_tool_coverage() {
        let mut model = ProcessFlowModel::sample();
        let etch = model
            .route
            .nodes
            .iter_mut()
            .find(|node| node.id.as_str() == "ETCH")
            .unwrap();
        etch.recipe = None;
        etch.eligible_tools.clear();

        let findings = model.findings();
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "missing_recipe")
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "missing_tool_coverage")
        );
    }

    #[test]
    fn validation_reports_broken_edges() {
        let mut model = ProcessFlowModel::sample();
        model.route.edges.push(ProcessFlowEdge {
            id: ProcessFlowEdgeId::new("BROKEN"),
            from: ProcessFlowNodeId::new("ETCH"),
            to: ProcessFlowNodeId::new("MISSING"),
            kind: ProcessFlowEdgeKind::Sequence,
            condition: String::new(),
        });

        let findings = model.findings();
        assert!(findings.iter().any(|finding| {
            finding.code == "edge_to_missing"
                && finding.edge_id == Some(ProcessFlowEdgeId::new("BROKEN"))
        }));
    }

    #[test]
    fn validation_rejects_empty_and_duplicate_stable_ids() {
        let mut model = ProcessFlowModel::sample();
        model.route.id = ProcessFlowRouteId::new("");
        model.route.version = 0;
        model.route.nodes[0].id = ProcessFlowNodeId::new("");
        let duplicate_edge = model.route.edges[0].clone();
        model.route.edges.push(duplicate_edge);
        model.route.edges.push(ProcessFlowEdge {
            id: ProcessFlowEdgeId::new(""),
            from: ProcessFlowNodeId::new("ETCH"),
            to: ProcessFlowNodeId::new("END"),
            kind: ProcessFlowEdgeKind::Sequence,
            condition: String::new(),
        });

        let findings = model.findings();

        for code in [
            "empty_route_id",
            "invalid_route_version",
            "empty_node_id",
            "duplicate_edge",
            "empty_edge_id",
        ] {
            assert!(
                findings.iter().any(|finding| finding.code == code),
                "missing {code}: {findings:?}"
            );
        }
    }

    #[test]
    fn validation_rejects_stale_version_history() {
        let mut model = ProcessFlowModel::sample();
        model.versions[2].node_count += 1;
        model.versions[2].author.clear();
        let mut duplicate = model.versions[2].clone();
        duplicate.node_count = model.route.nodes.len();
        duplicate.author = "process.engineer".to_string();
        model.versions.push(duplicate);
        model.versions.push(ProcessFlowVersion {
            version: 0,
            author: "integration.owner".to_string(),
            timestamp: "2026-05-07T09:00:00Z".to_string(),
            change_note: "invalid migration artifact".to_string(),
            node_count: model.route.nodes.len(),
            edge_count: model.route.edges.len(),
        });

        let findings = model.findings();

        for code in [
            "stale_current_version_counts",
            "empty_version_author",
            "duplicate_version_record",
            "invalid_version_record",
        ] {
            assert!(
                findings.iter().any(|finding| finding.code == code),
                "missing {code}: {findings:?}"
            );
        }

        let mut missing_current = ProcessFlowModel::sample();
        missing_current.route.version = 4;
        let findings = missing_current.findings();
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "missing_current_version_record")
        );
    }
}
