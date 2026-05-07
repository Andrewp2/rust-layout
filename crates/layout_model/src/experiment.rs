use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    ProcessLayer,
    mes::{Lot, LotId, ProcessRouteId, ProcessStepId, ToolId, WaferId},
    metrology::MeasurementKind,
    recipe::RecipeBinding,
};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

string_id!(ExperimentId);
string_id!(ExperimentRunId);
string_id!(FactorId);
string_id!(FactorLevelId);
string_id!(ResponseSpecId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Draft,
    MatrixGenerated,
    Running,
    AnalysisReady,
}

impl ExperimentStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::MatrixGenerated => "matrix ready",
            Self::Running => "running",
            Self::AnalysisReady => "analysis ready",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentPlan {
    pub id: ExperimentId,
    pub title: String,
    pub objective: String,
    pub owner: String,
    pub status: ExperimentStatus,
    pub route_id: ProcessRouteId,
    pub step_id: ProcessStepId,
    pub baseline_recipe: RecipeBinding,
    #[serde(default)]
    pub factors: Vec<ExperimentFactor>,
    #[serde(default)]
    pub responses: Vec<ResponseSpec>,
    #[serde(default)]
    pub runs: Vec<ExperimentRun>,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl ExperimentPlan {
    pub fn sample() -> Self {
        sample_experiment_plan()
    }

    pub fn matrix_size(&self) -> Result<usize, MatrixGenerationError> {
        if self.factors.is_empty() {
            return Err(MatrixGenerationError::NoFactors);
        }

        let mut size = 1usize;
        for factor in &self.factors {
            if factor.levels.is_empty() {
                return Err(MatrixGenerationError::FactorHasNoLevels(factor.id.clone()));
            }
            size = size.saturating_mul(factor.levels.len());
        }
        Ok(size)
    }

    pub fn generate_full_factorial(
        &mut self,
        assignments: Vec<ExperimentAssignment>,
    ) -> Result<(), MatrixGenerationError> {
        let combinations = self.factor_level_combinations()?;
        let required = combinations.len();
        if assignments.len() < required {
            return Err(MatrixGenerationError::InsufficientAssignments {
                required,
                available: assignments.len(),
            });
        }

        self.runs = combinations
            .into_iter()
            .enumerate()
            .map(|(index, factor_levels)| {
                let mut assignment = assignments[index].clone();
                assignment.run_order = (index + 1) as u32;
                ExperimentRun {
                    id: ExperimentRunId::new(format!("{}-R{:02}", self.id.as_str(), index + 1)),
                    assignment,
                    factor_levels,
                    recipe: Some(self.baseline_recipe.clone()),
                    tool_id: None,
                    status: ExperimentRunStatus::Ready,
                    responses: BTreeMap::new(),
                }
            })
            .collect();
        self.status = ExperimentStatus::MatrixGenerated;
        Ok(())
    }

    pub fn factor_level_combinations(
        &self,
    ) -> Result<Vec<BTreeMap<FactorId, FactorLevelId>>, MatrixGenerationError> {
        self.matrix_size()?;
        let mut combinations = vec![BTreeMap::new()];
        for factor in &self.factors {
            let mut next = Vec::with_capacity(combinations.len() * factor.levels.len());
            for combination in &combinations {
                for level in &factor.levels {
                    let mut candidate = combination.clone();
                    candidate.insert(factor.id.clone(), level.id.clone());
                    next.push(candidate);
                }
            }
            combinations = next;
        }
        Ok(combinations)
    }

    pub fn capture_response(
        &mut self,
        run_id: &ExperimentRunId,
        response_id: &ResponseSpecId,
        value: ResponseValue,
    ) -> Result<(), ResponseCaptureError> {
        if self.response(response_id).is_none() {
            return Err(ResponseCaptureError::UnknownResponse(response_id.clone()));
        }
        let expected_response_count = self.responses.len();
        let run = self
            .run_mut(run_id)
            .ok_or_else(|| ResponseCaptureError::UnknownRun(run_id.clone()))?;
        run.responses.insert(response_id.clone(), value);
        run.status =
            if expected_response_count > 0 && run.responses.len() >= expected_response_count {
                ExperimentRunStatus::Complete
            } else {
                ExperimentRunStatus::InProgress
            };
        self.refresh_status();
        Ok(())
    }

    pub fn analysis_summary(
        &self,
        primary_response_id: Option<&ResponseSpecId>,
    ) -> ExperimentAnalysisSummary {
        let primary_response_id = primary_response_id
            .cloned()
            .or_else(|| self.responses.first().map(|response| response.id.clone()));
        let response_stats = self
            .responses
            .iter()
            .map(|response| self.response_stats(response))
            .collect::<Vec<_>>();
        let factor_effects = primary_response_id
            .as_ref()
            .and_then(|response_id| self.factor_effects(response_id).ok())
            .unwrap_or_default();
        let best_run_id = primary_response_id.as_ref().and_then(|response_id| {
            self.best_run_for_response(response_id)
                .map(|run| run.id.clone())
        });
        let missing_response_count = self
            .runs
            .iter()
            .map(|run| self.responses.len().saturating_sub(run.responses.len()))
            .sum();

        let mut notes = Vec::new();
        if let Some(best_run_id) = &best_run_id {
            notes.push(format!(
                "Best observed run for primary response: {best_run_id}."
            ));
        }
        if missing_response_count > 0 {
            notes.push(format!(
                "{missing_response_count} response values are still pending."
            ));
        }
        if self.runs.is_empty() {
            notes.push(
                "Generate a run matrix before assigning tool runs or capturing responses."
                    .to_string(),
            );
        }

        ExperimentAnalysisSummary {
            run_count: self.runs.len(),
            completed_runs: self
                .runs
                .iter()
                .filter(|run| run.status == ExperimentRunStatus::Complete)
                .count(),
            missing_response_count,
            primary_response_id,
            best_run_id,
            response_stats,
            factor_effects,
            notes,
        }
    }

    pub fn factor(&self, factor_id: &FactorId) -> Option<&ExperimentFactor> {
        self.factors.iter().find(|factor| factor.id == *factor_id)
    }

    pub fn response(&self, response_id: &ResponseSpecId) -> Option<&ResponseSpec> {
        self.responses
            .iter()
            .find(|response| response.id == *response_id)
    }

    pub fn run(&self, run_id: &ExperimentRunId) -> Option<&ExperimentRun> {
        self.runs.iter().find(|run| run.id == *run_id)
    }

    pub fn run_mut(&mut self, run_id: &ExperimentRunId) -> Option<&mut ExperimentRun> {
        self.runs.iter_mut().find(|run| run.id == *run_id)
    }

    pub fn factor_level(
        &self,
        factor_id: &FactorId,
        level_id: &FactorLevelId,
    ) -> Option<&FactorLevel> {
        self.factor(factor_id)?
            .levels
            .iter()
            .find(|level| level.id == *level_id)
    }

    pub fn factor_level_label(&self, factor_id: &FactorId, level_id: &FactorLevelId) -> String {
        self.factor_level(factor_id, level_id)
            .map(|level| level.label.clone())
            .unwrap_or_else(|| level_id.to_string())
    }

    pub fn run_factor_label(&self, run: &ExperimentRun, factor: &ExperimentFactor) -> String {
        run.factor_levels
            .get(&factor.id)
            .map(|level_id| self.factor_level_label(&factor.id, level_id))
            .unwrap_or_else(|| "unassigned".to_string())
    }

    fn response_stats(&self, response: &ResponseSpec) -> ResponseStats {
        let values = self
            .runs
            .iter()
            .filter_map(|run| run.responses.get(&response.id).map(|value| value.value))
            .collect::<Vec<_>>();
        let sample_count = values.len();
        let missing_count = self.runs.len().saturating_sub(sample_count);
        let mean = mean(&values);
        let min = values.iter().copied().reduce(f64::min);
        let max = values.iter().copied().reduce(f64::max);
        let std_dev = mean.map(|mean| {
            let variance = values
                .iter()
                .map(|value| {
                    let delta = *value - mean;
                    delta * delta
                })
                .sum::<f64>()
                / values.len().max(1) as f64;
            variance.sqrt()
        });

        ResponseStats {
            response_id: response.id.clone(),
            sample_count,
            missing_count,
            mean,
            min,
            max,
            std_dev,
        }
    }

    fn factor_effects(
        &self,
        response_id: &ResponseSpecId,
    ) -> Result<Vec<FactorEffect>, ResponseCaptureError> {
        if self.response(response_id).is_none() {
            return Err(ResponseCaptureError::UnknownResponse(response_id.clone()));
        }

        let all_values = self
            .runs
            .iter()
            .filter_map(|run| run.responses.get(response_id).map(|value| value.value))
            .collect::<Vec<_>>();
        let Some(overall_mean) = mean(&all_values) else {
            return Ok(Vec::new());
        };

        let mut effects = Vec::new();
        for factor in &self.factors {
            for level in &factor.levels {
                let values = self
                    .runs
                    .iter()
                    .filter(|run| run.factor_levels.get(&factor.id) == Some(&level.id))
                    .filter_map(|run| run.responses.get(response_id).map(|value| value.value))
                    .collect::<Vec<_>>();
                if let Some(level_mean) = mean(&values) {
                    effects.push(FactorEffect {
                        factor_id: factor.id.clone(),
                        factor_name: factor.name.clone(),
                        level_id: level.id.clone(),
                        level_label: level.label.clone(),
                        sample_count: values.len(),
                        mean_response: level_mean,
                        delta_from_overall: level_mean - overall_mean,
                    });
                }
            }
        }
        Ok(effects)
    }

    fn best_run_for_response(&self, response_id: &ResponseSpecId) -> Option<&ExperimentRun> {
        let response = self.response(response_id)?;
        self.runs
            .iter()
            .filter(|run| run.responses.contains_key(response_id))
            .min_by(|left, right| {
                let left_value = left.responses[response_id].value;
                let right_value = right.responses[response_id].value;
                response
                    .score_value(left_value)
                    .partial_cmp(&response.score_value(right_value))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    fn refresh_status(&mut self) {
        if self.runs.is_empty() {
            self.status = ExperimentStatus::Draft;
        } else if self
            .runs
            .iter()
            .all(|run| run.status == ExperimentRunStatus::Complete)
        {
            self.status = ExperimentStatus::AnalysisReady;
        } else if self.runs.iter().any(|run| !run.responses.is_empty()) {
            self.status = ExperimentStatus::Running;
        } else {
            self.status = ExperimentStatus::MatrixGenerated;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentFactor {
    pub id: FactorId,
    pub name: String,
    pub unit: Option<String>,
    pub source: FactorSource,
    pub levels: Vec<FactorLevel>,
}

impl ExperimentFactor {
    pub fn level(&self, level_id: &FactorLevelId) -> Option<&FactorLevel> {
        self.levels.iter().find(|level| level.id == *level_id)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum FactorSource {
    RecipeParameter {
        recipe: RecipeBinding,
        parameter: String,
    },
    ToolSetting {
        tool_id: ToolId,
        setting: String,
    },
    ProcessStep {
        step_id: ProcessStepId,
        field: String,
    },
    Manual,
}

impl FactorSource {
    pub fn label(&self) -> String {
        match self {
            Self::RecipeParameter { recipe, parameter } => {
                format!("{recipe} parameter {parameter}")
            }
            Self::ToolSetting { tool_id, setting } => format!("{tool_id} setting {setting}"),
            Self::ProcessStep { step_id, field } => format!("{step_id} {field}"),
            Self::Manual => "manual split factor".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactorLevel {
    pub id: FactorLevelId,
    pub label: String,
    pub value: FactorValue,
}

impl FactorLevel {
    pub fn value_label(&self, unit: Option<&str>) -> String {
        let value = self.value.label();
        match unit {
            Some(unit) if !unit.is_empty() => format!("{value} {unit}"),
            _ => value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FactorValue {
    Numeric(f64),
    Integer(i64),
    Text(String),
    Boolean(bool),
}

impl FactorValue {
    pub fn label(&self) -> String {
        match self {
            Self::Numeric(value) => format_compact_number(*value),
            Self::Integer(value) => value.to_string(),
            Self::Text(value) => value.clone(),
            Self::Boolean(value) => value.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseSpec {
    pub id: ResponseSpecId,
    pub name: String,
    pub unit: String,
    pub goal: ResponseGoal,
    pub target: Option<f64>,
    pub lower_spec: Option<f64>,
    pub upper_spec: Option<f64>,
    pub source: ResponseSource,
}

impl ResponseSpec {
    pub fn score_value(&self, value: f64) -> f64 {
        match self.goal {
            ResponseGoal::Target => self
                .target
                .map(|target| (value - target).abs())
                .unwrap_or(value.abs()),
            ResponseGoal::Maximize => -value,
            ResponseGoal::Minimize => value,
        }
    }

    pub fn value_status(&self, value: f64) -> ResponseValueStatus {
        if self.lower_spec.is_some_and(|lower| value < lower) {
            ResponseValueStatus::BelowSpec
        } else if self.upper_spec.is_some_and(|upper| value > upper) {
            ResponseValueStatus::AboveSpec
        } else {
            ResponseValueStatus::InSpec
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseGoal {
    Target,
    Maximize,
    Minimize,
}

impl ResponseGoal {
    pub fn label(self) -> &'static str {
        match self {
            Self::Target => "target",
            Self::Maximize => "maximize",
            Self::Minimize => "minimize",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseValueStatus {
    InSpec,
    BelowSpec,
    AboveSpec,
}

impl ResponseValueStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::InSpec => "in spec",
            Self::BelowSpec => "below spec",
            Self::AboveSpec => "above spec",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum ResponseSource {
    MetrologyMeasurement {
        kind: MeasurementKind,
        process_step_id: ProcessStepId,
    },
    YieldMetric {
        metric: String,
    },
    ProcessMeasurement {
        name: String,
        layer: Option<ProcessLayer>,
    },
    Manual,
}

impl ResponseSource {
    pub fn label(&self) -> String {
        match self {
            Self::MetrologyMeasurement {
                kind,
                process_step_id,
            } => format!("{} at {process_step_id}", kind.label()),
            Self::YieldMetric { metric } => format!("yield metric {metric}"),
            Self::ProcessMeasurement { name, layer } => match layer {
                Some(layer) => format!("{name} on {layer:?}"),
                None => name.clone(),
            },
            Self::Manual => "manual response".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseValue {
    pub value: f64,
    pub measurement_id: Option<String>,
    pub captured_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentRunStatus {
    Ready,
    InProgress,
    Complete,
    Blocked,
}

impl ExperimentRunStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::InProgress => "in progress",
            Self::Complete => "complete",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentAssignment {
    pub lot_id: LotId,
    pub wafer_id: WaferId,
    pub slot: u8,
    pub block: Option<String>,
    pub run_order: u32,
}

impl ExperimentAssignment {
    pub fn wafer_assignments_for_lot(lot: &Lot, block: Option<String>) -> Vec<Self> {
        lot.wafers
            .iter()
            .filter(|wafer| !wafer.status.is_scrapped())
            .enumerate()
            .map(|(index, wafer)| Self {
                lot_id: lot.id.clone(),
                wafer_id: wafer.id.clone(),
                slot: wafer.slot,
                block: block.clone(),
                run_order: (index + 1) as u32,
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentRun {
    pub id: ExperimentRunId,
    pub assignment: ExperimentAssignment,
    pub factor_levels: BTreeMap<FactorId, FactorLevelId>,
    pub recipe: Option<RecipeBinding>,
    pub tool_id: Option<ToolId>,
    pub status: ExperimentRunStatus,
    #[serde(default)]
    pub responses: BTreeMap<ResponseSpecId, ResponseValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentAnalysisSummary {
    pub run_count: usize,
    pub completed_runs: usize,
    pub missing_response_count: usize,
    pub primary_response_id: Option<ResponseSpecId>,
    pub best_run_id: Option<ExperimentRunId>,
    pub response_stats: Vec<ResponseStats>,
    pub factor_effects: Vec<FactorEffect>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResponseStats {
    pub response_id: ResponseSpecId,
    pub sample_count: usize,
    pub missing_count: usize,
    pub mean: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub std_dev: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactorEffect {
    pub factor_id: FactorId,
    pub factor_name: String,
    pub level_id: FactorLevelId,
    pub level_label: String,
    pub sample_count: usize,
    pub mean_response: f64,
    pub delta_from_overall: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixGenerationError {
    NoFactors,
    FactorHasNoLevels(FactorId),
    InsufficientAssignments { required: usize, available: usize },
}

impl fmt::Display for MatrixGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFactors => f.write_str("experiment has no factors"),
            Self::FactorHasNoLevels(factor_id) => write!(f, "factor {factor_id} has no levels"),
            Self::InsufficientAssignments {
                required,
                available,
            } => write!(
                f,
                "run matrix needs {required} wafer assignments, but only {available} were provided"
            ),
        }
    }
}

impl Error for MatrixGenerationError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseCaptureError {
    UnknownRun(ExperimentRunId),
    UnknownResponse(ResponseSpecId),
}

impl fmt::Display for ResponseCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRun(run_id) => write!(f, "experiment run {run_id} was not found"),
            Self::UnknownResponse(response_id) => {
                write!(
                    f,
                    "response {response_id} is not defined for the experiment"
                )
            }
        }
    }
}

impl Error for ResponseCaptureError {}

pub fn sample_experiment_plan() -> ExperimentPlan {
    let baseline_recipe = RecipeBinding::new("LITHO_POLY_EXPOSE_001", 1);
    let mut plan = ExperimentPlan {
        id: ExperimentId::new("DOE-POLY-CD-0042"),
        title: "Poly lithography CD split".to_string(),
        objective:
            "Tune exposure dose and focus offset for tighter post-etch poly CD on the demo inverter lot."
                .to_string(),
        owner: "process.integration".to_string(),
        status: ExperimentStatus::Draft,
        route_id: ProcessRouteId::new("ROUTE-DEMO-INVERTER-POLY-A"),
        step_id: ProcessStepId::new("S020-EXPOSE"),
        baseline_recipe: baseline_recipe.clone(),
        factors: vec![
            ExperimentFactor {
                id: FactorId::new("dose"),
                name: "Exposure dose".to_string(),
                unit: Some("mJ/cm2".to_string()),
                source: FactorSource::RecipeParameter {
                    recipe: baseline_recipe.clone(),
                    parameter: "exposure_dose_mj_cm2".to_string(),
                },
                levels: vec![
                    FactorLevel {
                        id: FactorLevelId::new("dose_low"),
                        label: "88".to_string(),
                        value: FactorValue::Numeric(88.0),
                    },
                    FactorLevel {
                        id: FactorLevelId::new("dose_nominal"),
                        label: "95".to_string(),
                        value: FactorValue::Numeric(95.0),
                    },
                    FactorLevel {
                        id: FactorLevelId::new("dose_high"),
                        label: "102".to_string(),
                        value: FactorValue::Numeric(102.0),
                    },
                ],
            },
            ExperimentFactor {
                id: FactorId::new("focus"),
                name: "Focus offset".to_string(),
                unit: Some("um".to_string()),
                source: FactorSource::RecipeParameter {
                    recipe: baseline_recipe.clone(),
                    parameter: "focus_offset_um".to_string(),
                },
                levels: vec![
                    FactorLevel {
                        id: FactorLevelId::new("focus_minus"),
                        label: "-0.2".to_string(),
                        value: FactorValue::Numeric(-0.2),
                    },
                    FactorLevel {
                        id: FactorLevelId::new("focus_plus"),
                        label: "+0.2".to_string(),
                        value: FactorValue::Numeric(0.2),
                    },
                ],
            },
        ],
        responses: vec![
            ResponseSpec {
                id: ResponseSpecId::new("poly_cd_nm"),
                name: "Post-etch poly CD".to_string(),
                unit: "nm".to_string(),
                goal: ResponseGoal::Target,
                target: Some(72.0),
                lower_spec: Some(68.0),
                upper_spec: Some(76.0),
                source: ResponseSource::MetrologyMeasurement {
                    kind: MeasurementKind::CriticalDimensionNm,
                    process_step_id: ProcessStepId::new("S050-CD-METRO"),
                },
            },
            ResponseSpec {
                id: ResponseSpecId::new("defect_count"),
                name: "Defects per wafer".to_string(),
                unit: "count".to_string(),
                goal: ResponseGoal::Minimize,
                target: Some(0.0),
                lower_spec: Some(0.0),
                upper_spec: Some(8.0),
                source: ResponseSource::MetrologyMeasurement {
                    kind: MeasurementKind::DefectCount,
                    process_step_id: ProcessStepId::new("S050-CD-METRO"),
                },
            },
            ResponseSpec {
                id: ResponseSpecId::new("yield_fraction"),
                name: "Electrical yield".to_string(),
                unit: "%".to_string(),
                goal: ResponseGoal::Maximize,
                target: None,
                lower_spec: Some(0.0),
                upper_spec: Some(1.0),
                source: ResponseSource::YieldMetric {
                    metric: "wafer_yield_fraction".to_string(),
                },
            },
        ],
        runs: Vec::new(),
        notes: vec![
            "Generated as a full-factorial wafer split against MES lot L-00042.".to_string(),
            "Responses map to the metrology wafer map and FabOS yield dashboard concepts.".to_string(),
        ],
    };

    let mes = crate::mes::FabMesData::sample();
    let lot = mes
        .lots
        .get(&LotId::new("L-00042"))
        .expect("sample MES data includes L-00042");
    let assignments =
        ExperimentAssignment::wafer_assignments_for_lot(lot, Some("demo lot L-00042".to_string()));
    plan.generate_full_factorial(assignments)
        .expect("sample DOE has enough wafers");

    seed_sample_responses(&mut plan);
    plan
}

fn seed_sample_responses(plan: &mut ExperimentPlan) {
    for (run_id, cd, defects, yield_fraction) in [
        ("DOE-POLY-CD-0042-R01", 69.8, 7.0, 0.82),
        ("DOE-POLY-CD-0042-R02", 68.9, 9.0, 0.79),
        ("DOE-POLY-CD-0042-R03", 72.4, 3.0, 0.91),
        ("DOE-POLY-CD-0042-R04", 71.7, 2.0, 0.93),
    ] {
        let run_id = ExperimentRunId::new(run_id);
        for (response_id, value) in [
            ("poly_cd_nm", cd),
            ("defect_count", defects),
            ("yield_fraction", yield_fraction),
        ] {
            plan.capture_response(
                &run_id,
                &ResponseSpecId::new(response_id),
                ResponseValue {
                    value,
                    measurement_id: Some(format!("M-{run_id}-{response_id}")),
                    captured_at: "2026-05-06T15:00:00Z".to_string(),
                },
            )
            .expect("seeded DOE response is valid");
        }
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

pub fn format_compact_number(value: f64) -> String {
    if (value.fract()).abs() < 0.001 {
        format!("{value:.0}")
    } else if value.abs() < 10.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn minimal_plan() -> ExperimentPlan {
        ExperimentPlan {
            id: ExperimentId::new("DOE-TEST"),
            title: "test split".to_string(),
            objective: "exercise matrix generation".to_string(),
            owner: "test".to_string(),
            status: ExperimentStatus::Draft,
            route_id: ProcessRouteId::new("ROUTE"),
            step_id: ProcessStepId::new("STEP"),
            baseline_recipe: RecipeBinding::new("RECIPE", 1),
            factors: vec![
                ExperimentFactor {
                    id: FactorId::new("temperature"),
                    name: "Temperature".to_string(),
                    unit: Some("C".to_string()),
                    source: FactorSource::Manual,
                    levels: vec![
                        FactorLevel {
                            id: FactorLevelId::new("low"),
                            label: "90".to_string(),
                            value: FactorValue::Numeric(90.0),
                        },
                        FactorLevel {
                            id: FactorLevelId::new("high"),
                            label: "110".to_string(),
                            value: FactorValue::Numeric(110.0),
                        },
                    ],
                },
                ExperimentFactor {
                    id: FactorId::new("time"),
                    name: "Time".to_string(),
                    unit: Some("s".to_string()),
                    source: FactorSource::Manual,
                    levels: vec![
                        FactorLevel {
                            id: FactorLevelId::new("short"),
                            label: "30".to_string(),
                            value: FactorValue::Integer(30),
                        },
                        FactorLevel {
                            id: FactorLevelId::new("long"),
                            label: "60".to_string(),
                            value: FactorValue::Integer(60),
                        },
                    ],
                },
            ],
            responses: vec![ResponseSpec {
                id: ResponseSpecId::new("thickness"),
                name: "Thickness".to_string(),
                unit: "nm".to_string(),
                goal: ResponseGoal::Target,
                target: Some(100.0),
                lower_spec: Some(95.0),
                upper_spec: Some(105.0),
                source: ResponseSource::Manual,
            }],
            runs: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn assignments(count: usize) -> Vec<ExperimentAssignment> {
        (1..=count)
            .map(|slot| ExperimentAssignment {
                lot_id: LotId::new("L-TEST"),
                wafer_id: WaferId::new(format!("L-TEST-W{slot:02}")),
                slot: slot as u8,
                block: None,
                run_order: slot as u32,
            })
            .collect()
    }

    #[test]
    fn full_factorial_generation_assigns_unique_wafers_and_all_combinations() {
        let mut plan = minimal_plan();

        plan.generate_full_factorial(assignments(4)).unwrap();

        assert_eq!(plan.runs.len(), 4);
        assert_eq!(plan.status, ExperimentStatus::MatrixGenerated);
        let assigned_wafers = plan
            .runs
            .iter()
            .map(|run| run.assignment.wafer_id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(assigned_wafers.len(), 4);

        let combinations = plan
            .runs
            .iter()
            .map(|run| run.factor_levels.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(combinations.len(), 4);
        assert!(plan.runs.iter().any(|run| {
            run.factor_levels.get(&FactorId::new("temperature")) == Some(&FactorLevelId::new("low"))
                && run.factor_levels.get(&FactorId::new("time"))
                    == Some(&FactorLevelId::new("short"))
        }));
    }

    #[test]
    fn run_generation_requires_enough_wafer_assignments() {
        let mut plan = minimal_plan();

        let err = plan.generate_full_factorial(assignments(3)).unwrap_err();

        assert_eq!(
            err,
            MatrixGenerationError::InsufficientAssignments {
                required: 4,
                available: 3
            }
        );
        assert!(plan.runs.is_empty());
    }

    #[test]
    fn capture_response_marks_runs_complete_and_summarizes_effects() {
        let mut plan = minimal_plan();
        plan.generate_full_factorial(assignments(4)).unwrap();

        for (index, value) in [97.0, 101.0, 99.0, 104.0].into_iter().enumerate() {
            let run_id = plan.runs[index].id.clone();
            plan.capture_response(
                &run_id,
                &ResponseSpecId::new("thickness"),
                ResponseValue {
                    value,
                    measurement_id: Some(format!("M-{index}")),
                    captured_at: "2026-05-06T12:00:00Z".to_string(),
                },
            )
            .unwrap();
        }

        let summary = plan.analysis_summary(Some(&ResponseSpecId::new("thickness")));
        assert_eq!(plan.status, ExperimentStatus::AnalysisReady);
        assert_eq!(summary.completed_runs, 4);
        assert_eq!(summary.missing_response_count, 0);
        assert_eq!(
            summary.best_run_id,
            Some(ExperimentRunId::new("DOE-TEST-R02"))
        );
        assert_eq!(summary.response_stats[0].sample_count, 4);
        assert_eq!(summary.factor_effects.len(), 4);
        assert!(summary.factor_effects.iter().any(|effect| {
            effect.factor_id == FactorId::new("temperature")
                && effect.level_id == FactorLevelId::new("high")
                && effect.mean_response > 100.0
        }));
    }

    #[test]
    fn sample_plan_links_existing_fabos_lot_and_has_pending_responses() {
        let plan = sample_experiment_plan();

        assert_eq!(
            plan.route_id,
            ProcessRouteId::new("ROUTE-DEMO-INVERTER-POLY-A")
        );
        assert_eq!(plan.step_id, ProcessStepId::new("S020-EXPOSE"));
        assert_eq!(plan.runs.len(), 6);
        assert_eq!(plan.runs[0].assignment.lot_id, LotId::new("L-00042"));
        assert_eq!(plan.status, ExperimentStatus::Running);

        let summary = plan.analysis_summary(Some(&ResponseSpecId::new("poly_cd_nm")));
        assert_eq!(summary.completed_runs, 4);
        assert_eq!(summary.missing_response_count, 6);
    }
}
