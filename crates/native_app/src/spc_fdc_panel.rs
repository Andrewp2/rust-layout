use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Stroke, vec2};
use layout_model::{
    equipment::{Alarm, EquipmentSimulator, SensorSample},
    spc_fdc::{ControlChart, MonitorFinding, MonitorSeverity, SensorTrace, SpcFdcMonitor},
    yield_analysis::YieldAnalysis,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct SpcFdcPanel {
    selected_chart: String,
    selected_trace: String,
}

impl SpcFdcPanel {
    pub(crate) fn new() -> Self {
        Self {
            selected_chart: String::new(),
            selected_trace: String::new(),
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

        egui::ScrollArea::vertical()
            .id_salt("spc_fdc_dashboard")
            .show(ui, |ui| {
                let detail = format!("{} active finding(s)", monitor.findings.len());
                ui_chrome::module_header(ui, "Process monitor", "SPC / FDC", &detail, |_| {});

                monitor_metrics_ui(ui, &monitor);

                ui.separator();
                ui.columns(2, |columns| {
                    self.spc_section(&mut columns[0], &monitor);
                    self.fdc_section(&mut columns[1], &monitor);
                });

                ui.separator();
                findings_ui(ui, &monitor.findings);
            });
    }

    pub(crate) fn context_ui(
        &mut self,
        ui: &mut egui::Ui,
        analysis: &YieldAnalysis,
        equipment: &EquipmentSimulator,
    ) {
        let monitor = monitor_from_fab_context(analysis, equipment);
        self.ensure_selection(&monitor);

        ui_chrome::section_label(ui, "SPC / FDC");
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
        ui_chrome::section_label(ui, "SPC Control Chart");
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

        if let Some(point) = chart.latest_point() {
            ui.label(format!(
                "Latest {} {} on {} {}",
                compact_number(point.value),
                chart.unit,
                point.lot_id,
                point.wafer_id
            ));
        }
        ui.label(format!(
            "Limits: {}..{} {}",
            compact_number(chart.limits.lower_control),
            compact_number(chart.limits.upper_control),
            chart.unit
        ));
        draw_chart(ui, chart);

        if chart.violations.is_empty() {
            ui_chrome::status_pill(ui, "No SPC rule violations", Tone::Success);
        } else {
            egui::Grid::new("spc_rule_violations")
                .striped(true)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    ui.strong("Rule");
                    ui.strong("Severity");
                    ui.end_row();
                    for violation in &chart.violations {
                        ui.label(violation.rule.label());
                        ui.colored_label(
                            severity_color(violation.severity),
                            violation.severity.label(),
                        );
                        ui.end_row();
                    }
                });
        }
    }

    fn fdc_section(&mut self, ui: &mut egui::Ui, monitor: &SpcFdcMonitor) {
        ui_chrome::section_label(ui, "FDC Sensor Trace");
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

        if let Some(point) = trace.latest_point() {
            ui.label(format!(
                "Latest {} {} at t+{}s",
                compact_number(point.value),
                trace.unit,
                point.at_s
            ));
        }
        ui.label(limit_label(trace));
        draw_trace(ui, trace);

        if trace.violations.is_empty() {
            ui_chrome::status_pill(ui, "No FDC excursions", Tone::Success);
        } else {
            egui::Grid::new("fdc_violations")
                .striped(true)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    ui.strong("Time");
                    ui.strong("Value");
                    ui.strong("Finding");
                    ui.end_row();
                    for violation in &trace.violations {
                        ui.label(format!("{}s", violation.at_s));
                        ui.label(format!(
                            "{} {}",
                            compact_number(violation.value),
                            violation.unit
                        ));
                        ui.colored_label(
                            severity_color(violation.severity),
                            violation.severity.label(),
                        );
                        ui.end_row();
                    }
                });
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
    ui.horizontal_wrapped(|ui| {
        metric(
            ui,
            "Critical",
            monitor.finding_count_by_severity(MonitorSeverity::Critical),
        );
        metric(
            ui,
            "Warning",
            monitor.finding_count_by_severity(MonitorSeverity::Warning),
        );
        metric(ui, "SPC charts", monitor.charts.len());
        metric(ui, "FDC traces", monitor.traces.len());
        metric(ui, "Active alarms", monitor.alarm_summary.active_count);
    });
}

