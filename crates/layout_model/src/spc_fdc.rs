use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    equipment::{Alarm, AlarmSeverity, SensorSample},
    yield_analysis::ProcessMeasurement,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChartId(pub String);

impl ChartId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChartId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ChartId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ChartId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MonitorSeverity {
    Advisory,
    Warning,
    Critical,
}

impl MonitorSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Advisory => "Advisory",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
        }
    }
}

impl From<AlarmSeverity> for MonitorSeverity {
    fn from(value: AlarmSeverity) -> Self {
        match value {
            AlarmSeverity::Advisory => Self::Advisory,
            AlarmSeverity::Warning => Self::Warning,
            AlarmSeverity::Critical => Self::Critical,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlRule {
    OutsideControlLimit,
    TwoOfThreeBeyondTwoSigma,
    SixPointTrend,
    EightOnOneSide,
}

impl ControlRule {
    pub fn label(self) -> &'static str {
        match self {
            Self::OutsideControlLimit => "outside control limit",
            Self::TwoOfThreeBeyondTwoSigma => "two of three beyond 2 sigma",
            Self::SixPointTrend => "six point trend",
            Self::EightOnOneSide => "eight on one side",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FindingSource {
    SpcRule,
    FdcTrace,
    EquipmentAlarm,
}

impl FindingSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::SpcRule => "SPC",
            Self::FdcTrace => "FDC",
            Self::EquipmentAlarm => "Alarm",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlLimits {
    pub center: f64,
    pub lower_control: f64,
    pub upper_control: f64,
    pub one_sigma: f64,
    #[serde(default)]
    pub lower_spec: Option<f64>,
    #[serde(default)]
    pub upper_spec: Option<f64>,
}

impl ControlLimits {
    pub fn new(center: f64, lower_control: f64, upper_control: f64) -> Self {
        let raw_span = (upper_control - lower_control).abs();
        let span = raw_span.max(0.000_001);
        if span != raw_span {
            warn!(
                lower_control,
                upper_control, "SPC control limit span was zero; using epsilon span"
            );
        }
        Self {
            center,
            lower_control,
            upper_control,
            one_sigma: span / 6.0,
            lower_spec: None,
            upper_spec: None,
        }
    }

    pub fn with_specs(mut self, lower_spec: Option<f64>, upper_spec: Option<f64>) -> Self {
        self.lower_spec = lower_spec;
        self.upper_spec = upper_spec;
        self
    }

    pub fn two_sigma_low(&self) -> f64 {
        self.center - self.one_sigma * 2.0
    }

    pub fn two_sigma_high(&self) -> f64 {
        self.center + self.one_sigma * 2.0
    }

    pub fn contains(&self, value: f64) -> bool {
        value >= self.lower_control && value <= self.upper_control
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlPoint {
    pub sequence: usize,
    pub source_id: String,
    pub lot_id: String,
    pub wafer_id: String,
    pub step_id: String,
    pub tool_run_id: String,
    pub recipe_id: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleViolation {
    pub id: String,
    pub chart_id: ChartId,
    pub rule: ControlRule,
    pub severity: MonitorSeverity,
    pub message: String,
    pub point_indices: Vec<usize>,
    #[serde(default)]
    pub lot_id: Option<String>,
    #[serde(default)]
    pub wafer_id: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub recipe_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlChart {
    pub id: ChartId,
    pub name: String,
    pub metric: String,
    pub unit: String,
    pub limits: ControlLimits,
    pub points: Vec<ControlPoint>,
    pub violations: Vec<RuleViolation>,
}

impl ControlChart {
    pub fn highest_severity(&self) -> Option<MonitorSeverity> {
        self.violations
            .iter()
            .map(|violation| violation.severity)
            .max()
    }

    pub fn latest_point(&self) -> Option<&ControlPoint> {
        self.points.last()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorLimit {
    #[serde(default)]
    pub lower: Option<f64>,
    #[serde(default)]
    pub upper: Option<f64>,
}

impl SensorLimit {
    pub fn new(lower: Option<f64>, upper: Option<f64>) -> Self {
        Self { lower, upper }
    }

    pub fn contains(&self, value: f64) -> bool {
        self.lower.is_none_or(|lower| value >= lower)
            && self.upper.is_none_or(|upper| value <= upper)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorTracePoint {
    pub at_s: u64,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorViolation {
    pub trace_id: String,
    pub tool_id: String,
    pub sensor_name: String,
    pub at_s: u64,
    pub value: f64,
    pub unit: String,
    pub limit: SensorLimit,
    pub severity: MonitorSeverity,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorTrace {
    pub id: String,
    pub tool_id: String,
    pub sensor_name: String,
    pub unit: String,
    pub limit: SensorLimit,
    pub samples: Vec<SensorTracePoint>,
    pub violations: Vec<SensorViolation>,
}

impl SensorTrace {
    pub fn latest_point(&self) -> Option<&SensorTracePoint> {
        self.samples.last()
    }

    pub fn display_name(&self) -> String {
        format!("{} / {}", self.tool_id, self.sensor_name)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AlarmSummary {
    pub total_count: usize,
    pub active_count: usize,
    pub by_severity: BTreeMap<String, usize>,
    pub by_tool: BTreeMap<String, usize>,
    #[serde(default)]
    pub latest_alarm: Option<Alarm>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonitorFinding {
    pub source: FindingSource,
    pub severity: MonitorSeverity,
    pub title: String,
    pub detail: String,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub lot_id: Option<String>,
    #[serde(default)]
    pub wafer_id: Option<String>,
    #[serde(default)]
    pub recipe_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpcFdcMonitor {
    pub charts: Vec<ControlChart>,
    pub traces: Vec<SensorTrace>,
    pub alarm_summary: AlarmSummary,
    pub findings: Vec<MonitorFinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpcFdcValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpcFdcValidationFinding {
    pub severity: SpcFdcValidationSeverity,
    pub message: String,
}

impl SpcFdcValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: SpcFdcValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: SpcFdcValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

impl SpcFdcMonitor {
    pub fn from_fab_context(
        measurements: &[ProcessMeasurement],
        sensor_samples: &[SensorSample],
        alarms: &[Alarm],
    ) -> Self {
        let charts = control_charts_for_measurements(measurements);
        let traces = sensor_traces_for_samples(sensor_samples);
        let alarm_summary = summarize_alarms(alarms);
        let mut findings = Vec::new();

        for chart in &charts {
            for violation in &chart.violations {
                findings.push(MonitorFinding {
                    source: FindingSource::SpcRule,
                    severity: violation.severity,
                    title: format!(
                        "{}: {}",
                        chart.metric.replace('_', " "),
                        violation.rule.label()
                    ),
                    detail: violation.message.clone(),
                    tool_id: violation.tool_id.clone(),
                    lot_id: violation.lot_id.clone(),
                    wafer_id: violation.wafer_id.clone(),
                    recipe_id: violation.recipe_id.clone(),
                });
            }
        }

        for trace in &traces {
            for violation in &trace.violations {
                findings.push(MonitorFinding {
                    source: FindingSource::FdcTrace,
                    severity: violation.severity,
                    title: format!("{} excursion", trace.display_name()),
                    detail: violation.message.clone(),
                    tool_id: Some(trace.tool_id.clone()),
                    lot_id: None,
                    wafer_id: None,
                    recipe_id: None,
                });
            }
        }

        for alarm in alarms.iter().filter(|alarm| alarm.active) {
            findings.push(MonitorFinding {
                source: FindingSource::EquipmentAlarm,
                severity: alarm.severity.into(),
                title: format!("{} {}", alarm.tool_id, alarm.code),
                detail: alarm.message.clone(),
                tool_id: Some(alarm.tool_id.to_string()),
                lot_id: None,
                wafer_id: None,
                recipe_id: None,
            });
        }

        findings.sort_by(|left, right| {
            right
                .severity
                .cmp(&left.severity)
                .then_with(|| left.source.cmp(&right.source))
                .then_with(|| left.title.cmp(&right.title))
        });

        Self {
            charts,
            traces,
            alarm_summary,
            findings,
        }
    }

    pub fn finding_count_by_severity(&self, severity: MonitorSeverity) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .count()
    }

    pub fn validate(&self) -> Vec<SpcFdcValidationFinding> {
        let mut findings = Vec::new();
        let mut chart_ids = BTreeSet::new();
        let mut spc_violation_count = 0usize;
        for chart in &self.charts {
            validate_chart(chart, &mut chart_ids, &mut findings);
            spc_violation_count += chart.violations.len();
        }

        let mut trace_ids = BTreeSet::new();
        let mut fdc_violation_count = 0usize;
        for trace in &self.traces {
            validate_trace(trace, &mut trace_ids, &mut findings);
            fdc_violation_count += trace.violations.len();
        }

        validate_alarm_summary(&self.alarm_summary, &mut findings);
        validate_findings(
            &self.findings,
            spc_violation_count,
            fdc_violation_count,
            self.alarm_summary.active_count,
            &mut findings,
        );

        findings
    }
}

fn validate_chart(
    chart: &ControlChart,
    chart_ids: &mut BTreeSet<ChartId>,
    findings: &mut Vec<SpcFdcValidationFinding>,
) {
    if chart.id.as_str().trim().is_empty() {
        findings.push(SpcFdcValidationFinding::error("SPC chart id is empty"));
    } else if !chart_ids.insert(chart.id.clone()) {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {} is duplicated",
            chart.id
        )));
    }
    for (field, value) in [
        ("name", chart.name.as_str()),
        ("metric", chart.metric.as_str()),
        ("unit", chart.unit.as_str()),
    ] {
        if value.trim().is_empty() {
            findings.push(SpcFdcValidationFinding::warning(format!(
                "SPC chart {} has empty {field}",
                chart.id
            )));
        }
    }
    validate_control_limits(&chart.id, &chart.limits, findings);

    let mut source_ids = BTreeSet::new();
    for (index, point) in chart.points.iter().enumerate() {
        let expected_sequence = index + 1;
        if point.sequence != expected_sequence {
            findings.push(SpcFdcValidationFinding::error(format!(
                "SPC chart {} point {} has sequence {}, expected {}",
                chart.id, point.source_id, point.sequence, expected_sequence
            )));
        }
        validate_control_point(&chart.id, point, &mut source_ids, findings);
    }

    let mut violation_ids = BTreeSet::new();
    for violation in &chart.violations {
        validate_rule_violation(chart, violation, &mut violation_ids, findings);
    }
}

fn validate_control_limits(
    chart_id: &ChartId,
    limits: &ControlLimits,
    findings: &mut Vec<SpcFdcValidationFinding>,
) {
    for (label, value) in [
        ("center", Some(limits.center)),
        ("lower control", Some(limits.lower_control)),
        ("upper control", Some(limits.upper_control)),
        ("one sigma", Some(limits.one_sigma)),
        ("lower spec", limits.lower_spec),
        ("upper spec", limits.upper_spec),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            findings.push(SpcFdcValidationFinding::error(format!(
                "SPC chart {chart_id} has non-finite {label}"
            )));
        }
    }
    if limits.lower_control > limits.upper_control {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {chart_id} lower control is above upper control"
        )));
    }
    if limits.one_sigma <= 0.0 {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {chart_id} one sigma must be positive"
        )));
    }
    if let (Some(lower), Some(upper)) = (limits.lower_spec, limits.upper_spec)
        && lower > upper
    {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {chart_id} lower spec is above upper spec"
        )));
    }
}

fn validate_control_point(
    chart_id: &ChartId,
    point: &ControlPoint,
    source_ids: &mut BTreeSet<String>,
    findings: &mut Vec<SpcFdcValidationFinding>,
) {
    if point.source_id.trim().is_empty() {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {chart_id} has a point with empty source id"
        )));
    } else if !source_ids.insert(point.source_id.clone()) {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {chart_id} repeats source point {}",
            point.source_id
        )));
    }
    for (field, value) in [
        ("lot id", point.lot_id.as_str()),
        ("wafer id", point.wafer_id.as_str()),
        ("step id", point.step_id.as_str()),
        ("tool run id", point.tool_run_id.as_str()),
        ("recipe id", point.recipe_id.as_str()),
    ] {
        if value.trim().is_empty() {
            findings.push(SpcFdcValidationFinding::error(format!(
                "SPC chart {chart_id} point {} has empty {field}",
                point.source_id
            )));
        }
    }
    if !point.value.is_finite() {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {chart_id} point {} has non-finite value",
            point.source_id
        )));
    }
}

