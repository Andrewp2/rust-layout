#![allow(unused_imports)]
use super::*;

pub(crate) fn default_yield_lot(workspace: &WorkspaceDataset) -> Option<String> {
    workspace
        .yield_analysis
        .lots
        .first()
        .map(|lot| lot.id.clone())
}

pub(crate) fn default_yield_wafer(
    workspace: &WorkspaceDataset,
    lot_id: Option<&str>,
) -> Option<String> {
    lot_id.and_then(|lot_id| {
        workspace
            .yield_analysis
            .wafer_ids_for_lot(lot_id)
            .first()
            .cloned()
    })
}

pub(crate) fn default_experiment_run(workspace: &WorkspaceDataset) -> Option<ExperimentRunId> {
    workspace
        .experiment_plan
        .runs
        .first()
        .map(|run| run.id.clone())
}

pub(crate) fn default_experiment_response(workspace: &WorkspaceDataset) -> Option<ResponseSpecId> {
    workspace
        .experiment_plan
        .responses
        .first()
        .map(|response| response.id.clone())
}

pub(crate) fn workflow_lot_ids(workspace: &WorkspaceDataset) -> Vec<String> {
    let mut ids = workspace
        .mes
        .lots
        .values()
        .map(|lot| lot.id.as_str().to_string())
        .collect::<Vec<_>>();
    for lot in &workspace.yield_analysis.lots {
        if !ids.iter().any(|id| id == &lot.id) {
            ids.push(lot.id.clone());
        }
    }
    ids
}

pub(crate) fn default_workflow_focus_lot(workspace: &WorkspaceDataset) -> Option<String> {
    let lot_ids = workflow_lot_ids(workspace);
    lot_ids
        .iter()
        .find(|lot_id| lot_id.as_str() == "L-00042")
        .cloned()
        .or_else(|| lot_ids.first().cloned())
}

pub(crate) fn default_equipment_tool(workspace: &WorkspaceDataset) -> Option<EquipmentToolId> {
    workspace
        .equipment
        .tools()
        .next()
        .map(|tool| tool.id.clone())
}

pub(crate) fn default_maintenance_tool(workspace: &WorkspaceDataset) -> Option<EquipmentToolId> {
    workspace
        .maintenance
        .tools
        .first()
        .map(|tool| tool.tool_id.clone())
}

pub(crate) fn default_environment_sensor(workspace: &WorkspaceDataset) -> Option<String> {
    workspace
        .environment
        .sensors
        .first()
        .map(|sensor| sensor.id.clone())
}

pub(crate) fn default_environment_sensor_with_options(
    workspace: &WorkspaceDataset,
    alarm_sensors_first: bool,
) -> Option<String> {
    if alarm_sensors_first
        && let Some(alarm) = workspace
            .environment
            .evaluate_alarms()
            .into_iter()
            .find(|alarm| alarm.active)
    {
        return Some(alarm.sensor_id);
    }
    default_environment_sensor(workspace)
}

pub(crate) fn inventory_filter_matches(filter: InventoryQuickFilter, lot: &MaterialLot) -> bool {
    match filter {
        InventoryQuickFilter::All => true,
        InventoryQuickFilter::NeedsAction => {
            lot.is_expired(INVENTORY_DEMO_TODAY)
                || lot.is_low_stock()
                || lot.expires_within_days(INVENTORY_DEMO_TODAY, 30)
        }
        InventoryQuickFilter::ProductHold => lot.is_expired(INVENTORY_DEMO_TODAY),
        InventoryQuickFilter::LowStock => lot.is_low_stock(),
        InventoryQuickFilter::ExpiringSoon => lot.expires_within_days(INVENTORY_DEMO_TODAY, 30),
        InventoryQuickFilter::InUse => !lot.usage.is_empty(),
    }
}

pub(crate) fn inventory_filter_count(
    workspace: &WorkspaceDataset,
    filter: InventoryQuickFilter,
) -> usize {
    workspace
        .inventory
        .lots
        .values()
        .filter(|lot| inventory_filter_matches(filter, lot))
        .count()
}

pub(crate) fn default_inventory_lot(
    workspace: &WorkspaceDataset,
    filter: InventoryQuickFilter,
) -> Option<MaterialLotId> {
    workspace
        .inventory
        .sorted_lots()
        .into_iter()
        .find(|lot| inventory_filter_matches(filter, lot))
        .map(|lot| lot.id.clone())
        .or_else(|| {
            workspace
                .inventory
                .sorted_lots()
                .first()
                .map(|lot| lot.id.clone())
        })
}

pub(crate) fn selected_inventory_lot<'a>(app: &'a GlassworksApp) -> Option<&'a MaterialLot> {
    app.selected_inventory_lot
        .as_ref()
        .and_then(|id| app.workspace.inventory.lot(id))
        .or_else(|| app.workspace.inventory.sorted_lots().first().copied())
}

pub(crate) fn default_mask_lot(workspace: &WorkspaceDataset) -> Option<LotId> {
    default_workflow_focus_lot(workspace)
        .map(LotId::new)
        .filter(|lot_id| workspace.mes.lots.contains_key(lot_id))
        .or_else(|| workspace.mes.lots.keys().next().cloned())
}

pub(crate) fn reticle_prep_for_app(app: &GlassworksApp) -> ReticlePrep {
    app.mask_source_lot
        .as_ref()
        .and_then(|lot_id| app.workspace.mes.lots.get(lot_id))
        .and_then(|lot| app.workspace.mes.routes.get(&lot.route_id))
        .map(|route| ReticlePrep::from_document_and_route(&app.workspace.document, route))
        .unwrap_or_else(|| ReticlePrep::from_document(&app.workspace.document))
}

pub(crate) fn mask_check_report(app: &GlassworksApp) -> MaskCheckReport {
    reticle_prep_for_app(app).validate_document(&app.workspace.document)
}

