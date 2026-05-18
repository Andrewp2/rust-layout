#![allow(unused_imports)]
use super::*;

pub(crate) fn process_flow_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let error_nodes = app
        .workspace
        .process_flow
        .findings()
        .into_iter()
        .filter(|finding| {
            finding.severity == layout_model::process_flow::ProcessFlowFindingSeverity::Error
        })
        .filter_map(|finding| finding.node_id)
        .collect::<BTreeSet<_>>();
    app.workspace
        .process_flow
        .route
        .nodes
        .iter()
        .filter(|node| process_flow_node_matches(&app.workspace, app.process_flow_filter, node))
        .filter(|node| !app.process_flow_errors_only || error_nodes.contains(&node.id))
        .take(10)
        .map(|node| {
            let recipe = node
                .recipe
                .as_ref()
                .map(|recipe| display_recipe_identifier(recipe.recipe_id.as_str()))
                .unwrap_or_else(|| "no recipe".to_string());
            PrimaryRow::new(
                format!("process-node-{}", node.id),
                display_process_flow_node_label(node),
                format!("{} {}", node.kind.label(), node.area),
                format!(
                    "{}; {} eligible tools; {}",
                    recipe,
                    node.eligible_tools.len(),
                    node.notes
                ),
            )
            .action(
                format!("glassworks.viewctl.process_flow.node.{}", node.id),
                app.selected_process_node.as_ref() == Some(&node.id),
            )
        })
        .collect()
}