fn validate_rule_violation(
    chart: &ControlChart,
    violation: &RuleViolation,
    violation_ids: &mut BTreeSet<String>,
    findings: &mut Vec<SpcFdcValidationFinding>,
) {
    if violation.id.trim().is_empty() {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {} has a violation with empty id",
            chart.id
        )));
    } else if !violation_ids.insert(violation.id.clone()) {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC chart {} repeats violation {}",
            chart.id, violation.id
        )));
    }
    if violation.chart_id != chart.id {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC violation {} references chart {}, but is stored under {}",
            violation.id, violation.chart_id, chart.id
        )));
    }
    if violation.message.trim().is_empty() {
        findings.push(SpcFdcValidationFinding::warning(format!(
            "SPC violation {} has empty message",
            violation.id
        )));
    }
    if violation.point_indices.is_empty() {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC violation {} has no point indices",
            violation.id
        )));
    }
    for point_index in &violation.point_indices {
        if *point_index >= chart.points.len() {
            findings.push(SpcFdcValidationFinding::error(format!(
                "SPC violation {} references missing point index {}",
                violation.id, point_index
            )));
        }
    }
    for (field, value) in [
        ("lot id", violation.lot_id.as_deref()),
        ("wafer id", violation.wafer_id.as_deref()),
        ("tool id", violation.tool_id.as_deref()),
        ("recipe id", violation.recipe_id.as_deref()),
    ] {
        if value.is_some_and(|value| value.trim().is_empty()) {
            findings.push(SpcFdcValidationFinding::warning(format!(
                "SPC violation {} has empty {field}",
                violation.id
            )));
        }
    }
}