pub(crate) fn display_spc_trace_name(trace: &SensorTrace) -> String {
    format!(
        "{} / {}",
        trace.tool_id,
        display_measurement_identifier(&trace.sensor_name)
    )
}

pub(crate) fn display_spc_chart_label(chart: &ControlChart) -> String {
    if chart.name.trim().is_empty() {
        display_spc_control_identifier(&chart.metric)
    } else {
        chart.name.clone()
    }
}

pub(crate) fn display_spc_finding_title(finding: &layout_model::spc_fdc::MonitorFinding) -> String {
    if finding.source == FindingSource::FdcTrace {
        if let Some((tool_id, sensor_detail)) = finding.title.split_once(" / ") {
            let sensor_name = sensor_detail
                .strip_suffix(" excursion")
                .unwrap_or(sensor_detail);
            return format!(
                "{} / {} excursion",
                tool_id,
                display_measurement_identifier(sensor_name)
            );
        }
    }
    capitalize_ascii_first(finding.title.clone())
}

pub(crate) fn display_process_flow_route_name(value: &str) -> String {
    capitalize_ascii_first(humanize_identifier(
        value.strip_prefix("Demo ").unwrap_or(value),
    ))
}

pub(crate) fn display_process_flow_node_label(
    node: &layout_model::process_flow::ProcessFlowNode,
) -> String {
    if node.name.trim().is_empty() {
        capitalize_ascii_first(humanize_identifier(node.id.as_str()))
    } else {
        node.name.clone()
    }
}

pub(crate) fn compact_process_step_button_label(label: &str, max_chars: usize) -> String {
    match label {
        "Coat PR and soft bake" => "Coat PR bake".to_string(),
        "Align and expose poly mask" => "Align expose".to_string(),
        "Develop and inspect resist" => "Develop inspect".to_string(),
        "Poly plasma etch" => "Poly etch".to_string(),
        _ => compact_button_label(label, max_chars),
    }
}

pub(crate) fn process_flow_node_button_label(
    node: &layout_model::process_flow::ProcessFlowNode,
) -> String {
    let label = display_process_flow_node_label(node);
    compact_process_step_button_label(&label, 18)
}

pub(crate) fn experiment_run_order_label(run_order: u32) -> String {
    if run_order == 0 {
        "Run --".to_string()
    } else {
        format!("Run {run_order:02}")
    }
}

pub(crate) fn display_experiment_run_label(run: &ExperimentRun) -> String {
    experiment_run_order_label(run.assignment.run_order)
}

pub(crate) fn experiment_plan_title_label(plan: &ExperimentPlan, compact_rows: bool) -> String {
    if compact_rows {
        let title = match plan.title.as_str() {
            "Poly lithography CD split" => "Poly CD split".to_string(),
            _ => compact_button_label(&plan.title, 18),
        };
        format!("{title} - {}", plan.status.label())
    } else {
        format!(
            "{} - {} / owner {}",
            plan.title,
            plan.status.label(),
            display_owner_identifier(&plan.owner)
        )
    }
}

pub(crate) fn display_experiment_objective(value: &str) -> String {
    value
        .replace("demo inverter", "sample inverter")
        .replace("demo ", "sample ")
}

pub(crate) fn display_safety_incident_identifier(
    incident_id: &layout_model::safety::SafetyIncidentId,
) -> String {
    let value = incident_id.to_string();
    value
        .strip_prefix("INC-SIM-")
        .or_else(|| value.strip_prefix("INC-"))
        .map(|suffix| format!("Incident {suffix}"))
        .unwrap_or(value)
}

pub(crate) fn safety_tool_label(tool_id: &str, tool_name: &str) -> String {
    if tool_name.trim().is_empty() {
        tool_id.to_string()
    } else {
        tool_name.to_string()
    }
}

pub(crate) fn safety_selected_tool_label(app: &GlassworksApp) -> String {
    let Some(tool_id) = app.selected_safety_tool.as_deref() else {
        return "None".to_string();
    };
    app.workspace
        .safety
        .evaluate_lockouts()
        .into_iter()
        .find(|lockout| lockout.tool_id == tool_id)
        .map(|lockout| safety_tool_label(&lockout.tool_id, &lockout.tool_name))
        .unwrap_or_else(|| tool_id.to_string())
}

pub(crate) fn safety_incident_action_label(
    incident: &layout_model::safety::SafetyIncident,
) -> String {
    format!("Ack {}", incident.domain.label())
}

pub(crate) fn safety_ack_condition_label(sensor: &layout_model::safety::SafetySensor) -> String {
    format!("Ack {}", sensor.domain.label())
}

pub(crate) fn safety_sensor_tool_label(
    workspace: &WorkspaceDataset,
    sensor: &layout_model::safety::SafetySensor,
) -> String {
    sensor
        .tool_id
        .as_deref()
        .map(|tool_id| equipment_tool_label_for_raw_id(workspace, tool_id))
        .unwrap_or_else(|| "Facility".to_string())
}

pub(crate) fn display_environment_sensor_label(sensor: &EnvironmentSensor) -> String {
    sensor.name.clone()
}

pub(crate) fn environment_sensor_button_label(sensor: &EnvironmentSensor) -> String {
    match sensor.name.as_str() {
        "Litho bay temperature" => "Litho temp".to_string(),
        "Litho bay humidity" => "Litho humidity".to_string(),
        "CMP particle counter" => "CMP particles".to_string(),
        "Stepper slab vibration" => "Stepper vibe".to_string(),
        "DI loop resistivity" => "DI resistivity".to_string(),
        "Acid exhaust flow" => "Acid exhaust".to_string(),
        _ => compact_button_label(&display_environment_sensor_label(sensor), 18),
    }
}

