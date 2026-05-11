use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, vec2};
use layout_model::{
    equipment::{Alarm, EquipmentSimulator, SensorSample},
    spc_fdc::{
        ControlChart, ControlRule, FindingSource, MonitorFinding, MonitorSeverity, RuleViolation,
        SensorLimit, SensorTrace, SensorViolation, SpcFdcMonitor,
    },
    yield_analysis::YieldAnalysis,
};
use operad::{
    ApproxTextMeasurer, ClipBehavior, ColorRgba, FontWeight, InputBehavior, StrokeStyle, TextStyle,
    TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiSize, UiVisual, layout, root_style,
    widgets,
};

use crate::{
    operad_egui,
    operad_sidecar::{SidecarRow, SidecarSection, render_sidecar},
    ui_chrome::{self, Tone},
};

const OPERAD_HEADER_HEIGHT: f32 = 104.0;
const OPERAD_METRIC_HEIGHT: f32 = 88.0;
const OPERAD_SECTION_TITLE_HEIGHT: f32 = 26.0;
const OPERAD_ROW_HEIGHT: f32 = 58.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_SELECT_CHART: &str = "spc_fdc.action.select_chart.";
const OPERAD_ACTION_SELECT_TRACE: &str = "spc_fdc.action.select_trace.";
const OPERAD_ACTION_SET_SEVERITY: &str = "spc_fdc.action.set_severity.";
const OPERAD_ACTION_SET_SOURCE: &str = "spc_fdc.action.set_source.";
const OPERAD_ACTION_CLEAR_CONTEXT: &str = "spc_fdc.action.clear_context";

pub(crate) struct SpcFdcPanel {
    selected_chart: String,
    selected_trace: String,
    severity_filter: SeverityFilter,
    source_filter: SourceFilter,
    context_filter: String,
}

#[derive(Debug)]
struct SpcFdcOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct SpcFdcMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct SpcFdcOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeverityFilter {
    All,
    Critical,
    Warning,
    Advisory,
}

impl SeverityFilter {
    const ALL: [Self; 4] = [Self::All, Self::Critical, Self::Warning, Self::Advisory];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All severities",
            Self::Critical => "Critical",
            Self::Warning => "Warning",
            Self::Advisory => "Advisory",
        }
    }

    fn matches(self, severity: MonitorSeverity) -> bool {
        match self {
            Self::All => true,
            Self::Critical => severity == MonitorSeverity::Critical,
            Self::Warning => severity == MonitorSeverity::Warning,
            Self::Advisory => severity == MonitorSeverity::Advisory,
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Advisory => "advisory",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "all" => Some(Self::All),
            "critical" => Some(Self::Critical),
            "warning" => Some(Self::Warning),
            "advisory" => Some(Self::Advisory),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceFilter {
    All,
    Spc,
    Fdc,
    Alarm,
}

impl SourceFilter {
    const ALL: [Self; 4] = [Self::All, Self::Spc, Self::Fdc, Self::Alarm];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All sources",
            Self::Spc => "SPC",
            Self::Fdc => "FDC",
            Self::Alarm => "Alarm",
        }
    }

    fn matches(self, source: FindingSource) -> bool {
        match self {
            Self::All => true,
            Self::Spc => source == FindingSource::SpcRule,
            Self::Fdc => source == FindingSource::FdcTrace,
            Self::Alarm => source == FindingSource::EquipmentAlarm,
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Spc => "spc",
            Self::Fdc => "fdc",
            Self::Alarm => "alarm",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "all" => Some(Self::All),
            "spc" => Some(Self::Spc),
            "fdc" => Some(Self::Fdc),
            "alarm" => Some(Self::Alarm),
            _ => None,
        }
    }
}

