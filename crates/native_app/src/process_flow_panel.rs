use eframe::egui::{self, Color32, RichText};
use layout_model::process_flow::{
    ProcessFlowEdge, ProcessFlowFinding, ProcessFlowFindingSeverity, ProcessFlowModel,
    ProcessFlowNode, ProcessFlowNodeId, ProcessFlowNodeKind,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct ProcessFlowPanel {
    model: ProcessFlowModel,
    selected_node: Option<ProcessFlowNodeId>,
    show_errors_only: bool,
}

impl ProcessFlowPanel {
    pub(crate) fn from_model(model: ProcessFlowModel) -> Self {
        let selected_node = model.route.nodes.first().map(|node| node.id.clone());
        Self {
            model,
            selected_node,
            show_errors_only: false,
        }
    }

    pub(crate) fn model(&self) -> &ProcessFlowModel {
        &self.model
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let findings = self.model.findings();
        let error_count = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        ui_chrome::section_label(ui, "Process Flow");
        ui.label(&self.model.route.name);
        ui.small(format!(
            "{}  v{}",
            self.model.route.id, self.model.route.version
        ));
        ui.separator();

        ui_chrome::section_label(ui, "MES Export");
        ui.label(format!("Route: {}", self.model.route.mes_route_id));
        ui.label(format!("Mask: {}", self.model.route.mask_design_id));
        ui.label(format!("Layout: {}", self.model.route.layout_revision));
        if ui.button("Prepare MES-lite route export").clicked() {
            let mes_route = self.model.export_to_mes_route();
            *status = format!(
                "prepared {} MES steps for {}",
                mes_route.steps.len(),
                mes_route.id
            );
        }

        ui.separator();
        ui_chrome::section_label(ui, "Validation");
        ui_chrome::status_pill(
            ui,
            if error_count == 0 {
                "exportable"
            } else {
                "blocked"
            },
            if error_count == 0 {
                Tone::Success
            } else {
                Tone::Danger
            },
        );
        ui.checkbox(&mut self.show_errors_only, "Errors only");
        self.finding_list_ui(ui, &findings, 180.0);

        ui.separator();
        ui_chrome::section_label(ui, "Version History");
        for version in self.model.versions.iter().rev().take(4) {
            ui.group(|ui| {
                ui.strong(format!("v{} {}", version.version, version.timestamp));
                ui.small(format!(
                    "{} nodes / {} edges by {}",
                    version.node_count, version.edge_count, version.author
                ));
                ui.label(&version.change_note);
            });
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let findings = self.model.findings();
        egui::ScrollArea::vertical()
            .id_salt("process_flow_dashboard")
            .show(ui, |ui| {
                let detail = format!("Owner: {}", self.model.route.owner);
                ui_chrome::module_header(
                    ui,
                    "Process engineering",
                    "Process-flow Designer",
                    &detail,
                    |ui| {
                        if ui.button("Validate").clicked() {
                            let error_count = findings
                                .iter()
                                .filter(|finding| {
                                    finding.severity == ProcessFlowFindingSeverity::Error
                                })
                                .count();
                            *status =
                                format!("process flow validation found {} issue(s)", error_count);
                        }
                    },
                );

                self.summary_ui(ui, &findings);
                ui.separator();

                ui.columns(3, |columns| {
                    columns[0].set_min_width(250.0);
                    columns[1].set_min_width(280.0);
                    columns[2].set_min_width(280.0);

                    ui_chrome::section_label(&mut columns[0], "Nodes");
                    self.node_list_ui(&mut columns[0]);

                    ui_chrome::section_label(&mut columns[1], "Selected Step");
                    self.selected_node_ui(&mut columns[1]);

                    ui_chrome::section_label(&mut columns[2], "Connections");
                    self.connection_ui(&mut columns[2]);
                    columns[2].separator();
                    ui_chrome::section_label(&mut columns[2], "Findings");
                    self.finding_list_ui(&mut columns[2], &findings, 240.0);
                });

                ui.separator();
                self.mes_export_preview_ui(ui);
            });
    }

    fn summary_ui(&self, ui: &mut egui::Ui, findings: &[ProcessFlowFinding]) {
        let mes_steps = self.model.export_to_mes_route().steps.len();
        let hold_points = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point)
            .count();
        let errors = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        ui.horizontal_wrapped(|ui| {
            ui_chrome::metric_tile(
                ui,
                "Route graph",
                format!("{} nodes", self.model.route.nodes.len()),
                &format!("{} connections", self.model.route.edges.len()),
            );
            ui_chrome::metric_tile(
                ui,
                "MES route",
                format!("{mes_steps} steps"),
                &self.model.route.mes_route_id,
            );
            ui_chrome::metric_tile(
                ui,
                "Controls",
                format!("{hold_points} holds"),
                "signoff and branch checkpoints",
            );
            ui_chrome::metric_tile_tone(
                ui,
                "Validation",
                if errors == 0 {
                    "Ready".to_string()
                } else {
                    format!("{errors} errors")
                },
                "recipe and eligible-tool coverage",
                if errors == 0 {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            );
        });
    }

    fn node_list_ui(&mut self, ui: &mut egui::Ui) {
        for node in &self.model.route.nodes {
            let selected = self.selected_node.as_ref() == Some(&node.id);
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(selected, format!("{} {}", node.id, node.name))
                    .clicked()
                {
                    self.selected_node = Some(node.id.clone());
                }
                ui.colored_label(kind_color(node.kind), node.kind.label());
            });
            ui.small(format!(
                "{} / {}",
                node.area,
                node.recipe
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "no recipe".to_string())
            ));
            ui.add_space(4.0);
        }
    }

    fn selected_node_ui(&self, ui: &mut egui::Ui) {
        let Some(node) = self.selected_node() else {
            ui_chrome::empty_state(ui, "No node selected");
            return;
        };

        ui.strong(&node.name);
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, node.kind.label(), tone_for_kind(node.kind));
            if node.hold_point {
                ui_chrome::status_pill(ui, "hold point", Tone::Warning);
            }
        });
        ui.label(format!("Area: {}", node.area));
        if let Some(recipe) = &node.recipe {
            ui.label(format!("Recipe: {recipe}"));
        } else {
            ui.colored_label(Color32::from_rgb(226, 96, 96), "Recipe: unassigned");
        }
        if !node.allowed_tool_classes.is_empty() {
            ui.label(format!(
                "Allowed classes: {}",
                node.allowed_tool_classes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !node.eligible_tools.is_empty() {
            ui.label(format!(
                "Eligible tools: {}",
                node.eligible_tools.join(", ")
            ));
        }

        io_list(ui, "Inputs", &node.expected_inputs);
        io_list(ui, "Outputs", &node.expected_outputs);

        if let Some(checkpoint) = &node.measurement_checkpoint {
            ui.separator();
            ui_chrome::section_label(ui, "Measurement Checkpoint");
            ui.label(&checkpoint.measurement_name);
            ui.small(format!("Target: {}", checkpoint.target));
            ui.small(format!("Sample plan: {}", checkpoint.sample_plan));
        }
        if !node.notes.is_empty() {
            ui.separator();
            ui.label(RichText::new(&node.notes).color(ui.visuals().weak_text_color()));
        }
    }

    fn connection_ui(&mut self, ui: &mut egui::Ui) {
        let selected_id = self.selected_node.clone();
        let Some(selected_id) = selected_id else {
            ui_chrome::empty_state(ui, "Select a node to inspect connections");
            return;
        };

        let outgoing = self
            .model
            .route
            .edges
            .iter()
            .filter(|edge| edge.from == selected_id)
            .cloned()
            .collect::<Vec<_>>();
        let incoming = self
            .model
            .route
            .edges
            .iter()
            .filter(|edge| edge.to == selected_id)
            .cloned()
            .collect::<Vec<_>>();

        ui.label(RichText::new("Incoming").strong());
        if incoming.is_empty() {
            ui_chrome::muted(ui, "none");
        }
        for edge in &incoming {
            self.edge_row_ui(ui, edge, false);
        }

        ui.separator();
        ui.label(RichText::new("Outgoing").strong());
        if outgoing.is_empty() {
            ui_chrome::muted(ui, "none");
        }
        for edge in &outgoing {
            self.edge_row_ui(ui, edge, true);
        }
    }

    fn edge_row_ui(&mut self, ui: &mut egui::Ui, edge: &ProcessFlowEdge, outgoing: bool) {
        let target_id = if outgoing { &edge.to } else { &edge.from };
        let target_name = self
            .model
            .route
            .node(target_id)
            .map(|node| node.name.as_str())
            .unwrap_or("missing node");
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if outgoing { "Open next" } else { "Open prior" })
                .clicked()
            {
                self.selected_node = Some(target_id.clone());
            }
            ui.colored_label(edge_color(edge.kind), edge.kind.label());
            ui.label(format!("{} {}", target_id, target_name));
        });
        if !edge.condition.is_empty() {
            ui.small(&edge.condition);
        }
    }

    fn finding_list_ui(&self, ui: &mut egui::Ui, findings: &[ProcessFlowFinding], max_height: f32) {
        let visible = findings
            .iter()
            .filter(|finding| {
                !self.show_errors_only || finding.severity == ProcessFlowFindingSeverity::Error
            })
            .collect::<Vec<_>>();
        if visible.is_empty() {
            ui_chrome::empty_state(ui, "No validation findings");
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("process_flow_findings")
            .max_height(max_height)
            .show(ui, |ui| {
                for finding in visible {
                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                finding_color(finding.severity),
                                finding.severity.label(),
                            );
                            ui.strong(&finding.code);
                        });
                        ui.label(&finding.message);
                        if let Some(node_id) = &finding.node_id {
                            ui.small(format!("node {node_id}"));
                        }
                    });
                }
            });
    }

    fn mes_export_preview_ui(&self, ui: &mut egui::Ui) {
        let mes_route = self.model.export_to_mes_route();
        ui_chrome::section_label(ui, "MES-lite Export Preview");
        egui::Grid::new("process_flow_mes_export_grid")
            .striped(true)
            .min_col_width(80.0)
            .show(ui, |ui| {
                ui.strong("Seq");
                ui.strong("Step");
                ui.strong("Class");
                ui.strong("Recipe");
                ui.strong("Tools");
                ui.end_row();
                for step in &mes_route.steps {
                    ui.label(step.sequence.to_string());
                    ui.label(&step.name);
                    ui.label(step.required_tool_class.label());
                    ui.label(step.required_recipe.as_str());
                    ui.label(
                        step.eligible_tools
                            .iter()
                            .map(|tool| tool.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    ui.end_row();
                }
            });
    }

    fn selected_node(&self) -> Option<&ProcessFlowNode> {
        self.selected_node
            .as_ref()
            .and_then(|id| self.model.route.node(id))
            .or_else(|| self.model.route.nodes.first())
    }

    fn ensure_selection(&mut self) {
        if self
            .selected_node
            .as_ref()
            .and_then(|id| self.model.route.node(id))
            .is_none()
        {
            self.selected_node = self.model.route.nodes.first().map(|node| node.id.clone());
        }
    }
}

fn io_list(ui: &mut egui::Ui, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    ui.separator();
    ui.label(RichText::new(label).strong());
    for value in values {
        ui.label(format!("- {value}"));
    }
}

fn tone_for_kind(kind: ProcessFlowNodeKind) -> Tone {
    match kind {
        ProcessFlowNodeKind::Start | ProcessFlowNodeKind::End => Tone::Neutral,
        ProcessFlowNodeKind::Operation => Tone::Info,
        ProcessFlowNodeKind::Measurement => Tone::Success,
        ProcessFlowNodeKind::Hold => Tone::Warning,
    }
}

fn kind_color(kind: ProcessFlowNodeKind) -> Color32 {
    tone_for_kind(kind).color()
}

fn edge_color(kind: layout_model::process_flow::ProcessFlowEdgeKind) -> Color32 {
    match kind {
        layout_model::process_flow::ProcessFlowEdgeKind::Sequence => Tone::Info.color(),
        layout_model::process_flow::ProcessFlowEdgeKind::Branch => Tone::Warning.color(),
        layout_model::process_flow::ProcessFlowEdgeKind::Rework => Tone::Danger.color(),
    }
}

fn finding_color(severity: ProcessFlowFindingSeverity) -> Color32 {
    match severity {
        ProcessFlowFindingSeverity::Info => Tone::Info.color(),
        ProcessFlowFindingSeverity::Warning => Tone::Warning.color(),
        ProcessFlowFindingSeverity::Error => Tone::Danger.color(),
    }
}