fn validate_trace(
    trace: &SensorTrace,
    trace_ids: &mut BTreeSet<String>,
    findings: &mut Vec<SpcFdcValidationFinding>,
) {
    if trace.id.trim().is_empty() {
        findings.push(SpcFdcValidationFinding::error("FDC trace id is empty"));
    } else if !trace_ids.insert(trace.id.clone()) {
        findings.push(SpcFdcValidationFinding::error(format!(
            "FDC trace {} is duplicated",
            trace.id
        )));
    }
    for (field, value) in [
        ("tool id", trace.tool_id.as_str()),
        ("sensor name", trace.sensor_name.as_str()),
        ("unit", trace.unit.as_str()),
    ] {
        if value.trim().is_empty() {
            findings.push(SpcFdcValidationFinding::error(format!(
                "FDC trace {} has empty {field}",
                trace.id
            )));
        }
    }
    validate_sensor_limit(&trace.id, &trace.limit, findings);

    let mut last_sample_at = None;
    let mut sample_times = BTreeSet::new();
    for sample in &trace.samples {
        if let Some(last_sample_at) = last_sample_at
            && sample.at_s < last_sample_at
        {
            findings.push(SpcFdcValidationFinding::error(format!(
                "FDC trace {} samples are not sorted by timestamp",
                trace.id
            )));
        }
        last_sample_at = Some(sample.at_s);
        if !sample_times.insert(sample.at_s) {
            findings.push(SpcFdcValidationFinding::warning(format!(
                "FDC trace {} repeats sample timestamp {}",
                trace.id, sample.at_s
            )));
        }
        if !sample.value.is_finite() {
            findings.push(SpcFdcValidationFinding::error(format!(
                "FDC trace {} sample {} has non-finite value",
                trace.id, sample.at_s
            )));
        }
    }

    let mut violation_keys = BTreeSet::new();
    for violation in &trace.violations {
        validate_sensor_violation(trace, violation, &mut violation_keys, findings);
    }
}