impl SpcFdcPanel {
    pub(crate) fn new() -> Self {
        Self {
            selected_chart: String::new(),
            selected_trace: String::new(),
            severity_filter: SeverityFilter::All,
            source_filter: SourceFilter::All,
            context_filter: String::new(),
        }
    }

    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        analysis: &YieldAnalysis,
        equipment: &EquipmentSimulator,
    ) {
        let monitor = monitor_from_fab_context(analysis, equipment);
        self.ensure_selection(&monitor);
        if let Err(error) = self.operad_ui(ui, &monitor) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui, &monitor);
        }
    }

    fn operad_ui(&mut self, ui: &mut egui::Ui, monitor: &SpcFdcMonitor) -> Result<(), String> {
        let mut result = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("spc_fdc_dashboard_operad_scroll")
            .show(ui, |ui| {
                let width = ui.available_width().max(320.0);
                let mut view = self.build_operad_view(width, monitor);
                if let Err(error) = view
                    .document
                    .compute_layout(view.size, &mut ApproxTextMeasurer)
                    .map_err(|error| error.to_string())
                {
                    result = Err(error);
                    return;
                }

                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, view.size.height), Sense::click());
                if response.clicked()
                    && let Some(pointer) = response.interact_pointer_pos()
                    && let Some(node_name) =
                        operad_egui::hit_test_name(&view.document, rect, pointer)
                    && self.handle_operad_action(&node_name, monitor)
                {
                    view = self.build_operad_view(width, monitor);
                    if let Err(error) = view
                        .document
                        .compute_layout(view.size, &mut ApproxTextMeasurer)
                        .map_err(|error| error.to_string())
                    {
                        result = Err(error);
                        return;
                    }
                }

                if response.hovered()
                    && let Some(pointer) = ui.ctx().pointer_hover_pos()
                    && operad_egui::hit_test_name(&view.document, rect, pointer).is_some()
                {
                    ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::PointingHand);
                }
                operad_egui::paint_document_at(ui, &view.document, rect);
            });
        result
    }

    fn egui_dashboard_ui(&mut self, ui: &mut egui::Ui, monitor: &SpcFdcMonitor) {
        egui::ScrollArea::vertical()
            .id_salt("spc_fdc_dashboard")
            .show(ui, |ui| {
                let detail = format!(
                    "{} active finding(s) across {} chart(s), {} trace(s)",
                    monitor.findings.len(),
                    monitor.charts.len(),
                    monitor.traces.len()
                );
                ui_chrome::module_header(ui, "Process monitor", "SPC / FDC", &detail, |_| {});

                monitor_metrics_ui(ui, monitor);

                ui.separator();
                triage_summary_ui(ui, monitor);

                ui.separator();
                let available_width = ui.available_width();
                if available_width < 780.0 {
                    self.spc_section(ui, monitor);
                    ui.separator();
                    self.fdc_section(ui, monitor);
                    ui.separator();
                    self.context_section(ui, monitor);
                } else if available_width < 1120.0 {
                    ui.columns(2, |columns| {
                        self.spc_section(&mut columns[0], monitor);
                        self.fdc_section(&mut columns[1], monitor);
                    });
                    ui.separator();
                    self.context_section(ui, monitor);
                } else {
                    ui.columns(3, |columns| {
                        self.spc_section(&mut columns[0], monitor);
                        self.fdc_section(&mut columns[1], monitor);
                        self.context_section(&mut columns[2], monitor);
                    });
                }

                ui.separator();
                self.findings_ui(ui, &monitor.findings);
            });
    }

    fn build_operad_view(&self, width: f32, monitor: &SpcFdcMonitor) -> SpcFdcOperadView {
        let metrics = self.operad_metrics(monitor);
        let triage_rows = self.operad_triage_rows(monitor);
        let chart_rows = self.operad_chart_rows(monitor);
        let selected_chart_rows = self.operad_selected_chart_rows(monitor);
        let trace_rows = self.operad_trace_rows(monitor);
        let selected_trace_rows = self.operad_selected_trace_rows(monitor);
        let context_rows = self.operad_context_rows(monitor);
        let finding_rows = self.operad_finding_rows(monitor);
        let height = spc_fdc_operad_view_height(
            width,
            metrics.len(),
            &[
                triage_rows.len(),
                chart_rows.len(),
                selected_chart_rows.len(),
                trace_rows.len(),
                selected_trace_rows.len(),
                context_rows.len(),
                finding_rows.len(),
            ],
        );
        let size = UiSize::new(width, height);
        let mut document = UiDocument::new(root_style(width, height));
        let root = document.root;
        document.set_node_visual(
            root,
            UiVisual::panel(
                ColorRgba::new(15, 18, 21, 255),
                Some(StrokeStyle::new(ColorRgba::new(39, 46, 52, 255), 1.0)),
                0.0,
            ),
        );

        add_spc_fdc_operad_header(
            &mut document,
            root,
            "PROCESS MONITOR",
            "SPC / FDC",
            "Control charts, equipment traces, active alarms, and release triage",
            &format!(
                "{} active finding(s) across {} chart(s), {} trace(s)",
                monitor.findings.len(),
                monitor.charts.len(),
                monitor.traces.len()
            ),
        );
        add_spc_fdc_operad_spacer(&mut document, root, OPERAD_GAP);
        add_spc_fdc_operad_metric_grid(&mut document, root, width, &metrics);
        add_spc_fdc_operad_spacer(&mut document, root, OPERAD_GAP);
        add_spc_fdc_operad_section(
            &mut document,
            root,
            width,
            "spc_fdc.triage",
            "Actionable Triage",
            "No active SPC/FDC actions",
            &triage_rows,
        );
        add_spc_fdc_operad_spacer(&mut document, root, OPERAD_GAP);
        add_spc_fdc_operad_section(
            &mut document,
            root,
            width,
            "spc_fdc.charts",
            "SPC Control Charts",
            "No process measurements are available",
            &chart_rows,
        );
        add_spc_fdc_operad_spacer(&mut document, root, OPERAD_GAP);
        add_spc_fdc_operad_section(
            &mut document,
            root,
            width,
            "spc_fdc.selected_chart",
            "Selected Chart",
            "No SPC chart selected",
            &selected_chart_rows,
        );
        add_spc_fdc_operad_spacer(&mut document, root, OPERAD_GAP);
        add_spc_fdc_operad_section(
            &mut document,
            root,
            width,
            "spc_fdc.traces",
            "FDC Sensor Traces",
            "No equipment sensor samples are available",
            &trace_rows,
        );
        add_spc_fdc_operad_spacer(&mut document, root, OPERAD_GAP);
        add_spc_fdc_operad_section(
            &mut document,
            root,
            width,
            "spc_fdc.selected_trace",
            "Selected Trace",
            "No FDC trace selected",
            &selected_trace_rows,
        );
        add_spc_fdc_operad_spacer(&mut document, root, OPERAD_GAP);
        add_spc_fdc_operad_section(
            &mut document,
            root,
            width,
            "spc_fdc.context",
            "Sensor / Fault Context",
            "Select a sensor trace for tool context",
            &context_rows,
        );
        add_spc_fdc_operad_spacer(&mut document, root, OPERAD_GAP);
        add_spc_fdc_operad_section(
            &mut document,
            root,
            width,
            "spc_fdc.findings",
            "Monitor Findings",
            "No findings match the current filters",
            &finding_rows,
        );

        SpcFdcOperadView { document, size }
    }

    fn operad_metrics(&self, monitor: &SpcFdcMonitor) -> Vec<SpcFdcMetricTile> {
        let chart_violation_count = monitor
            .charts
            .iter()
            .map(|chart| chart.violations.len())
            .sum::<usize>();
        let excursion_count = monitor
            .traces
            .iter()
            .map(|trace| trace.violations.len())
            .sum::<usize>();
        vec![
            SpcFdcMetricTile {
                label: "Critical".to_string(),
                value: monitor
                    .finding_count_by_severity(MonitorSeverity::Critical)
                    .to_string(),
                detail: "release blockers".to_string(),
                tone: if monitor.finding_count_by_severity(MonitorSeverity::Critical) > 0 {
                    Tone::Danger
                } else {
                    Tone::Neutral
                },
            },
            SpcFdcMetricTile {
                label: "Warning".to_string(),
                value: monitor
                    .finding_count_by_severity(MonitorSeverity::Warning)
                    .to_string(),
                detail: "investigate".to_string(),
                tone: if monitor.finding_count_by_severity(MonitorSeverity::Warning) > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            },
            SpcFdcMetricTile {
                label: "SPC violations".to_string(),
                value: chart_violation_count.to_string(),
                detail: "rule hits".to_string(),
                tone: if chart_violation_count > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            },
            SpcFdcMetricTile {
                label: "FDC excursions".to_string(),
                value: excursion_count.to_string(),
                detail: "sensor samples".to_string(),
                tone: if excursion_count > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            },
            SpcFdcMetricTile {
                label: "Active alarms".to_string(),
                value: monitor.alarm_summary.active_count.to_string(),
                detail: "equipment tools".to_string(),
                tone: if monitor.alarm_summary.active_count > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            },
        ]
    }

    fn operad_triage_rows(&self, monitor: &SpcFdcMonitor) -> Vec<SpcFdcOperadRow> {
        let mut rows = vec![spc_fdc_operad_row(
            monitor_risk_label(monitor).to_string(),
            monitor
                .findings
                .first()
                .map(|finding| format!("{} · {}", finding.title, next_action_for_finding(finding)))
                .unwrap_or_else(|| {
                    "No active SPC rules, FDC excursions, or equipment alarms".to_string()
                }),
            monitor_tone(monitor),
            None,
            false,
        )];
        rows.extend(SeverityFilter::ALL.into_iter().map(|filter| {
            spc_fdc_operad_row(
                format!("Severity filter: {}", filter.label()),
                "Filter monitor findings".to_string(),
                if self.severity_filter == filter {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_SEVERITY}{}|triage",
                    filter.slug()
                )),
                self.severity_filter == filter,
            )
        }));
        rows.extend(SourceFilter::ALL.into_iter().map(|filter| {
            spc_fdc_operad_row(
                format!("Source filter: {}", filter.label()),
                "Filter by SPC, FDC, or equipment alarms".to_string(),
                if self.source_filter == filter {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_SOURCE}{}|triage",
                    filter.slug()
                )),
                self.source_filter == filter,
            )
        }));
        rows.push(spc_fdc_operad_row(
            if self.context_filter.trim().is_empty() {
                "Context search inactive".to_string()
            } else {
                format!("Context search: {}", self.context_filter.trim())
            },
            "Text filtering stays on the fallback egui path until Operad has full edit routing"
                .to_string(),
            Tone::Neutral,
            (!self.context_filter.trim().is_empty())
                .then(|| OPERAD_ACTION_CLEAR_CONTEXT.to_string()),
            false,
        ));
        rows
    }

    fn operad_chart_rows(&self, monitor: &SpcFdcMonitor) -> Vec<SpcFdcOperadRow> {
        monitor
            .charts
            .iter()
            .enumerate()
            .map(|(index, chart)| {
                let latest = chart.latest_point();
                spc_fdc_operad_row(
                    format!(
                        "{} · {} violation(s)",
                        chart.metric.replace('_', " "),
                        chart.violations.len()
                    ),
                    format!(
                        "{} point(s) · latest {} · {}",
                        chart.points.len(),
                        latest
                            .map(|point| format!(
                                "{} {} on {}",
                                compact_number(point.value),
                                chart.unit,
                                point.wafer_id
                            ))
                            .unwrap_or_else(|| "n/a".to_string()),
                        chart_trend_label(chart)
                    ),
                    chart
                        .highest_severity()
                        .map_or(Tone::Success, severity_tone),
                    Some(format!(
                        "{OPERAD_ACTION_SELECT_CHART}{}|chart.{index}",
                        chart.id.as_str()
                    )),
                    chart.id.as_str() == self.selected_chart,
                )
            })
            .collect()
    }

    fn operad_selected_chart_rows(&self, monitor: &SpcFdcMonitor) -> Vec<SpcFdcOperadRow> {
        let Some(chart) = monitor
            .charts
            .iter()
            .find(|chart| chart.id.as_str() == self.selected_chart)
        else {
            return Vec::new();
        };
        let latest = chart.latest_point();
        let status = latest
            .map(|point| {
                if chart.limits.contains(point.value) {
                    "inside control limits"
                } else {
                    "outside control limits"
                }
            })
            .unwrap_or("no samples");
        let mut rows = vec![
            spc_fdc_operad_row(
                chart.name.clone(),
                format!(
                    "{} · {} point(s), {} rule violation(s)",
                    status,
                    chart.points.len(),
                    chart.violations.len()
                ),
                chart
                    .highest_severity()
                    .map_or(Tone::Success, severity_tone),
                Some(format!(
                    "{OPERAD_ACTION_SELECT_CHART}{}|selected",
                    chart.id.as_str()
                )),
                true,
            ),
            spc_fdc_operad_row(
                "Center / sigma".to_string(),
                format!(
                    "{} / {} {}",
                    compact_number(chart.limits.center),
                    compact_number(chart.limits.one_sigma),
                    chart.unit
                ),
                Tone::Neutral,
                None,
                false,
            ),
            spc_fdc_operad_row(
                "Limits / capability".to_string(),
                format!(
                    "{}..{} {}, {}",
                    compact_number(chart.limits.lower_control),
                    compact_number(chart.limits.upper_control),
                    chart.unit,
                    chart_cpk(chart)
                        .map(|cpk| format!("Cpk {}", compact_number(cpk)))
                        .unwrap_or_else(|| "Cpk n/a".to_string())
                ),
                Tone::Neutral,
                None,
                false,
            ),
        ];
        for violation in chart.violations.iter().take(8) {
            rows.push(spc_fdc_operad_row(
                format!(
                    "{} · {}",
                    violation.rule.label(),
                    violation.severity.label()
                ),
                format!(
                    "{} · {} · {}",
                    violation_points_label(chart, violation),
                    violation_context(violation),
                    spc_action_for_violation(violation)
                ),
                severity_tone(violation.severity),
                None,
                false,
            ));
        }
        rows
    }

    fn operad_trace_rows(&self, monitor: &SpcFdcMonitor) -> Vec<SpcFdcOperadRow> {
        monitor
            .traces
            .iter()
            .enumerate()
            .map(|(index, trace)| {
                let latest = trace.latest_point();
                spc_fdc_operad_row(
                    format!(
                        "{} · {} excursion(s)",
                        trace.display_name(),
                        trace.violations.len()
                    ),
                    format!(
                        "{} sample(s) · latest {} · {} alarm(s)",
                        trace.samples.len(),
                        latest
                            .map(|point| format!(
                                "{} {} at t+{}s",
                                compact_number(point.value),
                                trace.unit,
                                point.at_s
                            ))
                            .unwrap_or_else(|| "n/a".to_string()),
                        same_tool_alarm_count(monitor, &trace.tool_id)
                    ),
                    trace_highest_severity(monitor, trace).map_or(Tone::Success, severity_tone),
                    Some(format!(
                        "{OPERAD_ACTION_SELECT_TRACE}{}|trace.{index}",
                        trace.id
                    )),
                    trace.id == self.selected_trace,
                )
            })
            .collect()
    }

    fn operad_selected_trace_rows(&self, monitor: &SpcFdcMonitor) -> Vec<SpcFdcOperadRow> {
        let Some(trace) = monitor
            .traces
            .iter()
            .find(|trace| trace.id == self.selected_trace)
        else {
            return Vec::new();
        };
        let latest = trace.latest_point();
        let latest_status = latest
            .map(|point| {
                if trace.limit.contains(point.value) {
                    "inside configured limits"
                } else {
                    "outside configured limits"
                }
            })
            .unwrap_or("no samples");
        let mut rows = vec![
            spc_fdc_operad_row(
                trace.display_name(),
                format!(
                    "{} · {} sample(s), {} excursion(s)",
                    latest_status,
                    trace.samples.len(),
                    trace.violations.len()
                ),
                trace_highest_severity(monitor, trace).map_or(Tone::Success, severity_tone),
                Some(format!("{OPERAD_ACTION_SELECT_TRACE}{}|selected", trace.id)),
                true,
            ),
            spc_fdc_operad_row(
                "Limits".to_string(),
                limit_label(trace),
                Tone::Neutral,
                None,
                false,
            ),
            spc_fdc_operad_row(
                "Observed range".to_string(),
                trace_range_label(trace),
                Tone::Neutral,
                None,
                false,
            ),
        ];
        for violation in trace.violations.iter().take(8) {
            rows.push(spc_fdc_operad_row(
                format!("t+{}s · {}", violation.at_s, violation.severity.label()),
                format!(
                    "{} {} · {} · {}",
                    compact_number(violation.value),
                    violation.unit,
                    sensor_delta_label(violation.value, &violation.limit, &violation.unit),
                    fdc_action_for_violation(violation)
                ),
                severity_tone(violation.severity),
                None,
                false,
            ));
        }
        rows
    }

    fn operad_context_rows(&self, monitor: &SpcFdcMonitor) -> Vec<SpcFdcOperadRow> {
        let mut rows = Vec::new();
        if let Some(trace) = monitor
            .traces
            .iter()
            .find(|trace| trace.id == self.selected_trace)
        {
            rows.push(spc_fdc_operad_row(
                format!("Tool context: {}", trace.tool_id),
                format!(
                    "{} · {} sample(s), {} active alarm finding(s)",
                    trace.sensor_name.replace('_', " "),
                    trace.samples.len(),
                    same_tool_alarm_count(monitor, &trace.tool_id)
                ),
                trace_highest_severity(monitor, trace).map_or(Tone::Neutral, severity_tone),
                Some(format!("{OPERAD_ACTION_SELECT_TRACE}{}|context", trace.id)),
                true,
            ));
            for finding in monitor
                .findings
                .iter()
                .filter(|finding| {
                    finding.source == FindingSource::EquipmentAlarm
                        && finding.tool_id.as_deref() == Some(trace.tool_id.as_str())
                })
                .take(4)
            {
                rows.push(spc_fdc_operad_row(
                    format!("Alarm · {}", finding.title),
                    format!("{} · {}", finding.detail, next_action_for_finding(finding)),
                    severity_tone(finding.severity),
                    None,
                    false,
                ));
            }
        }
        if monitor.alarm_summary.active_count == 0 {
            rows.push(spc_fdc_operad_row(
                "No active equipment alarms".to_string(),
                "Equipment alarm summary is clear".to_string(),
                Tone::Success,
                None,
                false,
            ));
        } else {
            rows.push(spc_fdc_operad_row(
                "Alarm summary".to_string(),
                format!(
                    "{} active alarm(s), latest {}",
                    monitor.alarm_summary.active_count,
                    monitor
                        .alarm_summary
                        .latest_alarm
                        .as_ref()
                        .map(|alarm| {
                            format!(
                                "{} {} at t+{}s",
                                alarm.tool_id, alarm.code, alarm.occurred_at_s
                            )
                        })
                        .unwrap_or_else(|| "n/a".to_string())
                ),
                Tone::Warning,
                None,
                false,
            ));
            for (tool_id, count) in monitor.alarm_summary.by_tool.iter().take(8) {
                rows.push(spc_fdc_operad_row(
                    format!("{tool_id} alarms"),
                    format!("{count} active alarm(s)"),
                    Tone::Warning,
                    None,
                    false,
                ));
            }
        }
        rows
    }

    fn operad_finding_rows(&self, monitor: &SpcFdcMonitor) -> Vec<SpcFdcOperadRow> {
        let filtered = monitor
            .findings
            .iter()
            .filter(|finding| self.finding_matches_filter(finding))
            .collect::<Vec<_>>();
        let total = filtered.len();
        let mut rows = filtered
            .into_iter()
            .take(28)
            .map(|finding| {
                spc_fdc_operad_row(
                    format!("{} · {}", finding.severity.label(), finding.title),
                    format!(
                        "{} · {} · {}",
                        finding.source.label(),
                        finding_context(finding),
                        next_action_for_finding(finding)
                    ),
                    severity_tone(finding.severity),
                    None,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if total > rows.len() {
            rows.push(spc_fdc_operad_row(
                format!("Showing first {} of {total} findings", rows.len()),
                "Narrow severity, source, or context filters to reduce this list".to_string(),
                Tone::Neutral,
                None,
                false,
            ));
        }
        rows
    }

    fn handle_operad_action(&mut self, node_name: &str, monitor: &SpcFdcMonitor) -> bool {
        if node_name == OPERAD_ACTION_CLEAR_CONTEXT {
            self.context_filter.clear();
            return true;
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_SEVERITY) {
            let slug = value.split_once('|').map(|(slug, _)| slug).unwrap_or(value);
            if let Some(filter) = SeverityFilter::from_slug(slug) {
                self.severity_filter = filter;
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_SOURCE) {
            let slug = value.split_once('|').map(|(slug, _)| slug).unwrap_or(value);
            if let Some(filter) = SourceFilter::from_slug(slug) {
                self.source_filter = filter;
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SELECT_CHART) {
            let chart_id = value.split_once('|').map(|(id, _)| id).unwrap_or(value);
            if monitor
                .charts
                .iter()
                .any(|chart| chart.id.as_str() == chart_id)
            {
                self.selected_chart = chart_id.to_string();
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SELECT_TRACE) {
            let trace_id = value.split_once('|').map(|(id, _)| id).unwrap_or(value);
            if monitor.traces.iter().any(|trace| trace.id == trace_id) {
                self.selected_trace = trace_id.to_string();
                return true;
            }
        }
        false
    }

    pub(crate) fn context_ui(
        &mut self,
        ui: &mut egui::Ui,
        analysis: &YieldAnalysis,
        equipment: &EquipmentSimulator,
    ) {
        let monitor = monitor_from_fab_context(analysis, equipment);
        self.ensure_selection(&monitor);

        if self.operad_context_ui(ui, &monitor).is_err() {
            self.egui_context_ui(ui, &monitor);
        }
    }

    fn operad_context_ui(
        &mut self,
        ui: &mut egui::Ui,
        monitor: &SpcFdcMonitor,
    ) -> Result<(), String> {
        let overview = SidecarSection::new("SPC / FDC")
            .row(SidecarRow::new(
                monitor_risk_label(monitor),
                format!("{} finding(s)", monitor.findings.len()),
                monitor_tone(monitor),
            ))
            .row(SidecarRow::new(
                "Signals",
                format!(
                    "{} SPC charts / {} FDC traces / {} active alarms",
                    monitor.charts.len(),
                    monitor.traces.len(),
                    monitor.alarm_summary.active_count
                ),
                Tone::Info,
            ))
            .row(SidecarRow::new(
                "Critical",
                format!(
                    "{} critical findings",
                    monitor.finding_count_by_severity(MonitorSeverity::Critical)
                ),
                if monitor.finding_count_by_severity(MonitorSeverity::Critical) > 0 {
                    Tone::Danger
                } else {
                    Tone::Neutral
                },
            ))
            .row(SidecarRow::new(
                "Warning",
                format!(
                    "{} warning findings",
                    monitor.finding_count_by_severity(MonitorSeverity::Warning)
                ),
                if monitor.finding_count_by_severity(MonitorSeverity::Warning) > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            ))
            .row(SidecarRow::new(
                "Advisory",
                format!(
                    "{} advisory findings",
                    monitor.finding_count_by_severity(MonitorSeverity::Advisory)
                ),
                if monitor.finding_count_by_severity(MonitorSeverity::Advisory) > 0 {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
            ));

        let top_action = if let Some(finding) = monitor.findings.first() {
            SidecarSection::new("Top Action")
                .row(SidecarRow::new(
                    format!("{} · {}", finding.severity.label(), finding.title),
                    next_action_for_finding(finding),
                    severity_tone(finding.severity),
                ))
                .row(SidecarRow::new(
                    "Context",
                    finding_context(finding),
                    Tone::Neutral,
                ))
        } else {
            SidecarSection::new("Top Action").empty("No active SPC/FDC actions")
        };

        let mut latest = SidecarSection::new("Latest Findings").empty("No active SPC/FDC findings");
        for finding in monitor.findings.iter().take(6) {
            latest = latest.row(SidecarRow::new(
                format!("{} · {}", finding.severity.label(), finding.title),
                finding_context(finding),
                severity_tone(finding.severity),
            ));
        }

        render_sidecar(ui, "spc_fdc.context", &[overview, top_action, latest])
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui, monitor: &SpcFdcMonitor) {
        ui_chrome::section_label(ui, "SPC / FDC");
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, monitor_risk_label(monitor), monitor_tone(monitor));
            ui.label(format!("{} finding(s)", monitor.findings.len()));
        });
        ui.label(format!("SPC charts: {}", monitor.charts.len()));
        ui.label(format!("FDC traces: {}", monitor.traces.len()));
        ui.label(format!(
            "Active alarms: {}",
            monitor.alarm_summary.active_count
        ));
        ui.separator();
        ui.colored_label(
            severity_color(MonitorSeverity::Critical),
            format!(
                "{} critical",
                monitor.finding_count_by_severity(MonitorSeverity::Critical)
            ),
        );
        ui.colored_label(
            severity_color(MonitorSeverity::Warning),
            format!(
                "{} warning",
                monitor.finding_count_by_severity(MonitorSeverity::Warning)
            ),
        );
        ui.colored_label(
            severity_color(MonitorSeverity::Advisory),
            format!(
                "{} advisory",
                monitor.finding_count_by_severity(MonitorSeverity::Advisory)
            ),
        );

        ui.separator();
        ui_chrome::section_label(ui, "Top Action");
        if let Some(finding) = monitor.findings.first() {
            finding_row(ui, finding);
            ui_chrome::muted(ui, next_action_for_finding(finding));
        } else {
            ui_chrome::empty_state(ui, "No active SPC/FDC actions");
        }

        ui.separator();
        ui_chrome::section_label(ui, "Latest Findings");
        for finding in monitor.findings.iter().take(6) {
            finding_row(ui, finding);
        }
        if monitor.findings.is_empty() {
            ui_chrome::empty_state(ui, "No active SPC/FDC findings");
        }
    }

    fn ensure_selection(&mut self, monitor: &SpcFdcMonitor) {
        let chart_valid = monitor
            .charts
            .iter()
            .any(|chart| chart.id.as_str() == self.selected_chart);
        if !chart_valid {
            self.selected_chart = monitor
                .charts
                .first()
                .map(|chart| chart.id.to_string())
                .unwrap_or_default();
        }

        let trace_valid = monitor
            .traces
            .iter()
            .any(|trace| trace.id == self.selected_trace);
        if !trace_valid {
            self.selected_trace = monitor
                .traces
                .first()
                .map(|trace| trace.id.clone())
                .unwrap_or_default();
        }
    }

    fn spc_section(&mut self, ui: &mut egui::Ui, monitor: &SpcFdcMonitor) {
        ui_chrome::section_label(ui, "SPC Control Charts");
        if monitor.charts.is_empty() {
            ui_chrome::empty_state(ui, "No process measurements are available");
            return;
        }

        ui.horizontal_wrapped(|ui| {
            for chart in &monitor.charts {
                let selected = chart.id.as_str() == self.selected_chart;
                let label = format!(
                    "{} ({})",
                    chart.metric.replace('_', " "),
                    chart.violations.len()
                );
                let color = chart
                    .highest_severity()
                    .map(severity_color)
                    .unwrap_or_else(|| ui.visuals().weak_text_color());
                let response = ui.selectable_label(selected, RichText::new(label).color(color));
                if response.clicked() {
                    self.selected_chart = chart.id.to_string();
                }
                response.on_hover_text(chart_hover_text(chart));
            }
        });

        let selected_text = monitor
            .charts
            .iter()
            .find(|chart| chart.id.as_str() == self.selected_chart)
            .map(|chart| chart.name.clone())
            .unwrap_or_else(|| "No SPC chart".to_string());

        egui::ComboBox::from_id_salt("spc_chart_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for chart in &monitor.charts {
                    ui.selectable_value(
                        &mut self.selected_chart,
                        chart.id.to_string(),
                        chart.name.clone(),
                    );
                }
            });

        let Some(chart) = monitor
            .charts
            .iter()
            .find(|chart| chart.id.as_str() == self.selected_chart)
        else {
            ui_chrome::empty_state(ui, "No process measurements are available");
            return;
        };

        chart_summary_ui(ui, chart);
        draw_chart(ui, chart);
        spc_violation_table(ui, chart);
    }

    fn fdc_section(&mut self, ui: &mut egui::Ui, monitor: &SpcFdcMonitor) {
        ui_chrome::section_label(ui, "FDC Sensor Traces");
        if monitor.traces.is_empty() {
            ui_chrome::empty_state(ui, "No equipment sensor samples are available");
            return;
        }

        ui.horizontal_wrapped(|ui| {
            for trace in &monitor.traces {
                let selected = trace.id == self.selected_trace;
                let severity = trace_highest_severity(monitor, trace);
                let label = format!("{} ({})", trace.display_name(), trace.violations.len());
                let color = severity
                    .map(severity_color)
                    .unwrap_or_else(|| ui.visuals().weak_text_color());
                let response = ui.selectable_label(selected, RichText::new(label).color(color));
                if response.clicked() {
                    self.selected_trace = trace.id.clone();
                }
                response.on_hover_text(trace_hover_text(monitor, trace));
            }
        });

        let selected_text = monitor
            .traces
            .iter()
            .find(|trace| trace.id == self.selected_trace)
            .map(SensorTrace::display_name)
            .unwrap_or_else(|| "No FDC trace".to_string());

        egui::ComboBox::from_id_salt("fdc_trace_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for trace in &monitor.traces {
                    ui.selectable_value(
                        &mut self.selected_trace,
                        trace.id.clone(),
                        trace.display_name(),
                    );
                }
            });

        let Some(trace) = monitor
            .traces
            .iter()
            .find(|trace| trace.id == self.selected_trace)
        else {
            ui_chrome::empty_state(ui, "No equipment sensor samples are available");
            return;
        };

        trace_summary_ui(ui, monitor, trace);
        draw_trace(ui, trace);
        fdc_violation_table(ui, trace);
    }

    fn context_section(&self, ui: &mut egui::Ui, monitor: &SpcFdcMonitor) {
        ui_chrome::section_label(ui, "Sensor / Fault Context");

        let selected_trace = monitor
            .traces
            .iter()
            .find(|trace| trace.id == self.selected_trace);
        if let Some(trace) = selected_trace {
            ui.strong(trace.display_name());
            egui::Grid::new("selected_trace_fault_context")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui_chrome::muted(ui, "Tool");
                    ui.label(&trace.tool_id);
                    ui.end_row();

                    ui_chrome::muted(ui, "Sensor");
                    ui.label(trace.sensor_name.replace('_', " "));
                    ui.end_row();

                    ui_chrome::muted(ui, "Samples");
                    ui.label(trace.samples.len().to_string());
                    ui.end_row();

                    ui_chrome::muted(ui, "Tool alarms");
                    ui.label(same_tool_alarm_count(monitor, &trace.tool_id).to_string());
                    ui.end_row();
                });

            let tool_alarm_findings = monitor
                .findings
                .iter()
                .filter(|finding| {
                    finding.source == FindingSource::EquipmentAlarm
                        && finding.tool_id.as_deref() == Some(trace.tool_id.as_str())
                })
                .take(3)
                .collect::<Vec<_>>();
            if !tool_alarm_findings.is_empty() {
                ui.add_space(4.0);
                ui_chrome::muted(ui, "Correlated active alarms");
                for finding in tool_alarm_findings {
                    finding_row(ui, finding);
                }
            }
        } else {
            ui_chrome::empty_state(ui, "Select a sensor trace for tool context");
        }

        ui.add_space(8.0);
        ui_chrome::section_label(ui, "Alarm Summary");
        if monitor.alarm_summary.active_count == 0 {
            ui_chrome::status_pill(ui, "No active equipment alarms", Tone::Success);
        } else {
            ui.horizontal_wrapped(|ui| {
                for (severity, count) in &monitor.alarm_summary.by_severity {
                    ui_chrome::status_pill(
                        ui,
                        &format!("{count} {severity}"),
                        alarm_tone(severity),
                    );
                }
            });
            egui::Grid::new("spc_fdc_alarm_by_tool")
                .striped(true)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    ui.strong("Tool");
                    ui.strong("Active alarms");
                    ui.end_row();
                    for (tool_id, count) in &monitor.alarm_summary.by_tool {
                        ui.label(tool_id);
                        ui.label(count.to_string());
                        ui.end_row();
                    }
                });
        }

        if let Some(alarm) = &monitor.alarm_summary.latest_alarm {
            ui.add_space(4.0);
            ui_chrome::muted(
                ui,
                format!(
                    "Latest fault: {} {} at t+{}s",
                    alarm.tool_id, alarm.code, alarm.occurred_at_s
                ),
            );
        }
    }
}