pub(crate) fn environment_alarm_short_label(
    severity: Option<EnvironmentAlarmSeverity>,
) -> &'static str {
    match severity {
        Some(EnvironmentAlarmSeverity::Critical) => "crit",
        Some(EnvironmentAlarmSeverity::Warning) => "warn",
        Some(EnvironmentAlarmSeverity::Advisory) => "adv",
        None => "nom",
    }
}

pub(crate) fn mask_issue_matches_filter(
    app: &GlassworksApp,
    issue: &layout_model::mask::MaskPrepIssue,
) -> bool {
    app.mask_issue_severity_filter.matches(issue.severity)
}

pub(crate) fn mask_filtered_issue_count(app: &GlassworksApp, report: &MaskCheckReport) -> usize {
    report
        .issues
        .iter()
        .filter(|issue| mask_issue_matches_filter(app, issue))
        .count()
}

pub(crate) fn mask_issue_page_count(app: &GlassworksApp, report: &MaskCheckReport) -> usize {
    mask_filtered_issue_count(app, report)
        .div_ceil(MASK_ISSUE_PAGE_SIZE)
        .max(1)
}

pub(crate) fn mask_issue_group_count(app: &GlassworksApp, report: &MaskCheckReport) -> usize {
    report
        .issues
        .iter()
        .filter(|issue| mask_issue_matches_filter(app, issue))
        .map(|issue| mask_issue_group_key(issue, app.mask_issue_grouping))
        .collect::<BTreeSet<_>>()
        .len()
}

pub(crate) fn mask_issue_group_key(
    issue: &layout_model::mask::MaskPrepIssue,
    grouping: MaskIssueGrouping,
) -> String {
    match grouping {
        MaskIssueGrouping::Code => issue.code.clone(),
        MaskIssueGrouping::Layer => issue
            .layer
            .map(|layer| format!("layer {}", layer.0))
            .unwrap_or_else(|| "no layer".to_string()),
        MaskIssueGrouping::Field => issue
            .field_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "no field".to_string()),
        MaskIssueGrouping::Block => issue
            .block_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "no block".to_string()),
    }
}

pub(crate) fn layout_diff_report(app: &GlassworksApp) -> LayoutDiffReport {
    let baseline = app.layout_diff_baseline.document(&app.workspace.document);
    let candidate = app.layout_diff_candidate.document(&app.workspace.document);
    diff_documents(
        app.layout_diff_baseline.detail_label(),
        &baseline,
        app.layout_diff_candidate.detail_label(),
        &candidate,
    )
}

pub(crate) fn layout_diff_filtered_change_count(
    app: &GlassworksApp,
    report: &LayoutDiffReport,
) -> usize {
    report
        .changes
        .iter()
        .filter(|change| app.layout_diff_change_filter.matches(change.kind))
        .count()
}

pub(crate) fn layout_diff_page_count(app: &GlassworksApp, report: &LayoutDiffReport) -> usize {
    let page_size = normalized_layout_diff_page_size(app);
    layout_diff_filtered_change_count(app, report)
        .div_ceil(page_size)
        .max(1)
}

pub(crate) fn normalized_layout_diff_page_size(app: &GlassworksApp) -> usize {
    if LAYOUT_DIFF_PAGE_SIZE_OPTIONS.contains(&app.layout_diff_page_size) {
        app.layout_diff_page_size
    } else {
        LAYOUT_DIFF_DEFAULT_PAGE_SIZE
    }
}

pub(crate) fn layout_diff_total_changes(report: &LayoutDiffReport) -> usize {
    report.summary.added_shapes + report.summary.removed_shapes + report.summary.modified_shapes
}

pub(crate) fn default_trace_lot(workspace: &WorkspaceDataset) -> Option<LotId> {
    workspace.genealogy.lot_ids().first().cloned()
}

pub(crate) fn default_trace_wafer(
    workspace: &WorkspaceDataset,
    lot_id: Option<&LotId>,
) -> Option<WaferId> {
    lot_id.and_then(|lot_id| {
        workspace
            .genealogy
            .wafer_refs_for_lot(lot_id)
            .first()
            .map(|wafer| wafer.wafer_id.clone())
    })
}

pub(crate) fn default_trace_selection(
    workspace: &WorkspaceDataset,
    lot_id: Option<&LotId>,
    wafer_id: Option<&WaferId>,
) -> Option<TraceSelection> {
    if let (Some(lot_id), Some(wafer_id)) = (lot_id, wafer_id) {
        let wafer = WaferRef {
            lot_id: lot_id.clone(),
            wafer_id: wafer_id.clone(),
        };
        if workspace.genealogy.wafer(&wafer).is_some() {
            return Some(TraceSelection::Wafer(wafer));
        }
    }
    lot_id
        .filter(|lot_id| workspace.genealogy.lots.contains_key(lot_id))
        .cloned()
        .map(TraceSelection::Lot)
}

pub(crate) fn selected_trace_wafer_ref(app: &GlassworksApp) -> Option<WaferRef> {
    Some(WaferRef {
        lot_id: app.selected_trace_lot.clone()?,
        wafer_id: app.selected_trace_wafer.clone()?,
    })
}

pub(crate) fn trace_selection_from_view_action(value: &str) -> Option<TraceSelection> {
    if let Some(lot_id) = value.strip_prefix("lot.") {
        return Some(TraceSelection::Lot(LotId::new(lot_id)));
    }
    if let Some(raw) = value.strip_prefix("wafer.") {
        let (lot_id, wafer_id) = raw.split_once('|')?;
        return Some(TraceSelection::Wafer(WaferRef {
            lot_id: LotId::new(lot_id),
            wafer_id: WaferId::new(wafer_id),
        }));
    }
    if let Some(sequence) = value.strip_prefix("process.") {
        return Some(TraceSelection::Process(sequence.parse().ok()?));
    }
    if let Some(sequence) = value.strip_prefix("material.") {
        return Some(TraceSelection::MaterialUse(sequence.parse().ok()?));
    }
    if let Some(sequence) = value.strip_prefix("event.") {
        return Some(TraceSelection::Event(sequence.parse().ok()?));
    }
    None
}

