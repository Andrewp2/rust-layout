#![allow(unused_imports)]
use super::*;

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
    pub(crate) fn error(
        parameter_key: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            parameter_key: parameter_key.into(),
            message: message.into(),
        }
    }

    pub(crate) fn warning(
        parameter_key: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            parameter_key: parameter_key.into(),
            message: message.into(),
        }
    }

    pub(crate) fn with_context(mut self, context: impl AsRef<str>) -> Self {
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

    pub(crate) fn validate_parameter_type(
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

    pub(crate) fn validate_parameter_rules(
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

pub(crate) type ProcessStepKey = (ProcessRouteId, u32, ProcessStepId);
pub(crate) type ProcessStepCatalogEntry = (Option<RecipeBinding>, ToolClass);

pub(crate) fn validate_catalog_recipe(
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

pub(crate) fn validate_recipe_approval_state(
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

pub(crate) fn validate_parameter_spec_options(
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

pub(crate) fn validate_catalog_route(
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

pub(crate) fn validate_catalog_tool_run(
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

pub(crate) fn validate_usage_references(
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

pub(crate) fn validate_recipe_binding(
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