fn monitor_from_fab_context(
    analysis: &YieldAnalysis,
    equipment: &EquipmentSimulator,
) -> SpcFdcMonitor {
    let sensor_samples = equipment
        .tools()
        .flat_map(|tool| tool.recent_sensors.iter().cloned())
        .collect::<Vec<SensorSample>>();
    let alarms = equipment
        .tools()
        .flat_map(|tool| tool.active_alarms.iter().cloned())
        .collect::<Vec<Alarm>>();
    SpcFdcMonitor::from_fab_context(&analysis.process_measurements, &sensor_samples, &alarms)
}

fn monitor_metrics_ui(ui: &mut egui::Ui, monitor: &SpcFdcMonitor) {
    let chart_violation_count = monitor
        .charts
        .iter()
        .map(|chart| chart.violations.len())
        .sum::<usize>();
    let excursion_count = monitor
        .traces
        .iter()
        .map(|trace| trace.violations.len())
        .sum::<usize>();

    ui_chrome::metric_tiles(
        ui,
        &[
            (
                "Critical",
                monitor
                    .finding_count_by_severity(MonitorSeverity::Critical)
                    .to_string(),
                "",
                if monitor.finding_count_by_severity(MonitorSeverity::Critical) > 0 {
                    Tone::Danger
                } else {
                    Tone::Neutral
                },
            ),
            (
                "Warning",
                monitor
                    .finding_count_by_severity(MonitorSeverity::Warning)
                    .to_string(),
                "",
                if monitor.finding_count_by_severity(MonitorSeverity::Warning) > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            ),
            (
                "SPC violations",
                chart_violation_count.to_string(),
                "rules",
                if chart_violation_count > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            ),
            (
                "FDC excursions",
                excursion_count.to_string(),
                "samples",
                if excursion_count > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            ),
            (
                "Active alarms",
                monitor.alarm_summary.active_count.to_string(),
                "tools",
                if monitor.alarm_summary.active_count > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            ),
        ],
    );
}