pub(crate) fn trace_detail_action(selection: &TraceSelection) -> String {
    match selection {
        TraceSelection::Lot(lot_id) => format!("glassworks.viewctl.trace.detail.lot.{lot_id}"),
        TraceSelection::Wafer(wafer) => format!(
            "glassworks.viewctl.trace.detail.wafer.{}|{}",
            wafer.lot_id, wafer.wafer_id
        ),
        TraceSelection::Process(sequence) => {
            format!("glassworks.viewctl.trace.detail.process.{sequence}")
        }
        TraceSelection::MaterialUse(sequence) => {
            format!("glassworks.viewctl.trace.detail.material.{sequence}")
        }
        TraceSelection::Event(sequence) => {
            format!("glassworks.viewctl.trace.detail.event.{sequence}")
        }
    }
}

pub(crate) fn trace_related_wafer_count(app: &GlassworksApp) -> usize {
    let Some(wafer) = selected_trace_wafer_ref(app) else {
        return 0;
    };
    app.workspace.genealogy.wafer_lineage(&wafer).len()
        + app.workspace.genealogy.wafer_descendants(&wafer).len()
}

pub(crate) fn trace_impact_count(app: &GlassworksApp) -> usize {
    trace_impact_query(app)
        .map(|query| {
            app.workspace
                .genealogy
                .impact_for(query)
                .impacted_wafers
                .len()
        })
        .unwrap_or(0)
}

pub(crate) fn trace_impact_query(
    app: &GlassworksApp,
) -> Option<layout_model::genealogy::ExcursionQuery> {
    use layout_model::genealogy::ExcursionQuery;
    let wafer = selected_trace_wafer_ref(app)?;
    match app.trace_impact_mode {
        TraceImpactMode::ToolRun => app
            .workspace
            .genealogy
            .inherited_process_history_for_wafer(&wafer)
            .last()
            .map(|record| ExcursionQuery::ToolRun {
                tool_run_id: record.tool_run_id.clone(),
            }),
        TraceImpactMode::Step => app
            .workspace
            .genealogy
            .inherited_process_history_for_wafer(&wafer)
            .last()
            .map(|record| ExcursionQuery::ProcessStep {
                step_id: record.step_id.clone(),
            }),
        TraceImpactMode::Material => app
            .workspace
            .genealogy
            .material_ancestry_for_wafer(&wafer)
            .last()
            .map(|record| ExcursionQuery::MaterialLot {
                material_lot_id: record.material_lot_id.clone(),
            }),
    }
}

pub(crate) fn trace_impact_query_label(
    genealogy: &layout_model::genealogy::LotGenealogy,
    query: &layout_model::genealogy::ExcursionQuery,
) -> String {
    match query {
        layout_model::genealogy::ExcursionQuery::ToolRun { tool_run_id } => {
            format!(
                "tool run {}",
                display_tool_run_identifier(tool_run_id.as_str())
            )
        }
        layout_model::genealogy::ExcursionQuery::ProcessStep { step_id } => {
            format!("process step {}", display_step_identifier(step_id.as_str()))
        }
        layout_model::genealogy::ExcursionQuery::MaterialLot { material_lot_id } => {
            format!(
                "material {}",
                trace_material_lot_label(genealogy, material_lot_id)
            )
        }
    }
}

pub(crate) fn default_notebook_entry(workspace: &WorkspaceDataset) -> Option<NotebookEntryId> {
    workspace
        .lab_notebook
        .entries
        .first()
        .map(|entry| entry.id.clone())
}

pub(crate) fn notebook_filter(app: &GlassworksApp) -> NotebookFilter {
    NotebookFilter {
        query: String::new(),
        tag: app.notebook_tag_filter.clone(),
        link_kind: app.notebook_link_kind_filter,
    }
}

pub(crate) fn notebook_filtered_entries(
    app: &GlassworksApp,
) -> Vec<&layout_model::notebook::NotebookEntry> {
    app.workspace
        .lab_notebook
        .filtered_entries(&notebook_filter(app))
        .into_iter()
        .filter(|entry| !app.notebook_followups_only || notebook_entry_has_followup(entry))
        .collect()
}

pub(crate) fn selected_notebook_entry<'a>(
    app: &'a GlassworksApp,
) -> Option<&'a layout_model::notebook::NotebookEntry> {
    app.selected_notebook_entry
        .as_ref()
        .and_then(|id| app.workspace.lab_notebook.entry(id))
        .or_else(|| notebook_filtered_entries(app).first().copied())
}

pub(crate) fn notebook_entry_has_followup(entry: &layout_model::notebook::NotebookEntry) -> bool {
    entry.tags.iter().any(|tag| tag.contains("follow"))
        || entry
            .body_markdown
            .to_ascii_lowercase()
            .contains("follow-up")
}

pub(crate) fn notebook_link_count_for_kind(app: &GlassworksApp, kind: NotebookLinkKind) -> usize {
    app.workspace
        .lab_notebook
        .entries
        .iter()
        .filter(|entry| entry.has_link_kind(kind))
        .count()
}

pub(crate) fn notebook_link_kind_slug(kind: NotebookLinkKind) -> &'static str {
    match kind {
        NotebookLinkKind::Lot => "lot",
        NotebookLinkKind::Wafer => "wafer",
        NotebookLinkKind::Recipe => "recipe",
        NotebookLinkKind::ToolRun => "tool-run",
        NotebookLinkKind::Metrology => "metrology",
        NotebookLinkKind::Image => "image",
    }
}