fn validate_sensor_limit(
    trace_id: &str,
    limit: &SensorLimit,
    findings: &mut Vec<SpcFdcValidationFinding>,
) {
    for (label, value) in [("lower", limit.lower), ("upper", limit.upper)] {
        if value.is_some_and(|value| !value.is_finite()) {
            findings.push(SpcFdcValidationFinding::error(format!(
                "FDC trace {trace_id} has non-finite {label} limit"
            )));
        }
    }
    if let (Some(lower), Some(upper)) = (limit.lower, limit.upper)
        && lower > upper
    {
        findings.push(SpcFdcValidationFinding::error(format!(
            "FDC trace {trace_id} lower limit is above upper limit"
        )));
    }
}

fn validate_sensor_violation(
    trace: &SensorTrace,
    violation: &SensorViolation,
    violation_keys: &mut BTreeSet<(u64, String)>,
    findings: &mut Vec<SpcFdcValidationFinding>,
) {
    if !violation_keys.insert((violation.at_s, violation.message.clone())) {
        findings.push(SpcFdcValidationFinding::warning(format!(
            "FDC trace {} repeats violation at {}",
            trace.id, violation.at_s
        )));
    }
    if violation.trace_id != trace.id {
        findings.push(SpcFdcValidationFinding::error(format!(
            "FDC violation for {} references trace {}",
            trace.id, violation.trace_id
        )));
    }
    if violation.tool_id != trace.tool_id {
        findings.push(SpcFdcValidationFinding::error(format!(
            "FDC violation for trace {} references tool {}",
            trace.id, violation.tool_id
        )));
    }
    if violation.sensor_name != trace.sensor_name {
        findings.push(SpcFdcValidationFinding::error(format!(
            "FDC violation for trace {} references sensor {}",
            trace.id, violation.sensor_name
        )));
    }
    if violation.unit != trace.unit {
        findings.push(SpcFdcValidationFinding::warning(format!(
            "FDC violation for trace {} uses unit {}, expected {}",
            trace.id, violation.unit, trace.unit
        )));
    }
    if violation.limit != trace.limit {
        findings.push(SpcFdcValidationFinding::error(format!(
            "FDC violation for trace {} uses stale limits",
            trace.id
        )));
    }
    match trace
        .samples
        .iter()
        .find(|sample| sample.at_s == violation.at_s)
    {
        Some(sample) if (sample.value - violation.value).abs() > 0.001 => {
            findings.push(SpcFdcValidationFinding::warning(format!(
                "FDC violation for trace {} does not match sample value at {}",
                trace.id, violation.at_s
            )));
        }
        Some(_) => {}
        None => findings.push(SpcFdcValidationFinding::error(format!(
            "FDC violation for trace {} references missing sample {}",
            trace.id, violation.at_s
        ))),
    }
    if !violation.value.is_finite() {
        findings.push(SpcFdcValidationFinding::error(format!(
            "FDC violation for trace {} has non-finite value",
            trace.id
        )));
    }
    if violation.message.trim().is_empty() {
        findings.push(SpcFdcValidationFinding::warning(format!(
            "FDC violation for trace {} has empty message",
            trace.id
        )));
    }
}

fn validate_alarm_summary(summary: &AlarmSummary, findings: &mut Vec<SpcFdcValidationFinding>) {
    if summary.active_count > summary.total_count {
        findings.push(SpcFdcValidationFinding::error(
            "SPC/FDC alarm summary active count exceeds total count",
        ));
    }
    let by_severity_total = summary.by_severity.values().sum::<usize>();
    if by_severity_total != summary.active_count {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC/FDC alarm summary severity count {} does not match active count {}",
            by_severity_total, summary.active_count
        )));
    }
    let by_tool_total = summary.by_tool.values().sum::<usize>();
    if by_tool_total != summary.active_count {
        findings.push(SpcFdcValidationFinding::error(format!(
            "SPC/FDC alarm summary tool count {} does not match active count {}",
            by_tool_total, summary.active_count
        )));
    }
    if summary.total_count == 0 && summary.latest_alarm.is_some() {
        findings.push(SpcFdcValidationFinding::warning(
            "SPC/FDC alarm summary has latest alarm despite zero total count",
        ));
    }
    if summary.total_count > 0 && summary.latest_alarm.is_none() {
        findings.push(SpcFdcValidationFinding::warning(
            "SPC/FDC alarm summary has no latest alarm despite nonzero total count",
        ));
    }
}