impl SpcFdcPanel {
    fn findings_ui(&mut self, ui: &mut egui::Ui, findings: &[MonitorFinding]) {
        ui_chrome::section_label(ui, "Monitor Findings");
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("spc_fdc_severity_filter")
                .selected_text(self.severity_filter.label())
                .show_ui(ui, |ui| {
                    for filter in SeverityFilter::ALL {
                        ui.selectable_value(&mut self.severity_filter, filter, filter.label());
                    }
                });

            egui::ComboBox::from_id_salt("spc_fdc_source_filter")
                .selected_text(self.source_filter.label())
                .show_ui(ui, |ui| {
                    for filter in SourceFilter::ALL {
                        ui.selectable_value(&mut self.source_filter, filter, filter.label());
                    }
                });

            ui.add_sized(
                [200.0, 22.0],
                egui::TextEdit::singleline(&mut self.context_filter)
                    .hint_text("lot / wafer / tool / recipe"),
            );
            if !self.context_filter.is_empty() && ui.button("Clear").clicked() {
                self.context_filter.clear();
            }
        });

        if findings.is_empty() {
            ui_chrome::empty_state(ui, "No active SPC/FDC findings");
            return;
        }

        let filtered = findings
            .iter()
            .filter(|finding| self.finding_matches_filter(finding))
            .collect::<Vec<_>>();
        ui_chrome::muted(
            ui,
            format!(
                "Showing {} of {} finding(s)",
                filtered.len(),
                findings.len()
            ),
        );
        if filtered.is_empty() {
            ui_chrome::empty_state(ui, "No findings match the current filters");
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("spc_fdc_findings_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("spc_fdc_findings")
                    .striped(true)
                    .min_col_width(72.0)
                    .show(ui, |ui| {
                        ui.strong("Severity");
                        ui.strong("Source");
                        ui.strong("Finding");
                        ui.strong("Detail");
                        ui.strong("Context");
                        ui.strong("Next action");
                        ui.end_row();
                        for finding in filtered {
                            ui.colored_label(
                                severity_color(finding.severity),
                                finding.severity.label(),
                            );
                            ui.label(finding.source.label());
                            ui.add(egui::Label::new(finding.title.as_str()).wrap());
                            ui.add(egui::Label::new(finding.detail.as_str()).wrap());
                            ui.label(finding_context(finding));
                            ui.add(egui::Label::new(next_action_for_finding(finding)).wrap());
                            ui.end_row();
                        }
                    });
            });
    }

    fn finding_matches_filter(&self, finding: &MonitorFinding) -> bool {
        if !self.severity_filter.matches(finding.severity)
            || !self.source_filter.matches(finding.source)
        {
            return false;
        }

        let needle = self.context_filter.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return true;
        }

        let haystack = format!(
            "{} {} {} {} {} {}",
            finding.title,
            finding.detail,
            finding_context(finding),
            finding.source.label(),
            finding.severity.label(),
            next_action_for_finding(finding)
        )
        .to_ascii_lowercase();
        haystack.contains(&needle)
    }
}