pub(crate) fn notebook_link_kind_from_slug(value: &str) -> Option<NotebookLinkKind> {
    match value {
        "lot" => Some(NotebookLinkKind::Lot),
        "wafer" => Some(NotebookLinkKind::Wafer),
        "recipe" => Some(NotebookLinkKind::Recipe),
        "tool-run" => Some(NotebookLinkKind::ToolRun),
        "metrology" => Some(NotebookLinkKind::Metrology),
        "image" => Some(NotebookLinkKind::Image),
        _ => None,
    }
}

pub(crate) fn dispatch_policy_slug(policy: DispatchPolicy) -> &'static str {
    match policy {
        DispatchPolicy::Fifo => "fifo",
        DispatchPolicy::PriorityThenFifo => "priority",
        DispatchPolicy::DueDateThenPriority => "due-date",
    }
}

pub(crate) fn dispatch_policy_from_slug(value: &str) -> Option<DispatchPolicy> {
    match value {
        "fifo" => Some(DispatchPolicy::Fifo),
        "priority" => Some(DispatchPolicy::PriorityThenFifo),
        "due-date" => Some(DispatchPolicy::DueDateThenPriority),
        _ => None,
    }
}

pub(crate) fn display_dispatch_policy_label(policy: DispatchPolicy) -> &'static str {
    match policy {
        DispatchPolicy::Fifo => "FIFO",
        DispatchPolicy::PriorityThenFifo => "Priority + FIFO",
        DispatchPolicy::DueDateThenPriority => "Due date + priority",
    }
}

pub(crate) fn default_scheduler_tool(workspace: &WorkspaceDataset) -> Option<SchedulerToolId> {
    workspace
        .scheduler
        .tools
        .first()
        .map(|tool| tool.id.clone())
}

pub(crate) fn scheduler_assignment_count(app: &GlassworksApp) -> usize {
    app.workspace
        .scheduler
        .dispatch(app.scheduler_policy)
        .assignments
        .len()
}

pub(crate) fn scheduler_unscheduled_count(app: &GlassworksApp) -> usize {
    app.workspace
        .scheduler
        .dispatch(app.scheduler_policy)
        .unscheduled_lots
        .len()
}

pub(crate) fn scheduler_filtered_lot_count(app: &GlassworksApp) -> usize {
    app.workspace
        .scheduler
        .lots
        .iter()
        .filter(|lot| lot.priority >= app.scheduler_min_priority)
        .filter(|lot| {
            !app.scheduler_focus_selected_tool
                || app
                    .selected_scheduler_tool
                    .as_ref()
                    .and_then(|tool_id| {
                        app.workspace
                            .scheduler
                            .tools
                            .iter()
                            .find(|tool| &tool.id == tool_id)
                    })
                    .is_some_and(|tool| tool.can_process(lot))
        })
        .count()
}

pub(crate) fn default_safety_tool(workspace: &WorkspaceDataset) -> Option<String> {
    workspace
        .safety
        .evaluate_lockouts()
        .first()
        .map(|lockout| lockout.tool_id.clone())
}

pub(crate) fn selected_safety_lockout(
    app: &GlassworksApp,
) -> Option<layout_model::safety::ToolLockout> {
    let lockouts = app.workspace.safety.evaluate_lockouts();
    app.selected_safety_tool
        .as_deref()
        .and_then(|tool_id| {
            lockouts
                .iter()
                .find(|lockout| lockout.tool_id == tool_id)
                .cloned()
        })
        .or_else(|| lockouts.first().cloned())
}

pub(crate) fn safety_highest_label(app: &GlassworksApp) -> &'static str {
    app.workspace
        .safety
        .summary()
        .highest_severity
        .unwrap_or(SafetySeverity::Normal)
        .label()
}

pub(crate) fn process_flow_node_matches(
    workspace: &WorkspaceDataset,
    filter: ProcessFlowNodeFilter,
    node: &layout_model::process_flow::ProcessFlowNode,
) -> bool {
    match filter {
        ProcessFlowNodeFilter::All => true,
        ProcessFlowNodeFilter::RecipeSteps => node.kind.requires_recipe() || node.recipe.is_some(),
        ProcessFlowNodeFilter::Metrology => {
            node.kind == ProcessFlowNodeKind::Measurement || node.measurement_checkpoint.is_some()
        }
        ProcessFlowNodeFilter::Holds => node.kind == ProcessFlowNodeKind::Hold || node.hold_point,
        ProcessFlowNodeFilter::Rework => workspace.process_flow.route.edges.iter().any(|edge| {
            edge.kind == layout_model::process_flow::ProcessFlowEdgeKind::Rework
                && (edge.from == node.id || edge.to == node.id)
        }),
    }
}

pub(crate) fn process_flow_filter_count(
    workspace: &WorkspaceDataset,
    filter: ProcessFlowNodeFilter,
) -> usize {
    workspace
        .process_flow
        .route
        .nodes
        .iter()
        .filter(|node| process_flow_node_matches(workspace, filter, node))
        .count()
}

pub(crate) fn default_process_flow_node(workspace: &WorkspaceDataset) -> Option<ProcessFlowNodeId> {
    workspace
        .process_flow
        .route
        .nodes
        .first()
        .map(|node| node.id.clone())
}

pub(crate) fn default_process_flow_node_for_filter(
    workspace: &WorkspaceDataset,
    filter: ProcessFlowNodeFilter,
) -> Option<ProcessFlowNodeId> {
    workspace
        .process_flow
        .route
        .nodes
        .iter()
        .find(|node| process_flow_node_matches(workspace, filter, node))
        .map(|node| node.id.clone())
        .or_else(|| default_process_flow_node(workspace))
}

pub(crate) fn selected_process_flow_node<'a>(
    app: &'a GlassworksApp,
) -> Option<&'a layout_model::process_flow::ProcessFlowNode> {
    app.selected_process_node
        .as_ref()
        .and_then(|id| {
            app.workspace
                .process_flow
                .route
                .nodes
                .iter()
                .find(|node| &node.id == id)
        })
        .or_else(|| app.workspace.process_flow.route.nodes.first())
}

