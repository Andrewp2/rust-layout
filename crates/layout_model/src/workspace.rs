use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    CURRENT_SCHEMA_VERSION, Document, MarkerState,
    cross_section::{CrossSectionProcess, CrossSectionValidationSeverity},
    environment::{CleanroomEnvironment, EnvironmentValidationSeverity},
    equipment::{EquipmentSimulator, EquipmentValidationContext, EquipmentValidationSeverity},
    experiment::{
        ExperimentPlan, ExperimentStatus, ExperimentValidationContext, ExperimentValidationSeverity,
    },
    genealogy::{GenealogyValidationContext, GenealogyValidationSeverity, LotGenealogy},
    inventory::{Inventory, InventoryValidationContext, InventoryValidationSeverity},
    maintenance::{MaintenanceModel, MaintenanceValidationContext, MaintenanceValidationSeverity},
    mes::{FabMesData, FabMesValidationSeverity, ProcessRouteId, ProcessStepId},
    metrology::{
        FabObjectLinks, MetrologyValidationContext, MetrologyValidationSeverity, WaferGeometry,
        WaferMap,
    },
    notebook::{LabNotebook, NotebookValidationContext, NotebookValidationSeverity},
    process_control::{
        ProcessControlModel, ProcessControlValidationContext, ProcessControlValidationSeverity,
    },
    process_flow::{ProcessFlowFindingSeverity, ProcessFlowModel, ProcessFlowValidationContext},
    recipe::{RecipeBinding, RecipeCatalog, ValidationSeverity},
    safety::{SafetySystem, SafetyValidationContext, SafetyValidationSeverity},
    scheduler::{DispatchSchedule, SchedulerValidationContext, SchedulerValidationSeverity},
    spc_fdc::{SpcFdcMonitor, SpcFdcValidationSeverity},
    yield_analysis::{YieldAnalysis, YieldValidationSeverity},
};

pub const WORKSPACE_DATASET_SCHEMA_VERSION: u32 = 1;
pub const WORKSPACE_PRODUCER: &str = "fabricad_native_app";
pub const WORKSPACE_FEATURE_FLAGS: &[&str] = &[
    "layout_document",
    "loro_crdt_log",
    "fab_domain_models",
    "operad_panels",
];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceSnapshotMetadata {
    #[serde(default)]
    pub producer: String,
    #[serde(default)]
    pub producer_version: String,
    #[serde(default)]
    pub workspace_schema_version: u32,
    #[serde(default)]
    pub document_schema_version: u32,
    #[serde(default)]
    pub feature_flags: Vec<String>,
    #[serde(default)]
    pub migration_history: Vec<String>,
}

impl WorkspaceSnapshotMetadata {
    pub fn current(document_schema_version: u32) -> Self {
        Self {
            producer: WORKSPACE_PRODUCER.to_string(),
            producer_version: env!("CARGO_PKG_VERSION").to_string(),
            workspace_schema_version: WORKSPACE_DATASET_SCHEMA_VERSION,
            document_schema_version,
            feature_flags: WORKSPACE_FEATURE_FLAGS
                .iter()
                .map(|flag| (*flag).to_string())
                .collect(),
            migration_history: vec![format!(
                "workspace_schema:{}",
                WORKSPACE_DATASET_SCHEMA_VERSION
            )],
        }
    }

