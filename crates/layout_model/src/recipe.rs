use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecipeId(pub String);

impl RecipeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for RecipeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RecipeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for RecipeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecipeVersionNumber(pub u32);

impl RecipeVersionNumber {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

impl fmt::Display for RecipeVersionNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessRouteId(pub String);

impl ProcessRouteId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for ProcessRouteId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ProcessRouteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcessStepId(pub String);

impl ProcessStepId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for ProcessStepId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ProcessStepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolRunId(pub String);

impl ToolRunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for ToolRunId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ToolRunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolClass {
    SpinCoater,
    PlasmaEtcher,
    FurnaceHotplate,
    LithographyExposure,
}

impl ToolClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::SpinCoater => "Spin coater",
            Self::PlasmaEtcher => "Plasma etcher",
            Self::FurnaceHotplate => "Furnace / hotplate",
            Self::LithographyExposure => "Lithography exposure",
        }
    }
}

impl fmt::Display for ToolClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeUnit {
    Rpm,
    RpmPerSecond,
    Second,
    Minute,
    Celsius,
    CelsiusPerMinute,
    Watt,
    MilliTorr,
    Sccm,
    Micrometer,
    Nanometer,
    MillijoulePerSquareCentimeter,
    Volt,
    Percent,
    Milliliter,
    Count,
}