pub(crate) fn process_flow_error_count(workspace: &WorkspaceDataset) -> usize {
    workspace
        .process_flow
        .findings()
        .iter()
        .filter(|finding| {
            finding.severity == layout_model::process_flow::ProcessFlowFindingSeverity::Error
        })
        .count()
}

pub(crate) fn default_control_loop(workspace: &WorkspaceDataset) -> Option<ControlLoopId> {
    workspace
        .process_control
        .loops
        .first()
        .map(|loop_definition| loop_definition.id.clone())
}

pub(crate) fn control_loop_matches(
    workspace: &WorkspaceDataset,
    filter: ControlLoopFilter,
    loop_definition: &ControlLoop,
) -> bool {
    match filter {
        ControlLoopFilter::All => true,
        ControlLoopFilter::Active => workspace
            .process_control
            .actions_for_loop(&loop_definition.id)
            .iter()
            .any(|action| {
                !matches!(
                    action.state,
                    ControlActionState::Applied | ControlActionState::Rejected
                )
            }),
        ControlLoopFilter::Attention => workspace
            .process_control
            .actions_for_loop(&loop_definition.id)
            .iter()
            .any(|action| {
                matches!(
                    action.state,
                    ControlActionState::Proposed | ControlActionState::Held
                )
            }),
    }
}

pub(crate) fn default_control_loop_for_filter(
    workspace: &WorkspaceDataset,
    filter: ControlLoopFilter,
) -> Option<ControlLoopId> {
    workspace
        .process_control
        .loops
        .iter()
        .find(|loop_definition| control_loop_matches(workspace, filter, loop_definition))
        .map(|loop_definition| loop_definition.id.clone())
        .or_else(|| default_control_loop(workspace))
}

pub(crate) fn default_control_action(
    workspace: &WorkspaceDataset,
    loop_id: Option<&ControlLoopId>,
) -> Option<ControlActionId> {
    loop_id
        .and_then(|loop_id| {
            workspace
                .process_control
                .actions_for_loop(loop_id)
                .first()
                .map(|action| action.id.clone())
        })
        .or_else(|| {
            workspace
                .process_control
                .actions
                .first()
                .map(|action| action.id.clone())
        })
}

pub(crate) fn selected_control_loop<'a>(
    app: &'a GlassworksApp,
) -> Option<&'a layout_model::process_control::ControlLoop> {
    app.selected_control_loop
        .as_ref()
        .and_then(|id| app.workspace.process_control.loop_by_id(id))
        .filter(|loop_definition| {
            control_loop_matches(
                &app.workspace,
                app.process_control_loop_filter,
                loop_definition,
            )
        })
        .or_else(|| {
            app.workspace
                .process_control
                .loops
                .iter()
                .find(|loop_definition| {
                    control_loop_matches(
                        &app.workspace,
                        app.process_control_loop_filter,
                        loop_definition,
                    )
                })
        })
        .or_else(|| app.workspace.process_control.loops.first())
}

pub(crate) fn filtered_control_loops(app: &GlassworksApp) -> Vec<&ControlLoop> {
    app.workspace
        .process_control
        .loops
        .iter()
        .filter(|loop_definition| {
            control_loop_matches(
                &app.workspace,
                app.process_control_loop_filter,
                loop_definition,
            )
        })
        .collect()
}

pub(crate) fn selected_control_action<'a>(
    app: &'a GlassworksApp,
) -> Option<&'a layout_model::process_control::ControlAction> {
    app.selected_control_action
        .as_ref()
        .and_then(|id| {
            app.workspace
                .process_control
                .actions
                .iter()
                .find(|action| &action.id == id)
        })
        .or_else(|| {
            selected_control_loop(app).and_then(|loop_definition| {
                app.workspace
                    .process_control
                    .actions_for_loop(&loop_definition.id)
                    .first()
                    .copied()
            })
        })
}

pub(crate) fn process_control_action_count(
    app: &GlassworksApp,
    state: ControlActionState,
) -> usize {
    app.workspace
        .process_control
        .actions
        .iter()
        .filter(|action| action.state == state)
        .count()
}

pub(crate) fn process_control_transition_options(
    state: ControlActionState,
) -> &'static [&'static str] {
    match state {
        ControlActionState::Proposed => &["approve", "reject"],
        ControlActionState::Approved => &["apply", "reject"],
        ControlActionState::Held => &["reject"],
        ControlActionState::Rejected | ControlActionState::Applied => &[],
    }
}

pub(crate) fn process_control_transition_label(value: &str) -> &'static str {
    match value {
        "approve" => "Approve",
        "reject" => "Reject",
        "apply" => "Apply",
        _ => "Transition",
    }
}

pub(crate) fn control_run_label(run_index: u32) -> String {
    format!("Run {run_index:02}")
}

pub(crate) fn process_control_action_short_label(action: &ControlAction) -> String {
    control_run_label(action.source.run_index)
}

pub(crate) fn process_control_action_button_label(action: &ControlAction) -> String {
    format!(
        "{} {}",
        process_control_action_short_label(action),
        action.state.label()
    )
}

pub(crate) fn process_control_latest_label(
    point: Option<&ControlTrendPoint>,
    compact_rows: bool,
) -> String {
    point
        .map(|point| {
            if compact_rows {
                format!(
                    "latest {} {}",
                    format_compact_number(point.value),
                    point.unit
                )
            } else {
                format!(
                    "latest {:.3} {} error {:+.3}",
                    point.value, point.unit, point.error
                )
            }
        })
        .unwrap_or_else(|| "No trend samples".to_string())
}