pub(crate) fn process_control_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let mut rows = selected_control_loop(app)
        .map(|loop_definition| {
            app.workspace
                .process_control
                .actions_for_loop(&loop_definition.id)
                .into_iter()
                .take(10)
                .map(|action| {
                    PrimaryRow::new(
                        format!("control-action-{}", action.id),
                        process_control_action_short_label(action),
                        format!(
                            "{} {:.0}% confidence",
                            action.state.label(),
                            action.confidence * 100.0
                        ),
                        format!(
                            "error {:.3}; {} adjustments; {}",
                            action.error,
                            action.adjustments.len(),
                            action.rationale
                        ),
                    )
                    .action(
                        format!("glassworks.viewctl.process_control.action.{}", action.id),
                        app.selected_control_action.as_ref() == Some(&action.id),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if rows.is_empty() {
        rows.extend(
            filtered_control_loops(app)
                .into_iter()
                .take(10)
                .map(|loop_definition| {
                    PrimaryRow::new(
                        format!("control-loop-{}", loop_definition.id),
                        loop_definition.name.clone(),
                        equipment_tool_label_for_raw_id(&app.workspace, &loop_definition.tool_id),
                        format!(
                            "{} target {:.3} {}",
                            loop_definition.output.label,
                            loop_definition.output.target,
                            loop_definition.output.unit
                        ),
                    )
                    .action(
                        format!(
                            "glassworks.viewctl.process_control.loop.{}",
                            loop_definition.id
                        ),
                        app.selected_control_loop.as_ref() == Some(&loop_definition.id),
                    )
                }),
        );
    }
    rows
}

pub(crate) fn spc_fdc_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let monitor = spc_fdc_monitor(&app.workspace);
    let mut rows = monitor
        .findings
        .iter()
        .filter(|finding| app.spc_severity_filter.matches(finding.severity))
        .filter(|finding| app.spc_source_filter.matches(finding.source))
        .filter(|finding| {
            app.spc_context_filter.is_empty()
                || finding.title.contains(&app.spc_context_filter)
                || finding.detail.contains(&app.spc_context_filter)
                || finding
                    .tool_id
                    .as_deref()
                    .is_some_and(|tool_id| tool_id.contains(&app.spc_context_filter))
                || finding
                    .lot_id
                    .as_deref()
                    .is_some_and(|lot_id| lot_id.contains(&app.spc_context_filter))
                || finding
                    .recipe_id
                    .as_deref()
                    .is_some_and(|recipe_id| recipe_id.contains(&app.spc_context_filter))
        })
        .take(10)
        .map(|finding| {
            let context = [
                finding.tool_id.as_ref().map(|id| {
                    format!(
                        "tool {}",
                        equipment_tool_label_for_raw_id(&app.workspace, id)
                    )
                }),
                finding
                    .lot_id
                    .as_ref()
                    .map(|id| format!("lot {}", workflow_lot_label(&app.workspace, id, 28))),
                finding
                    .wafer_id
                    .as_ref()
                    .map(|id| display_wafer_identifier(id)),
                finding
                    .recipe_id
                    .as_ref()
                    .map(|id| format!("recipe {}", display_recipe_identifier(id))),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
            let detail = display_spc_finding_detail(&app.workspace, finding);
            PrimaryRow::new(
                format!("spc-{}-{}", finding.source.label(), finding.title),
                display_spc_finding_title(finding),
                format!("{} {}", finding.severity.label(), finding.source.label()),
                if context.is_empty() {
                    detail
                } else {
                    format!("{context}; {detail}")
                },
            )
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows.extend(monitor.charts.iter().take(10).map(|chart| {
            PrimaryRow::new(
                format!("spc-chart-{}", chart.id),
                display_measurement_identifier(&chart.metric),
                format!("{} violations", chart.violations.len()),
                format!(
                    "center {:.2}; limits {:.2}..{:.2}",
                    chart.limits.center, chart.limits.lower_control, chart.limits.upper_control
                ),
            )
            .action(
                format!("glassworks.viewctl.spc.chart.{}", chart.id),
                app.selected_spc_chart.as_deref() == Some(chart.id.as_str()),
            )
        }));
    }
    rows
}

pub(crate) fn cross_section_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let mut rows = vec![
        PrimaryRow::new(
            "cross-section-substrate",
            "Step 0 Starting substrate",
            app.workspace.cross_section.substrate_material.as_str(),
            format!(
                "{:.2} um thick; width {:.1} um",
                app.workspace.cross_section.substrate_thickness_um,
                app.workspace.cross_section.width_um
            ),
        )
        .action(
            "glassworks.viewctl.cross_section.step.0",
            app.cross_section_step == 0,
        ),
    ];
    rows.extend(
        app.workspace
            .cross_section
            .steps
            .iter()
            .enumerate()
            .take(9)
            .map(|(index, step)| {
                let step_index = index + 1;
                PrimaryRow::new(
                    format!("cross-section-step-{step_index}"),
                    format!("Step {step_index} {}", step.name),
                    cross_section_step_kind_label(&step.kind),
                    step.detail.clone(),
                )
                .action(
                    format!("glassworks.viewctl.cross_section.step.{step_index}"),
                    app.cross_section_step == step_index,
                )
            }),
    );
    rows
}

pub(crate) fn metrology_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    app.workspace
        .wafer_map
        .measurements
        .iter()
        .filter(|measurement| measurement.kind == app.metrology_kind)
        .filter(|measurement| {
            !app.metrology_failed_only || measurement.status != MeasurementStatus::Pass
        })
        .take(10)
        .map(|measurement| {
            PrimaryRow::new(
                format!("measurement-{}", measurement.id),
                metrology_measurement_title(measurement),
                format!(
                    "{} {}",
                    metrology_measurement_value(measurement),
                    measurement.status.label()
                ),
                format!(
                    "lot {} / wafer {} / {}",
                    measurement.links.lot_id,
                    measurement.links.wafer_id,
                    measurement.links.process_step_id
                ),
            )
            .action(
                format!(
                    "glassworks.viewctl.metrology.die.{}|{}",
                    measurement.die.column, measurement.die.row
                ),
                app.selected_die == Some(measurement.die),
            )
        })
        .collect()
}

pub(crate) fn metrology_measurement_title(
    measurement: &layout_model::metrology::Measurement,
) -> String {
    format!(
        "{} at {}",
        measurement.kind.label(),
        die_coord_label(Some(measurement.die))
    )
}

pub(crate) fn metrology_map_display_name(workspace: &WorkspaceDataset, map: &WaferMap) -> String {
    let lot_id = map.links.lot_id.trim();
    let wafer_id = map.links.wafer_id.trim();
    let mut name = map.name.trim().to_string();
    for raw_prefix in [
        (!lot_id.is_empty() && !wafer_id.is_empty()).then(|| format!("{lot_id} {wafer_id}")),
        (!lot_id.is_empty()).then(|| lot_id.to_string()),
        (!wafer_id.is_empty()).then(|| wafer_id.to_string()),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(stripped) = name.strip_prefix(&raw_prefix) {
            name = stripped
                .trim_start_matches([' ', '-', '/', ':'])
                .to_string();
        }
    }

    let mut parts = Vec::new();
    if !lot_id.is_empty() {
        parts.push(workflow_lot_label(workspace, lot_id, 32));
    }
    if !wafer_id.is_empty() {
        parts.push(yield_wafer_short_label(wafer_id));
    }
    if !name.is_empty() {
        parts.push(capitalize_ascii_first(name));
    }
    if parts.is_empty() {
        "Wafer map".to_string()
    } else {
        parts.join(" ")
    }
}

pub(crate) fn metrology_measurement_value(
    measurement: &layout_model::metrology::Measurement,
) -> String {
    let value = metrology_format_value(measurement.kind, measurement.value);
    let unit = measurement.kind.unit();
    if unit.is_empty() {
        value
    } else {
        format!("{value} {unit}")
    }
}

pub(crate) fn metrology_attention_site_label(die: DieCoord, score: usize, compact: bool) -> String {
    let level = if score >= 72 {
        "critical"
    } else if score >= 42 {
        "review"
    } else {
        "watch"
    };
    if compact {
        format!(
            "{} {}",
            die_coord_label(Some(die)),
            &level[..level.len().min(4)]
        )
    } else {
        format!("{} {level}", die_coord_label(Some(die)))
    }
}

pub(crate) fn yield_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    if let Some(lot_id) = app.selected_yield_lot.as_deref() {
        let rows = app
            .workspace
            .yield_analysis
            .wafer_summaries_for_lot(lot_id)
            .into_iter()
            .filter(|summary| yield_summary_matches(app, summary))
            .take(10)
            .map(|summary| {
                let wafer_id = summary
                    .wafer_id
                    .clone()
                    .unwrap_or_else(|| "lot".to_string());
                PrimaryRow::new(
                    format!("yield-wafer-{wafer_id}"),
                    wafer_id.clone(),
                    percent_label(summary.yield_fraction),
                    yield_summary_detail(summary),
                )
                .action(
                    format!("glassworks.viewctl.yield.wafer.{wafer_id}"),
                    app.selected_yield_wafer.as_deref() == Some(wafer_id.as_str()),
                )
            })
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            return rows;
        }
    }

    app.workspace
        .yield_analysis
        .lot_summaries
        .iter()
        .filter(|summary| yield_summary_matches(app, summary))
        .take(10)
        .map(|summary| {
            PrimaryRow::new(
                format!("yield-lot-{}", summary.lot_id),
                summary.lot_id.clone(),
                percent_label(summary.yield_fraction),
                yield_summary_detail(summary),
            )
            .action(
                format!("glassworks.viewctl.yield.lot.{}", summary.lot_id),
                app.selected_yield_lot.as_deref() == Some(summary.lot_id.as_str()),
            )
        })
        .collect()
}

pub(crate) fn experiment_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    app.filtered_experiment_run_rows()
        .into_iter()
        .take(10)
        .map(|row| {
            let factors = app
                .workspace
                .experiment_plan
                .factors
                .iter()
                .take(2)
                .map(|factor| factor.name.as_str())
                .zip(row.factor_values.iter().map(String::as_str))
                .map(|(factor, value)| format!("{factor}={value}"))
                .collect::<Vec<_>>()
                .join(", ");
            PrimaryRow::new(
                format!("experiment-run-{}", row.run_id),
                experiment_run_order_label(row.run_order),
                row.status.clone(),
                format!(
                    "order {} / {} {} / split {} / responses {} / primary {}",
                    row.run_order,
                    workflow_lot_label(&app.workspace, &row.lot_id, 28),
                    yield_wafer_short_label(&row.wafer_id),
                    row.block,
                    row.capture_summary,
                    row.primary_value
                ),
            )
            .with_detail_suffix(factors)
            .action(
                format!("glassworks.viewctl.experiment.run.{}", row.run_id),
                row.selected,
            )
        })
        .collect()
}