impl RecipeUnit {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Rpm => "rpm",
            Self::RpmPerSecond => "rpm/s",
            Self::Second => "s",
            Self::Minute => "min",
            Self::Celsius => "C",
            Self::CelsiusPerMinute => "C/min",
            Self::Watt => "W",
            Self::MilliTorr => "mTorr",
            Self::Sccm => "sccm",
            Self::Micrometer => "um",
            Self::Nanometer => "nm",
            Self::MillijoulePerSquareCentimeter => "mJ/cm2",
            Self::Volt => "V",
            Self::Percent => "%",
            Self::Milliliter => "mL",
            Self::Count => "count",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecipeParameterType {
    Decimal,
    Integer,
    Boolean,
    Text,
    Choice { options: Vec<String> },
}

impl RecipeParameterType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Decimal => "decimal",
            Self::Integer => "integer",
            Self::Boolean => "boolean",
            Self::Text => "text",
            Self::Choice { .. } => "choice",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum RecipeParameterValue {
    Decimal(f64),
    Integer(i64),
    Boolean(bool),
    Text(String),
    Choice(String),
}

impl RecipeParameterValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Decimal(value) => Some(*value),
            Self::Integer(value) => Some(*value as f64),
            Self::Boolean(_) | Self::Text(_) | Self::Choice(_) => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Decimal(_) | Self::Boolean(_) | Self::Text(_) | Self::Choice(_) => None,
        }
    }

    pub fn as_str_value(&self) -> Option<&str> {
        match self {
            Self::Text(value) | Self::Choice(value) => Some(value),
            Self::Decimal(_) | Self::Integer(_) | Self::Boolean(_) => None,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Decimal(_) => "decimal",
            Self::Integer(_) => "integer",
            Self::Boolean(_) => "boolean",
            Self::Text(_) => "text",
            Self::Choice(_) => "choice",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum ValidationRule {
    NumericRange {
        min: Option<f64>,
        max: Option<f64>,
        inclusive: bool,
    },
    IntegerRange {
        min: Option<i64>,
        max: Option<i64>,
        inclusive: bool,
    },
    OneOf {
        values: Vec<String>,
    },
    NonEmpty,
}

impl ValidationRule {
    pub fn numeric_range(min: impl Into<Option<f64>>, max: impl Into<Option<f64>>) -> Self {
        Self::NumericRange {
            min: min.into(),
            max: max.into(),
            inclusive: true,
        }
    }

    pub fn integer_range(min: impl Into<Option<i64>>, max: impl Into<Option<i64>>) -> Self {
        Self::IntegerRange {
            min: min.into(),
            max: max.into(),
            inclusive: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeParameterSpec {
    pub key: String,
    pub label: String,
    pub value_type: RecipeParameterType,
    pub unit: Option<RecipeUnit>,
    pub required: bool,
    pub display_order: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub validation_rules: Vec<ValidationRule>,
    #[serde(default)]
    pub default_value: Option<RecipeParameterValue>,
}

impl RecipeParameterSpec {
    pub fn fallback_value(&self) -> RecipeParameterValue {
        if let Some(value) = &self.default_value {
            return value.clone();
        }
        warn!(
            parameter_key = %self.key,
            parameter_label = %self.label,
            "recipe parameter default missing; using type fallback value"
        );
        match &self.value_type {
            RecipeParameterType::Decimal => RecipeParameterValue::Decimal(0.0),
            RecipeParameterType::Integer => RecipeParameterValue::Integer(0),
            RecipeParameterType::Boolean => RecipeParameterValue::Boolean(false),
            RecipeParameterType::Text => RecipeParameterValue::Text(String::new()),
            RecipeParameterType::Choice { options } => {
                RecipeParameterValue::Choice(options.first().cloned().unwrap_or_else(|| {
                    warn!(
                        parameter_key = %self.key,
                        "choice recipe parameter has no options; using empty string fallback"
                    );
                    String::new()
                }))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeValidationIssue {
    pub severity: ValidationSeverity,
    pub parameter_key: Option<String>,
    pub message: String,
}

impl RecipeValidationIssue {
    fn error(parameter_key: impl Into<Option<String>>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            parameter_key: parameter_key.into(),
            message: message.into(),
        }
    }

    fn warning(parameter_key: impl Into<Option<String>>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            parameter_key: parameter_key.into(),
            message: message.into(),
        }
    }

    fn with_context(mut self, context: impl AsRef<str>) -> Self {
        self.message = format!("{}{}", context.as_ref(), self.message);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Draft,
    InReview,
    Approved,
    Retired,
}

impl ApprovalState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::InReview => "In review",
            Self::Approved => "Approved",
            Self::Retired => "Retired",
        }
    }
}

impl fmt::Display for ApprovalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAction {
    SubmitForReview,
    Approve,
    RequestChanges,
    Retire,
}

impl ApprovalAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::SubmitForReview => "Submit for review",
            Self::Approve => "Approve",
            Self::RequestChanges => "Request changes",
            Self::Retire => "Retire",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalEvent {
    pub action: ApprovalAction,
    pub from_state: ApprovalState,
    pub to_state: ApprovalState,
    pub actor: String,
    pub timestamp: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalTransitionError {
    InvalidTransition {
        from: ApprovalState,
        action: ApprovalAction,
    },
    ValidationFailed {
        error_count: usize,
    },
}

impl fmt::Display for ApprovalTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, action } => {
                write!(f, "cannot {} while version is {from}", action.label())
            }
            Self::ValidationFailed { error_count } => {
                write!(
                    f,
                    "cannot approve version with {error_count} validation error(s)"
                )
            }
        }
    }
}

impl Error for ApprovalTransitionError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeDependency {
    pub recipe_id: RecipeId,
    #[serde(default)]
    pub version: Option<RecipeVersionNumber>,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeVersion {
    pub recipe_id: RecipeId,
    pub version: RecipeVersionNumber,
    pub name: String,
    pub change_summary: String,
    pub author: String,
    pub created_at: String,
    pub approval_state: ApprovalState,
    #[serde(default)]
    pub approved_by: Option<String>,
    #[serde(default)]
    pub approved_at: Option<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, RecipeParameterValue>,
    #[serde(default)]
    pub dependencies: Vec<RecipeDependency>,
    #[serde(default)]
    pub approval_history: Vec<ApprovalEvent>,
}

impl RecipeVersion {
    pub fn apply_approval_action(
        &mut self,
        action: ApprovalAction,
        actor: impl Into<String>,
        timestamp: impl Into<String>,
        note: impl Into<String>,
        validation_issues: &[RecipeValidationIssue],
    ) -> Result<(), ApprovalTransitionError> {
        let from_state = self.approval_state;
        let to_state = match (from_state, action) {
            (ApprovalState::Draft, ApprovalAction::SubmitForReview) => ApprovalState::InReview,
            (ApprovalState::InReview, ApprovalAction::Approve) => {
                let error_count = validation_issues
                    .iter()
                    .filter(|issue| issue.severity == ValidationSeverity::Error)
                    .count();
                if error_count > 0 {
                    return Err(ApprovalTransitionError::ValidationFailed { error_count });
                }
                ApprovalState::Approved
            }
            (ApprovalState::InReview, ApprovalAction::RequestChanges) => ApprovalState::Draft,
            (ApprovalState::Approved, ApprovalAction::Retire) => ApprovalState::Retired,
            _ => {
                return Err(ApprovalTransitionError::InvalidTransition {
                    from: from_state,
                    action,
                });
            }
        };

        let actor = actor.into();
        let timestamp = timestamp.into();
        self.approval_state = to_state;
        if to_state == ApprovalState::Approved {
            self.approved_by = Some(actor.clone());
            self.approved_at = Some(timestamp.clone());
        }
        if to_state == ApprovalState::Draft {
            self.approved_by = None;
            self.approved_at = None;
        }
        self.approval_history.push(ApprovalEvent {
            action,
            from_state,
            to_state,
            actor,
            timestamp,
            note: note.into(),
        });
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBinding {
    pub recipe_id: RecipeId,
    pub version: RecipeVersionNumber,
}

impl RecipeBinding {
    pub fn new(recipe_id: impl Into<RecipeId>, version: u32) -> Self {
        Self {
            recipe_id: recipe_id.into(),
            version: RecipeVersionNumber(version),
        }
    }
}

impl fmt::Display for RecipeBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.recipe_id, self.version)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeUsageReference {
    pub binding: RecipeBinding,
    pub target: RecipeUsageTarget,
    #[serde(default)]
    pub context: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target_type", rename_all = "snake_case")]
pub enum RecipeUsageTarget {
    ProcessStep {
        route_id: ProcessRouteId,
        route_version: u32,
        step_id: ProcessStepId,
    },
    ToolRun {
        tool_run_id: ToolRunId,
        tool_id: String,
    },
    Lot {
        lot_id: String,
    },
}

impl RecipeUsageTarget {
    pub fn label(&self) -> String {
        match self {
            Self::ProcessStep {
                route_id,
                route_version,
                step_id,
            } => format!("route {route_id} v{route_version} step {step_id}"),
            Self::ToolRun {
                tool_run_id,
                tool_id,
            } => format!("tool run {tool_run_id} on {tool_id}"),
            Self::Lot { lot_id } => format!("lot {lot_id}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    pub tool_class: ToolClass,
    pub owner: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parameter_specs: BTreeMap<String, RecipeParameterSpec>,
    #[serde(default)]
    pub versions: Vec<RecipeVersion>,
    #[serde(default)]
    pub usage_references: Vec<RecipeUsageReference>,
}

impl Recipe {
    pub fn version(&self, version: RecipeVersionNumber) -> Option<&RecipeVersion> {
        self.versions
            .iter()
            .find(|candidate| candidate.version == version)
    }

    pub fn version_mut(&mut self, version: RecipeVersionNumber) -> Option<&mut RecipeVersion> {
        self.versions
            .iter_mut()
            .find(|candidate| candidate.version == version)
    }

    pub fn latest_version(&self) -> Option<&RecipeVersion> {
        self.versions.iter().max_by_key(|version| version.version)
    }

    pub fn sorted_parameter_specs(&self) -> Vec<&RecipeParameterSpec> {
        let mut specs: Vec<_> = self.parameter_specs.values().collect();
        specs.sort_by_key(|spec| (spec.display_order, spec.key.as_str()));
        specs
    }

    pub fn validate_version(&self, version: &RecipeVersion) -> Vec<RecipeValidationIssue> {
        let mut issues = Vec::new();
        if version.recipe_id != self.id {
            issues.push(RecipeValidationIssue::error(
                None,
                format!(
                    "version belongs to recipe {}, not {}",
                    version.recipe_id, self.id
                ),
            ));
        }

        for spec in self.sorted_parameter_specs() {
            match version.parameters.get(&spec.key) {
                Some(value) => {
                    self.validate_parameter_type(spec, value, &mut issues);
                    self.validate_parameter_rules(spec, value, &mut issues);
                }
                None if spec.required => issues.push(RecipeValidationIssue::error(
                    Some(spec.key.clone()),
                    format!("{} is required", spec.label),
                )),
                None => {}
            }
        }

        for key in version.parameters.keys() {
            if !self.parameter_specs.contains_key(key) {
                issues.push(RecipeValidationIssue::warning(
                    Some(key.clone()),
                    format!("{key} is not defined in the recipe schema"),
                ));
            }
        }

        issues
    }

    fn validate_parameter_type(
        &self,
        spec: &RecipeParameterSpec,
        value: &RecipeParameterValue,
        issues: &mut Vec<RecipeValidationIssue>,
    ) {
        let type_ok = matches!(
            (&spec.value_type, value),
            (
                RecipeParameterType::Decimal,
                RecipeParameterValue::Decimal(_)
            ) | (
                RecipeParameterType::Integer,
                RecipeParameterValue::Integer(_)
            ) | (
                RecipeParameterType::Boolean,
                RecipeParameterValue::Boolean(_)
            ) | (RecipeParameterType::Text, RecipeParameterValue::Text(_))
                | (
                    RecipeParameterType::Choice { .. },
                    RecipeParameterValue::Choice(_)
                )
        );
        if !type_ok {
            issues.push(RecipeValidationIssue::error(
                Some(spec.key.clone()),
                format!(
                    "{} expects {}, got {}",
                    spec.label,
                    spec.value_type.label(),
                    value.kind_label()
                ),
            ));
            return;
        }
        if let RecipeParameterValue::Decimal(value) = value
            && !value.is_finite()
        {
            issues.push(RecipeValidationIssue::error(
                Some(spec.key.clone()),
                format!("{} must be finite", spec.label),
            ));
        }
        if let (RecipeParameterType::Choice { options }, RecipeParameterValue::Choice(selected)) =
            (&spec.value_type, value)
            && !options.iter().any(|option| option == selected)
        {
            issues.push(RecipeValidationIssue::error(
                Some(spec.key.clone()),
                format!("{} must be one of {}", spec.label, options.join(", ")),
            ));
        }
    }

    fn validate_parameter_rules(
        &self,
        spec: &RecipeParameterSpec,
        value: &RecipeParameterValue,
        issues: &mut Vec<RecipeValidationIssue>,
    ) {
        for rule in &spec.validation_rules {
            match rule {
                ValidationRule::NumericRange {
                    min,
                    max,
                    inclusive,
                } => {
                    if let Some(number) = value.as_f64() {
                        if !number.is_finite() {
                            continue;
                        }
                        if range_f64_failed(number, *min, *max, *inclusive) {
                            issues.push(RecipeValidationIssue::error(
                                Some(spec.key.clone()),
                                format!(
                                    "{} must be {}",
                                    spec.label,
                                    format_numeric_range(*min, *max, *inclusive)
                                ),
                            ));
                        }
                    }
                }
                ValidationRule::IntegerRange {
                    min,
                    max,
                    inclusive,
                } => {
                    if let Some(number) = value.as_i64()
                        && range_i64_failed(number, *min, *max, *inclusive)
                    {
                        issues.push(RecipeValidationIssue::error(
                            Some(spec.key.clone()),
                            format!(
                                "{} must be {}",
                                spec.label,
                                format_integer_range(*min, *max, *inclusive)
                            ),
                        ));
                    }
                }
                ValidationRule::OneOf { values } => {
                    if let Some(text) = value.as_str_value()
                        && !values.iter().any(|value| value == text)
                    {
                        issues.push(RecipeValidationIssue::error(
                            Some(spec.key.clone()),
                            format!("{} must be one of {}", spec.label, values.join(", ")),
                        ));
                    }
                }
                ValidationRule::NonEmpty => {
                    if value.as_str_value().is_some_and(str::is_empty) {
                        issues.push(RecipeValidationIssue::error(
                            Some(spec.key.clone()),
                            format!("{} cannot be empty", spec.label),
                        ));
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeDiffKind {
    ParameterAdded,
    ParameterRemoved,
    ParameterChanged,
    ApprovalChanged,
    DependencyChanged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeDiffEntry {
    pub kind: RecipeDiffKind,
    pub label: String,
    #[serde(default)]
    pub parameter_key: Option<String>,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub after: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeDiff {
    pub recipe_id: RecipeId,
    pub from_version: RecipeVersionNumber,
    pub to_version: RecipeVersionNumber,
    pub entries: Vec<RecipeDiffEntry>,
}

impl RecipeDiff {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn diff_recipe_versions(
    recipe: &Recipe,
    from: &RecipeVersion,
    to: &RecipeVersion,
) -> RecipeDiff {
    let mut entries = Vec::new();
    let mut keys = BTreeSet::new();
    keys.extend(from.parameters.keys().cloned());
    keys.extend(to.parameters.keys().cloned());

    for key in keys {
        let before = from.parameters.get(&key);
        let after = to.parameters.get(&key);
        if before == after {
            continue;
        }
        let spec = recipe.parameter_specs.get(&key);
        let label = spec
            .map(|spec| spec.label.clone())
            .unwrap_or_else(|| key.clone());
        let unit = spec.and_then(|spec| spec.unit);
        let kind = match (before, after) {
            (None, Some(_)) => RecipeDiffKind::ParameterAdded,
            (Some(_), None) => RecipeDiffKind::ParameterRemoved,
            (Some(_), Some(_)) => RecipeDiffKind::ParameterChanged,
            (None, None) => continue,
        };
        entries.push(RecipeDiffEntry {
            kind,
            label,
            parameter_key: Some(key),
            before: before.map(|value| format_parameter_value(value, unit)),
            after: after.map(|value| format_parameter_value(value, unit)),
        });
    }

    if from.approval_state != to.approval_state {
        entries.push(RecipeDiffEntry {
            kind: RecipeDiffKind::ApprovalChanged,
            label: "Approval state".to_string(),
            parameter_key: None,
            before: Some(from.approval_state.label().to_string()),
            after: Some(to.approval_state.label().to_string()),
        });
    }

    if from.dependencies != to.dependencies {
        entries.push(RecipeDiffEntry {
            kind: RecipeDiffKind::DependencyChanged,
            label: "Dependencies".to_string(),
            parameter_key: None,
            before: Some(format!("{} link(s)", from.dependencies.len())),
            after: Some(format!("{} link(s)", to.dependencies.len())),
        });
    }

    RecipeDiff {
        recipe_id: recipe.id.clone(),
        from_version: from.version,
        to_version: to.version,
        entries,
    }
}

pub fn format_parameter_value(value: &RecipeParameterValue, unit: Option<RecipeUnit>) -> String {
    let rendered = match value {
        RecipeParameterValue::Decimal(value) => format_decimal(*value),
        RecipeParameterValue::Integer(value) => value.to_string(),
        RecipeParameterValue::Boolean(value) => {
            if *value {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        RecipeParameterValue::Text(value) | RecipeParameterValue::Choice(value) => value.clone(),
    };

    match unit {
        Some(unit)
            if matches!(
                value,
                RecipeParameterValue::Decimal(_) | RecipeParameterValue::Integer(_)
            ) =>
        {
            format!("{rendered} {}", unit.symbol())
        }
        _ => rendered,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStepReference {
    pub route_id: ProcessRouteId,
    pub route_version: u32,
    pub step_id: ProcessStepId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStep {
    pub id: ProcessStepId,
    pub sequence: u32,
    pub name: String,
    pub tool_class: ToolClass,
    #[serde(default)]
    pub recipe: Option<RecipeBinding>,
    pub operator_signoff_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRoute {
    pub id: ProcessRouteId,
    pub version: u32,
    pub name: String,
    pub steps: Vec<ProcessStep>,
}

impl ProcessRoute {
    pub fn step(&self, step_id: &ProcessStepId) -> Option<&ProcessStep> {
        self.steps.iter().find(|step| &step.id == step_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRunState {
    Queued,
    Running,
    Completed,
    Alarmed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRun {
    pub id: ToolRunId,
    pub tool_id: String,
    pub tool_class: ToolClass,
    pub recipe: RecipeBinding,
    #[serde(default)]
    pub process_step: Option<ProcessStepReference>,
    pub state: ToolRunState,
    pub started_at: String,
    #[serde(default)]
    pub completed_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecipeCatalog {
    #[serde(default)]
    pub recipes: BTreeMap<RecipeId, Recipe>,
    #[serde(default)]
    pub process_routes: Vec<ProcessRoute>,
    #[serde(default)]
    pub tool_runs: Vec<ToolRun>,
}

impl RecipeCatalog {
    pub fn sample() -> Self {
        sample_recipe_catalog()
    }

    pub fn sorted_recipes(&self) -> Vec<&Recipe> {
        let mut recipes: Vec<_> = self.recipes.values().collect();
        recipes.sort_by_key(|recipe| recipe.id.as_str());
        recipes
    }

    pub fn recipe(&self, id: &RecipeId) -> Option<&Recipe> {
        self.recipes.get(id)
    }

    pub fn recipe_mut(&mut self, id: &RecipeId) -> Option<&mut Recipe> {
        self.recipes.get_mut(id)
    }

    pub fn validate(&self) -> Vec<RecipeValidationIssue> {
        let mut issues = Vec::new();
        for (recipe_id, recipe) in &self.recipes {
            validate_catalog_recipe(recipe_id, recipe, &mut issues);
        }

        let mut route_keys = BTreeSet::new();
        let mut process_steps = BTreeMap::new();
        for route in &self.process_routes {
            validate_catalog_route(
                route,
                &self.recipes,
                &mut route_keys,
                &mut process_steps,
                &mut issues,
            );
        }

        let mut tool_run_ids = BTreeSet::new();
        let mut tool_runs = BTreeMap::new();
        for run in &self.tool_runs {
            validate_catalog_tool_run(
                run,
                &self.recipes,
                &process_steps,
                &mut tool_run_ids,
                &mut tool_runs,
                &mut issues,
            );
        }

        for recipe in self.recipes.values() {
            validate_usage_references(
                recipe,
                &self.recipes,
                &process_steps,
                &tool_runs,
                &mut issues,
            );
        }

        issues
    }
}

type ProcessStepKey = (ProcessRouteId, u32, ProcessStepId);
type ProcessStepCatalogEntry = (Option<RecipeBinding>, ToolClass);

fn validate_catalog_recipe(
    recipe_id: &RecipeId,
    recipe: &Recipe,
    issues: &mut Vec<RecipeValidationIssue>,
) {
    if recipe_id != &recipe.id {
        issues.push(RecipeValidationIssue::error(
            None,
            format!(
                "recipe map key {recipe_id} does not match recipe id {}",
                recipe.id
            ),
        ));
    }
    if recipe.id.as_str().trim().is_empty() {
        issues.push(RecipeValidationIssue::error(None, "recipe id is empty"));
    }
    if recipe.name.trim().is_empty() {
        issues.push(RecipeValidationIssue::warning(
            None,
            format!("recipe {} has an empty name", recipe.id),
        ));
    }
    if recipe.versions.is_empty() {
        issues.push(RecipeValidationIssue::error(
            None,
            format!("recipe {} has no versions", recipe.id),
        ));
    }

    let mut versions = BTreeSet::new();
    for version in &recipe.versions {
        if !versions.insert(version.version) {
            issues.push(RecipeValidationIssue::error(
                None,
                format!(
                    "recipe {} has duplicate version {}",
                    recipe.id, version.version
                ),
            ));
        }
        for issue in recipe.validate_version(version) {
            issues.push(issue.with_context(format!("recipe {} {}: ", recipe.id, version.version)));
        }
        validate_recipe_approval_state(recipe, version, issues);
    }

    for (key, spec) in &recipe.parameter_specs {
        if key != &spec.key {
            issues.push(RecipeValidationIssue::error(
                Some(key.clone()),
                format!(
                    "recipe {} parameter map key {key} does not match spec key {}",
                    recipe.id, spec.key
                ),
            ));
        }
        if spec.key.trim().is_empty() {
            issues.push(RecipeValidationIssue::error(
                Some(key.clone()),
                format!("recipe {} has an empty parameter key", recipe.id),
            ));
        }
        validate_parameter_spec_options(&recipe.id, spec, issues);
    }
}

fn validate_recipe_approval_state(
    recipe: &Recipe,
    version: &RecipeVersion,
    issues: &mut Vec<RecipeValidationIssue>,
) {
    if version.approval_state == ApprovalState::Approved {
        if version
            .approved_by
            .as_ref()
            .is_none_or(|actor| actor.trim().is_empty())
        {
            issues.push(RecipeValidationIssue::warning(
                None,
                format!(
                    "recipe {} {} is approved without an approver",
                    recipe.id, version.version
                ),
            ));
        }
        if version
            .approved_at
            .as_ref()
            .is_none_or(|time| time.trim().is_empty())
        {
            issues.push(RecipeValidationIssue::warning(
                None,
                format!(
                    "recipe {} {} is approved without an approval timestamp",
                    recipe.id, version.version
                ),
            ));
        }
    } else if version.approved_by.is_some() || version.approved_at.is_some() {
        issues.push(RecipeValidationIssue::warning(
            None,
            format!(
                "recipe {} {} keeps approval metadata while state is {}",
                recipe.id,
                version.version,
                version.approval_state.label()
            ),
        ));
    }
}

fn validate_parameter_spec_options(
    recipe_id: &RecipeId,
    spec: &RecipeParameterSpec,
    issues: &mut Vec<RecipeValidationIssue>,
) {
    if let RecipeParameterType::Choice { options } = &spec.value_type {
        let mut seen = BTreeSet::new();
        if options.is_empty() {
            issues.push(RecipeValidationIssue::error(
                Some(spec.key.clone()),
                format!("recipe {recipe_id} parameter {} has no choices", spec.key),
            ));
        }
        for option in options {
            if option.trim().is_empty() {
                issues.push(RecipeValidationIssue::error(
                    Some(spec.key.clone()),
                    format!(
                        "recipe {recipe_id} parameter {} has an empty choice",
                        spec.key
                    ),
                ));
            }
            if !seen.insert(option) {
                issues.push(RecipeValidationIssue::error(
                    Some(spec.key.clone()),
                    format!(
                        "recipe {recipe_id} parameter {} has duplicate choice {option}",
                        spec.key
                    ),
                ));
            }
        }
    }
}

fn validate_catalog_route(
    route: &ProcessRoute,
    recipes: &BTreeMap<RecipeId, Recipe>,
    route_keys: &mut BTreeSet<(ProcessRouteId, u32)>,
    process_steps: &mut BTreeMap<ProcessStepKey, ProcessStepCatalogEntry>,
    issues: &mut Vec<RecipeValidationIssue>,
) {
    let route_key = (route.id.clone(), route.version);
    if !route_keys.insert(route_key.clone()) {
        issues.push(RecipeValidationIssue::error(
            None,
            format!(
                "process route {} v{} is duplicated",
                route.id, route.version
            ),
        ));
    }
    if route.id.0.trim().is_empty() {
        issues.push(RecipeValidationIssue::error(
            None,
            "process route id is empty",
        ));
    }
    if route.version == 0 {
        issues.push(RecipeValidationIssue::error(
            None,
            format!("process route {} has invalid version 0", route.id),
        ));
    }
    if route.steps.is_empty() {
        issues.push(RecipeValidationIssue::warning(
            None,
            format!("process route {} v{} has no steps", route.id, route.version),
        ));
    }

    let mut step_ids = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    for step in &route.steps {
        if step.id.0.trim().is_empty() {
            issues.push(RecipeValidationIssue::error(
                None,
                format!(
                    "process route {} v{} has an empty step id",
                    route.id, route.version
                ),
            ));
        }
        if !step_ids.insert(step.id.clone()) {
            issues.push(RecipeValidationIssue::error(
                None,
                format!(
                    "process route {} v{} contains duplicate step {}",
                    route.id, route.version, step.id
                ),
            ));
        }
        if !sequences.insert(step.sequence) {
            issues.push(RecipeValidationIssue::warning(
                None,
                format!(
                    "process route {} v{} contains duplicate step sequence {}",
                    route.id, route.version, step.sequence
                ),
            ));
        }
        if step.sequence == 0 {
            issues.push(RecipeValidationIssue::error(
                None,
                format!(
                    "process route {} v{} step {} has sequence 0",
                    route.id, route.version, step.id
                ),
            ));
        }
        if let Some(binding) = &step.recipe {
            validate_recipe_binding(
                binding,
                recipes,
                issues,
                format!(
                    "process route {} v{} step {}",
                    route.id, route.version, step.id
                ),
            );
        }
        process_steps.insert(
            (route.id.clone(), route.version, step.id.clone()),
            (step.recipe.clone(), step.tool_class),
        );
    }
}

fn validate_catalog_tool_run(
    run: &ToolRun,
    recipes: &BTreeMap<RecipeId, Recipe>,
    process_steps: &BTreeMap<ProcessStepKey, ProcessStepCatalogEntry>,
    tool_run_ids: &mut BTreeSet<ToolRunId>,
    tool_runs: &mut BTreeMap<ToolRunId, (String, RecipeBinding)>,
    issues: &mut Vec<RecipeValidationIssue>,
) {
    if run.id.0.trim().is_empty() {
        issues.push(RecipeValidationIssue::error(None, "tool run id is empty"));
    }
    if !tool_run_ids.insert(run.id.clone()) {
        issues.push(RecipeValidationIssue::error(
            None,
            format!("tool run {} is duplicated", run.id),
        ));
    }
    if run.tool_id.trim().is_empty() {
        issues.push(RecipeValidationIssue::error(
            None,
            format!("tool run {} has an empty tool id", run.id),
        ));
    }
    validate_recipe_binding(&run.recipe, recipes, issues, format!("tool run {}", run.id));
    if let Some(step_ref) = &run.process_step {
        if step_ref.route_id.0.trim().is_empty() {
            issues.push(RecipeValidationIssue::error(
                None,
                format!("tool run {} has an empty process route id", run.id),
            ));
        }
        if step_ref.route_version == 0 {
            issues.push(RecipeValidationIssue::error(
                None,
                format!("tool run {} references process route version 0", run.id),
            ));
        }
        if step_ref.step_id.0.trim().is_empty() {
            issues.push(RecipeValidationIssue::error(
                None,
                format!("tool run {} has an empty process step id", run.id),
            ));
        }
        let step_key = (
            step_ref.route_id.clone(),
            step_ref.route_version,
            step_ref.step_id.clone(),
        );
        match process_steps.get(&step_key) {
            Some((Some(step_recipe), step_tool_class)) if step_recipe != &run.recipe => {
                issues.push(RecipeValidationIssue::error(
                    None,
                    format!(
                        "tool run {} recipe {} does not match process step {} v{} {} recipe {}",
                        run.id,
                        run.recipe,
                        step_ref.route_id,
                        step_ref.route_version,
                        step_ref.step_id,
                        step_recipe
                    ),
                ));
                if *step_tool_class != run.tool_class {
                    issues.push(RecipeValidationIssue::error(
                        None,
                        format!(
                            "tool run {} class {} does not match process step {} v{} {} class {}",
                            run.id,
                            run.tool_class,
                            step_ref.route_id,
                            step_ref.route_version,
                            step_ref.step_id,
                            step_tool_class
                        ),
                    ));
                }
            }
            Some((_, step_tool_class)) => {
                if *step_tool_class != run.tool_class {
                    issues.push(RecipeValidationIssue::error(
                        None,
                        format!(
                            "tool run {} class {} does not match process step {} v{} {} class {}",
                            run.id,
                            run.tool_class,
                            step_ref.route_id,
                            step_ref.route_version,
                            step_ref.step_id,
                            step_tool_class
                        ),
                    ));
                }
            }
            None => issues.push(RecipeValidationIssue::error(
                None,
                format!(
                    "tool run {} references missing process step {} v{} {}",
                    run.id, step_ref.route_id, step_ref.route_version, step_ref.step_id
                ),
            )),
        }
    }
    match run.state {
        ToolRunState::Running | ToolRunState::Completed | ToolRunState::Alarmed
            if run.started_at.trim().is_empty() =>
        {
            issues.push(RecipeValidationIssue::error(
                None,
                format!("tool run {} has no started_at timestamp", run.id),
            ));
        }
        ToolRunState::Queued if !run.started_at.trim().is_empty() => {
            issues.push(RecipeValidationIssue::warning(
                None,
                format!(
                    "queued tool run {} already has started_at timestamp",
                    run.id
                ),
            ));
        }
        _ => {}
    }
    if run.state == ToolRunState::Completed
        && run
            .completed_at
            .as_deref()
            .is_none_or(|timestamp| timestamp.trim().is_empty())
    {
        issues.push(RecipeValidationIssue::warning(
            None,
            format!(
                "completed tool run {} has no completed_at timestamp",
                run.id
            ),
        ));
    }
    if run.state != ToolRunState::Completed
        && run
            .completed_at
            .as_deref()
            .is_some_and(|timestamp| !timestamp.trim().is_empty())
    {
        issues.push(RecipeValidationIssue::warning(
            None,
            format!(
                "non-completed tool run {} already has completed_at timestamp",
                run.id
            ),
        ));
    }
    if let Some(completed_at) = run.completed_at.as_deref()
        && !run.started_at.trim().is_empty()
        && !completed_at.trim().is_empty()
        && completed_at < run.started_at.as_str()
    {
        issues.push(RecipeValidationIssue::error(
            None,
            format!(
                "tool run {} completed_at timestamp is before started_at timestamp",
                run.id
            ),
        ));
    }
    tool_runs.insert(run.id.clone(), (run.tool_id.clone(), run.recipe.clone()));
}

fn validate_usage_references(
    recipe: &Recipe,
    recipes: &BTreeMap<RecipeId, Recipe>,
    process_steps: &BTreeMap<ProcessStepKey, ProcessStepCatalogEntry>,
    tool_runs: &BTreeMap<ToolRunId, (String, RecipeBinding)>,
    issues: &mut Vec<RecipeValidationIssue>,
) {
    for reference in &recipe.usage_references {
        validate_recipe_binding(
            &reference.binding,
            recipes,
            issues,
            format!("recipe {} usage reference", recipe.id),
        );
        match &reference.target {
            RecipeUsageTarget::ProcessStep {
                route_id,
                route_version,
                step_id,
            } => {
                let step_key = (route_id.clone(), *route_version, step_id.clone());
                match process_steps.get(&step_key) {
                    Some((Some(step_recipe), _)) if step_recipe != &reference.binding => {
                        issues.push(RecipeValidationIssue::error(
                            None,
                            format!(
                                "recipe {} usage reference {} does not match process step {} v{} {} recipe {}",
                                recipe.id, reference.binding, route_id, route_version, step_id, step_recipe
                            ),
                        ));
                    }
                    Some(_) => {}
                    None => issues.push(RecipeValidationIssue::warning(
                        None,
                        format!(
                            "recipe {} usage reference targets missing process step {} v{} {}",
                            recipe.id, route_id, route_version, step_id
                        ),
                    )),
                }
            }
            RecipeUsageTarget::ToolRun {
                tool_run_id,
                tool_id,
            } => match tool_runs.get(tool_run_id) {
                Some((run_tool_id, run_recipe)) => {
                    if run_tool_id != tool_id {
                        issues.push(RecipeValidationIssue::error(
                            None,
                            format!(
                                "recipe {} usage reference targets tool run {} on {}, but run belongs to {}",
                                recipe.id, tool_run_id, tool_id, run_tool_id
                            ),
                        ));
                    }
                    if run_recipe != &reference.binding {
                        issues.push(RecipeValidationIssue::error(
                            None,
                            format!(
                                "recipe {} usage reference {} does not match tool run {} recipe {}",
                                recipe.id, reference.binding, tool_run_id, run_recipe
                            ),
                        ));
                    }
                }
                None => issues.push(RecipeValidationIssue::warning(
                    None,
                    format!(
                        "recipe {} usage reference targets missing tool run {}",
                        recipe.id, tool_run_id
                    ),
                )),
            },
            RecipeUsageTarget::Lot { lot_id } => {
                if lot_id.trim().is_empty() {
                    issues.push(RecipeValidationIssue::error(
                        None,
                        format!("recipe {} usage reference has an empty lot id", recipe.id),
                    ));
                }
            }
        }
    }
}

fn validate_recipe_binding(
    binding: &RecipeBinding,
    recipes: &BTreeMap<RecipeId, Recipe>,
    issues: &mut Vec<RecipeValidationIssue>,
    context: impl AsRef<str>,
) {
    let context = context.as_ref();
    if binding.recipe_id.as_str().trim().is_empty() {
        issues.push(RecipeValidationIssue::error(
            None,
            format!("{context} has an empty recipe id"),
        ));
        return;
    }
    if binding.version.0 == 0 {
        issues.push(RecipeValidationIssue::error(
            None,
            format!("{context} references {} version 0", binding.recipe_id),
        ));
        return;
    }
    match recipes.get(&binding.recipe_id) {
        Some(recipe) if recipe.version(binding.version).is_some() => {}
        Some(_) => issues.push(RecipeValidationIssue::error(
            None,
            format!("{context} references missing recipe binding {binding}"),
        )),
        None => issues.push(RecipeValidationIssue::error(
            None,
            format!("{context} references missing recipe {}", binding.recipe_id),
        )),
    }
}

pub fn sample_recipe_catalog() -> RecipeCatalog {
    let mut recipes = BTreeMap::new();
    for recipe in [
        sample_spin_coater_recipe(),
        sample_plasma_etcher_recipe(),
        sample_thermal_recipe(),
        sample_lithography_recipe(),
        sample_develop_recipe(),
        sample_cd_metrology_recipe(),
    ] {
        recipes.insert(recipe.id.clone(), recipe);
    }

    RecipeCatalog {
        recipes,
        process_routes: vec![sample_process_route()],
        tool_runs: vec![sample_spin_tool_run(), sample_etch_tool_run()],
    }
}

fn sample_spin_coater_recipe() -> Recipe {
    let id = RecipeId::from("SPIN_PR_3000");
    Recipe {
        id: id.clone(),
        name: "Photoresist spin coat".to_string(),
        tool_class: ToolClass::SpinCoater,
        owner: "Lithography".to_string(),
        description: "Positive resist coat for poly gate patterning.".to_string(),
        parameter_specs: specs([
            choice_spec(
                "resist",
                "Resist",
                10,
                ["AZ1512", "S1813", "PMMA A4"],
                "AZ1512",
            ),
            decimal_spec(
                "dispense_volume_ml",
                "Dispense volume",
                20,
                RecipeUnit::Milliliter,
                0.2,
                5.0,
                1.2,
            ),
            integer_spec(
                "spin_rpm",
                "Spin speed",
                30,
                RecipeUnit::Rpm,
                500,
                7000,
                4000,
            ),
            integer_spec(
                "acceleration_rpm_s",
                "Acceleration",
                40,
                RecipeUnit::RpmPerSecond,
                100,
                5000,
                1000,
            ),
            decimal_spec(
                "spin_time_s",
                "Spin time",
                50,
                RecipeUnit::Second,
                5.0,
                120.0,
                45.0,
            ),
        ]),
        versions: vec![
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(1),
                name: "Baseline 1.3 um coat".to_string(),
                change_summary: "Initial coat target for contact aligner lithography.".to_string(),
                author: "process.eng".to_string(),
                created_at: "2026-04-01T09:00:00Z".to_string(),
                approval_state: ApprovalState::Approved,
                approved_by: Some("lead.process.eng".to_string()),
                approved_at: Some("2026-04-02T15:30:00Z".to_string()),
                parameters: parameters([
                    ("resist", RecipeParameterValue::Choice("AZ1512".to_string())),
                    ("dispense_volume_ml", RecipeParameterValue::Decimal(1.2)),
                    ("spin_rpm", RecipeParameterValue::Integer(4000)),
                    ("acceleration_rpm_s", RecipeParameterValue::Integer(1000)),
                    ("spin_time_s", RecipeParameterValue::Decimal(45.0)),
                ]),
                dependencies: Vec::new(),
                approval_history: vec![approved_event()],
            },
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(2),
                name: "Reduced edge bead trial".to_string(),
                change_summary: "Slightly higher spin speed and shorter spin time.".to_string(),
                author: "litho.dev".to_string(),
                created_at: "2026-04-18T10:15:00Z".to_string(),
                approval_state: ApprovalState::InReview,
                approved_by: None,
                approved_at: None,
                parameters: parameters([
                    ("resist", RecipeParameterValue::Choice("AZ1512".to_string())),
                    ("dispense_volume_ml", RecipeParameterValue::Decimal(1.2)),
                    ("spin_rpm", RecipeParameterValue::Integer(4200)),
                    ("acceleration_rpm_s", RecipeParameterValue::Integer(1200)),
                    ("spin_time_s", RecipeParameterValue::Decimal(40.0)),
                ]),
                dependencies: Vec::new(),
                approval_history: vec![ApprovalEvent {
                    action: ApprovalAction::SubmitForReview,
                    from_state: ApprovalState::Draft,
                    to_state: ApprovalState::InReview,
                    actor: "litho.dev".to_string(),
                    timestamp: "2026-04-18T11:00:00Z".to_string(),
                    note: "Edge bead reduction trial.".to_string(),
                }],
            },
        ],
        usage_references: vec![
            route_usage(
                "SPIN_PR_3000",
                1,
                "coat_photoresist",
                "Process traveler coat step",
            ),
            tool_run_usage("SPIN_PR_3000", 1, "RUN-SPIN-0007", "SPIN-01"),
        ],
    }
}

fn sample_plasma_etcher_recipe() -> Recipe {
    let id = RecipeId::from("ETCH_CF4_POLY_001");
    Recipe {
        id: id.clone(),
        name: "CF4 poly etch".to_string(),
        tool_class: ToolClass::PlasmaEtcher,
        owner: "Etch".to_string(),
        description: "Timed poly etch for the demo CMOS stack.".to_string(),
        parameter_specs: specs([
            choice_spec(
                "gas_recipe",
                "Gas recipe",
                10,
                ["CF4", "CHF3/O2", "SF6"],
                "CF4",
            ),
            integer_spec(
                "pressure_mtorr",
                "Chamber pressure",
                20,
                RecipeUnit::MilliTorr,
                20,
                250,
                80,
            ),
            integer_spec("rf_power_w", "RF power", 30, RecipeUnit::Watt, 25, 500, 150),
            decimal_spec(
                "etch_time_s",
                "Etch time",
                40,
                RecipeUnit::Second,
                5.0,
                300.0,
                60.0,
            ),
            integer_spec("o2_flow_sccm", "O2 flow", 50, RecipeUnit::Sccm, 0, 100, 8),
            boolean_spec("endpoint_enabled", "Endpoint enabled", 60, false),
        ]),
        versions: vec![
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(1),
                name: "Baseline poly clear".to_string(),
                change_summary: "Qualified timed etch.".to_string(),
                author: "etch.eng".to_string(),
                created_at: "2026-03-22T14:00:00Z".to_string(),
                approval_state: ApprovalState::Approved,
                approved_by: Some("etch.lead".to_string()),
                approved_at: Some("2026-03-25T18:00:00Z".to_string()),
                parameters: parameters([
                    (
                        "gas_recipe",
                        RecipeParameterValue::Choice("CF4".to_string()),
                    ),
                    ("pressure_mtorr", RecipeParameterValue::Integer(80)),
                    ("rf_power_w", RecipeParameterValue::Integer(150)),
                    ("etch_time_s", RecipeParameterValue::Decimal(60.0)),
                    ("o2_flow_sccm", RecipeParameterValue::Integer(8)),
                    ("endpoint_enabled", RecipeParameterValue::Boolean(false)),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("LITHO_POLY_EXPOSE_001"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Etch assumes the poly lithography stack from exposure v1.".to_string(),
                }],
                approval_history: vec![approved_event()],
            },
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(2),
                name: "Lower pressure split".to_string(),
                change_summary: "Lower pressure split for sidewall profile evaluation.".to_string(),
                author: "etch.dev".to_string(),
                created_at: "2026-04-20T16:45:00Z".to_string(),
                approval_state: ApprovalState::Draft,
                approved_by: None,
                approved_at: None,
                parameters: parameters([
                    (
                        "gas_recipe",
                        RecipeParameterValue::Choice("CF4".to_string()),
                    ),
                    ("pressure_mtorr", RecipeParameterValue::Integer(65)),
                    ("rf_power_w", RecipeParameterValue::Integer(150)),
                    ("etch_time_s", RecipeParameterValue::Decimal(55.0)),
                    ("o2_flow_sccm", RecipeParameterValue::Integer(8)),
                    ("endpoint_enabled", RecipeParameterValue::Boolean(false)),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("LITHO_POLY_EXPOSE_001"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Same lithography dependency as baseline.".to_string(),
                }],
                approval_history: Vec::new(),
            },
        ],
        usage_references: vec![
            route_usage(
                "ETCH_CF4_POLY_001",
                1,
                "etch_poly",
                "Process traveler etch step",
            ),
            tool_run_usage("ETCH_CF4_POLY_001", 1, "RUN-ETCH-0012", "ETCH-02"),
        ],
    }
}

fn sample_thermal_recipe() -> Recipe {
    let id = RecipeId::from("THERMAL_BAKE_110C_001");
    Recipe {
        id: id.clone(),
        name: "Photoresist soft bake".to_string(),
        tool_class: ToolClass::FurnaceHotplate,
        owner: "Lithography".to_string(),
        description: "Hotplate or furnace bake after spin coat.".to_string(),
        parameter_specs: specs([
            choice_spec(
                "thermal_mode",
                "Thermal mode",
                10,
                ["hotplate_contact", "proximity_hotplate", "tube_furnace"],
                "hotplate_contact",
            ),
            decimal_spec(
                "temperature_c",
                "Temperature",
                20,
                RecipeUnit::Celsius,
                50.0,
                1100.0,
                110.0,
            ),
            decimal_spec(
                "duration_min",
                "Duration",
                30,
                RecipeUnit::Minute,
                0.5,
                180.0,
                1.5,
            ),
            decimal_spec(
                "ramp_rate_c_min",
                "Ramp rate",
                40,
                RecipeUnit::CelsiusPerMinute,
                0.1,
                50.0,
                10.0,
            ),
            choice_spec(
                "atmosphere",
                "Atmosphere",
                50,
                ["air", "nitrogen", "forming_gas"],
                "air",
            ),
        ]),
        versions: vec![
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(1),
                name: "Baseline hotplate bake".to_string(),
                change_summary: "Qualified post-coat soft bake.".to_string(),
                author: "litho.eng".to_string(),
                created_at: "2026-04-01T09:20:00Z".to_string(),
                approval_state: ApprovalState::Approved,
                approved_by: Some("lead.process.eng".to_string()),
                approved_at: Some("2026-04-02T15:35:00Z".to_string()),
                parameters: parameters([
                    (
                        "thermal_mode",
                        RecipeParameterValue::Choice("hotplate_contact".to_string()),
                    ),
                    ("temperature_c", RecipeParameterValue::Decimal(110.0)),
                    ("duration_min", RecipeParameterValue::Decimal(1.5)),
                    ("ramp_rate_c_min", RecipeParameterValue::Decimal(10.0)),
                    (
                        "atmosphere",
                        RecipeParameterValue::Choice("air".to_string()),
                    ),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("SPIN_PR_3000"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Bake follows the qualified photoresist spin coat.".to_string(),
                }],
                approval_history: vec![approved_event()],
            },
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(2),
                name: "Nitrogen split".to_string(),
                change_summary: "Evaluate nitrogen bake atmosphere for adhesion.".to_string(),
                author: "litho.dev".to_string(),
                created_at: "2026-04-19T08:40:00Z".to_string(),
                approval_state: ApprovalState::Draft,
                approved_by: None,
                approved_at: None,
                parameters: parameters([
                    (
                        "thermal_mode",
                        RecipeParameterValue::Choice("proximity_hotplate".to_string()),
                    ),
                    ("temperature_c", RecipeParameterValue::Decimal(112.0)),
                    ("duration_min", RecipeParameterValue::Decimal(1.5)),
                    ("ramp_rate_c_min", RecipeParameterValue::Decimal(10.0)),
                    (
                        "atmosphere",
                        RecipeParameterValue::Choice("nitrogen".to_string()),
                    ),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("SPIN_PR_3000"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Bake follows the qualified photoresist spin coat.".to_string(),
                }],
                approval_history: Vec::new(),
            },
        ],
        usage_references: vec![route_usage(
            "THERMAL_BAKE_110C_001",
            1,
            "soft_bake",
            "Process traveler soft bake step",
        )],
    }
}

fn sample_develop_recipe() -> Recipe {
    let id = RecipeId::from("LITHO_DEVELOP_001");
    Recipe {
        id: id.clone(),
        name: "Poly resist develop".to_string(),
        tool_class: ToolClass::SpinCoater,
        owner: "Lithography".to_string(),
        description: "Developer puddle and rinse after poly exposure.".to_string(),
        parameter_specs: specs([
            choice_spec(
                "developer",
                "Developer",
                10,
                ["AZ 300 MIF", "MF-319", "TMAH 2.38%"],
                "AZ 300 MIF",
            ),
            decimal_spec(
                "develop_time_s",
                "Develop time",
                20,
                RecipeUnit::Second,
                10.0,
                180.0,
                60.0,
            ),
            decimal_spec(
                "rinse_time_s",
                "Rinse time",
                30,
                RecipeUnit::Second,
                5.0,
                120.0,
                30.0,
            ),
            integer_spec("dry_rpm", "Dry spin", 40, RecipeUnit::Rpm, 500, 5000, 2500),
        ]),
        versions: vec![RecipeVersion {
            recipe_id: id.clone(),
            version: RecipeVersionNumber(1),
            name: "Baseline develop".to_string(),
            change_summary: "Qualified puddle develop for the demo poly mask.".to_string(),
            author: "litho.process".to_string(),
            created_at: "2026-04-03T10:00:00Z".to_string(),
            approval_state: ApprovalState::Approved,
            approved_by: Some("lead.process.eng".to_string()),
            approved_at: Some("2026-04-04T15:30:00Z".to_string()),
            parameters: parameters([
                (
                    "developer",
                    RecipeParameterValue::Choice("AZ 300 MIF".to_string()),
                ),
                ("develop_time_s", RecipeParameterValue::Decimal(60.0)),
                ("rinse_time_s", RecipeParameterValue::Decimal(30.0)),
                ("dry_rpm", RecipeParameterValue::Integer(2500)),
            ]),
            dependencies: vec![RecipeDependency {
                recipe_id: RecipeId::from("LITHO_POLY_EXPOSE_001"),
                version: Some(RecipeVersionNumber(1)),
                reason: "Develop is qualified against the poly exposure dose.".to_string(),
            }],
            approval_history: vec![approved_event()],
        }],
        usage_references: vec![route_usage(
            "LITHO_DEVELOP_001",
            1,
            "develop_resist",
            "Process traveler develop step",
        )],
    }
}

fn sample_lithography_recipe() -> Recipe {
    let id = RecipeId::from("LITHO_POLY_EXPOSE_001");
    Recipe {
        id: id.clone(),
        name: "Poly lithography exposure".to_string(),
        tool_class: ToolClass::LithographyExposure,
        owner: "Lithography".to_string(),
        description: "Mask aligner exposure recipe for poly gate definition.".to_string(),
        parameter_specs: specs([
            text_spec("mask_id", "Mask ID", 10, "RETICLE-POLY-DEMO"),
            decimal_spec(
                "exposure_dose_mj_cm2",
                "Exposure dose",
                20,
                RecipeUnit::MillijoulePerSquareCentimeter,
                20.0,
                300.0,
                95.0,
            ),
            decimal_spec(
                "focus_offset_um",
                "Focus offset",
                30,
                RecipeUnit::Micrometer,
                -10.0,
                10.0,
                0.0,
            ),
            integer_spec(
                "wavelength_nm",
                "Wavelength",
                40,
                RecipeUnit::Nanometer,
                250,
                450,
                365,
            ),
            choice_spec(
                "alignment_mode",
                "Alignment mode",
                50,
                ["global", "local", "manual"],
                "global",
            ),
            decimal_spec(
                "contact_force_percent",
                "Contact force",
                60,
                RecipeUnit::Percent,
                0.0,
                100.0,
                35.0,
            ),
        ]),
        versions: vec![
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(1),
                name: "Baseline poly expose".to_string(),
                change_summary: "Qualified exposure dose for AZ1512 on demo poly.".to_string(),
                author: "litho.eng".to_string(),
                created_at: "2026-04-01T10:00:00Z".to_string(),
                approval_state: ApprovalState::Approved,
                approved_by: Some("lead.process.eng".to_string()),
                approved_at: Some("2026-04-03T13:15:00Z".to_string()),
                parameters: parameters([
                    (
                        "mask_id",
                        RecipeParameterValue::Text("RETICLE-POLY-DEMO".to_string()),
                    ),
                    ("exposure_dose_mj_cm2", RecipeParameterValue::Decimal(95.0)),
                    ("focus_offset_um", RecipeParameterValue::Decimal(0.0)),
                    ("wavelength_nm", RecipeParameterValue::Integer(365)),
                    (
                        "alignment_mode",
                        RecipeParameterValue::Choice("global".to_string()),
                    ),
                    ("contact_force_percent", RecipeParameterValue::Decimal(35.0)),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("THERMAL_BAKE_110C_001"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Exposure assumes the qualified soft bake.".to_string(),
                }],
                approval_history: vec![approved_event()],
            },
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(2),
                name: "Dose focus split".to_string(),
                change_summary: "Slight dose reduction for line-width split.".to_string(),
                author: "litho.dev".to_string(),
                created_at: "2026-04-21T12:10:00Z".to_string(),
                approval_state: ApprovalState::Draft,
                approved_by: None,
                approved_at: None,
                parameters: parameters([
                    (
                        "mask_id",
                        RecipeParameterValue::Text("RETICLE-POLY-DEMO".to_string()),
                    ),
                    ("exposure_dose_mj_cm2", RecipeParameterValue::Decimal(88.0)),
                    ("focus_offset_um", RecipeParameterValue::Decimal(0.4)),
                    ("wavelength_nm", RecipeParameterValue::Integer(365)),
                    (
                        "alignment_mode",
                        RecipeParameterValue::Choice("global".to_string()),
                    ),
                    ("contact_force_percent", RecipeParameterValue::Decimal(35.0)),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("THERMAL_BAKE_110C_001"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Exposure assumes the qualified soft bake.".to_string(),
                }],
                approval_history: Vec::new(),
            },
        ],
        usage_references: vec![route_usage(
            "LITHO_POLY_EXPOSE_001",
            1,
            "expose_poly",
            "Process traveler exposure step",
        )],
    }
}

fn sample_cd_metrology_recipe() -> Recipe {
    let id = RecipeId::from("METRO_POLY_CD_001");
    Recipe {
        id: id.clone(),
        name: "Poly CD metrology".to_string(),
        tool_class: ToolClass::LithographyExposure,
        owner: "Metrology".to_string(),
        description: "Nine-site critical-dimension measurement after poly etch.".to_string(),
        parameter_specs: specs([
            text_spec("target_feature", "Target feature", 10, "poly gate"),
            integer_spec(
                "sites_per_wafer",
                "Sites per wafer",
                20,
                RecipeUnit::Count,
                1,
                49,
                9,
            ),
            decimal_spec(
                "target_cd_nm",
                "Target CD",
                30,
                RecipeUnit::Nanometer,
                10.0,
                500.0,
                45.0,
            ),
        ]),
        versions: vec![RecipeVersion {
            recipe_id: id.clone(),
            version: RecipeVersionNumber(1),
            name: "Baseline poly CD map".to_string(),
            change_summary: "Initial nine-site CD sampling plan for the demo poly module."
                .to_string(),
            author: "metrology.process".to_string(),
            created_at: "2026-04-05T09:45:00Z".to_string(),
            approval_state: ApprovalState::Approved,
            approved_by: Some("lead.process.eng".to_string()),
            approved_at: Some("2026-04-05T16:00:00Z".to_string()),
            parameters: parameters([
                (
                    "target_feature",
                    RecipeParameterValue::Text("poly gate".to_string()),
                ),
                ("sites_per_wafer", RecipeParameterValue::Integer(9)),
                ("target_cd_nm", RecipeParameterValue::Decimal(45.0)),
            ]),
            dependencies: vec![RecipeDependency {
                recipe_id: RecipeId::from("ETCH_CF4_POLY_001"),
                version: Some(RecipeVersionNumber(1)),
                reason: "Measurement validates the poly etch step.".to_string(),
            }],
            approval_history: vec![approved_event()],
        }],
        usage_references: vec![route_usage(
            "METRO_POLY_CD_001",
            1,
            "poly_cd_metrology",
            "Process traveler CD metrology step",
        )],
    }
}

fn sample_process_route() -> ProcessRoute {
    ProcessRoute {
        id: ProcessRouteId::from("ROUTE_POLY_GATE_DEMO"),
        version: 1,
        name: "Demo poly gate traveler".to_string(),
        steps: vec![
            ProcessStep {
                id: ProcessStepId::from("coat_photoresist"),
                sequence: 10,
                name: "Coat photoresist".to_string(),
                tool_class: ToolClass::SpinCoater,
                recipe: Some(RecipeBinding::new("SPIN_PR_3000", 1)),
                operator_signoff_required: true,
            },
            ProcessStep {
                id: ProcessStepId::from("soft_bake"),
                sequence: 20,
                name: "Soft bake".to_string(),
                tool_class: ToolClass::FurnaceHotplate,
                recipe: Some(RecipeBinding::new("THERMAL_BAKE_110C_001", 1)),
                operator_signoff_required: true,
            },
            ProcessStep {
                id: ProcessStepId::from("expose_poly"),
                sequence: 30,
                name: "Expose poly".to_string(),
                tool_class: ToolClass::LithographyExposure,
                recipe: Some(RecipeBinding::new("LITHO_POLY_EXPOSE_001", 1)),
                operator_signoff_required: true,
            },
            ProcessStep {
                id: ProcessStepId::from("etch_poly"),
                sequence: 40,
                name: "Etch poly".to_string(),
                tool_class: ToolClass::PlasmaEtcher,
                recipe: Some(RecipeBinding::new("ETCH_CF4_POLY_001", 1)),
                operator_signoff_required: true,
            },
        ],
    }
}

fn sample_spin_tool_run() -> ToolRun {
    ToolRun {
        id: ToolRunId::from("RUN-SPIN-0007"),
        tool_id: "SPIN-01".to_string(),
        tool_class: ToolClass::SpinCoater,
        recipe: RecipeBinding::new("SPIN_PR_3000", 1),
        process_step: Some(ProcessStepReference {
            route_id: ProcessRouteId::from("ROUTE_POLY_GATE_DEMO"),
            route_version: 1,
            step_id: ProcessStepId::from("coat_photoresist"),
        }),
        state: ToolRunState::Completed,
        started_at: "2026-04-22T09:15:00Z".to_string(),
        completed_at: Some("2026-04-22T09:18:00Z".to_string()),
    }
}

fn sample_etch_tool_run() -> ToolRun {
    ToolRun {
        id: ToolRunId::from("RUN-ETCH-0012"),
        tool_id: "ETCH-02".to_string(),
        tool_class: ToolClass::PlasmaEtcher,
        recipe: RecipeBinding::new("ETCH_CF4_POLY_001", 1),
        process_step: Some(ProcessStepReference {
            route_id: ProcessRouteId::from("ROUTE_POLY_GATE_DEMO"),
            route_version: 1,
            step_id: ProcessStepId::from("etch_poly"),
        }),
        state: ToolRunState::Completed,
        started_at: "2026-04-22T11:00:00Z".to_string(),
        completed_at: Some("2026-04-22T11:03:00Z".to_string()),
    }
}

fn specs(
    values: impl IntoIterator<Item = (&'static str, RecipeParameterSpec)>,
) -> BTreeMap<String, RecipeParameterSpec> {
    values
        .into_iter()
        .map(|(key, spec)| (key.to_string(), spec))
        .collect()
}

fn parameters(
    values: impl IntoIterator<Item = (&'static str, RecipeParameterValue)>,
) -> BTreeMap<String, RecipeParameterValue> {
    values
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn decimal_spec(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    unit: RecipeUnit,
    min: f64,
    max: f64,
    default_value: f64,
) -> (&'static str, RecipeParameterSpec) {
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Decimal,
            unit: Some(unit),
            required: true,
            display_order,
            description: String::new(),
            validation_rules: vec![ValidationRule::numeric_range(Some(min), Some(max))],
            default_value: Some(RecipeParameterValue::Decimal(default_value)),
        },
    )
}

fn integer_spec(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    unit: RecipeUnit,
    min: i64,
    max: i64,
    default_value: i64,
) -> (&'static str, RecipeParameterSpec) {
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Integer,
            unit: Some(unit),
            required: true,
            display_order,
            description: String::new(),
            validation_rules: vec![ValidationRule::integer_range(Some(min), Some(max))],
            default_value: Some(RecipeParameterValue::Integer(default_value)),
        },
    )
}

fn boolean_spec(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    default_value: bool,
) -> (&'static str, RecipeParameterSpec) {
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Boolean,
            unit: None,
            required: true,
            display_order,
            description: String::new(),
            validation_rules: Vec::new(),
            default_value: Some(RecipeParameterValue::Boolean(default_value)),
        },
    )
}

fn text_spec(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    default_value: &'static str,
) -> (&'static str, RecipeParameterSpec) {
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Text,
            unit: None,
            required: true,
            display_order,
            description: String::new(),
            validation_rules: vec![ValidationRule::NonEmpty],
            default_value: Some(RecipeParameterValue::Text(default_value.to_string())),
        },
    )
}

fn choice_spec<const N: usize>(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    options: [&'static str; N],
    default_value: &'static str,
) -> (&'static str, RecipeParameterSpec) {
    let options: Vec<_> = options.into_iter().map(str::to_string).collect();
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Choice {
                options: options.clone(),
            },
            unit: None,
            required: true,
            display_order,
            description: String::new(),
            validation_rules: Vec::new(),
            default_value: Some(RecipeParameterValue::Choice(default_value.to_string())),
        },
    )
}

fn approved_event() -> ApprovalEvent {
    ApprovalEvent {
        action: ApprovalAction::Approve,
        from_state: ApprovalState::InReview,
        to_state: ApprovalState::Approved,
        actor: "lead.process.eng".to_string(),
        timestamp: "2026-04-02T15:30:00Z".to_string(),
        note: "Initial qualified version.".to_string(),
    }
}

fn route_usage(
    recipe_id: &'static str,
    version: u32,
    step_id: &'static str,
    context: &'static str,
) -> RecipeUsageReference {
    RecipeUsageReference {
        binding: RecipeBinding::new(recipe_id, version),
        target: RecipeUsageTarget::ProcessStep {
            route_id: ProcessRouteId::from("ROUTE_POLY_GATE_DEMO"),
            route_version: 1,
            step_id: ProcessStepId::from(step_id),
        },
        context: context.to_string(),
    }
}

fn tool_run_usage(
    recipe_id: &'static str,
    version: u32,
    tool_run_id: &'static str,
    tool_id: &'static str,
) -> RecipeUsageReference {
    RecipeUsageReference {
        binding: RecipeBinding::new(recipe_id, version),
        target: RecipeUsageTarget::ToolRun {
            tool_run_id: ToolRunId::from(tool_run_id),
            tool_id: tool_id.to_string(),
        },
        context: "Historical tool run".to_string(),
    }
}

fn range_f64_failed(value: f64, min: Option<f64>, max: Option<f64>, inclusive: bool) -> bool {
    if inclusive {
        min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max)
    } else {
        min.is_some_and(|min| value <= min) || max.is_some_and(|max| value >= max)
    }
}

fn range_i64_failed(value: i64, min: Option<i64>, max: Option<i64>, inclusive: bool) -> bool {
    if inclusive {
        min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max)
    } else {
        min.is_some_and(|min| value <= min) || max.is_some_and(|max| value >= max)
    }
}

fn format_numeric_range(min: Option<f64>, max: Option<f64>, inclusive: bool) -> String {
    match (min, max, inclusive) {
        (Some(min), Some(max), true) => format!(
            "between {} and {}",
            format_decimal(min),
            format_decimal(max)
        ),
        (Some(min), Some(max), false) => format!(
            "greater than {} and less than {}",
            format_decimal(min),
            format_decimal(max)
        ),
        (Some(min), None, true) => format!("at least {}", format_decimal(min)),
        (Some(min), None, false) => format!("greater than {}", format_decimal(min)),
        (None, Some(max), true) => format!("at most {}", format_decimal(max)),
        (None, Some(max), false) => format!("less than {}", format_decimal(max)),
        (None, None, _) => "finite".to_string(),
    }
}

fn format_integer_range(min: Option<i64>, max: Option<i64>, inclusive: bool) -> String {
    match (min, max, inclusive) {
        (Some(min), Some(max), true) => format!("between {min} and {max}"),
        (Some(min), Some(max), false) => format!("greater than {min} and less than {max}"),
        (Some(min), None, true) => format!("at least {min}"),
        (Some(min), None, false) => format!("greater than {min}"),
        (None, Some(max), true) => format!("at most {max}"),
        (None, Some(max), false) => format!("less than {max}"),
        (None, None, _) => "finite".to_string(),
    }
}

fn format_decimal(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let mut rendered = format!("{value:.3}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_parameter_ranges_types_and_unknowns() {
        let catalog = sample_recipe_catalog();
        let recipe = catalog
            .recipe(&RecipeId::from("ETCH_CF4_POLY_001"))
            .expect("etch recipe");
        let mut version = recipe.version(RecipeVersionNumber(1)).unwrap().clone();
        assert!(recipe.validate_version(&version).is_empty());

        version
            .parameters
            .insert("rf_power_w".to_string(), RecipeParameterValue::Integer(900));
        version.parameters.insert(
            "gas_recipe".to_string(),
            RecipeParameterValue::Choice("argon".to_string()),
        );
        version.parameters.insert(
            "operator_note".to_string(),
            RecipeParameterValue::Text("split".to_string()),
        );

        let issues = recipe.validate_version(&version);
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.severity == ValidationSeverity::Error)
                .count(),
            2
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.parameter_key.as_deref() == Some("rf_power_w"))
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.parameter_key.as_deref() == Some("gas_recipe"))
        );
        assert!(issues.iter().any(|issue| {
            issue.severity == ValidationSeverity::Warning
                && issue.parameter_key.as_deref() == Some("operator_note")
        }));
    }

    #[test]
    fn sample_recipe_catalog_validates() {
        let catalog = sample_recipe_catalog();
        let issues = catalog.validate();
        assert!(
            !issues
                .iter()
                .any(|issue| issue.severity == ValidationSeverity::Error),
            "{issues:?}"
        );
    }

    #[test]
    fn recipe_catalog_validation_rejects_bad_catalog_references() {
        let mut catalog = sample_recipe_catalog();
        let spin_id = RecipeId::from("SPIN_PR_3000");
        let first_spin_version = catalog.recipes[&spin_id].versions[0].clone();
        catalog
            .recipes
            .get_mut(&spin_id)
            .unwrap()
            .versions
            .push(first_spin_version);
        catalog
            .process_routes
            .push(catalog.process_routes[0].clone());
        catalog.process_routes[0].steps[0].recipe = Some(RecipeBinding::new("MISSING_RECIPE", 1));
        catalog.process_routes[0].steps[1].sequence = 0;
        let mut duplicate_run = catalog.tool_runs[0].clone();
        duplicate_run.recipe = RecipeBinding::new("SPIN_PR_3000", 99);
        duplicate_run.process_step.as_mut().unwrap().step_id = ProcessStepId::from("missing_step");
        catalog.tool_runs.push(duplicate_run);
        let mut stale_run = catalog.tool_runs[1].clone();
        stale_run.id = ToolRunId::from("RUN-STALE-META");
        stale_run.tool_class = ToolClass::SpinCoater;
        stale_run.started_at = "2026-04-22T11:05:00Z".to_string();
        stale_run.completed_at = Some("2026-04-22T11:00:00Z".to_string());
        catalog.tool_runs.push(stale_run);
        let mut queued_run = catalog.tool_runs[1].clone();
        queued_run.id = ToolRunId::from("RUN-QUEUED-META");
        queued_run.state = ToolRunState::Queued;
        queued_run.completed_at = Some("2026-04-22T11:03:00Z".to_string());
        catalog.tool_runs.push(queued_run);
        let mut zero_version_run = catalog.tool_runs[1].clone();
        zero_version_run.id = ToolRunId::from("RUN-ZERO-STEP-VERSION");
        zero_version_run
            .process_step
            .as_mut()
            .unwrap()
            .route_version = 0;
        catalog.tool_runs.push(zero_version_run);

        let issues = catalog.validate();
        let messages = issues
            .iter()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages
                .iter()
                .any(|message| message.contains("recipe SPIN_PR_3000 has duplicate version v1"))
        );
        assert!(messages.iter().any(|message| {
            message.contains("process route ROUTE_POLY_GATE_DEMO v1 is duplicated")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("process route ROUTE_POLY_GATE_DEMO v1 step soft_bake has sequence 0")
        }));
        assert!(
            messages.iter().any(|message| message.contains(
                "process route ROUTE_POLY_GATE_DEMO v1 step coat_photoresist references missing recipe MISSING_RECIPE"
            ))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("tool run RUN-SPIN-0007 is duplicated"))
        );
        assert!(messages.iter().any(|message| message.contains(
            "tool run RUN-SPIN-0007 references missing recipe binding SPIN_PR_3000 v99"
        )));
        assert!(
            messages.iter().any(|message| message.contains(
                "tool run RUN-SPIN-0007 references missing process step ROUTE_POLY_GATE_DEMO v1 missing_step"
            ))
        );
        assert!(messages.iter().any(|message| message.contains(
            "tool run RUN-STALE-META class Spin coater does not match process step ROUTE_POLY_GATE_DEMO v1 etch_poly class Plasma etcher"
        )));
        assert!(messages.iter().any(|message| message.contains(
            "tool run RUN-STALE-META completed_at timestamp is before started_at timestamp"
        )));
        assert!(messages.iter().any(|message| {
            message.contains("queued tool run RUN-QUEUED-META already has started_at timestamp")
        }));
        assert!(messages.iter().any(|message| message.contains(
            "non-completed tool run RUN-QUEUED-META already has completed_at timestamp"
        )));
        assert!(messages.iter().any(|message| {
            message.contains("tool run RUN-ZERO-STEP-VERSION references process route version 0")
        }));
    }

    #[test]
    fn diffs_versions_as_readable_parameter_changes() {
        let catalog = sample_recipe_catalog();
        let recipe = catalog
            .recipe(&RecipeId::from("SPIN_PR_3000"))
            .expect("spin recipe");
        let from = recipe.version(RecipeVersionNumber(1)).unwrap();
        let to = recipe.version(RecipeVersionNumber(2)).unwrap();

        let diff = diff_recipe_versions(recipe, from, to);
        assert!(diff.entries.iter().any(|entry| {
            entry.parameter_key.as_deref() == Some("spin_rpm")
                && entry.before.as_deref() == Some("4000 rpm")
                && entry.after.as_deref() == Some("4200 rpm")
        }));
        assert!(diff.entries.iter().any(|entry| {
            entry.kind == RecipeDiffKind::ApprovalChanged
                && entry.before.as_deref() == Some("Approved")
                && entry.after.as_deref() == Some("In review")
        }));
    }

    #[test]
    fn approval_transitions_require_review_and_clean_validation() {
        let catalog = sample_recipe_catalog();
        let recipe = catalog
            .recipe(&RecipeId::from("LITHO_POLY_EXPOSE_001"))
            .expect("lithography recipe");
        let mut version = recipe.version(RecipeVersionNumber(2)).unwrap().clone();

        let valid_issues = recipe.validate_version(&version);
        let direct_approve = version.apply_approval_action(
            ApprovalAction::Approve,
            "qa.eng",
            "2026-04-22T12:00:00Z",
            "",
            &valid_issues,
        );
        assert!(matches!(
            direct_approve,
            Err(ApprovalTransitionError::InvalidTransition {
                from: ApprovalState::Draft,
                action: ApprovalAction::Approve
            })
        ));

        version
            .apply_approval_action(
                ApprovalAction::SubmitForReview,
                "litho.dev",
                "2026-04-22T12:05:00Z",
                "",
                &valid_issues,
            )
            .unwrap();
        assert_eq!(version.approval_state, ApprovalState::InReview);

        version.parameters.insert(
            "exposure_dose_mj_cm2".to_string(),
            RecipeParameterValue::Decimal(1000.0),
        );
        let invalid_issues = recipe.validate_version(&version);
        let blocked = version.apply_approval_action(
            ApprovalAction::Approve,
            "qa.eng",
            "2026-04-22T12:10:00Z",
            "",
            &invalid_issues,
        );
        assert!(matches!(
            blocked,
            Err(ApprovalTransitionError::ValidationFailed { error_count: 1 })
        ));

        version.parameters.insert(
            "exposure_dose_mj_cm2".to_string(),
            RecipeParameterValue::Decimal(90.0),
        );
        let fixed_issues = recipe.validate_version(&version);
        version
            .apply_approval_action(
                ApprovalAction::Approve,
                "qa.eng",
                "2026-04-22T12:20:00Z",
                "Split approved.",
                &fixed_issues,
            )
            .unwrap();
        assert_eq!(version.approval_state, ApprovalState::Approved);
        assert_eq!(version.approved_by.as_deref(), Some("qa.eng"));
    }

    #[test]
    fn serializes_schema_friendly_catalog_and_route_recipe_refs() {
        let catalog = sample_recipe_catalog();
        let json = serde_json::to_value(&catalog).expect("serialize catalog");

        let spin_params = &json["recipes"]["SPIN_PR_3000"]["versions"][0]["parameters"];
        assert_eq!(spin_params["spin_rpm"]["type"], "integer");
        assert_eq!(spin_params["spin_rpm"]["value"], 4000);
        assert_eq!(
            json["process_routes"][0]["steps"][0]["recipe"]["recipe_id"],
            "SPIN_PR_3000"
        );
        assert_eq!(
            json["process_routes"][0]["steps"][0]["recipe"]["version"],
            1
        );

        let round_tripped: RecipeCatalog =
            serde_json::from_value(json).expect("round-trip catalog");
        let route = &round_tripped.process_routes[0];
        let step = route
            .step(&ProcessStepId::from("etch_poly"))
            .expect("etch step");
        assert_eq!(
            step.recipe.as_ref(),
            Some(&RecipeBinding::new("ETCH_CF4_POLY_001", 1))
        );
    }
}