pub(crate) fn process_control_target_label(
    loop_definition: &ControlLoop,
    compact_rows: bool,
) -> String {
    if compact_rows {
        format!(
            "target {} {}",
            format_compact_number(loop_definition.output.target),
            loop_definition.output.unit
        )
    } else {
        format!(
            "target {:.3} +/-{:.3} {}",
            loop_definition.output.target,
            loop_definition.deadband.abs(),
            loop_definition.output.unit
        )
    }
}

pub(crate) fn process_control_loop_button_label(name: &str) -> String {
    match name {
        "Poly etch edge oxide trim" => "Edge oxide trim".to_string(),
        "Contact resistance cleanup" => "Contact resistance".to_string(),
        _ => compact_button_label(name, 18),
    }
}

pub(crate) fn spc_fdc_monitor(workspace: &WorkspaceDataset) -> SpcFdcMonitor {
    let sensor_samples = workspace
        .equipment
        .tools()
        .flat_map(|tool| tool.recent_sensors.iter().cloned())
        .collect::<Vec<_>>();
    let alarms = workspace
        .equipment
        .tools()
        .flat_map(|tool| tool.active_alarms.iter().cloned())
        .collect::<Vec<_>>();
    SpcFdcMonitor::from_fab_context(
        &workspace.yield_analysis.process_measurements,
        &sensor_samples,
        &alarms,
    )
}

pub(crate) fn default_spc_chart(workspace: &WorkspaceDataset) -> Option<String> {
    spc_fdc_monitor(workspace)
        .charts
        .first()
        .map(|chart| chart.id.as_str().to_string())
}

pub(crate) fn default_fdc_trace(workspace: &WorkspaceDataset) -> Option<String> {
    spc_fdc_monitor(workspace)
        .traces
        .first()
        .map(|trace| trace.id.clone())
}

pub(crate) fn selected_spc_chart<'a>(
    app: &'a GlassworksApp,
    monitor: &'a SpcFdcMonitor,
) -> Option<&'a layout_model::spc_fdc::ControlChart> {
    app.selected_spc_chart
        .as_deref()
        .and_then(|id| monitor.charts.iter().find(|chart| chart.id.as_str() == id))
        .or_else(|| monitor.charts.first())
}

pub(crate) fn selected_fdc_trace<'a>(
    app: &'a GlassworksApp,
    monitor: &'a SpcFdcMonitor,
) -> Option<&'a layout_model::spc_fdc::SensorTrace> {
    app.selected_fdc_trace
        .as_deref()
        .and_then(|id| monitor.traces.iter().find(|trace| trace.id == id))
        .or_else(|| monitor.traces.first())
}

pub(crate) fn spc_filtered_finding_count(app: &GlassworksApp, monitor: &SpcFdcMonitor) -> usize {
    monitor
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
        .count()
}

pub(crate) fn default_cross_section_material(workspace: &WorkspaceDataset) -> Option<MaterialId> {
    Some(workspace.cross_section.substrate_material.clone()).or_else(|| {
        workspace
            .cross_section
            .materials
            .first()
            .map(|material| material.id.clone())
    })
}

pub(crate) fn cross_section_snapshot_count(workspace: &WorkspaceDataset) -> usize {
    workspace.cross_section.steps.len() + 1
}

pub(crate) fn cross_section_selected_title(app: &GlassworksApp) -> String {
    if app.cross_section_step == 0 {
        "Starting substrate".to_string()
    } else {
        app.workspace
            .cross_section
            .steps
            .get(app.cross_section_step.saturating_sub(1))
            .map(|step| step.name.clone())
            .unwrap_or_else(|| format!("Step {}", app.cross_section_step))
    }
}

pub(crate) fn maintenance_today(workspace: &WorkspaceDataset) -> FabDate {
    workspace
        .maintenance
        .today
        .unwrap_or(FabDate::new(2026, 5, 11))
}

pub(crate) fn maintenance_work_matches(
    workspace: &WorkspaceDataset,
    filter: MaintenanceWorkFilter,
    tool_id: &EquipmentToolId,
    kind: MaintenanceKind,
    due_state: DueState,
    due_date: FabDate,
) -> bool {
    let today = maintenance_today(workspace);
    let release = workspace.maintenance.release_for_tool(tool_id, today);
    let blocked = !release.state.released_to_production();
    let days_until = today.days_until(due_date);
    match filter {
        MaintenanceWorkFilter::Actionable => blocked || due_state.blocks_release(),
        MaintenanceWorkFilter::Upcoming => due_state != DueState::Overdue && days_until <= 14,
        MaintenanceWorkFilter::Calibration => {
            kind == MaintenanceKind::Calibration
                || release.state == ToolReleaseState::CalibrationLockout
        }
        MaintenanceWorkFilter::Locked => blocked,
        MaintenanceWorkFilter::All => true,
    }
}

pub(crate) fn maintenance_filtered_work_count(
    workspace: &WorkspaceDataset,
    filter: MaintenanceWorkFilter,
) -> usize {
    let today = maintenance_today(workspace);
    workspace
        .maintenance
        .tools
        .iter()
        .flat_map(|tool| {
            tool.schedules.iter().filter(move |schedule| {
                maintenance_work_matches(
                    workspace,
                    filter,
                    &tool.tool_id,
                    schedule.kind,
                    schedule.due_state(today, tool.run_count),
                    schedule.next_due,
                )
            })
        })
        .count()
}

pub(crate) fn maintenance_locked_count(workspace: &WorkspaceDataset) -> usize {
    let today = maintenance_today(workspace);
    workspace
        .maintenance
        .tools
        .iter()
        .filter(|tool| {
            !workspace
                .maintenance
                .release_for_tool(&tool.tool_id, today)
                .state
                .released_to_production()
        })
        .count()
}

pub(crate) fn selected_maintenance_tool<'a>(
    app: &'a GlassworksApp,
) -> Option<&'a layout_model::maintenance::ToolMaintenanceState> {
    app.selected_maintenance_tool
        .as_ref()
        .and_then(|id| app.workspace.maintenance.tool(id))
        .or_else(|| app.workspace.maintenance.tools.first())
}