fn validate_findings(
    monitor_findings: &[MonitorFinding],
    spc_violation_count: usize,
    fdc_violation_count: usize,
    active_alarm_count: usize,
    findings: &mut Vec<SpcFdcValidationFinding>,
) {
    let spc_findings = monitor_findings
        .iter()
        .filter(|finding| finding.source == FindingSource::SpcRule)
        .count();
    let fdc_findings = monitor_findings
        .iter()
        .filter(|finding| finding.source == FindingSource::FdcTrace)
        .count();
    let alarm_findings = monitor_findings
        .iter()
        .filter(|finding| finding.source == FindingSource::EquipmentAlarm)
        .count();
    for (label, expected, actual) in [
        ("SPC", spc_violation_count, spc_findings),
        ("FDC", fdc_violation_count, fdc_findings),
        ("alarm", active_alarm_count, alarm_findings),
    ] {
        if expected != actual {
            findings.push(SpcFdcValidationFinding::error(format!(
                "SPC/FDC monitor has {actual} {label} findings but expected {expected}"
            )));
        }
    }
    for finding in monitor_findings {
        if finding.title.trim().is_empty() || finding.detail.trim().is_empty() {
            findings.push(SpcFdcValidationFinding::warning(format!(
                "SPC/FDC {} finding has incomplete text",
                finding.source.label()
            )));
        }
        for (field, value) in [
            ("tool id", finding.tool_id.as_deref()),
            ("lot id", finding.lot_id.as_deref()),
            ("wafer id", finding.wafer_id.as_deref()),
            ("recipe id", finding.recipe_id.as_deref()),
        ] {
            if value.is_some_and(|value| value.trim().is_empty()) {
                findings.push(SpcFdcValidationFinding::warning(format!(
                    "SPC/FDC {} finding has empty {field}",
                    finding.source.label()
                )));
            }
        }
    }
}

pub fn control_charts_for_measurements(measurements: &[ProcessMeasurement]) -> Vec<ControlChart> {
    let mut grouped: BTreeMap<(String, String), Vec<&ProcessMeasurement>> = BTreeMap::new();
    for measurement in measurements {
        grouped
            .entry((measurement.name.clone(), measurement.unit.clone()))
            .or_default()
            .push(measurement);
    }
    let mut units_by_metric: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (metric, unit) in grouped.keys() {
        units_by_metric
            .entry(metric.clone())
            .or_default()
            .insert(unit.clone());
    }

    grouped
        .into_iter()
        .map(|((metric, unit), mut rows)| {
            rows.sort_by(|left, right| {
                (
                    left.lot_id.as_str(),
                    left.wafer_id.as_str(),
                    left.tool_run_id.as_str(),
                    left.measurement_id.as_str(),
                )
                    .cmp(&(
                        right.lot_id.as_str(),
                        right.wafer_id.as_str(),
                        right.tool_run_id.as_str(),
                        right.measurement_id.as_str(),
                    ))
            });

            let id = spc_chart_id(
                &metric,
                &unit,
                units_by_metric
                    .get(&metric)
                    .is_some_and(|units| units.len() > 1),
            );
            let limits = limits_for_measurements(&rows);
            let points = rows
                .iter()
                .enumerate()
                .map(|(index, measurement)| ControlPoint {
                    sequence: index + 1,
                    source_id: measurement.measurement_id.clone(),
                    lot_id: measurement.lot_id.clone(),
                    wafer_id: measurement.wafer_id.clone(),
                    step_id: measurement.step_id.clone(),
                    tool_run_id: measurement.tool_run_id.clone(),
                    recipe_id: measurement.recipe_id.clone(),
                    value: measurement.value,
                })
                .collect::<Vec<_>>();
            let violations = evaluate_control_rules(&id, &metric, &unit, &limits, &points);
            ControlChart {
                id,
                name: format!("{} SPC", metric.replace('_', " ")),
                metric,
                unit,
                limits,
                points,
                violations,
            }
        })
        .collect()
}

fn spc_chart_id(metric: &str, unit: &str, include_unit: bool) -> ChartId {
    let metric = metric.trim();
    let metric = if metric.is_empty() {
        "unnamed_metric"
    } else {
        metric
    };
    if include_unit {
        let unit = unit.trim();
        let unit = if unit.is_empty() { "unitless" } else { unit };
        ChartId::new(format!("spc:{metric}:{unit}"))
    } else {
        ChartId::new(format!("spc:{metric}"))
    }
}

pub fn sensor_traces_for_samples(samples: &[SensorSample]) -> Vec<SensorTrace> {
    let mut grouped: BTreeMap<(String, String, String), Vec<&SensorSample>> = BTreeMap::new();
    for sample in samples {
        grouped
            .entry((
                sample.tool_id.to_string(),
                sample.name.clone(),
                sample.unit.clone(),
            ))
            .or_default()
            .push(sample);
    }

    grouped
        .into_iter()
        .map(|((tool_id, sensor_name, unit), mut rows)| {
            rows.sort_by_key(|sample| sample.at_s);
            let id = format!("fdc:{tool_id}:{sensor_name}");
            let limit = default_sensor_limit(&sensor_name);
            let samples = rows
                .iter()
                .map(|sample| SensorTracePoint {
                    at_s: sample.at_s,
                    value: sample.value,
                })
                .collect::<Vec<_>>();
            let violations = rows
                .iter()
                .filter(|sample| !limit.contains(sample.value))
                .map(|sample| SensorViolation {
                    trace_id: id.clone(),
                    tool_id: tool_id.clone(),
                    sensor_name: sensor_name.clone(),
                    at_s: sample.at_s,
                    value: sample.value,
                    unit: unit.clone(),
                    limit: limit.clone(),
                    severity: MonitorSeverity::Warning,
                    message: sensor_violation_message(&sensor_name, sample.value, &unit, &limit),
                })
                .collect::<Vec<_>>();
            SensorTrace {
                id,
                tool_id,
                sensor_name,
                unit,
                limit,
                samples,
                violations,
            }
        })
        .collect()
}