pub(crate) fn notebook_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    notebook_filtered_entries(app)
        .into_iter()
        .take(10)
        .map(|entry| {
            let tags = if entry.tags.is_empty() {
                "untagged".to_string()
            } else {
                entry
                    .tags
                    .iter()
                    .map(|tag| format!("#{tag}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            PrimaryRow::new(
                format!("notebook-{}", entry.id),
                entry.title.clone(),
                format!(
                    "{} {}",
                    entry.updated_at,
                    display_owner_identifier(&entry.author)
                ),
                format!("{} links; {tags}", entry.link_count()),
            )
        })
        .collect()
}

pub(crate) fn notebook_tag_summary(entry: &layout_model::notebook::NotebookEntry) -> String {
    if entry.tags.is_empty() {
        "No tags".to_string()
    } else {
        entry
            .tags
            .iter()
            .take(6)
            .map(|tag| format!("#{tag}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub(crate) fn notebook_link_summary(entry: &layout_model::notebook::NotebookEntry) -> String {
    let links = &entry.links;
    format!(
        "lots {}, wafers {}, recipes {}, runs {}, metrology {}, images {}",
        links.lots.len(),
        links.wafers.len(),
        links.recipes.len(),
        links.tool_runs.len(),
        links.metrology.len(),
        links.images.len()
    )
}

pub(crate) fn notebook_link_summary_compact(
    entry: &layout_model::notebook::NotebookEntry,
) -> String {
    let links = &entry.links;
    let link_types = [
        links.lots.len(),
        links.wafers.len(),
        links.recipes.len(),
        links.tool_runs.len(),
        links.metrology.len(),
        links.images.len(),
    ]
    .into_iter()
    .filter(|count| *count > 0)
    .count();
    if link_types == 0 {
        "No link types".to_string()
    } else {
        format!("{link_types} link types")
    }
}

pub(crate) fn notebook_body_display_text(
    entry: &layout_model::notebook::NotebookEntry,
    preview_mode: bool,
) -> String {
    if !preview_mode {
        return entry.body_markdown.replace('\n', " ");
    }

    entry
        .body_markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let heading = trimmed
                .strip_prefix('#')
                .map(|value| value.trim_start_matches('#').trim())
                .filter(|value| !value.is_empty());
            Some(match heading {
                Some(value) => format!("{value}:"),
                None => trimmed.to_string(),
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn notebook_entry_link_focus_tokens(
    entry: &layout_model::notebook::NotebookEntry,
) -> Vec<(NotebookLinkKind, String)> {
    let mut tokens = Vec::new();
    tokens.extend(
        entry
            .links
            .lots
            .iter()
            .map(|value| (NotebookLinkKind::Lot, value.to_string())),
    );
    tokens.extend(
        entry
            .links
            .wafers
            .iter()
            .map(|value| (NotebookLinkKind::Wafer, value.to_string())),
    );
    tokens.extend(
        entry
            .links
            .recipes
            .iter()
            .map(|value| (NotebookLinkKind::Recipe, value.to_string())),
    );
    tokens.extend(
        entry
            .links
            .tool_runs
            .iter()
            .map(|value| (NotebookLinkKind::ToolRun, value.to_string())),
    );
    tokens.extend(
        entry
            .links
            .metrology
            .iter()
            .map(|value| (NotebookLinkKind::Metrology, value.id.to_string())),
    );
    tokens.extend(
        entry
            .links
            .images
            .iter()
            .map(|value| (NotebookLinkKind::Image, value.id.to_string())),
    );
    tokens
}

pub(crate) fn notebook_entry_has_link(
    entry: &layout_model::notebook::NotebookEntry,
    kind: NotebookLinkKind,
    query: &str,
) -> bool {
    match kind {
        NotebookLinkKind::Lot => entry.links.lots.iter().any(|value| value.as_str() == query),
        NotebookLinkKind::Wafer => entry
            .links
            .wafers
            .iter()
            .any(|value| value.as_str() == query),
        NotebookLinkKind::Recipe => entry
            .links
            .recipes
            .iter()
            .any(|value| value.as_str() == query),
        NotebookLinkKind::ToolRun => entry
            .links
            .tool_runs
            .iter()
            .any(|value| value.to_string() == query),
        NotebookLinkKind::Metrology => entry
            .links
            .metrology
            .iter()
            .any(|value| value.id.as_str() == query),
        NotebookLinkKind::Image => entry
            .links
            .images
            .iter()
            .any(|value| value.id.as_str() == query),
    }
}

pub(crate) fn display_notebook_shared_link_label(value: &str) -> String {
    if value.contains("-W") || value.starts_with('W') {
        return display_wafer_identifier(value);
    }
    if value.starts_with("L-") {
        return display_lot_identifier(value);
    }
    if value.starts_with("RUN-") || value.contains("-RUN-") {
        return display_tool_run_identifier(value);
    }
    capitalize_ascii_first(humanize_identifier(value))
}

pub(crate) fn notebook_link_value_label(kind: NotebookLinkKind, value: &str) -> String {
    match kind {
        NotebookLinkKind::Lot => display_lot_identifier(value),
        NotebookLinkKind::Wafer => display_wafer_identifier(value),
        NotebookLinkKind::Recipe => display_recipe_identifier(value),
        NotebookLinkKind::ToolRun => display_tool_run_identifier(value),
        NotebookLinkKind::Metrology | NotebookLinkKind::Image => {
            display_notebook_shared_link_label(value)
        }
    }
}

pub(crate) fn notebook_shared_link_summary(shared_links: &[String]) -> String {
    shared_links
        .iter()
        .map(|link| display_notebook_shared_link_label(link))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn add_notebook_tag_once(entry: &mut layout_model::notebook::NotebookEntry, tag: &str) {
    if !entry.tags.iter().any(|candidate| candidate == tag) {
        entry.tags.push(tag.to_string());
        entry.tags.sort();
    }
}

pub(crate) fn append_notebook_section_once(body: &mut String, heading: &str, content: &str) {
    if body.contains(heading) {
        return;
    }
    if !body.ends_with('\n') {
        body.push('\n');
    }
    if !body.ends_with("\n\n") {
        body.push('\n');
    }
    body.push_str(heading);
    body.push('\n');
    body.push_str(content);
}

pub(crate) fn notebook_related_entries(
    app: &GlassworksApp,
    entry: &layout_model::notebook::NotebookEntry,
) -> Vec<(String, String)> {
    let entry_tags = entry.tags.iter().cloned().collect::<BTreeSet<_>>();
    let entry_links = entry
        .links
        .search_labels()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut related = app
        .workspace
        .lab_notebook
        .entries
        .iter()
        .filter(|candidate| candidate.id != entry.id)
        .filter_map(|candidate| {
            let shared_tags = candidate
                .tags
                .iter()
                .filter(|tag| entry_tags.contains(*tag))
                .cloned()
                .collect::<Vec<_>>();
            let candidate_links = candidate
                .links
                .search_labels()
                .into_iter()
                .collect::<BTreeSet<_>>();
            let shared_links = candidate_links
                .intersection(&entry_links)
                .cloned()
                .collect::<Vec<_>>();
            let score = shared_tags.len() + shared_links.len();
            if score == 0 {
                return None;
            }
            let reason = if !shared_tags.is_empty() && !shared_links.is_empty() {
                format!(
                    "shared tags {}; shared links {}",
                    shared_tags.join(", "),
                    notebook_shared_link_summary(&shared_links)
                )
            } else if !shared_tags.is_empty() {
                format!("shared tags {}", shared_tags.join(", "))
            } else {
                format!(
                    "shared links {}",
                    notebook_shared_link_summary(&shared_links)
                )
            };
            Some((
                score,
                candidate.updated_at.clone(),
                candidate.title.clone(),
                reason,
            ))
        })
        .collect::<Vec<_>>();
    related.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    related
        .into_iter()
        .map(|(_, _, title, reason)| (title, reason))
        .collect()
}

pub(crate) fn environment_sensor_status_for(
    workspace: &WorkspaceDataset,
    sensor: &layout_model::environment::EnvironmentSensor,
) -> String {
    workspace
        .environment
        .latest_reading(&sensor.id)
        .map(|reading| {
            let severity = sensor.thresholds.evaluate(reading.value);
            let state = severity.map_or("Nominal", |severity| severity.label());
            format!("{:.2} {} {state}", reading.value, sensor.unit)
        })
        .unwrap_or_else(|| "No samples".to_string())
}

pub(crate) fn cross_section_step_kind_label(
    kind: &layout_model::cross_section::ProcessStepKind,
) -> String {
    match kind {
        layout_model::cross_section::ProcessStepKind::Deposit {
            material,
            thickness_um,
        } => format!("deposit {} {:.2} um", material.as_str(), thickness_um),
        layout_model::cross_section::ProcessStepKind::Etch { material, depth_um } => {
            format!("etch {} {:.2} um", material.as_str(), depth_um)
        }
        layout_model::cross_section::ProcessStepKind::Pattern { openings } => {
            format!("pattern {} openings", openings.len())
        }
    }
}

pub(crate) fn yield_summary_matches(
    app: &GlassworksApp,
    summary: &layout_model::yield_analysis::YieldSummary,
) -> bool {
    let filter_matches = match app.yield_map_filter {
        YieldMapFilter::All => true,
        YieldMapFilter::Failing => summary.failing_dies > 0,
        YieldMapFilter::Passing => summary.failing_dies == 0,
    };
    filter_matches
        && (!app.show_only_attention_wafers || summary.yield_fraction < 0.98)
        && (!app.show_only_excursions || !summary.root_cause_hints.is_empty())
}

pub(crate) fn yield_filter_summary(app: &GlassworksApp) -> String {
    let mut parts = vec![match app.yield_map_filter {
        YieldMapFilter::All => "All wafers".to_string(),
        YieldMapFilter::Failing => "Failing wafers".to_string(),
        YieldMapFilter::Passing => "Passing wafers".to_string(),
    }];
    if app.show_only_attention_wafers {
        parts.push("attention only".to_string());
    }
    if app.show_only_excursions {
        parts.push("excursions only".to_string());
    }
    parts.join(" / ")
}

pub(crate) fn enabled_state_label(enabled: bool) -> &'static str {
    if enabled { "On" } else { "Off" }
}

pub(crate) fn yield_summary_detail(summary: &layout_model::yield_analysis::YieldSummary) -> String {
    let failure = summary
        .dominant_failure
        .map(|failure| failure.label())
        .unwrap_or("no dominant failure");
    let root_cause = summary
        .root_cause_hints
        .first()
        .map(String::as_str)
        .unwrap_or("no root-cause hint");
    format!(
        "{} failing dies; {}; {}; {}",
        summary.failing_dies,
        failure,
        summary.spatial_pattern.label(),
        root_cause
    )
}

pub(crate) fn percent_label(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}