pub(crate) fn selected_environment_sensor<'a>(
    app: &'a GlassworksApp,
) -> Option<&'a layout_model::environment::EnvironmentSensor> {
    app.selected_environment_sensor
        .as_deref()
        .and_then(|id| {
            app.workspace
                .environment
                .sensors
                .iter()
                .find(|sensor| sensor.id == id)
        })
        .or_else(|| app.workspace.environment.sensors.first())
}

pub(crate) fn environment_sensor_status(app: &GlassworksApp) -> String {
    let Some(sensor) = selected_environment_sensor(app) else {
        return "No sensor".to_string();
    };
    app.workspace
        .environment
        .latest_reading(&sensor.id)
        .map(|reading| {
            let severity = sensor.thresholds.evaluate(reading.value);
            let state = severity.map_or("Nominal", |severity| severity.label());
            format!("{:.2} {} {}", reading.value, sensor.unit, state)
        })
        .unwrap_or_else(|| "No samples".to_string())
}

pub(crate) fn ordered_environment_sensors(app: &GlassworksApp) -> Vec<&EnvironmentSensor> {
    let mut sensors = app.workspace.environment.sensors.iter().collect::<Vec<_>>();
    if app.app_options.domains.environment.show_alarm_sensors_first {
        let alarm_severity_by_sensor = app
            .workspace
            .environment
            .evaluate_alarms()
            .into_iter()
            .filter(|alarm| alarm.active)
            .map(|alarm| (alarm.sensor_id, alarm.severity))
            .collect::<BTreeMap<_, _>>();
        sensors.sort_by(|left, right| {
            let left_severity = alarm_severity_by_sensor.get(&left.id);
            let right_severity = alarm_severity_by_sensor.get(&right.id);
            right_severity
                .cmp(&left_severity)
                .then_with(|| left.id.cmp(&right.id))
        });
    }
    sensors
}

pub(crate) fn equipment_active_alarm_count(tool: &EquipmentTool) -> usize {
    tool.active_alarms
        .iter()
        .filter(|alarm| alarm.active)
        .count()
}

pub(crate) fn equipment_running_count(workspace: &WorkspaceDataset) -> usize {
    workspace
        .equipment
        .tools()
        .filter(|tool| tool.state == EquipmentToolState::Running)
        .count()
}

pub(crate) fn equipment_alarm_count(workspace: &WorkspaceDataset) -> usize {
    workspace
        .equipment
        .tools()
        .map(equipment_active_alarm_count)
        .sum()
}

pub(crate) fn equipment_sample_count(workspace: &WorkspaceDataset) -> usize {
    workspace
        .equipment
        .tools()
        .map(|tool| tool.recent_sensors.len())
        .sum()
}

pub(crate) fn selected_equipment_tool<'a>(app: &'a GlassworksApp) -> Option<&'a EquipmentTool> {
    app.selected_equipment_tool
        .as_ref()
        .and_then(|id| app.workspace.equipment.tool(id))
        .or_else(|| app.workspace.equipment.tools().next())
}

pub(crate) fn workflow_focus_traveler_label(app: &GlassworksApp) -> String {
    let Some(lot_id) = app.workflow_focus_lot.as_deref() else {
        return "No lot".to_string();
    };
    app.workspace
        .mes
        .travelers
        .values()
        .find(|traveler| traveler.lot_id.as_str() == lot_id)
        .map(|traveler| {
            format!(
                "{} {}",
                traveler.status.label(),
                traveler
                    .current_step_id
                    .as_ref()
                    .map(|step| step.as_str())
                    .unwrap_or("complete")
            )
        })
        .unwrap_or_else(|| "No traveler".to_string())
}

pub(crate) fn workflow_focus_yield_label(app: &GlassworksApp) -> String {
    let Some(lot_id) = app.workflow_focus_lot.as_deref() else {
        return "No lot".to_string();
    };
    app.workspace
        .yield_analysis
        .lot_summary(lot_id)
        .map(|summary| format!("{:.1}%", summary.yield_fraction * 100.0))
        .unwrap_or_else(|| "No yield summary".to_string())
}

pub(crate) fn equipment_recipe_summary(tool: &EquipmentTool) -> String {
    if let Some(run) = &tool.active_run {
        return format!(
            "{} {}",
            display_recipe_identifier(run.recipe.recipe_id.as_str()),
            run.status.label()
        );
    }
    if let Some(selection) = &tool.selected_recipe {
        return format!(
            "{} loaded",
            display_recipe_identifier(selection.recipe_id.as_str())
        );
    }
    "No recipe".to_string()
}

pub(crate) fn measurement_kind_slug(kind: MeasurementKind) -> &'static str {
    match kind {
        MeasurementKind::ThicknessNm => "thickness",
        MeasurementKind::SheetResistanceOhmsPerSq => "sheet-r",
        MeasurementKind::CriticalDimensionNm => "cd",
        MeasurementKind::DefectCount => "defects",
        MeasurementKind::PassFail => "pass-fail",
    }
}

pub(crate) fn measurement_kind_from_slug(value: &str) -> Option<MeasurementKind> {
    match value {
        "thickness" => Some(MeasurementKind::ThicknessNm),
        "sheet-r" => Some(MeasurementKind::SheetResistanceOhmsPerSq),
        "cd" => Some(MeasurementKind::CriticalDimensionNm),
        "defects" => Some(MeasurementKind::DefectCount),
        "pass-fail" => Some(MeasurementKind::PassFail),
        _ => None,
    }
}

pub(crate) fn die_coord_label(die: Option<DieCoord>) -> String {
    die.map(|die| format!("C{} R{}", die.column, die.row))
        .unwrap_or_else(|| "None".to_string())
}