pub fn summarize_alarms(alarms: &[Alarm]) -> AlarmSummary {
    let mut by_severity = BTreeMap::new();
    let mut by_tool = BTreeMap::new();
    let mut latest_alarm = None;

    for alarm in alarms {
        if alarm.active {
            *by_severity
                .entry(alarm.severity.label().to_string())
                .or_insert(0) += 1;
            *by_tool.entry(alarm.tool_id.to_string()).or_insert(0) += 1;
        }
        if latest_alarm
            .as_ref()
            .is_none_or(|latest: &Alarm| alarm.occurred_at_s >= latest.occurred_at_s)
        {
            latest_alarm = Some(alarm.clone());
        }
    }

    AlarmSummary {
        total_count: alarms.len(),
        active_count: alarms.iter().filter(|alarm| alarm.active).count(),
        by_severity,
        by_tool,
        latest_alarm,
    }
}

fn limits_for_measurements(measurements: &[&ProcessMeasurement]) -> ControlLimits {
    let values = measurements
        .iter()
        .map(|measurement| measurement.value)
        .collect::<Vec<_>>();
    let center = measurements
        .iter()
        .find_map(|measurement| measurement.target)
        .unwrap_or_else(|| {
            warn!("SPC measurements missing target; using measured mean as center");
            mean(&values)
        });
    let lower_spec = measurements
        .iter()
        .find_map(|measurement| measurement.lower_spec);
    let upper_spec = measurements
        .iter()
        .find_map(|measurement| measurement.upper_spec);

    match (lower_spec, upper_spec) {
        (Some(lower), Some(upper)) if upper > lower => {
            let sigma = ((upper - lower) / 6.0).max(0.000_001);
            ControlLimits {
                center,
                lower_control: lower,
                upper_control: upper,
                one_sigma: sigma,
                lower_spec,
                upper_spec,
            }
        }
        _ => {
            let raw_sigma = sample_stddev(&values);
            let sigma = raw_sigma.max(0.000_001);
            if sigma != raw_sigma {
                warn!(
                    raw_sigma,
                    "SPC calculated sigma was zero; using epsilon sigma"
                );
            }
            if lower_spec.is_none() || upper_spec.is_none() {
                warn!(
                    "SPC measurements missing spec limits; deriving control limits from sample sigma"
                );
            }
            ControlLimits {
                center,
                lower_control: center - sigma * 3.0,
                upper_control: center + sigma * 3.0,
                one_sigma: sigma,
                lower_spec,
                upper_spec,
            }
        }
    }
}

fn evaluate_control_rules(
    chart_id: &ChartId,
    metric: &str,
    unit: &str,
    limits: &ControlLimits,
    points: &[ControlPoint],
) -> Vec<RuleViolation> {
    let mut violations = Vec::new();

    for (index, point) in points.iter().enumerate() {
        if point.value < limits.lower_control || point.value > limits.upper_control {
            let direction = if point.value > limits.upper_control {
                "above UCL"
            } else {
                "below LCL"
            };
            violations.push(rule_violation(
                chart_id,
                ControlRule::OutsideControlLimit,
                MonitorSeverity::Critical,
                vec![index],
                point,
                format!(
                    "{} {} is {} at {:.3} {} (limits {:.3}..{:.3} {})",
                    point.wafer_id,
                    metric.replace('_', " "),
                    direction,
                    point.value,
                    unit,
                    limits.lower_control,
                    limits.upper_control,
                    unit
                ),
            ));
        }
    }

    let mut index = 0;
    while index + 3 <= points.len() {
        let window = &points[index..index + 3];
        let high = window
            .iter()
            .filter(|point| point.value > limits.two_sigma_high())
            .count();
        let low = window
            .iter()
            .filter(|point| point.value < limits.two_sigma_low())
            .count();
        if high >= 2 || low >= 2 {
            let point_indices = (index..index + 3).collect::<Vec<_>>();
            let point = &points[index + 2];
            violations.push(rule_violation(
                chart_id,
                ControlRule::TwoOfThreeBeyondTwoSigma,
                MonitorSeverity::Warning,
                point_indices,
                point,
                format!(
                    "{} has {} of 3 points beyond 2 sigma around {:.3} {}",
                    metric.replace('_', " "),
                    high.max(low),
                    limits.center,
                    unit
                ),
            ));
            index += 3;
        } else {
            index += 1;
        }
    }

    append_trend_violations(chart_id, metric, unit, points, &mut violations);
    append_one_side_violations(chart_id, metric, unit, limits, points, &mut violations);
    violations
}

fn append_trend_violations(
    chart_id: &ChartId,
    metric: &str,
    unit: &str,
    points: &[ControlPoint],
    violations: &mut Vec<RuleViolation>,
) {
    let mut index = 0;
    while index + 6 <= points.len() {
        let window = &points[index..index + 6];
        let increasing = window.windows(2).all(|pair| pair[1].value > pair[0].value);
        let decreasing = window.windows(2).all(|pair| pair[1].value < pair[0].value);
        if increasing || decreasing {
            let point_indices = (index..index + 6).collect::<Vec<_>>();
            let point = &points[index + 5];
            let direction = if increasing {
                "increasing"
            } else {
                "decreasing"
            };
            violations.push(rule_violation(
                chart_id,
                ControlRule::SixPointTrend,
                MonitorSeverity::Advisory,
                point_indices,
                point,
                format!(
                    "{} has a six point {} trend ending at {:.3} {}",
                    metric.replace('_', " "),
                    direction,
                    point.value,
                    unit
                ),
            ));
            index += 6;
        } else {
            index += 1;
        }
    }
}