    pub fn is_missing(&self) -> bool {
        self.producer.is_empty()
            && self.producer_version.is_empty()
            && self.workspace_schema_version == 0
            && self.document_schema_version == 0
            && self.feature_flags.is_empty()
            && self.migration_history.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceDataset {
    #[serde(default = "legacy_workspace_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub metadata: WorkspaceSnapshotMetadata,
    pub document: Document,
    pub mes: FabMesData,
    pub yield_analysis: YieldAnalysis,
    pub wafer_map: WaferMap,
    pub recipe_catalog: RecipeCatalog,
    pub genealogy: LotGenealogy,
    pub inventory: Inventory,
    pub maintenance: MaintenanceModel,
    pub environment: CleanroomEnvironment,
    pub scheduler: DispatchSchedule,
    pub safety: SafetySystem,
    pub experiment_plan: ExperimentPlan,
    pub process_control: ProcessControlModel,
    pub process_flow: ProcessFlowModel,
    #[serde(default)]
    pub cross_section: CrossSectionProcess,
    pub lab_notebook: LabNotebook,
    pub equipment: EquipmentSimulator,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl WorkspaceValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn push_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    pub fn push_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn error_summary(&self) -> String {
        if self.errors.is_empty() {
            return "valid".to_string();
        }
        let shown = self.errors.iter().take(3).cloned().collect::<Vec<_>>();
        let remaining = self.errors.len().saturating_sub(shown.len());
        let suffix = if remaining > 0 {
            format!("; {remaining} more")
        } else {
            String::new()
        };
        format!(
            "{} error(s): {}{suffix}",
            self.errors.len(),
            shown.join("; ")
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltinDemoWorkspaceReport {
    pub source: &'static str,
    pub workspace_schema_version: u32,
    pub document_schema_version: u32,
    pub document_shapes: usize,
    pub mes_lots: usize,
    pub yield_lots: usize,
    pub wafer_dies: usize,
    pub recipes: usize,
    pub process_flow_nodes: usize,
    pub cross_section_steps: usize,
    pub notebook_entries: usize,
    pub equipment_tools: usize,
}

impl WorkspaceDataset {
    pub fn blank() -> Self {
        let document = blank_layout_document("Untitled layout");
        Self {
            schema_version: WORKSPACE_DATASET_SCHEMA_VERSION,
            metadata: WorkspaceSnapshotMetadata::current(document.schema_version),
            document,
            mes: empty_mes_data(),
            yield_analysis: empty_yield_analysis(),
            wafer_map: empty_wafer_map(),
            recipe_catalog: RecipeCatalog::default(),
            genealogy: LotGenealogy::new(),
            inventory: Inventory::default(),
            maintenance: MaintenanceModel::default(),
            environment: CleanroomEnvironment::default(),
            scheduler: DispatchSchedule::default(),
            safety: SafetySystem::default(),
            experiment_plan: empty_experiment_plan(),
            process_control: ProcessControlModel::default(),
            process_flow: ProcessFlowModel::default(),
            cross_section: CrossSectionProcess::blank(),
            lab_notebook: LabNotebook::default(),
            equipment: EquipmentSimulator::new(Vec::new()),
        }
    }

    pub fn demo() -> Self {
        let yield_analysis = YieldAnalysis::synthetic();
        let document = Document::demo();
        Self {
            schema_version: WORKSPACE_DATASET_SCHEMA_VERSION,
            metadata: WorkspaceSnapshotMetadata::current(document.schema_version),
            document,
            mes: FabMesData::sample(),
            yield_analysis: yield_analysis.clone(),
            wafer_map: WaferMap::synthetic_demo(),
            recipe_catalog: RecipeCatalog::sample(),
            genealogy: LotGenealogy::sample(),
            inventory: Inventory::sample(),
            maintenance: MaintenanceModel::sample(),
            environment: CleanroomEnvironment::sample(),
            scheduler: DispatchSchedule::sample(),
            safety: SafetySystem::simulated_demo(),
            experiment_plan: ExperimentPlan::sample(),
            process_control: ProcessControlModel::from_yield_analysis(&yield_analysis),
            process_flow: ProcessFlowModel::sample(),
            cross_section: CrossSectionProcess::sample_sequence(),
            lab_notebook: LabNotebook::sample(),
            equipment: EquipmentSimulator::demo_fab(),
        }
    }

    pub fn builtin_demo_report(&self) -> Result<BuiltinDemoWorkspaceReport, String> {
        let validation = self.validate();
        if !validation.is_valid() {
            return Err(validation.error_summary());
        }
        Ok(BuiltinDemoWorkspaceReport {
            source: "builtin",
            workspace_schema_version: self.schema_version,
            document_schema_version: self.document.schema_version,
            document_shapes: self.document.shapes.len(),
            mes_lots: self.mes.lots.len(),
            yield_lots: self.yield_analysis.lots.len(),
            wafer_dies: self.wafer_map.dies.len(),
            recipes: self.recipe_catalog.recipes.len(),
            process_flow_nodes: self.process_flow.route.nodes.len(),
            cross_section_steps: self.cross_section.steps.len(),
            notebook_entries: self.lab_notebook.entries.len(),
            equipment_tools: self.equipment.tools().count(),
        })
    }

    pub fn from_json_str(contents: &str) -> Result<Self, String> {
        let value = serde_json::from_str::<serde_json::Value>(contents)
            .map_err(|err| format!("failed to parse workspace: {err}"))?;
        preflight_workspace_schema(&value)?;
        let mut dataset = serde_json::from_value::<Self>(value)
            .map_err(|err| format!("failed to parse workspace: {err}"))?;
        dataset.migrate_to_current()?;
        let validation = dataset.validate();
        if !validation.is_valid() {
            return Err(format!(
                "workspace validation failed: {}",
                validation.error_summary()
            ));
        }
        Ok(dataset)
    }

    pub fn migrate_to_current(&mut self) -> Result<Vec<String>, String> {
        match self.schema_version {
            WORKSPACE_DATASET_SCHEMA_VERSION => Ok(Vec::new()),
            0 => {
                self.schema_version = WORKSPACE_DATASET_SCHEMA_VERSION;
                self.metadata = WorkspaceSnapshotMetadata::current(self.document.schema_version);
                let migration = format!("workspace_schema:0->{WORKSPACE_DATASET_SCHEMA_VERSION}");
                self.metadata.migration_history.insert(0, migration.clone());
                Ok(vec![migration])
            }
            schema_version => Err(format!(
                "workspace schema {schema_version} is unsupported; expected {WORKSPACE_DATASET_SCHEMA_VERSION}"
            )),
        }
    }

    pub fn validate(&self) -> WorkspaceValidationReport {
        let mut report = WorkspaceValidationReport::default();
        if self.schema_version != WORKSPACE_DATASET_SCHEMA_VERSION {
            report.push_error(format!(
                "workspace schema {} is unsupported; expected {WORKSPACE_DATASET_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        self.validate_metadata(&mut report);
        validate_document_integrity(&self.document, &mut report);
        validate_mes_links(&self.mes, &mut report);
        validate_recipe_catalog_links(&self.recipe_catalog, &mut report);
        validate_process_flow_links(self, &mut report);
        validate_scheduler_links(self, &mut report);
        validate_yield_links(&self.yield_analysis, &mut report);
        validate_spc_fdc_links(self, &mut report);
        validate_metrology_links(self, &mut report);
        validate_inventory_links(self, &mut report);
        validate_maintenance_links(&self.maintenance, &self.equipment, &mut report);
        validate_environment_links(&self.environment, &mut report);
        validate_safety_links(&self.safety, &self.equipment, &mut report);
        validate_experiment_links(self, &mut report);
        validate_process_control_links(self, &mut report);
        validate_genealogy_links(self, &mut report);
        validate_equipment_links(self, &mut report);
        validate_cross_section_links(&self.cross_section, &mut report);
        validate_notebook_links(self, &mut report);
        report
    }

    fn validate_metadata(&self, report: &mut WorkspaceValidationReport) {
        if self.metadata.is_missing() {
            report.push_warning(
                "workspace snapshot metadata is missing; treating bundle as a legacy snapshot",
            );
            return;
        }
        if self.metadata.producer.trim().is_empty() {
            report.push_error("workspace snapshot metadata producer is empty");
        }
        if self.metadata.producer_version.trim().is_empty() {
            report.push_error("workspace snapshot metadata producer version is empty");
        }
        if self.metadata.workspace_schema_version != self.schema_version {
            report.push_error(format!(
                "workspace metadata schema {} does not match bundle schema {}",
                self.metadata.workspace_schema_version, self.schema_version
            ));
        }
        if self.metadata.document_schema_version != self.document.schema_version {
            report.push_error(format!(
                "workspace metadata document schema {} does not match document schema {}",
                self.metadata.document_schema_version, self.document.schema_version
            ));
        }
        if self.metadata.feature_flags.is_empty() {
            report.push_warning("workspace snapshot metadata has no feature flags");
        } else {
            let supported_flags = WORKSPACE_FEATURE_FLAGS
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            let mut seen_flags = BTreeSet::new();
            for flag in &self.metadata.feature_flags {
                let flag = flag.trim();
                if flag.is_empty() {
                    report.push_error("workspace snapshot metadata has an empty feature flag");
                    continue;
                }
                if !seen_flags.insert(flag.to_string()) {
                    report.push_warning(format!(
                        "workspace snapshot metadata repeats feature flag {flag:?}"
                    ));
                }
                if !supported_flags.contains(flag) {
                    report.push_error(format!(
                        "workspace snapshot requires unsupported feature flag {flag:?}"
                    ));
                }
            }
        }
        if self.metadata.migration_history.is_empty() {
            report.push_warning("workspace snapshot metadata has no migration history");
        } else {
            let mut seen_entries = BTreeSet::new();
            for entry in &self.metadata.migration_history {
                let entry = entry.trim();
                if entry.is_empty() {
                    report.push_error(
                        "workspace snapshot metadata has an empty migration history entry",
                    );
                    continue;
                }
                if !seen_entries.insert(entry.to_string()) {
                    report.push_warning(format!(
                        "workspace snapshot metadata repeats migration history entry {entry:?}"
                    ));
                }
            }
        }
    }
}

fn legacy_workspace_schema_version() -> u32 {
    0
}

fn preflight_workspace_schema(value: &serde_json::Value) -> Result<(), String> {
    if let Some(schema_version) =
        preflight_schema_version(value.get("schema_version"), "workspace schema version")?
        && schema_version > u64::from(WORKSPACE_DATASET_SCHEMA_VERSION)
    {
        return Err(format!(
            "workspace schema {schema_version} is unsupported; expected {WORKSPACE_DATASET_SCHEMA_VERSION}"
        ));
    }
    if let Some(metadata_schema_version) = preflight_schema_version(
        value
            .get("metadata")
            .and_then(|metadata| metadata.get("workspace_schema_version")),
        "workspace metadata schema version",
    )? && metadata_schema_version > u64::from(WORKSPACE_DATASET_SCHEMA_VERSION)
    {
        return Err(format!(
            "workspace metadata schema {metadata_schema_version} is unsupported; expected {WORKSPACE_DATASET_SCHEMA_VERSION}"
        ));
    }
    if let Some(document_schema_version) = preflight_schema_version(
        value
            .get("document")
            .and_then(|document| document.get("schema_version")),
        "document schema version",
    )? && document_schema_version > u64::from(CURRENT_SCHEMA_VERSION)
    {
        return Err(format!(
            "document schema {document_schema_version} is unsupported; expected {CURRENT_SCHEMA_VERSION}"
        ));
    }
    if let Some(metadata_document_schema_version) = preflight_schema_version(
        value
            .get("metadata")
            .and_then(|metadata| metadata.get("document_schema_version")),
        "workspace metadata document schema version",
    )? && metadata_document_schema_version > u64::from(CURRENT_SCHEMA_VERSION)
    {
        return Err(format!(
            "workspace metadata document schema {metadata_document_schema_version} is unsupported; expected {CURRENT_SCHEMA_VERSION}"
        ));
    }
    preflight_string_array(
        value
            .get("metadata")
            .and_then(|metadata| metadata.get("feature_flags")),
        "workspace metadata feature flags",
    )?;
    preflight_string_array(
        value
            .get("metadata")
            .and_then(|metadata| metadata.get("migration_history")),
        "workspace metadata migration history",
    )?;
    Ok(())
}

fn preflight_schema_version(
    value: Option<&serde_json::Value>,
    label: &str,
) -> Result<Option<u64>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if let Some(version) = value.as_u64() {
        return Ok(Some(version));
    }
    if let Some(version) = value.as_i64()
        && version < 0
    {
        return Err(format!(
            "{label} {version} is invalid; expected an unsigned integer"
        ));
    }
    Err(format!("{label} must be an unsigned integer"))
}

fn preflight_string_array(value: Option<&serde_json::Value>, label: &str) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(entries) = value.as_array() else {
        return Err(format!("{label} must be an array of strings"));
    };
    for (index, entry) in entries.iter().enumerate() {
        if !entry.is_string() {
            return Err(format!("{label} entry {index} must be a string"));
        }
    }
    Ok(())
}

fn validate_document_integrity(document: &Document, report: &mut WorkspaceValidationReport) {
    if document.schema_version > CURRENT_SCHEMA_VERSION {
        report.push_error(format!(
            "document schema {} is newer than supported schema {CURRENT_SCHEMA_VERSION}",
            document.schema_version
        ));
    }
    if document.grid <= 0 {
        report.push_error("document grid must be positive");
    }
    if !document.cells.contains_key(&document.top_cell) {
        report.push_error(format!(
            "document top cell {} is missing",
            document.top_cell.0
        ));
    }

    let mut max_shape_id = 0_u64;
    for (shape_id, shape) in document.shapes.iter() {
        max_shape_id = max_shape_id.max(shape_id.0);
        if !document.layers.contains_key(&shape.layer) {
            report.push_error(format!(
                "shape {} references missing layer {}",
                shape_id.0, shape.layer.0
            ));
        }
    }

    let mut max_layer_id = 0_u32;
    let mut layer_names = BTreeMap::<String, Vec<u32>>::new();
    for (layer_id, layer) in document.layers.iter() {
        max_layer_id = max_layer_id.max(layer_id.0);
        if layer.name.trim().is_empty() {
            report.push_error(format!("layer {} has an empty name", layer_id.0));
        } else {
            layer_names
                .entry(layer.name.trim().to_ascii_lowercase())
                .or_default()
                .push(layer_id.0);
        }
        for (channel, value) in ["red", "green", "blue", "alpha"]
            .iter()
            .zip(layer.color.iter())
        {
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                report.push_error(format!(
                    "layer {} color {channel} channel {value} is outside 0..1",
                    layer_id.0
                ));
            }
        }
    }
    for (name, layer_ids) in layer_names {
        if layer_ids.len() > 1 {
            let ids = layer_ids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            report.push_warning(format!(
                "document layer name {name:?} is reused by layers {ids}"
            ));
        }
    }
    if document.next_layer_id <= max_layer_id {
        report.push_error(format!(
            "next layer id {} is not above existing layer id {}",
            document.next_layer_id, max_layer_id
        ));
    }

    let mut max_cell_id = 0_u64;
    let mut max_instance_id = 0_u64;
    for (cell_id, cell) in document.cells.iter() {
        max_cell_id = max_cell_id.max(cell_id.0);
        for (shape_id, shape) in cell.shapes.iter() {
            max_shape_id = max_shape_id.max(shape_id.0);
            if !document.layers.contains_key(&shape.layer) {
                report.push_error(format!(
                    "cell {} shape {} references missing layer {}",
                    cell_id.0, shape_id.0, shape.layer.0
                ));
            }
        }
        for instance in cell.instances.values() {
            max_instance_id = max_instance_id.max(instance.id.0);
            if !document.cells.contains_key(&instance.cell) {
                report.push_error(format!(
                    "cell {} instance {} references missing cell {}",
                    cell_id.0, instance.id.0, instance.cell.0
                ));
            }
        }
    }
    if document.next_shape_id <= max_shape_id {
        report.push_error(format!(
            "next shape id {} is not above existing shape id {}",
            document.next_shape_id, max_shape_id
        ));
    }
    if document.next_cell_id <= max_cell_id {
        report.push_error(format!(
            "next cell id {} is not above existing cell id {}",
            document.next_cell_id, max_cell_id
        ));
    }
    if document.next_instance_id <= max_instance_id {
        report.push_error(format!(
            "next instance id {} is not above existing instance id {}",
            document.next_instance_id, max_instance_id
        ));
    }
    validate_issue_state_keys("DRC marker", &document.marker_states, report);
    validate_issue_state_keys(
        "connectivity issue",
        &document.connectivity_issue_states,
        report,
    );
}

fn validate_issue_state_keys(
    label: &str,
    states: &BTreeMap<String, MarkerState>,
    report: &mut WorkspaceValidationReport,
) {
    for (key, state) in states {
        if key.trim().is_empty() {
            report.push_error(format!("{label} state has an empty stable key"));
        }
        if *state == MarkerState::default() {
            report.push_error(format!(
                "{label} state {key:?} stores a default no-op review state"
            ));
        }
        if state
            .note
            .as_deref()
            .is_some_and(|note| note.trim().is_empty())
        {
            report.push_error(format!("{label} state {key:?} has an empty note"));
        }
    }
}

fn validate_mes_links(mes: &FabMesData, report: &mut WorkspaceValidationReport) {
    for finding in mes.validate() {
        match finding.severity {
            FabMesValidationSeverity::Error => report.push_error(finding.message),
            FabMesValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_recipe_catalog_links(catalog: &RecipeCatalog, report: &mut WorkspaceValidationReport) {
    for finding in catalog.validate() {
        match finding.severity {
            ValidationSeverity::Error => report.push_error(finding.message),
            ValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_process_flow_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    if dataset.process_flow.route.nodes.is_empty()
        && dataset.process_flow.route.edges.is_empty()
        && dataset.process_flow.route.mes_route_id.trim().is_empty()
    {
        return;
    }
    let context = ProcessFlowValidationContext::from_recipe_ids_and_mes(
        dataset
            .recipe_catalog
            .recipes
            .keys()
            .map(|recipe_id| recipe_id.as_str().to_string()),
        &dataset.mes,
    );
    for finding in dataset.process_flow.validate_with_context(&context) {
        match finding.severity {
            ProcessFlowFindingSeverity::Error => report.push_error(finding.message),
            ProcessFlowFindingSeverity::Warning | ProcessFlowFindingSeverity::Info => {
                report.push_warning(finding.message)
            }
        }
    }
}

fn validate_scheduler_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    let context = SchedulerValidationContext::from_recipe_ids_and_mes(
        dataset
            .recipe_catalog
            .recipes
            .keys()
            .map(|recipe_id| recipe_id.as_str().to_string()),
        &dataset.mes,
    )
    .with_equipment(&dataset.equipment);
    for finding in dataset.scheduler.validate_with_context(&context) {
        match finding.severity {
            SchedulerValidationSeverity::Error => report.push_error(finding.message),
            SchedulerValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_yield_links(analysis: &YieldAnalysis, report: &mut WorkspaceValidationReport) {
    for finding in analysis.validate() {
        match finding.severity {
            YieldValidationSeverity::Error => report.push_error(finding.message),
            YieldValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_spc_fdc_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    let sensor_samples = dataset
        .equipment
        .tools()
        .flat_map(|tool| tool.recent_sensors.iter().cloned())
        .collect::<Vec<_>>();
    let alarms = dataset
        .equipment
        .tools()
        .flat_map(|tool| tool.active_alarms.iter().cloned())
        .collect::<Vec<_>>();
    let monitor = SpcFdcMonitor::from_fab_context(
        &dataset.yield_analysis.process_measurements,
        &sensor_samples,
        &alarms,
    );
    for finding in monitor.validate() {
        match finding.severity {
            SpcFdcValidationSeverity::Error => report.push_error(finding.message),
            SpcFdcValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_metrology_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    let yield_lot_wafers = dataset
        .yield_analysis
        .lots
        .iter()
        .map(|lot| {
            (
                lot.id.clone(),
                lot.wafers
                    .iter()
                    .map(|wafer| wafer.id.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let context =
        MetrologyValidationContext::from_mes_and_lot_wafers(&dataset.mes, yield_lot_wafers);
    for finding in dataset.wafer_map.validate_with_context(&context) {
        match finding.severity {
            MetrologyValidationSeverity::Error => report.push_error(finding.message),
            MetrologyValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_inventory_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    let context = InventoryValidationContext::from_mes_and_shape_ids(
        &dataset.mes,
        dataset.document.shapes.keys().map(|shape_id| shape_id.0),
    );
    for finding in dataset.inventory.validate_with_context(&context) {
        match finding.severity {
            InventoryValidationSeverity::Error => report.push_error(finding.message),
            InventoryValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_maintenance_links(
    maintenance: &MaintenanceModel,
    equipment: &EquipmentSimulator,
    report: &mut WorkspaceValidationReport,
) {
    let context = MaintenanceValidationContext::from_equipment(equipment);
    for finding in maintenance.validate_with_context(&context) {
        match finding.severity {
            MaintenanceValidationSeverity::Error => report.push_error(finding.message),
            MaintenanceValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_environment_links(
    environment: &CleanroomEnvironment,
    report: &mut WorkspaceValidationReport,
) {
    for finding in environment.validate() {
        match finding.severity {
            EnvironmentValidationSeverity::Error => report.push_error(finding.message),
            EnvironmentValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_safety_links(
    safety: &SafetySystem,
    equipment: &EquipmentSimulator,
    report: &mut WorkspaceValidationReport,
) {
    let context = SafetyValidationContext::from_equipment(equipment);
    for finding in safety.validate_with_context(&context) {
        match finding.severity {
            SafetyValidationSeverity::Error => report.push_error(finding.message),
            SafetyValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_experiment_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    let context = ExperimentValidationContext::from_workspace_parts(
        &dataset.mes,
        &dataset.recipe_catalog,
        &dataset.equipment,
        &dataset.yield_analysis,
    );
    for finding in dataset.experiment_plan.validate_with_context(&context) {
        match finding.severity {
            ExperimentValidationSeverity::Error => report.push_error(finding.message),
            ExperimentValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_process_control_links(
    dataset: &WorkspaceDataset,
    report: &mut WorkspaceValidationReport,
) {
    let context = ProcessControlValidationContext::from_yield_analysis(&dataset.yield_analysis);
    for finding in dataset.process_control.validate_with_context(&context) {
        match finding.severity {
            ProcessControlValidationSeverity::Error => report.push_error(finding.message),
            ProcessControlValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_genealogy_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    let context = GenealogyValidationContext::from_mes(&dataset.mes);
    for finding in dataset.genealogy.validate_with_context(&context) {
        match finding.severity {
            GenealogyValidationSeverity::Error => report.push_error(finding.message),
            GenealogyValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_equipment_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    let context = EquipmentValidationContext::from_mes_and_recipe_catalog(
        &dataset.mes,
        &dataset.recipe_catalog,
    );
    for finding in dataset.equipment.validate_with_context(&context) {
        match finding.severity {
            EquipmentValidationSeverity::Error => report.push_error(finding.message),
            EquipmentValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_cross_section_links(
    cross_section: &CrossSectionProcess,
    report: &mut WorkspaceValidationReport,
) {
    for finding in cross_section.validate() {
        match finding.severity {
            CrossSectionValidationSeverity::Error => report.push_error(finding.message),
            CrossSectionValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn validate_notebook_links(dataset: &WorkspaceDataset, report: &mut WorkspaceValidationReport) {
    let context = NotebookValidationContext::from_mes_recipes_and_metrology(
        &dataset.mes,
        dataset
            .recipe_catalog
            .recipes
            .keys()
            .map(|recipe_id| recipe_id.as_str().to_string()),
        dataset
            .wafer_map
            .measurements
            .iter()
            .map(|measurement| measurement.id.clone()),
    );
    for finding in dataset.lab_notebook.validate_with_context(&context) {
        match finding.severity {
            NotebookValidationSeverity::Error => report.push_error(finding.message),
            NotebookValidationSeverity::Warning => report.push_warning(finding.message),
        }
    }
}

fn blank_layout_document(name: impl Into<String>) -> Document {
    let mut document = Document::new(name);
    document.layers.clear();
    document.next_layer_id = 1;
    document
}

fn empty_mes_data() -> FabMesData {
    FabMesData {
        routes: BTreeMap::new(),
        lots: BTreeMap::new(),
        travelers: BTreeMap::new(),
    }
}

fn empty_yield_analysis() -> YieldAnalysis {
    YieldAnalysis {
        lots: Vec::new(),
        recipes: Vec::new(),
        test_results: Vec::new(),
        process_measurements: Vec::new(),
        lot_summaries: Vec::new(),
        wafer_summaries: Vec::new(),
        lot_comparisons: Vec::new(),
        correlations: Vec::new(),
    }
}

fn empty_wafer_map() -> WaferMap {
    WaferMap {
        id: String::new(),
        name: "No wafer map loaded".to_string(),
        geometry: WaferGeometry::default(),
        links: FabObjectLinks::default(),
        dies: Vec::new(),
        measurements: Vec::new(),
        defects: Vec::new(),
        annotations: Vec::new(),
    }
}

fn empty_experiment_plan() -> ExperimentPlan {
    ExperimentPlan {
        id: Default::default(),
        title: "No experiment plan loaded".to_string(),
        objective: String::new(),
        owner: String::new(),
        status: ExperimentStatus::Draft,
        route_id: ProcessRouteId::default(),
        step_id: ProcessStepId::default(),
        baseline_recipe: RecipeBinding::new("", 0),
        factors: Vec::new(),
        responses: Vec::new(),
        runs: Vec::new(),
        notes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MarkerState, ShapeKind};

    #[test]
    fn blank_and_demo_workspace_validate_without_app_shell() {
        for dataset in [WorkspaceDataset::blank(), WorkspaceDataset::demo()] {
            let validation = dataset.validate();
            assert!(validation.is_valid(), "{}", validation.error_summary());
        }
    }

    #[test]
    fn workspace_validation_accepts_spc_metric_repeated_across_units() {
        let mut dataset = WorkspaceDataset::demo();
        let mut alternate_unit = dataset
            .yield_analysis
            .process_measurements
            .first()
            .expect("demo workspace has process measurements")
            .clone();
        alternate_unit.measurement_id = "SPC-ALT-UNIT".to_string();
        alternate_unit.unit = "um".to_string();
        dataset
            .yield_analysis
            .process_measurements
            .push(alternate_unit);

        let validation = dataset.validate();

        assert!(validation.is_valid(), "{:?}", validation.errors);
    }

    #[test]
    fn workspace_from_json_defaults_missing_cross_section_process() {
        let mut value = serde_json::to_value(WorkspaceDataset::demo()).unwrap();
        value.as_object_mut().unwrap().remove("cross_section");
        let encoded = serde_json::to_string(&value).unwrap();

        let restored = WorkspaceDataset::from_json_str(&encoded).unwrap();

        assert!(restored.cross_section.steps.is_empty());
        assert!(restored.validate().is_valid());
    }

    #[test]
    fn workspace_validation_rejects_corrupt_cross_section_process() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.cross_section.substrate_material =
            crate::cross_section::MaterialId::from("missing");

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation.errors.iter().any(|error| {
                error.contains("substrate material") && error.contains("material catalog")
            }),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn legacy_workspace_metadata_is_warning_not_error() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.metadata = WorkspaceSnapshotMetadata::default();

        let validation = dataset.validate();

        assert!(validation.is_valid(), "{:?}", validation.errors);
        assert!(
            validation
                .warnings
                .iter()
                .any(|warning| warning.contains("metadata is missing")),
            "{:?}",
            validation.warnings
        );
    }

    #[test]
    fn workspace_from_json_migrates_schema_zero_snapshot() {
        let mut dataset = WorkspaceDataset::blank();
        dataset.schema_version = 0;
        dataset.metadata = WorkspaceSnapshotMetadata::default();
        let encoded = serde_json::to_string(&dataset).unwrap();

        let restored = WorkspaceDataset::from_json_str(&encoded).unwrap();

        assert_eq!(restored.schema_version, WORKSPACE_DATASET_SCHEMA_VERSION);
        assert_eq!(
            restored.metadata.workspace_schema_version,
            WORKSPACE_DATASET_SCHEMA_VERSION
        );
        assert_eq!(
            restored.metadata.document_schema_version,
            restored.document.schema_version
        );
        assert!(
            restored
                .metadata
                .migration_history
                .iter()
                .any(|entry| entry == "workspace_schema:0->1"),
            "{:?}",
            restored.metadata.migration_history
        );
        assert!(restored.validate().is_valid());
    }

    #[test]
    fn workspace_from_json_migrates_source_controlled_schema_zero_fixture() {
        let encoded =
            include_str!("../../../fixtures/persistence/workspace_legacy_schema0_blank.json");

        let restored = WorkspaceDataset::from_json_str(encoded).unwrap();

        assert_eq!(restored.schema_version, WORKSPACE_DATASET_SCHEMA_VERSION);
        assert_eq!(
            restored.metadata.workspace_schema_version,
            WORKSPACE_DATASET_SCHEMA_VERSION
        );
        assert_eq!(
            restored.metadata.document_schema_version,
            restored.document.schema_version
        );
        assert!(
            restored
                .metadata
                .migration_history
                .iter()
                .any(|entry| entry == "workspace_schema:0->1"),
            "{:?}",
            restored.metadata.migration_history
        );
        assert_eq!(restored.document.name, "Legacy schema 0 blank layout");
        assert!(restored.document.layers.is_empty());
        assert!(restored.document.shapes.is_empty());
        assert_eq!(
            restored
                .document
                .marker_states
                .get("legacy_rule|1|0,0,10,10|200|80.000")
                .and_then(|state| state.note.as_deref()),
            Some("legacy hidden marker")
        );
        assert_eq!(
            restored
                .document
                .connectivity_issue_states
                .get("short|VDD,VSS|0,0,100,50")
                .and_then(|state| state.note.as_deref()),
            Some("legacy connectivity waiver")
        );

        let encoded_current = serde_json::to_string_pretty(&restored).unwrap();
        let reparsed = WorkspaceDataset::from_json_str(&encoded_current).unwrap();
        assert_eq!(reparsed.schema_version, WORKSPACE_DATASET_SCHEMA_VERSION);
        assert!(reparsed.validate().is_valid());
    }

    #[test]
    fn workspace_from_json_migrates_missing_schema_snapshot() {
        let dataset = WorkspaceDataset::blank();
        let mut value = serde_json::to_value(&dataset).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("schema_version");
        object.remove("metadata");
        let encoded = serde_json::to_string(&value).unwrap();

        let restored = WorkspaceDataset::from_json_str(&encoded).unwrap();

        assert_eq!(restored.schema_version, WORKSPACE_DATASET_SCHEMA_VERSION);
        assert!(
            restored
                .metadata
                .migration_history
                .iter()
                .any(|entry| entry == "workspace_schema:0->1")
        );
        assert!(restored.validate().is_valid());
    }

    #[test]
    fn workspace_from_json_rejects_future_schema_snapshot() {
        let mut dataset = WorkspaceDataset::blank();
        dataset.schema_version = WORKSPACE_DATASET_SCHEMA_VERSION + 1;
        let encoded = serde_json::to_string(&dataset).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(err.contains("unsupported"));
        assert!(err.contains(&WORKSPACE_DATASET_SCHEMA_VERSION.to_string()));
    }

    #[test]
    fn workspace_from_json_rejects_future_schema_before_payload_deserialization() {
        let encoded =
            include_str!("../../../fixtures/persistence/workspace_future_schema_preflight.json");

        let err = WorkspaceDataset::from_json_str(encoded).unwrap_err();

        assert!(err.contains("unsupported"), "{err}");
        assert!(err.contains(&WORKSPACE_DATASET_SCHEMA_VERSION.to_string()));
        assert!(!err.contains("missing field"), "{err}");
    }

    #[test]
    fn workspace_from_json_rejects_future_metadata_before_legacy_migration() {
        let encoded =
            include_str!("../../../fixtures/persistence/workspace_future_metadata_preflight.json");

        let err = WorkspaceDataset::from_json_str(encoded).unwrap_err();

        assert!(err.contains("metadata schema 999"), "{err}");
        assert!(err.contains(&WORKSPACE_DATASET_SCHEMA_VERSION.to_string()));
        assert!(!err.contains("missing field"), "{err}");
    }

    #[test]
    fn workspace_from_json_rejects_future_document_before_payload_deserialization() {
        let encoded = include_str!(
            "../../../fixtures/persistence/workspace_future_document_schema_preflight.json"
        );

        let err = WorkspaceDataset::from_json_str(encoded).unwrap_err();

        assert!(err.contains("document schema 999"), "{err}");
        assert!(err.contains(&CURRENT_SCHEMA_VERSION.to_string()));
        assert!(!err.contains("missing field"), "{err}");
    }

    #[test]
    fn workspace_from_json_rejects_malformed_schema_fields_before_deserialization() {
        let mut value = serde_json::to_value(WorkspaceDataset::blank()).unwrap();
        value["schema_version"] = serde_json::Value::String("2".to_string());
        let encoded = serde_json::to_string(&value).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(err.contains("workspace schema version"), "{err}");
        assert!(err.contains("unsigned integer"), "{err}");
        assert!(!err.contains("failed to parse workspace"), "{err}");

        let mut value = serde_json::to_value(WorkspaceDataset::blank()).unwrap();
        value["metadata"]["document_schema_version"] = serde_json::json!(-1);
        let encoded = serde_json::to_string(&value).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(
            err.contains("workspace metadata document schema version -1"),
            "{err}"
        );
        assert!(err.contains("unsigned integer"), "{err}");
        assert!(!err.contains("failed to parse workspace"), "{err}");
    }

    #[test]
    fn workspace_from_json_rejects_malformed_metadata_arrays_before_deserialization() {
        let mut value = serde_json::to_value(WorkspaceDataset::blank()).unwrap();
        value["metadata"]["feature_flags"] = serde_json::Value::String("layout_document".into());
        let encoded = serde_json::to_string(&value).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(err.contains("workspace metadata feature flags"), "{err}");
        assert!(err.contains("array of strings"), "{err}");
        assert!(!err.contains("failed to parse workspace"), "{err}");

        let mut value = serde_json::to_value(WorkspaceDataset::blank()).unwrap();
        value["metadata"]["migration_history"] = serde_json::json!(["workspace_schema:1", false]);
        let encoded = serde_json::to_string(&value).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(
            err.contains("workspace metadata migration history"),
            "{err}"
        );
        assert!(err.contains("entry 1"), "{err}");
        assert!(err.contains("must be a string"), "{err}");
        assert!(!err.contains("failed to parse workspace"), "{err}");
    }

    #[test]
    fn workspace_from_json_rejects_source_controlled_malformed_metadata_fixture() {
        let encoded = include_str!(
            "../../../fixtures/persistence/workspace_malformed_metadata_arrays_preflight.json"
        );

        let err = WorkspaceDataset::from_json_str(encoded).unwrap_err();

        assert!(err.contains("workspace metadata feature flags"), "{err}");
        assert!(err.contains("entry 1"), "{err}");
        assert!(!err.contains("failed to parse workspace"), "{err}");
        assert!(!err.contains("missing field"), "{err}");
    }

    #[test]
    fn workspace_from_json_rejects_unsupported_feature_flags() {
        let mut dataset = WorkspaceDataset::blank();
        dataset
            .metadata
            .feature_flags
            .push("future_mask_revision_model".to_string());
        let encoded = serde_json::to_string(&dataset).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(err.contains("unsupported feature flag"), "{err}");
        assert!(err.contains("future_mask_revision_model"), "{err}");
    }

    #[test]
    fn workspace_from_json_rejects_source_controlled_unsupported_feature_fixture() {
        let encoded =
            include_str!("../../../fixtures/persistence/workspace_unsupported_feature_flag.json");

        let err = WorkspaceDataset::from_json_str(encoded).unwrap_err();

        assert!(err.contains("unsupported feature flag"), "{err}");
        assert!(err.contains("future_mask_revision_model"), "{err}");
        assert!(!err.contains("missing field"), "{err}");
    }

    #[test]
    fn workspace_metadata_repeated_feature_flags_are_warning_only() {
        let mut dataset = WorkspaceDataset::blank();
        dataset
            .metadata
            .feature_flags
            .push("layout_document".to_string());

        let validation = dataset.validate();

        assert!(validation.is_valid(), "{:?}", validation.errors);
        assert!(
            validation
                .warnings
                .iter()
                .any(|warning| warning.contains("repeats feature flag")),
            "{:?}",
            validation.warnings
        );
    }

    #[test]
    fn workspace_metadata_rejects_empty_producer_version() {
        let mut dataset = WorkspaceDataset::blank();
        dataset.metadata.producer_version = "   ".to_string();
        let encoded = serde_json::to_string(&dataset).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(err.contains("producer version is empty"), "{err}");
    }

    #[test]
    fn workspace_metadata_rejects_empty_migration_history_entries() {
        let mut dataset = WorkspaceDataset::blank();
        dataset.metadata.migration_history.push("   ".to_string());
        let encoded = serde_json::to_string(&dataset).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(err.contains("empty migration history entry"), "{err}");
    }

    #[test]
    fn workspace_metadata_repeated_migration_history_entries_are_warning_only() {
        let mut dataset = WorkspaceDataset::blank();
        dataset
            .metadata
            .migration_history
            .push("workspace_schema:1".to_string());

        let validation = dataset.validate();

        assert!(validation.is_valid(), "{:?}", validation.errors);
        assert!(
            validation
                .warnings
                .iter()
                .any(|warning| warning.contains("repeats migration history entry")),
            "{:?}",
            validation.warnings
        );
    }

    #[test]
    fn workspace_from_json_rejects_cross_domain_reference_corruption() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.recipe_catalog.recipes.clear();
        let encoded = serde_json::to_string(&dataset).unwrap();

        let err = WorkspaceDataset::from_json_str(&encoded).unwrap_err();

        assert!(err.contains("workspace validation failed"), "{err}");
        assert!(err.contains("recipe"), "{err}");
    }

    #[test]
    fn workspace_validation_rejects_document_and_domain_corruption() {
        let mut dataset = WorkspaceDataset::demo();
        let layer = dataset
            .document
            .shapes
            .values()
            .next()
            .expect("demo has shapes")
            .layer;
        dataset.document.layers.remove(&layer);
        dataset.mes.lots.clear();
        dataset.safety.tool_interlocks[0]
            .required_sensors
            .push("MISSING_SENSOR".into());

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("missing layer")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("missing lot")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("safety tool interlock")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_rejects_corrupt_scheduler_assignments() {
        let mut dataset = WorkspaceDataset::demo();
        let duplicate = dataset
            .scheduler
            .assignments
            .first()
            .expect("demo scheduler has assignments")
            .clone();
        dataset.scheduler.assignments.push(duplicate);

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("multiple persisted assignments")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("overlapping assignments")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_rejects_corrupt_layer_metadata() {
        let mut dataset = WorkspaceDataset::demo();
        let layer_id = dataset
            .document
            .layer_by_process(crate::ProcessLayer::Metal1)
            .unwrap();
        let layer = dataset.document.layers.get_mut(&layer_id).unwrap();
        layer.name = "   ".to_string();
        layer.color[0] = 1.5;
        layer.color[3] = f32::INFINITY;

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("empty name")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("color red")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("color alpha")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_warns_about_duplicate_layer_names() {
        let mut dataset = WorkspaceDataset::demo();
        let metal1 = dataset
            .document
            .layer_by_process(crate::ProcessLayer::Metal1)
            .unwrap();
        let metal2 = dataset
            .document
            .layer_by_process(crate::ProcessLayer::Metal2)
            .unwrap();
        let name = dataset.document.layers.get(&metal1).unwrap().name.clone();
        dataset.document.layers.get_mut(&metal2).unwrap().name = name;

        let validation = dataset.validate();

        assert!(validation.is_valid(), "{:?}", validation.errors);
        assert!(
            validation
                .warnings
                .iter()
                .any(|warning| warning.contains("layer name")),
            "{:?}",
            validation.warnings
        );
    }

    #[test]
    fn workspace_validation_checks_genealogy_against_mes_context() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.genealogy.process_history[0].step_id = ProcessStepId::new("MISSING_STEP");

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("missing MES step MISSING_STEP")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_checks_hierarchy_id_counters() {
        let mut dataset = WorkspaceDataset::blank();
        dataset.document = Document::hierarchy_demo();
        dataset.metadata = WorkspaceSnapshotMetadata::current(dataset.document.schema_version);

        let max_child_shape_id = dataset
            .document
            .cells
            .values()
            .flat_map(|cell| cell.shapes.keys().map(|shape_id| shape_id.0))
            .max()
            .expect("hierarchy demo has child-cell shapes");
        let max_cell_id = dataset
            .document
            .cells
            .keys()
            .map(|cell_id| cell_id.0)
            .max()
            .expect("hierarchy demo has cells");
        let max_instance_id = dataset
            .document
            .cells
            .values()
            .flat_map(|cell| cell.instances.keys().map(|instance_id| instance_id.0))
            .max()
            .expect("hierarchy demo has instances");

        dataset.document.next_shape_id = max_child_shape_id;
        dataset.document.next_cell_id = max_cell_id;
        dataset.document.next_instance_id = max_instance_id;

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("next shape id")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("next cell id")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("next instance id")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_rejects_recipe_catalog_corruption() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.recipe_catalog.process_routes[0].steps[0].recipe =
            Some(RecipeBinding::new("MISSING_RECIPE", 1));

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("references missing recipe MISSING_RECIPE")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_checks_maintenance_against_equipment_context() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.maintenance.qualification_results[0].recipe_id = "NO_SUCH_RECIPE".to_string();

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation.errors.iter().any(|error| error
                .contains("qualification result qual-1 references recipe NO_SUCH_RECIPE")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_checks_process_control_against_yield_context() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.process_control.actions[0].target_recipe =
            RecipeBinding::new("NO_SUCH_R2R_RECIPE", 1);

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("process-control action")
                    && error.contains("missing target recipe binding NO_SUCH_R2R_RECIPE v1")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_checks_safety_against_equipment_context() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.safety.tool_interlocks[0].tool_id = "MISSING-SAFETY-TOOL".to_string();

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation.errors.iter().any(|error| error.contains(
                "safety tool interlock MISSING-SAFETY-TOOL references missing equipment tool"
            )),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_checks_equipment_against_mes_and_recipe_context() {
        let mut dataset = WorkspaceDataset::demo();
        let tool = dataset
            .equipment
            .tool_mut(&crate::equipment::ToolId::new("COAT-01"))
            .unwrap();
        let run = tool.active_run.as_mut().unwrap();
        run.recipe.wafer_id = Some("L-00042-W99".to_string());
        run.recipe.process_step_id = Some("S999-MISSING".to_string());

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("missing MES wafer L-00042-W99")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("missing MES step S999-MISSING")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_checks_experiment_against_workspace_context() {
        let mut dataset = WorkspaceDataset::demo();
        dataset.experiment_plan.runs[0].assignment.wafer_id =
            crate::mes::WaferId::new("MISSING-DOE-WAFER");

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation.errors.iter().any(|error| error
                .contains("experiment run DOE-POLY-CD-0042-R01 references missing MES lot/wafer")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn builtin_demo_report_validates_shared_workspace() {
        let report = WorkspaceDataset::demo().builtin_demo_report().unwrap();

        assert_eq!(report.source, "builtin");
        assert_eq!(
            report.workspace_schema_version,
            WORKSPACE_DATASET_SCHEMA_VERSION
        );
        assert!(report.document_shapes > 0);
        assert!(report.mes_lots > 0);
        assert!(report.cross_section_steps > 0);
        assert!(report.equipment_tools > 0);
    }

    #[test]
    fn marker_states_remain_part_of_workspace_document() {
        let mut dataset = WorkspaceDataset::blank();
        dataset.document.marker_states.insert(
            "drc|fixture".to_string(),
            MarkerState {
                hidden: true,
                waived: true,
                note: Some("reviewed".to_string()),
            },
        );
        dataset.document.connectivity_issue_states.insert(
            "short|VDD,VSS|0,0,100,50".to_string(),
            MarkerState {
                hidden: false,
                waived: true,
                note: Some("net reviewed".to_string()),
            },
        );
        let encoded = serde_json::to_string(&dataset).unwrap();
        let restored: WorkspaceDataset = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            restored.document.marker_states["drc|fixture"]
                .note
                .as_deref(),
            Some("reviewed")
        );
        assert_eq!(
            restored.document.connectivity_issue_states["short|VDD,VSS|0,0,100,50"]
                .note
                .as_deref(),
            Some("net reviewed")
        );
    }

    #[test]
    fn workspace_validation_rejects_empty_issue_state_keys() {
        let mut dataset = WorkspaceDataset::blank();
        dataset
            .document
            .connectivity_issue_states
            .insert(" ".to_string(), MarkerState::default());

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("connectivity issue state")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn workspace_validation_rejects_noop_issue_review_states() {
        let mut dataset = WorkspaceDataset::blank();
        dataset
            .document
            .marker_states
            .insert("drc|noop".to_string(), MarkerState::default());
        dataset.document.connectivity_issue_states.insert(
            "short|noop".to_string(),
            MarkerState {
                hidden: false,
                waived: false,
                note: Some("   ".to_string()),
            },
        );

        let validation = dataset.validate();

        assert!(!validation.is_valid());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("default no-op review state")),
            "{:?}",
            validation.errors
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("empty note")),
            "{:?}",
            validation.errors
        );
    }

    #[test]
    fn blank_workspace_has_no_layers_or_shapes() {
        let dataset = WorkspaceDataset::blank();

        assert!(dataset.document.layers.is_empty());
        assert!(dataset.document.shapes.is_empty());
        assert!(
            dataset
                .document
                .cells
                .get(&dataset.document.top_cell)
                .is_some_and(|cell| cell.shapes.is_empty())
        );
        assert!(dataset.document.shapes.values().all(|shape| {
            !matches!(
                shape.kind,
                ShapeKind::Rectangle(_) | ShapeKind::Polygon(_) | ShapeKind::Path { .. }
            )
        }));
    }
}