fn triage_summary_ui(ui: &mut egui::Ui, monitor: &SpcFdcMonitor) {
    ui_chrome::section_label(ui, "Actionable Triage");
    if monitor.findings.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, "Process in control", Tone::Success);
            ui_chrome::muted(
                ui,
                "No active SPC rules, FDC excursions, or equipment alarms",
            );
        });
        return;
    }

    let top = &monitor.findings[0];
    ui.horizontal_wrapped(|ui| {
        ui_chrome::status_pill(ui, monitor_risk_label(monitor), monitor_tone(monitor));
        ui.strong(&top.title);
    });
    ui.add(egui::Label::new(top.detail.as_str()).wrap());
    ui_chrome::muted(ui, next_action_for_finding(top));

    if monitor.findings.len() > 1 {
        ui.add_space(4.0);
        egui::Grid::new("spc_fdc_triage_queue")
            .striped(true)
            .min_col_width(76.0)
            .show(ui, |ui| {
                ui.strong("Priority");
                ui.strong("Context");
                ui.strong("Action");
                ui.end_row();
                for finding in monitor.findings.iter().skip(1).take(3) {
                    ui.colored_label(severity_color(finding.severity), finding.severity.label());
                    ui.label(finding_context(finding));
                    ui.add(egui::Label::new(next_action_for_finding(finding)).wrap());
                    ui.end_row();
                }
            });
    }
}

fn chart_summary_ui(ui: &mut egui::Ui, chart: &ControlChart) {
    let latest = chart.latest_point();
    let status = latest
        .map(|point| {
            if chart.limits.contains(point.value) {
                "inside control limits"
            } else {
                "outside control limits"
            }
        })
        .unwrap_or("no samples");
    let tone = chart
        .highest_severity()
        .map(severity_tone)
        .unwrap_or(Tone::Success);

    ui.horizontal_wrapped(|ui| {
        ui_chrome::status_pill(ui, status, tone);
        ui_chrome::muted(
            ui,
            format!(
                "{} point(s), {} rule violation(s)",
                chart.points.len(),
                chart.violations.len()
            ),
        );
    });

    egui::Grid::new(("spc_chart_summary", chart.id.as_str()))
        .num_columns(2)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            ui_chrome::muted(ui, "Latest");
            ui.label(
                latest
                    .map(|point| {
                        format!(
                            "{} {} on {} {}",
                            compact_number(point.value),
                            chart.unit,
                            point.lot_id,
                            point.wafer_id
                        )
                    })
                    .unwrap_or_else(|| "n/a".to_string()),
            );
            ui.end_row();

            ui_chrome::muted(ui, "Center / sigma");
            ui.label(format!(
                "{} / {} {}",
                compact_number(chart.limits.center),
                compact_number(chart.limits.one_sigma),
                chart.unit
            ));
            ui.end_row();

            ui_chrome::muted(ui, "Limits");
            ui.label(format!(
                "{}..{} {}",
                compact_number(chart.limits.lower_control),
                compact_number(chart.limits.upper_control),
                chart.unit
            ));
            ui.end_row();

            ui_chrome::muted(ui, "Capability");
            ui.label(
                chart_cpk(chart)
                    .map(|cpk| format!("Cpk {}", compact_number(cpk)))
                    .unwrap_or_else(|| "Cpk n/a".to_string()),
            );
            ui.end_row();

            ui_chrome::muted(ui, "Trend");
            ui.label(chart_trend_label(chart));
            ui.end_row();

            ui_chrome::muted(ui, "Recipe context");
            ui.label(
                latest
                    .map(|point| point.recipe_id.clone())
                    .unwrap_or_else(|| "n/a".to_string()),
            );
            ui.end_row();
        });
}

fn trace_summary_ui(ui: &mut egui::Ui, monitor: &SpcFdcMonitor, trace: &SensorTrace) {
    let latest = trace.latest_point();
    let latest_status = latest
        .map(|point| {
            if trace.limit.contains(point.value) {
                "inside configured limits"
            } else {
                "outside configured limits"
            }
        })
        .unwrap_or("no samples");
    let tone = trace_highest_severity(monitor, trace)
        .map(severity_tone)
        .unwrap_or(Tone::Success);

    ui.horizontal_wrapped(|ui| {
        ui_chrome::status_pill(ui, latest_status, tone);
        ui_chrome::muted(
            ui,
            format!(
                "{} sample(s), {} excursion(s)",
                trace.samples.len(),
                trace.violations.len()
            ),
        );
    });

    egui::Grid::new(("fdc_trace_summary", trace.id.as_str()))
        .num_columns(2)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            ui_chrome::muted(ui, "Latest");
            ui.label(
                latest
                    .map(|point| {
                        format!(
                            "{} {} at t+{}s",
                            compact_number(point.value),
                            trace.unit,
                            point.at_s
                        )
                    })
                    .unwrap_or_else(|| "n/a".to_string()),
            );
            ui.end_row();

            ui_chrome::muted(ui, "Limits");
            ui.label(limit_label(trace));
            ui.end_row();

            ui_chrome::muted(ui, "Observed range");
            ui.label(trace_range_label(trace));
            ui.end_row();

            ui_chrome::muted(ui, "Tool alarms");
            ui.label(same_tool_alarm_count(monitor, &trace.tool_id).to_string());
            ui.end_row();

            ui_chrome::muted(ui, "Last excursion");
            ui.label(
                trace
                    .violations
                    .last()
                    .map(|violation| {
                        format!(
                            "t+{}s, {}",
                            violation.at_s,
                            sensor_delta_label(violation.value, &violation.limit, &violation.unit)
                        )
                    })
                    .unwrap_or_else(|| "none".to_string()),
            );
            ui.end_row();
        });
}

fn spc_violation_table(ui: &mut egui::Ui, chart: &ControlChart) {
    if chart.violations.is_empty() {
        ui_chrome::status_pill(ui, "No SPC rule violations", Tone::Success);
        return;
    }

    ui.add_space(4.0);
    ui_chrome::muted(ui, "SPC rule violations");
    egui::ScrollArea::horizontal()
        .id_salt(("spc_rule_violations_scroll", chart.id.as_str()))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new(("spc_rule_violations", chart.id.as_str()))
                .striped(true)
                .min_col_width(76.0)
                .show(ui, |ui| {
                    ui.strong("Rule");
                    ui.strong("Severity");
                    ui.strong("Point(s)");
                    ui.strong("Context");
                    ui.strong("Next action");
                    ui.end_row();
                    for violation in &chart.violations {
                        ui.label(violation.rule.label());
                        ui.colored_label(
                            severity_color(violation.severity),
                            violation.severity.label(),
                        );
                        ui.label(violation_points_label(chart, violation));
                        ui.label(violation_context(violation));
                        ui.add(egui::Label::new(spc_action_for_violation(violation)).wrap());
                        ui.end_row();
                    }
                });
        });
}