fn append_one_side_violations(
    chart_id: &ChartId,
    metric: &str,
    unit: &str,
    limits: &ControlLimits,
    points: &[ControlPoint],
    violations: &mut Vec<RuleViolation>,
) {
    let mut index = 0;
    while index + 8 <= points.len() {
        let window = &points[index..index + 8];
        let above = window.iter().all(|point| point.value > limits.center);
        let below = window.iter().all(|point| point.value < limits.center);
        if above || below {
            let point_indices = (index..index + 8).collect::<Vec<_>>();
            let point = &points[index + 7];
            let side = if above { "above" } else { "below" };
            violations.push(rule_violation(
                chart_id,
                ControlRule::EightOnOneSide,
                MonitorSeverity::Warning,
                point_indices,
                point,
                format!(
                    "{} has eight consecutive points {} center {:.3} {}",
                    metric.replace('_', " "),
                    side,
                    limits.center,
                    unit
                ),
            ));
            index += 8;
        } else {
            index += 1;
        }
    }
}

fn rule_violation(
    chart_id: &ChartId,
    rule: ControlRule,
    severity: MonitorSeverity,
    point_indices: Vec<usize>,
    representative: &ControlPoint,
    message: String,
) -> RuleViolation {
    let last_index = point_indices.last().copied().unwrap_or_else(|| {
        warn!("SPC rule violation has no point indices; using index 0");
        0
    }) + 1;
    RuleViolation {
        id: format!(
            "{}:{}:{last_index}",
            chart_id.as_str(),
            rule.label().replace(' ', "_")
        ),
        chart_id: chart_id.clone(),
        rule,
        severity,
        message,
        point_indices,
        lot_id: Some(representative.lot_id.clone()),
        wafer_id: Some(representative.wafer_id.clone()),
        tool_id: None,
        recipe_id: Some(representative.recipe_id.clone()),
    }
}

fn default_sensor_limit(sensor_name: &str) -> SensorLimit {
    match sensor_name {
        "chuck_rpm" => SensorLimit::new(Some(0.0), Some(4_500.0)),
        "exhaust_pressure" => SensorLimit::new(Some(-130.0), Some(-100.0)),
        "surface_temp" => SensorLimit::new(Some(20.0), Some(125.0)),
        "zone_delta" => SensorLimit::new(Some(0.0), Some(1.5)),
        "alignment_error" => SensorLimit::new(Some(0.0), Some(3.0)),
        "lamp_power" => SensorLimit::new(Some(0.0), Some(16.0)),
        "chamber_pressure" => SensorLimit::new(Some(1.0), Some(120.0)),
        "rf_power" => SensorLimit::new(Some(0.0), Some(220.0)),
        "endpoint_signal" => SensorLimit::new(Some(0.0), Some(100.0)),
        "focus_z" => SensorLimit::new(Some(51.0), Some(53.0)),
        "illumination" => SensorLimit::new(Some(35.0), Some(85.0)),
        "contact_resistance" => SensorLimit::new(Some(0.0), Some(1.2)),
        "chuck_temp" => SensorLimit::new(Some(20.0), Some(30.0)),
        _ => SensorLimit::new(None, None),
    }
}