fn metric(ui: &mut egui::Ui, label: &str, value: usize) {
    let tone = match label {
        "Critical" if value > 0 => Tone::Danger,
        "Warning" if value > 0 => Tone::Warning,
        _ => Tone::Neutral,
    };
    ui_chrome::metric_tile_tone(ui, label, value, "", tone);
}

fn findings_ui(ui: &mut egui::Ui, findings: &[MonitorFinding]) {
    ui_chrome::section_label(ui, "Monitor Findings");
    if findings.is_empty() {
        ui_chrome::empty_state(ui, "No active SPC/FDC findings");
        return;
    }

    egui::Grid::new("spc_fdc_findings")
        .striped(true)
        .min_col_width(72.0)
        .show(ui, |ui| {
            ui.strong("Severity");
            ui.strong("Source");
            ui.strong("Finding");
            ui.strong("Context");
            ui.end_row();
            for finding in findings {
                ui.colored_label(severity_color(finding.severity), finding.severity.label());
                ui.label(finding.source.label());
                ui.label(&finding.title);
                ui.label(finding_context(finding));
                ui.end_row();
            }
        });
}

fn finding_row(ui: &mut egui::Ui, finding: &MonitorFinding) {
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(severity_color(finding.severity), finding.severity.label());
        ui.label(&finding.title);
    });
}

fn finding_context(finding: &MonitorFinding) -> String {
    [
        finding.tool_id.as_deref(),
        finding.lot_id.as_deref(),
        finding.wafer_id.as_deref(),
        finding.recipe_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" / ")
}

fn draw_chart(ui: &mut egui::Ui, chart: &ControlChart) {
    let values = chart
        .points
        .iter()
        .map(|point| {
            (
                point.sequence as f32,
                point.value,
                point.value > chart.limits.upper_control
                    || point.value < chart.limits.lower_control,
            )
        })
        .collect::<Vec<_>>();
    draw_plot(
        ui,
        &values,
        Some((chart.limits.lower_control, chart.limits.upper_control)),
    );
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
    let limits = match (trace.limit.lower, trace.limit.upper) {
        (Some(lower), Some(upper)) => Some((lower, upper)),
        _ => None,
    };
    draw_plot(ui, &values, limits);
}

fn draw_plot(ui: &mut egui::Ui, values: &[(f32, f64, bool)], limits: Option<(f64, f64)>) {
    let (rect, _) = ui.allocate_exact_size(ui_chrome::stable_plot_size(ui, 190.0), Sense::hover());
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
    if let Some((lower, upper)) = limits {
        min_value = min_value.min(lower);
        max_value = max_value.max(upper);
    }
    let padding = ((max_value - min_value) * 0.12).max(0.5);
    min_value -= padding;
    max_value += padding;

    let min_x = values.first().map(|point| point.0).unwrap_or(0.0);
    let max_x = values.last().map(|point| point.0).unwrap_or(min_x + 1.0);
    let plot = rect.shrink2(vec2(30.0, 18.0));
    let x_at = |x: f32| {
        let span = (max_x - min_x).max(1.0);
        plot.left() + ((x - min_x) / span).clamp(0.0, 1.0) * plot.width()
    };
    let y_at = |value: f64| {
        let span = (max_value - min_value).max(f64::EPSILON);
        let fraction = ((value - min_value) / span) as f32;
        plot.bottom() - fraction.clamp(0.0, 1.0) * plot.height()
    };

    if let Some((lower, upper)) = limits {
        let lower_y = y_at(lower);
        let upper_y = y_at(upper);
        painter.line_segment(
            [
                Pos2::new(plot.left(), lower_y),
                Pos2::new(plot.right(), lower_y),
            ],
            Stroke::new(1.0, Color32::from_rgb(130, 145, 160)),
        );
        painter.line_segment(
            [
                Pos2::new(plot.left(), upper_y),
                Pos2::new(plot.right(), upper_y),
            ],
            Stroke::new(1.0, Color32::from_rgb(130, 145, 160)),
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
    for (x, value, outside) in values {
        painter.circle_filled(
            Pos2::new(x_at(*x), y_at(*value)),
            4.0,
            if *outside {
                Color32::LIGHT_RED
            } else {
                Color32::LIGHT_GREEN
            },
        );
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