fn fdc_violation_table(ui: &mut egui::Ui, trace: &SensorTrace) {
    if trace.violations.is_empty() {
        ui_chrome::status_pill(ui, "No FDC excursions", Tone::Success);
        return;
    }

    ui.add_space(4.0);
    ui_chrome::muted(ui, "FDC excursions");
    egui::ScrollArea::horizontal()
        .id_salt(("fdc_violations_scroll", trace.id.as_str()))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new(("fdc_violations", trace.id.as_str()))
                .striped(true)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    ui.strong("Time");
                    ui.strong("Value");
                    ui.strong("Delta");
                    ui.strong("Severity");
                    ui.strong("Next action");
                    ui.end_row();
                    for violation in &trace.violations {
                        ui.label(format!("t+{}s", violation.at_s));
                        ui.label(format!(
                            "{} {}",
                            compact_number(violation.value),
                            violation.unit
                        ));
                        ui.label(sensor_delta_label(
                            violation.value,
                            &violation.limit,
                            &violation.unit,
                        ));
                        ui.colored_label(
                            severity_color(violation.severity),
                            violation.severity.label(),
                        );
                        ui.add(egui::Label::new(fdc_action_for_violation(violation)).wrap());
                        ui.end_row();
                    }
                });
        });
}

fn finding_row(ui: &mut egui::Ui, finding: &MonitorFinding) {
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(severity_color(finding.severity), finding.severity.label());
        ui.label(&finding.title);
    });
}

fn finding_context(finding: &MonitorFinding) -> String {
    let context = [
        finding.tool_id.as_deref(),
        finding.lot_id.as_deref(),
        finding.wafer_id.as_deref(),
        finding.recipe_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" / ");
    if context.is_empty() {
        "n/a".to_string()
    } else {
        context
    }
}

fn monitor_tone(monitor: &SpcFdcMonitor) -> Tone {
    if monitor.finding_count_by_severity(MonitorSeverity::Critical) > 0 {
        Tone::Danger
    } else if monitor.finding_count_by_severity(MonitorSeverity::Warning) > 0 {
        Tone::Warning
    } else if monitor.finding_count_by_severity(MonitorSeverity::Advisory) > 0 {
        Tone::Info
    } else {
        Tone::Success
    }
}

fn monitor_risk_label(monitor: &SpcFdcMonitor) -> &'static str {
    if monitor.finding_count_by_severity(MonitorSeverity::Critical) > 0 {
        "Immediate containment"
    } else if monitor.finding_count_by_severity(MonitorSeverity::Warning) > 0 {
        "Investigate before release"
    } else if monitor.finding_count_by_severity(MonitorSeverity::Advisory) > 0 {
        "Watch trend"
    } else {
        "Stable"
    }
}

fn severity_tone(severity: MonitorSeverity) -> Tone {
    match severity {
        MonitorSeverity::Advisory => Tone::Info,
        MonitorSeverity::Warning => Tone::Warning,
        MonitorSeverity::Critical => Tone::Danger,
    }
}

fn alarm_tone(severity: &str) -> Tone {
    match severity {
        "Critical" => Tone::Danger,
        "Warning" => Tone::Warning,
        "Advisory" => Tone::Info,
        _ => Tone::Neutral,
    }
}

fn chart_hover_text(chart: &ControlChart) -> String {
    let latest = chart
        .latest_point()
        .map(|point| {
            format!(
                "Latest {} {} on {} {}",
                compact_number(point.value),
                chart.unit,
                point.lot_id,
                point.wafer_id
            )
        })
        .unwrap_or_else(|| "No samples".to_string());
    format!(
        "{}\n{} point(s)\n{} rule violation(s)\n{}",
        chart.name,
        chart.points.len(),
        chart.violations.len(),
        latest
    )
}

fn trace_hover_text(monitor: &SpcFdcMonitor, trace: &SensorTrace) -> String {
    let latest = trace
        .latest_point()
        .map(|point| {
            format!(
                "Latest {} {} at t+{}s",
                compact_number(point.value),
                trace.unit,
                point.at_s
            )
        })
        .unwrap_or_else(|| "No samples".to_string());
    format!(
        "{}\n{} sample(s)\n{} excursion(s)\n{} correlated alarm(s)\n{}",
        trace.display_name(),
        trace.samples.len(),
        trace.violations.len(),
        same_tool_alarm_count(monitor, &trace.tool_id),
        latest
    )
}

fn chart_cpk(chart: &ControlChart) -> Option<f64> {
    if chart.points.is_empty() || chart.limits.one_sigma <= 0.0 {
        return None;
    }
    let mean =
        chart.points.iter().map(|point| point.value).sum::<f64>() / chart.points.len() as f64;
    let lower = chart
        .limits
        .lower_spec
        .unwrap_or(chart.limits.lower_control);
    let upper = chart
        .limits
        .upper_spec
        .unwrap_or(chart.limits.upper_control);
    let denominator = 3.0 * chart.limits.one_sigma;
    Some(((upper - mean).min(mean - lower)) / denominator)
}

fn chart_trend_label(chart: &ControlChart) -> String {
    let Some(latest) = chart.points.last() else {
        return "n/a".to_string();
    };
    let Some(previous) = chart.points.iter().rev().nth(1) else {
        return "single point".to_string();
    };
    let delta = latest.value - previous.value;
    let direction = if delta.abs() <= chart.limits.one_sigma * 0.05 {
        "flat"
    } else if delta > 0.0 {
        "rising"
    } else {
        "falling"
    };
    let signed_delta = if delta >= 0.0 {
        format!("+{}", compact_number(delta))
    } else {
        compact_number(delta)
    };
    format!("{direction} ({signed_delta} {})", chart.unit)
}

fn trace_range_label(trace: &SensorTrace) -> String {
    let Some(first) = trace.samples.first() else {
        return "n/a".to_string();
    };
    let (min_value, max_value) = trace.samples.iter().fold(
        (first.value, first.value),
        |(min_value, max_value), sample| (min_value.min(sample.value), max_value.max(sample.value)),
    );
    format!(
        "{}..{} {}",
        compact_number(min_value),
        compact_number(max_value),
        trace.unit
    )
}

fn sensor_delta_label(value: f64, limit: &SensorLimit, unit: &str) -> String {
    if let Some(upper) = limit.upper
        && value > upper
    {
        return format!("+{} {} above upper", compact_number(value - upper), unit);
    }
    if let Some(lower) = limit.lower
        && value < lower
    {
        return format!("{} {} below lower", compact_number(value - lower), unit);
    }
    "within limits".to_string()
}

fn same_tool_alarm_count(monitor: &SpcFdcMonitor, tool_id: &str) -> usize {
    monitor
        .findings
        .iter()
        .filter(|finding| {
            finding.source == FindingSource::EquipmentAlarm
                && finding.tool_id.as_deref() == Some(tool_id)
        })
        .count()
}

fn trace_highest_severity(monitor: &SpcFdcMonitor, trace: &SensorTrace) -> Option<MonitorSeverity> {
    let trace_severities = trace.violations.iter().map(|violation| violation.severity);
    let alarm_severities = monitor
        .findings
        .iter()
        .filter(|finding| {
            finding.source == FindingSource::EquipmentAlarm
                && finding.tool_id.as_deref() == Some(trace.tool_id.as_str())
        })
        .map(|finding| finding.severity);
    trace_severities.chain(alarm_severities).max()
}

fn violation_points_label(chart: &ControlChart, violation: &RuleViolation) -> String {
    let points = violation
        .point_indices
        .iter()
        .filter_map(|index| chart.points.get(*index))
        .collect::<Vec<_>>();
    match points.as_slice() {
        [] => "n/a".to_string(),
        [point] => format!("#{} {}", point.sequence, point.wafer_id),
        [first, .., last] => format!(
            "#{}-#{} {} -> {}",
            first.sequence, last.sequence, first.wafer_id, last.wafer_id
        ),
    }
}

fn violation_context(violation: &RuleViolation) -> String {
    let context = [
        violation.tool_id.as_deref(),
        violation.lot_id.as_deref(),
        violation.wafer_id.as_deref(),
        violation.recipe_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" / ");
    if context.is_empty() {
        "n/a".to_string()
    } else {
        context
    }
}

fn spc_action_for_violation(violation: &RuleViolation) -> &'static str {
    match violation.rule {
        ControlRule::OutsideControlLimit => {
            "Hold affected lot, verify metrology, and block release until dispositioned."
        }
        ControlRule::TwoOfThreeBeyondTwoSigma => {
            "Compare the last three wafers against recipe/tool drift before release."
        }
        ControlRule::SixPointTrend => {
            "Check for recipe or tool drift and re-center before the next run."
        }
        ControlRule::EightOnOneSide => {
            "Review target calibration and re-center the process window."
        }
    }
}

fn fdc_action_for_violation(violation: &SensorViolation) -> &'static str {
    match violation.sensor_name.as_str() {
        "chamber_pressure" | "exhaust_pressure" => {
            "Check vacuum and exhaust path, then hold affected run until pressure recovers."
        }
        "surface_temp" | "zone_delta" | "chuck_temp" => {
            "Verify thermal control and requalify the tool before the next wafer."
        }
        "chuck_rpm" => "Inspect spindle speed control and recipe load before continuing.",
        "alignment_error" | "focus_z" | "illumination" | "lamp_power" => {
            "Requalify optics or alignment setup before exposing more wafers."
        }
        "rf_power" | "endpoint_signal" => {
            "Review plasma endpoint and RF stability before releasing the lot."
        }
        "contact_resistance" => "Reprobe affected wafer and clean probe contacts.",
        _ => "Review the trace around the excursion and disposition affected WIP.",
    }
}