fn sensor_violation_message(
    sensor_name: &str,
    value: f64,
    unit: &str,
    limit: &SensorLimit,
) -> String {
    match (limit.lower, limit.upper) {
        (Some(lower), Some(upper)) => format!(
            "{} {:.3} {} outside {:.3}..{:.3} {}",
            sensor_name.replace('_', " "),
            value,
            unit,
            lower,
            upper,
            unit
        ),
        (Some(lower), None) => format!(
            "{} {:.3} {} below lower limit {:.3} {}",
            sensor_name.replace('_', " "),
            value,
            unit,
            lower,
            unit
        ),
        (None, Some(upper)) => format!(
            "{} {:.3} {} above upper limit {:.3} {}",
            sensor_name.replace('_', " "),
            value,
            unit,
            upper,
            unit
        ),
        (None, None) => format!("{} {:.3} {}", sensor_name.replace('_', " "), value, unit),
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn sample_stddev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 1.0;
    }
    let center = mean(values);
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - center;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() - 1) as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equipment::ToolId;

    fn measurement(id: &str, value: f64) -> ProcessMeasurement {
        ProcessMeasurement {
            measurement_id: id.to_string(),
            lot_id: "L-TEST".to_string(),
            wafer_id: id.to_string(),
            step_id: "MET_TEST".to_string(),
            tool_run_id: format!("RUN-{id}"),
            recipe_id: "ETCH_TEST".to_string(),
            process_layer: None,
            name: "critical_dimension".to_string(),
            unit: "nm".to_string(),
            value,
            target: Some(10.0),
            lower_spec: Some(7.0),
            upper_spec: Some(13.0),
        }
    }

    #[test]
    fn process_measurements_create_control_limit_violation() {
        let charts = control_charts_for_measurements(&[
            measurement("W01", 10.0),
            measurement("W02", 10.4),
            measurement("W03", 14.2),
        ]);

        assert_eq!(charts.len(), 1);
        assert_eq!(charts[0].limits.center, 10.0);
        assert!(charts[0].violations.iter().any(|violation| violation.rule
            == ControlRule::OutsideControlLimit
            && violation.severity == MonitorSeverity::Critical));
    }

    #[test]
    fn same_metric_with_different_units_gets_distinct_chart_ids() {
        let mut micron_measurement = measurement("W02", 10.0);
        micron_measurement.unit = "um".to_string();
        let monitor = SpcFdcMonitor::from_fab_context(
            &[measurement("W01", 10.0), micron_measurement],
            &[],
            &[],
        );
        let chart_ids = monitor
            .charts
            .iter()
            .map(|chart| chart.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(monitor.charts.len(), 2);
        assert_eq!(chart_ids.len(), 2);
        assert!(chart_ids.contains("spc:critical_dimension:nm"));
        assert!(chart_ids.contains("spc:critical_dimension:um"));
        assert_eq!(monitor.validate(), Vec::new());
    }

    #[test]
    fn eight_points_on_one_side_are_reported() {
        let measurements = (1..=8)
            .map(|slot| measurement(&format!("W{slot:02}"), 10.1 + slot as f64 * 0.03))
            .collect::<Vec<_>>();

        let charts = control_charts_for_measurements(&measurements);

        assert!(
            charts[0]
                .violations
                .iter()
                .any(|violation| violation.rule == ControlRule::EightOnOneSide
                    && violation.severity == MonitorSeverity::Warning)
        );
    }

    #[test]
    fn sensor_trace_flags_out_of_band_samples() {
        let samples = vec![
            SensorSample {
                tool_id: ToolId::new("ETCH-T"),
                at_s: 1,
                name: "chamber_pressure".to_string(),
                value: 82.0,
                unit: "mTorr".to_string(),
            },
            SensorSample {
                tool_id: ToolId::new("ETCH-T"),
                at_s: 2,
                name: "chamber_pressure".to_string(),
                value: 140.0,
                unit: "mTorr".to_string(),
            },
        ];

        let traces = sensor_traces_for_samples(&samples);

        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].violations.len(), 1);
        assert_eq!(traces[0].violations[0].severity, MonitorSeverity::Warning);
    }

    #[test]
    fn monitor_combines_spc_fdc_and_active_alarm_findings() {
        let measurements = vec![measurement("W01", 14.0)];
        let samples = vec![SensorSample {
            tool_id: ToolId::new("PROBE-T"),
            at_s: 1,
            name: "contact_resistance".to_string(),
            value: 1.8,
            unit: "ohm".to_string(),
        }];
        let alarms = vec![Alarm {
            id: "A1".to_string(),
            tool_id: ToolId::new("ETCH-T"),
            code: "VAC".to_string(),
            message: "vacuum warning".to_string(),
            severity: AlarmSeverity::Warning,
            active: true,
            occurred_at_s: 5,
            cleared_at_s: None,
        }];

        let monitor = SpcFdcMonitor::from_fab_context(&measurements, &samples, &alarms);

        assert_eq!(monitor.validate(), Vec::new());
        assert_eq!(monitor.alarm_summary.active_count, 1);
        assert!(
            monitor
                .findings
                .iter()
                .any(|finding| finding.source == FindingSource::SpcRule)
        );
        assert!(
            monitor
                .findings
                .iter()
                .any(|finding| finding.source == FindingSource::FdcTrace)
        );
        assert!(
            monitor
                .findings
                .iter()
                .any(|finding| finding.source == FindingSource::EquipmentAlarm)
        );
    }

    #[test]
    fn monitor_validation_rejects_stale_summaries_and_links() {
        let measurements = vec![measurement("W01", 14.0)];
        let samples = vec![SensorSample {
            tool_id: ToolId::new("PROBE-T"),
            at_s: 1,
            name: "contact_resistance".to_string(),
            value: 1.8,
            unit: "ohm".to_string(),
        }];
        let alarms = vec![Alarm {
            id: "A1".to_string(),
            tool_id: ToolId::new("ETCH-T"),
            code: "VAC".to_string(),
            message: "vacuum warning".to_string(),
            severity: AlarmSeverity::Warning,
            active: true,
            occurred_at_s: 5,
            cleared_at_s: None,
        }];
        let mut monitor = SpcFdcMonitor::from_fab_context(&measurements, &samples, &alarms);

        monitor.charts[0].limits.lower_control = 20.0;
        monitor.charts[0].limits.upper_control = 10.0;
        monitor.charts[0].points[0].sequence = 99;
        monitor.charts[0].violations[0].point_indices.push(99);
        monitor.traces[0].samples.push(SensorTracePoint {
            at_s: 0,
            value: f64::NAN,
        });
        monitor.traces[0].violations[0].limit = SensorLimit::new(None, None);
        monitor.alarm_summary.active_count = 2;
        monitor.findings.pop();

        let findings = monitor.validate();

        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("lower control is above upper control")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("has sequence 99")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("missing point index 99")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("samples are not sorted")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("non-finite value")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("uses stale limits")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("severity count")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("alarm findings")),
            "{findings:?}"
        );
    }
}