fn next_action_for_finding(finding: &MonitorFinding) -> &'static str {
    match finding.source {
        FindingSource::SpcRule => match finding.severity {
            MonitorSeverity::Critical => {
                "Contain lot/wafer immediately and validate measurement before release."
            }
            MonitorSeverity::Warning => "Investigate drift and hold release if the trend persists.",
            MonitorSeverity::Advisory => "Watch the next run and confirm the process re-centers.",
        },
        FindingSource::FdcTrace => match finding.severity {
            MonitorSeverity::Critical => {
                "Stop the affected tool and inspect the failed sensor path."
            }
            MonitorSeverity::Warning => {
                "Check correlated tool state and rerun qualification if needed."
            }
            MonitorSeverity::Advisory => "Track the trace on the next run for recurrence.",
        },
        FindingSource::EquipmentAlarm => match finding.severity {
            MonitorSeverity::Critical => {
                "Stop WIP on the tool until the alarm is cleared and qualified."
            }
            MonitorSeverity::Warning => {
                "Clear or acknowledge the alarm before releasing affected WIP."
            }
            MonitorSeverity::Advisory => "Log the alarm context and watch for repeat faults.",
        },
    }
}

#[derive(Clone, Copy)]
struct PlotGuide {
    value: f64,
    label: &'static str,
    color: Color32,
    width: f32,
}

#[derive(Clone, Copy)]
struct PlotHighlight {
    start_x: f32,
    end_x: f32,
    severity: MonitorSeverity,
}

fn draw_chart(ui: &mut egui::Ui, chart: &ControlChart) {
    let values = chart
        .points
        .iter()
        .map(|point| {
            (
                point.sequence as f32,
                point.value,
                !chart.limits.contains(point.value),
            )
        })
        .collect::<Vec<_>>();

    let guide_color = Color32::from_rgb(130, 145, 160);
    let center_color = Color32::from_rgb(93, 168, 232);
    let two_sigma_color = Color32::from_rgb(220, 176, 72);
    let spec_color = Color32::from_rgb(226, 96, 96);
    let mut guides = vec![
        PlotGuide {
            value: chart.limits.upper_control,
            label: "UCL",
            color: guide_color,
            width: 1.2,
        },
        PlotGuide {
            value: chart.limits.lower_control,
            label: "LCL",
            color: guide_color,
            width: 1.2,
        },
        PlotGuide {
            value: chart.limits.center,
            label: "CL",
            color: center_color,
            width: 1.0,
        },
        PlotGuide {
            value: chart.limits.two_sigma_high(),
            label: "+2s",
            color: two_sigma_color,
            width: 0.8,
        },
        PlotGuide {
            value: chart.limits.two_sigma_low(),
            label: "-2s",
            color: two_sigma_color,
            width: 0.8,
        },
    ];
    if let Some(upper_spec) = chart.limits.upper_spec
        && (upper_spec - chart.limits.upper_control).abs() > f64::EPSILON
    {
        guides.push(PlotGuide {
            value: upper_spec,
            label: "USL",
            color: spec_color,
            width: 1.0,
        });
    }
    if let Some(lower_spec) = chart.limits.lower_spec
        && (lower_spec - chart.limits.lower_control).abs() > f64::EPSILON
    {
        guides.push(PlotGuide {
            value: lower_spec,
            label: "LSL",
            color: spec_color,
            width: 1.0,
        });
    }

    let highlights = chart
        .violations
        .iter()
        .filter_map(|violation| chart_highlight(chart, violation))
        .collect::<Vec<_>>();
    draw_plot(ui, &values, &guides, &highlights, 220.0);
}

fn draw_trace(ui: &mut egui::Ui, trace: &SensorTrace) {
    let values = trace
        .samples
        .iter()
        .map(|point| {
            let outside = !trace.limit.contains(point.value);
            (point.at_s as f32, point.value, outside)
        })
        .collect::<Vec<_>>();

    let guide_color = Color32::from_rgb(130, 145, 160);
    let mut guides = Vec::new();
    if let Some(lower) = trace.limit.lower {
        guides.push(PlotGuide {
            value: lower,
            label: "LOW",
            color: guide_color,
            width: 1.2,
        });
    }
    if let Some(upper) = trace.limit.upper {
        guides.push(PlotGuide {
            value: upper,
            label: "HIGH",
            color: guide_color,
            width: 1.2,
        });
    }

    let span = trace_sample_span(trace);
    let highlights = trace
        .violations
        .iter()
        .map(|violation| {
            let x = violation.at_s as f32;
            PlotHighlight {
                start_x: x - span * 0.35,
                end_x: x + span * 0.35,
                severity: violation.severity,
            }
        })
        .collect::<Vec<_>>();
    draw_plot(ui, &values, &guides, &highlights, 220.0);
}

fn draw_plot(
    ui: &mut egui::Ui,
    values: &[(f32, f64, bool)],
    guides: &[PlotGuide],
    highlights: &[PlotHighlight],
    height: f32,
) {
    let (rect, _) = ui.allocate_exact_size(ui_chrome::stable_plot_size(ui, height), Sense::hover());
    let painter = ui.painter_at(rect);
    ui_chrome::plot_background(ui, rect);

    if values.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "no samples",
            FontId::proportional(14.0),
            ui.visuals().weak_text_color(),
        );
        return;
    }

    let mut min_value = values
        .iter()
        .map(|(_, value, _)| *value)
        .fold(f64::INFINITY, f64::min);
    let mut max_value = values
        .iter()
        .map(|(_, value, _)| *value)
        .fold(f64::NEG_INFINITY, f64::max);
    for guide in guides {
        min_value = min_value.min(guide.value);
        max_value = max_value.max(guide.value);
    }
    let padding = ((max_value - min_value) * 0.12).max(0.5);
    min_value -= padding;
    max_value += padding;

    let min_x = values.first().map(|point| point.0).unwrap_or(0.0);
    let max_x = values.last().map(|point| point.0).unwrap_or(min_x + 1.0);
    let plot = rect.shrink2(vec2(38.0, 24.0));
    let x_at = |x: f32| {
        let span = (max_x - min_x).max(1.0);
        plot.left() + ((x - min_x) / span).clamp(0.0, 1.0) * plot.width()
    };
    let y_at = |value: f64| {
        let span = (max_value - min_value).max(f64::EPSILON);
        let fraction = ((value - min_value) / span) as f32;
        plot.bottom() - fraction.clamp(0.0, 1.0) * plot.height()
    };

    for highlight in highlights {
        let left = x_at(highlight.start_x);
        let right = x_at(highlight.end_x);
        let highlight_rect = Rect::from_min_max(
            Pos2::new(left.min(right), plot.top()),
            Pos2::new(left.max(right), plot.bottom()),
        );
        painter.rect_filled(
            highlight_rect,
            0.0,
            translucent(severity_color(highlight.severity), 38),
        );
    }

    let label_font = FontId::proportional(10.0);
    let weak_color = ui.visuals().weak_text_color();
    painter.text(
        Pos2::new(rect.left() + 6.0, plot.top()),
        Align2::LEFT_TOP,
        compact_number(max_value),
        label_font.clone(),
        weak_color,
    );
    painter.text(
        Pos2::new(rect.left() + 6.0, plot.bottom()),
        Align2::LEFT_BOTTOM,
        compact_number(min_value),
        label_font.clone(),
        weak_color,
    );
    painter.text(
        Pos2::new(plot.left(), rect.bottom() - 4.0),
        Align2::LEFT_BOTTOM,
        compact_axis(min_x),
        label_font.clone(),
        weak_color,
    );
    painter.text(
        Pos2::new(plot.right(), rect.bottom() - 4.0),
        Align2::RIGHT_BOTTOM,
        compact_axis(max_x),
        label_font.clone(),
        weak_color,
    );

    for guide in guides {
        let guide_y = y_at(guide.value);
        painter.line_segment(
            [
                Pos2::new(plot.left(), guide_y),
                Pos2::new(plot.right(), guide_y),
            ],
            Stroke::new(guide.width, guide.color),
        );
        painter.text(
            Pos2::new(plot.right() - 2.0, guide_y - 2.0),
            Align2::RIGHT_BOTTOM,
            guide.label,
            label_font.clone(),
            guide.color,
        );
    }

    for pair in values.windows(2) {
        let left = Pos2::new(x_at(pair[0].0), y_at(pair[0].1));
        let right = Pos2::new(x_at(pair[1].0), y_at(pair[1].1));
        painter.line_segment(
            [left, right],
            Stroke::new(2.0, Color32::from_rgb(82, 156, 219)),
        );
    }
    for (index, (x, value, outside)) in values.iter().enumerate() {
        let is_latest = index + 1 == values.len();
        painter.circle_filled(
            Pos2::new(x_at(*x), y_at(*value)),
            if is_latest { 5.0 } else { 3.8 },
            if *outside {
                Color32::LIGHT_RED
            } else {
                Color32::LIGHT_GREEN
            },
        );
    }

    if let Some((x, value, _)) = values.last() {
        let point = Pos2::new(x_at(*x), y_at(*value));
        let right_side = point.x > plot.center().x;
        painter.text(
            Pos2::new(
                if right_side {
                    point.x - 7.0
                } else {
                    point.x + 7.0
                },
                point.y,
            ),
            if right_side {
                Align2::RIGHT_CENTER
            } else {
                Align2::LEFT_CENTER
            },
            compact_number(*value),
            FontId::proportional(11.0),
            ui.visuals().text_color(),
        );
    }
}

fn chart_highlight(chart: &ControlChart, violation: &RuleViolation) -> Option<PlotHighlight> {
    let mut points = violation
        .point_indices
        .iter()
        .filter_map(|index| chart.points.get(*index));
    let first = points.next()?;
    let mut start_x = first.sequence as f32;
    let mut end_x = start_x;
    for point in points {
        let x = point.sequence as f32;
        start_x = start_x.min(x);
        end_x = end_x.max(x);
    }
    let padding = if (end_x - start_x).abs() < f32::EPSILON {
        0.35
    } else {
        0.2
    };
    Some(PlotHighlight {
        start_x: start_x - padding,
        end_x: end_x + padding,
        severity: violation.severity,
    })
}

fn trace_sample_span(trace: &SensorTrace) -> f32 {
    let span = trace
        .samples
        .windows(2)
        .filter_map(|pair| {
            let span = pair[1].at_s.saturating_sub(pair[0].at_s);
            (span > 0).then_some(span as f32)
        })
        .fold(f32::INFINITY, f32::min);
    if span.is_finite() { span.max(1.0) } else { 1.0 }
}

fn translucent(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn compact_axis(value: f32) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn limit_label(trace: &SensorTrace) -> String {
    match (trace.limit.lower, trace.limit.upper) {
        (Some(lower), Some(upper)) => format!(
            "Limits: {}..{} {}",
            compact_number(lower),
            compact_number(upper),
            trace.unit
        ),
        (Some(lower), None) => format!("Lower limit: {} {}", compact_number(lower), trace.unit),
        (None, Some(upper)) => format!("Upper limit: {} {}", compact_number(upper), trace.unit),
        (None, None) => "No configured limits".to_string(),
    }
}

fn severity_color(severity: MonitorSeverity) -> Color32 {
    match severity {
        MonitorSeverity::Advisory => Color32::LIGHT_BLUE,
        MonitorSeverity::Warning => Color32::YELLOW,
        MonitorSeverity::Critical => Color32::LIGHT_RED,
    }
}

fn compact_number(value: f64) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn spc_fdc_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += spc_fdc_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += spc_fdc_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn spc_fdc_operad_metric_columns(width: f32) -> usize {
    if width >= 1020.0 {
        4
    } else if width >= 680.0 {
        3
    } else if width >= 440.0 {
        2
    } else {
        1
    }
}

fn spc_fdc_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = spc_fdc_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn spc_fdc_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_spc_fdc_operad_header(
    document: &mut UiDocument,
    parent: UiNodeId,
    eyebrow: &str,
    title: &str,
    detail: &str,
    meta: &str,
) {
    let header = document.add_child(
        parent,
        UiNode::container(
            "spc_fdc.header",
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(OPERAD_HEADER_HEIGHT),
                    ),
                    OPERAD_PAD,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 27, 32, 255),
            Some(StrokeStyle::new(ColorRgba::new(46, 55, 64, 255), 1.0)),
            6.0,
        )),
    );
    add_spc_fdc_operad_text(
        document,
        header,
        "spc_fdc.header.eyebrow",
        eyebrow,
        spc_fdc_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_spc_fdc_operad_text(
        document,
        header,
        "spc_fdc.header.title",
        title,
        spc_fdc_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_spc_fdc_operad_text(
        document,
        header,
        "spc_fdc.header.detail",
        detail,
        spc_fdc_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_spc_fdc_operad_text(
        document,
        header,
        "spc_fdc.header.meta",
        meta,
        spc_fdc_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_spc_fdc_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[SpcFdcMetricTile],
) {
    let columns = spc_fdc_operad_metric_columns(width);
    let grid_height = spc_fdc_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "spc_fdc.metrics",
            UiNodeStyle {
                layout: layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(grid_height),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    let tile_width =
        ((width - OPERAD_GAP * (columns.saturating_sub(1) as f32)) / columns as f32).max(120.0);
    for (row_index, chunk) in metrics.chunks(columns).enumerate() {
        let row = document.add_child(
            grid,
            UiNode::container(
                format!("spc_fdc.metrics.row.{row_index}"),
                UiNodeStyle {
                    layout: layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(OPERAD_METRIC_HEIGHT),
                    ),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        for (column, metric) in chunk.iter().enumerate() {
            add_spc_fdc_operad_metric_tile(
                document,
                row,
                &format!("spc_fdc.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_spc_fdc_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &SpcFdcMetricTile,
) {
    let tile = document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_margin_all(
                        layout::with_size(
                            layout::column(),
                            layout::px(width.max(116.0)),
                            layout::px(OPERAD_METRIC_HEIGHT - 8.0),
                        ),
                        3.0,
                    ),
                    9.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(29, 35, 40, 255),
            Some(StrokeStyle::new(
                spc_fdc_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_spc_fdc_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        spc_fdc_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_spc_fdc_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        spc_fdc_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_spc_fdc_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        spc_fdc_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            spc_fdc_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_spc_fdc_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[SpcFdcOperadRow],
) {
    let height = spc_fdc_operad_section_height(rows.len());
    let section = document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::px(height)),
                    OPERAD_PAD,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(21, 26, 31, 255),
            Some(StrokeStyle::new(ColorRgba::new(45, 53, 61, 255), 1.0)),
            6.0,
        )),
    );
    add_spc_fdc_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        spc_fdc_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_spc_fdc_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_spc_fdc_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_spc_fdc_operad_empty_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    label: &str,
) {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.empty"),
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(OPERAD_EMPTY_ROW_HEIGHT),
                    ),
                    8.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(27, 32, 37, 255),
            Some(StrokeStyle::new(ColorRgba::new(43, 50, 58, 255), 1.0)),
            5.0,
        )),
    );
    add_spc_fdc_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        spc_fdc_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_spc_fdc_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &SpcFdcOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        spc_fdc_operad_tone_color(Tone::Info)
    } else {
        ColorRgba::new(42, 50, 58, 255)
    };
    let fill = if row.selected {
        ColorRgba::new(26, 42, 56, 255)
    } else {
        ColorRgba::new(26, 31, 36, 255)
    };
    let mut node = UiNode::container(
        row_name,
        UiNodeStyle {
            layout: layout::with_padding_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(OPERAD_ROW_HEIGHT),
                ),
                6.0,
            ),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(UiVisual::panel(
        fill,
        Some(StrokeStyle::new(stroke_color, 1.0)),
        4.0,
    ));
    if row.action_name.is_some() {
        node = node.with_input(InputBehavior::BUTTON).with_accessibility(
            crate::ui_chrome::operad_button_accessibility(&row.title, &row.detail),
        );
    }
    let row_node = document.add_child(parent, node);
    document.add_child(
        row_node,
        UiNode::container(
            format!("{section_name}.row.{index}.tone"),
            UiNodeStyle {
                layout: layout::fixed(5.0, OPERAD_ROW_HEIGHT - 12.0),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            spc_fdc_operad_tone_color(row.tone),
            None,
            2.0,
        )),
    );
    let text_column = document.add_child(
        row_node,
        UiNode::container(
            format!("{section_name}.row.{index}.text"),
            UiNodeStyle {
                layout: layout::with_size(
                    layout::column(),
                    layout::px((row_width - 28.0).max(120.0)),
                    layout::px(OPERAD_ROW_HEIGHT - 12.0),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    add_spc_fdc_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        spc_fdc_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_spc_fdc_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        spc_fdc_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        19.0,
    );
}

fn add_spc_fdc_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("spc_fdc.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_spc_fdc_operad_text(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    text: impl Into<String>,
    style: TextStyle,
    height: f32,
) {
    widgets::label(
        document,
        parent,
        name,
        text,
        style,
        layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
    );
}

fn spc_fdc_operad_text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn spc_fdc_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn spc_fdc_operad_row(
    title: impl Into<String>,
    detail: impl Into<String>,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
) -> SpcFdcOperadRow {
    SpcFdcOperadRow {
        title: title.into(),
        detail: detail.into(),
        tone,
        action_name,
        selected,
    }
}

fn truncate_middle(text: impl AsRef<str>, max_chars: usize) -> String {
    let text = text.as_ref();
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let head = keep / 2;
    let tail = keep - head;
    let start = text.chars().take(head).collect::<String>();
    let end = text
        .chars()
        .rev()
        .take(tail)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{start}...{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_monitor() -> SpcFdcMonitor {
        monitor_from_fab_context(&YieldAnalysis::synthetic(), &EquipmentSimulator::demo_fab())
    }

    #[test]
    fn spc_fdc_operad_view_audits_common_widths() {
        let monitor = sample_monitor();
        let mut panel = SpcFdcPanel::new();
        panel.ensure_selection(&monitor);
        for width in [360.0, 760.0, 1200.0] {
            let mut view = panel.build_operad_view(width, &monitor);
            view.document
                .compute_layout(view.size, &mut ApproxTextMeasurer)
                .unwrap();
            let warnings = view.document.audit_layout();
            assert!(warnings.is_empty(), "{warnings:#?}");
            assert!(view.document.node_count() > 20);
            assert!(!view.document.paint_list().items.is_empty());
        }
    }

    #[test]
    fn spc_fdc_operad_actions_update_panel_state() {
        let monitor = sample_monitor();
        let mut panel = SpcFdcPanel::new();
        panel.ensure_selection(&monitor);

        let target_chart = monitor.charts.last().unwrap().id.as_str().to_string();
        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_SELECT_CHART}{target_chart}|test"),
            &monitor,
        ));
        assert_eq!(panel.selected_chart, target_chart);

        let target_trace = monitor.traces.last().unwrap().id.clone();
        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_SELECT_TRACE}{target_trace}|test"),
            &monitor,
        ));
        assert_eq!(panel.selected_trace, target_trace);

        assert!(panel.handle_operad_action(
            &format!(
                "{OPERAD_ACTION_SET_SEVERITY}{}|test",
                SeverityFilter::Critical.slug()
            ),
            &monitor,
        ));
        assert_eq!(panel.severity_filter, SeverityFilter::Critical);

        assert!(panel.handle_operad_action(
            &format!(
                "{OPERAD_ACTION_SET_SOURCE}{}|test",
                SourceFilter::Fdc.slug()
            ),
            &monitor,
        ));
        assert_eq!(panel.source_filter, SourceFilter::Fdc);

        panel.context_filter = "COAT".to_string();
        assert!(panel.handle_operad_action(OPERAD_ACTION_CLEAR_CONTEXT, &monitor));
        assert!(panel.context_filter.is_empty());
    }
}
