use eframe::egui::{self, Color32, RichText};
use layout_model::process_flow::{
    ProcessFlowEdge, ProcessFlowEdgeKind, ProcessFlowFinding, ProcessFlowFindingSeverity,
    ProcessFlowModel, ProcessFlowNode, ProcessFlowNodeId, ProcessFlowNodeKind,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct ProcessFlowPanel {
    model: ProcessFlowModel,
    selected_node: Option<ProcessFlowNodeId>,
    show_errors_only: bool,
    node_filter: ProcessFlowNodeFilter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessFlowNodeFilter {
    All,
    RecipeSteps,
    Metrology,
    Holds,
    Rework,
}

impl ProcessFlowNodeFilter {
    const ALL: [Self; 5] = [
        Self::All,
        Self::RecipeSteps,
        Self::Metrology,
        Self::Holds,
        Self::Rework,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::RecipeSteps => "Recipes",
            Self::Metrology => "Metrology",
            Self::Holds => "Holds",
            Self::Rework => "Rework",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::All => "Full route traveler",
            Self::RecipeSteps => "Recipe-bound process work",
            Self::Metrology => "Measurement and sample-plan checkpoints",
            Self::Holds => "Engineer signoff and explicit hold points",
            Self::Rework => "Nodes that send or receive rework loops",
        }
    }
}

impl ProcessFlowPanel {
    pub(crate) fn from_model(model: ProcessFlowModel) -> Self {
        let selected_node = model.route.nodes.first().map(|node| node.id.clone());
        Self {
            model,
            selected_node,
            show_errors_only: false,
            node_filter: ProcessFlowNodeFilter::All,
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
        self.compact_route_status_ui(ui, &findings);
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
                let detail = format!(
                    "Owner: {} | {} v{}",
                    self.model.route.owner, self.model.route.id, self.model.route.version
                );
                ui_chrome::module_header(
                    ui,
                    "Process engineering",
                    "Process-flow Designer",
                    &detail,
                    |ui| {
                        if ui.button("Prepare MES-lite route export").clicked() {
                            let mes_route = self.model.export_to_mes_route();
                            *status = format!(
                                "prepared {} MES steps for {}",
                                mes_route.steps.len(),
                                mes_route.id
                            );
                        }
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
                self.filter_bar_ui(ui);
                ui.separator();

                let available_width = ui.available_width();
                if available_width < 760.0 {
                    ui_chrome::section_label(ui, "Route Timeline");
                    self.route_timeline_ui(ui, &findings, 360.0);
                    ui.separator();
                    ui_chrome::section_label(ui, "Selected Step");
                    self.selected_node_ui(ui, &findings);
                    ui.separator();
                    ui_chrome::section_label(ui, "Dependencies");
                    self.dependency_ui(ui);
                    ui.separator();
                    ui_chrome::section_label(ui, "Controls");
                    self.controls_overview_ui(ui);
                    ui.separator();
                    ui_chrome::section_label(ui, "Findings");
                    self.finding_list_ui(ui, &findings, 240.0);
                } else if available_width < 1120.0 {
                    ui.columns(2, |columns| {
                        columns[0].set_min_width(320.0);
                        columns[1].set_min_width(360.0);

                        ui_chrome::section_label(&mut columns[0], "Route Timeline");
                        self.route_timeline_ui(&mut columns[0], &findings, 560.0);
                        columns[0].separator();
                        ui_chrome::section_label(&mut columns[0], "Controls");
                        self.controls_overview_ui(&mut columns[0]);

                        ui_chrome::section_label(&mut columns[1], "Selected Step");
                        self.selected_node_ui(&mut columns[1], &findings);
                        columns[1].separator();
                        ui_chrome::section_label(&mut columns[1], "Dependencies");
                        self.dependency_ui(&mut columns[1]);
                        columns[1].separator();
                        ui_chrome::section_label(&mut columns[1], "Findings");
                        self.finding_list_ui(&mut columns[1], &findings, 260.0);
                    });
                } else {
                    ui.columns(3, |columns| {
                        columns[0].set_min_width(320.0);
                        columns[1].set_min_width(360.0);
                        columns[2].set_min_width(320.0);

                        ui_chrome::section_label(&mut columns[0], "Route Timeline");
                        self.route_timeline_ui(&mut columns[0], &findings, 600.0);

                        ui_chrome::section_label(&mut columns[1], "Selected Step");
                        self.selected_node_ui(&mut columns[1], &findings);

                        ui_chrome::section_label(&mut columns[2], "Dependencies");
                        self.dependency_ui(&mut columns[2]);
                        columns[2].separator();
                        ui_chrome::section_label(&mut columns[2], "Controls");
                        self.controls_overview_ui(&mut columns[2]);
                        columns[2].separator();
                        ui_chrome::section_label(&mut columns[2], "Findings");
                        self.finding_list_ui(&mut columns[2], &findings, 240.0);
                    });
                }

                ui.separator();
                self.mes_export_preview_ui(ui);
            });
    }

    fn summary_ui(&self, ui: &mut egui::Ui, findings: &[ProcessFlowFinding]) {
        let recipe_required = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.kind.requires_recipe())
            .count();
        let recipe_ready = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.kind.requires_recipe())
            .filter(|node| node.recipe.is_some() && node.has_tool_coverage())
            .count();
        let checkpoint_count = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .count();
        let hold_points = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point)
            .count();
        let rework_edges = self
            .model
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            .count();
        let errors = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        let warnings = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Warning)
            .count();
        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Route graph",
                    format!(
                        "{} nodes / {} edges",
                        self.model.route.nodes.len(),
                        self.model.route.edges.len()
                    ),
                    "topology",
                    Tone::Neutral,
                ),
                (
                    "Recipes",
                    format!("{recipe_ready}/{recipe_required} ready"),
                    "bound and tool-covered",
                    if recipe_ready == recipe_required {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                ),
                (
                    "Metrology",
                    format!("{checkpoint_count} checkpoints"),
                    "sample plans",
                    if checkpoint_count > 0 {
                        Tone::Success
                    } else {
                        Tone::Warning
                    },
                ),
                (
                    "Holds / rework",
                    format!("{hold_points} holds / {rework_edges} paths"),
                    "release controls",
                    if hold_points > 0 || rework_edges > 0 {
                        Tone::Warning
                    } else {
                        Tone::Neutral
                    },
                ),
                (
                    "Validation",
                    if errors == 0 {
                        format!("{warnings} warnings")
                    } else {
                        format!("{errors} errors")
                    },
                    "recipe and eligible-tool coverage",
                    if errors == 0 {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                ),
            ],
        );
    }

    fn compact_route_status_ui(&self, ui: &mut egui::Ui, findings: &[ProcessFlowFinding]) {
        let errors = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        let recipe_required = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.kind.requires_recipe())
            .count();
        let checkpoints = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .count();
        let holds = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point)
            .count();

        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                if errors == 0 { "exportable" } else { "blocked" },
                if errors == 0 {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            );
            ui_chrome::status_pill(ui, &format!("{recipe_required} recipes"), Tone::Info);
            ui_chrome::status_pill(ui, &format!("{checkpoints} checkpoints"), Tone::Success);
            if holds > 0 {
                ui_chrome::status_pill(ui, &format!("{holds} holds"), Tone::Warning);
            }
        });
    }

    fn filter_bar_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Filter").strong());
            for filter in ProcessFlowNodeFilter::ALL {
                let count = self.count_nodes_for_filter(filter);
                let selected = self.node_filter == filter;
                if ui
                    .selectable_label(selected, format!("{} ({count})", filter.label()))
                    .clicked()
                {
                    self.node_filter = filter;
                }
            }
        });
        ui.small(RichText::new(self.node_filter.detail()).color(ui.visuals().weak_text_color()));
    }

    fn route_timeline_ui(
        &mut self,
        ui: &mut egui::Ui,
        findings: &[ProcessFlowFinding],
        max_height: f32,
    ) {
        let visible_nodes = self.visible_nodes();
        if visible_nodes.is_empty() {
            ui_chrome::empty_state(ui, "No route steps match this filter");
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("process_flow_route_timeline")
            .max_height(max_height)
            .show(ui, |ui| {
                for (index, node) in visible_nodes {
                    let selected = self.selected_node.as_ref() == Some(&node.id);
                    let issue_count = self.node_findings(&node.id, findings).len();
                    let incoming_count = self.incoming_edges(&node.id).len();
                    let outgoing = self.outgoing_edges(&node.id);
                    let outgoing_count = outgoing.len();
                    let rework_count = self.rework_edge_count_for_node(&node.id);
                    let fill = if selected {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().faint_bg_color
                    };
                    let stroke_color = if selected {
                        Tone::Info.color()
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke.color
                    };
                    let mut open_node = false;

                    egui::Frame::new()
                        .fill(fill)
                        .stroke(egui::Stroke::new(1.0, stroke_color))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(
                                        selected,
                                        format!("{:02}. {}", index + 1, node.name),
                                    )
                                    .clicked()
                                {
                                    open_node = true;
                                }
                                ui.colored_label(kind_color(node.kind), node.kind.label());
                            });
                            ui.horizontal_wrapped(|ui| {
                                if node.hold_point {
                                    ui_chrome::status_pill(ui, "hold", Tone::Warning);
                                }
                                if node.measurement_checkpoint.is_some() {
                                    ui_chrome::status_pill(ui, "checkpoint", Tone::Success);
                                }
                                if rework_count > 0 {
                                    ui_chrome::status_pill(ui, "rework", Tone::Danger);
                                }
                                if issue_count > 0 {
                                    ui_chrome::status_pill(
                                        ui,
                                        &format!("{issue_count} findings"),
                                        Tone::Danger,
                                    );
                                }
                            });
                            ui.small(format!(
                                "{} | {} in / {} out",
                                node.area, incoming_count, outgoing_count
                            ));
                            if node.kind.requires_recipe() {
                                let recipe = node
                                    .recipe
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or_else(|| "unassigned recipe".to_string());
                                ui.small(format!("Recipe: {recipe}"));
                            }
                            if let Some(next_sequence) = outgoing
                                .iter()
                                .find(|edge| edge.kind == ProcessFlowEdgeKind::Sequence)
                            {
                                ui.small(format!("Next: {}", self.node_label(&next_sequence.to)));
                            }
                        });

                    if open_node {
                        self.selected_node = Some(node.id.clone());
                    }
                    ui.add_space(4.0);
                }
            });
    }

    fn selected_node_ui(&self, ui: &mut egui::Ui, findings: &[ProcessFlowFinding]) {
        let Some(node) = self.selected_node() else {
            ui_chrome::empty_state(ui, "No node selected");
            return;
        };

        ui.strong(&node.name);
        ui.small(format!("{} | {}", node.id, node.area));
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, node.kind.label(), tone_for_kind(node.kind));
            if node.kind.requires_recipe() {
                ui_chrome::status_pill(ui, "MES step", Tone::Info);
            }
            if node.hold_point {
                ui_chrome::status_pill(ui, "hold point", Tone::Warning);
            }
            if self
                .outgoing_edges(&node.id)
                .iter()
                .any(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            {
                ui_chrome::status_pill(ui, "rework allowed", Tone::Danger);
            }
        });

        ui.separator();
        ui_chrome::section_label(ui, "Recipe and Equipment");
        if node.kind.requires_recipe() {
            if let Some(recipe) = &node.recipe {
                ui.label(format!("Recipe: {recipe}"));
            } else {
                ui.colored_label(Color32::from_rgb(226, 96, 96), "Recipe: unassigned");
            }
            ui.horizontal_wrapped(|ui| {
                ui_chrome::status_pill(
                    ui,
                    if node.recipe.is_some() {
                        "recipe bound"
                    } else {
                        "recipe missing"
                    },
                    if node.recipe.is_some() {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                );
                ui_chrome::status_pill(
                    ui,
                    if node.has_tool_coverage() {
                        "tool coverage"
                    } else {
                        "tools missing"
                    },
                    if node.has_tool_coverage() {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                );
            });
        } else {
            ui_chrome::muted(ui, "No recipe required for this route node");
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

        ui.separator();
        ui_chrome::section_label(ui, "Material Contract");
        io_list(ui, "Inputs", &node.expected_inputs);
        io_list(ui, "Outputs", &node.expected_outputs);

        if let Some(checkpoint) = &node.measurement_checkpoint {
            ui.separator();
            ui_chrome::section_label(ui, "Measurement Checkpoint");
            ui.label(&checkpoint.measurement_name);
            ui.small(format!("Target: {}", checkpoint.target));
            ui.small(format!("Sample plan: {}", checkpoint.sample_plan));
        }

        ui.separator();
        ui_chrome::section_label(ui, "Dependency State");
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &format!("{} prerequisites", self.incoming_edges(&node.id).len()),
                Tone::Neutral,
            );
            ui_chrome::status_pill(
                ui,
                &format!("{} outcomes", self.outgoing_edges(&node.id).len()),
                Tone::Neutral,
            );
        });

        let step_findings = self.node_findings(&node.id, findings);
        ui.separator();
        ui_chrome::section_label(ui, "Selected-step Findings");
        if step_findings.is_empty() {
            ui_chrome::status_pill(ui, "no selected-step findings", Tone::Success);
        } else {
            for finding in step_findings {
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(finding_color(finding.severity), finding.severity.label());
                        ui.strong(&finding.code);
                    });
                    ui.label(&finding.message);
                });
            }
        }

        if !node.notes.is_empty() {
            ui.separator();
            ui.label(RichText::new(&node.notes).color(ui.visuals().weak_text_color()));
        }
    }

    fn dependency_ui(&mut self, ui: &mut egui::Ui) {
        let selected_id = self.selected_node.clone();
        let Some(selected_id) = selected_id else {
            ui_chrome::empty_state(ui, "Select a node to inspect dependencies");
            return;
        };

        ui.small(format!("For {}", self.node_label(&selected_id)));

        let outgoing = self
            .outgoing_edges(&selected_id)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let incoming = self
            .incoming_edges(&selected_id)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        ui.label(RichText::new("Prerequisites").strong());
        if incoming.is_empty() {
            ui_chrome::muted(ui, "No upstream dependency");
        }
        for edge in &incoming {
            self.edge_dependency_row_ui(ui, edge, false);
        }

        ui.separator();
        ui.label(RichText::new("Outcomes").strong());
        if outgoing.is_empty() {
            ui_chrome::muted(ui, "No downstream outcome");
        }
        for edge in &outgoing {
            self.edge_dependency_row_ui(ui, edge, true);
        }
    }

    fn edge_dependency_row_ui(
        &mut self,
        ui: &mut egui::Ui,
        edge: &ProcessFlowEdge,
        outgoing: bool,
    ) {
        let target_id = if outgoing { &edge.to } else { &edge.from };
        let target_name = self
            .model
            .route
            .node(target_id)
            .map(|node| node.name.clone())
            .unwrap_or_else(|| "missing node".to_string());
        ui.group(|ui| {
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
            if edge.condition.is_empty() {
                ui.small("Condition: always");
            } else {
                ui.small(format!("Condition: {}", edge.condition));
            }
        });
    }

    fn controls_overview_ui(&mut self, ui: &mut egui::Ui) {
        let checkpoints = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let holds = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point || node.kind == ProcessFlowNodeKind::Hold)
            .cloned()
            .collect::<Vec<_>>();
        let rework_edges = self
            .model
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            .cloned()
            .collect::<Vec<_>>();

        ui.label(RichText::new("Metrology checkpoints").strong());
        if checkpoints.is_empty() {
            ui_chrome::empty_state(ui, "No measurement checkpoints");
        }
        for node in checkpoints {
            if let Some(checkpoint) = &node.measurement_checkpoint {
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(node.id.to_string()).clicked() {
                            self.selected_node = Some(node.id.clone());
                        }
                        ui.strong(&checkpoint.measurement_name);
                    });
                    ui.small(format!("Step: {}", node.name));
                    ui.small(format!("Target: {}", checkpoint.target));
                    ui.small(format!("Sample plan: {}", checkpoint.sample_plan));
                });
            }
        }

        ui.separator();
        ui.label(RichText::new("Holds and rework").strong());
        if holds.is_empty() && rework_edges.is_empty() {
            ui_chrome::empty_state(ui, "No holds or rework paths");
        }
        for node in holds {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(node.id.to_string()).clicked() {
                        self.selected_node = Some(node.id.clone());
                    }
                    ui_chrome::status_pill(ui, "hold", Tone::Warning);
                    ui.strong(&node.name);
                });
                ui.small(format!("Area: {}", node.area));
                if !node.notes.is_empty() {
                    ui.small(&node.notes);
                }
            });
        }
        for edge in rework_edges {
            let from_label = self.node_label(&edge.from);
            let to_label = self.node_label(&edge.to);
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(edge.from.to_string()).clicked() {
                        self.selected_node = Some(edge.from.clone());
                    }
                    ui_chrome::status_pill(ui, "rework", Tone::Danger);
                    ui.label(format!("{from_label} -> {to_label}"));
                });
                if !edge.condition.is_empty() {
                    ui.small(format!("Condition: {}", edge.condition));
                }
            });
        }
    }

    fn finding_list_ui(
        &mut self,
        ui: &mut egui::Ui,
        findings: &[ProcessFlowFinding],
        max_height: f32,
    ) {
        let visible = findings
            .iter()
            .filter(|finding| {
                !self.show_errors_only || finding.severity == ProcessFlowFindingSeverity::Error
            })
            .collect::<Vec<_>>();
        if visible.is_empty() {
            ui_chrome::empty_state(
                ui,
                if self.show_errors_only {
                    "No error findings"
                } else {
                    "No validation findings"
                },
            );
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
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("Open node").clicked() {
                                    self.selected_node = Some(node_id.clone());
                                }
                                ui.small(format!("Node: {node_id}"));
                            });
                        }
                        if let Some(edge_id) = &finding.edge_id {
                            ui.small(format!("Edge: {edge_id}"));
                        }
                    });
                }
            });
    }

    fn mes_export_preview_ui(&self, ui: &mut egui::Ui) {
        let mes_route = self.model.export_to_mes_route();
        ui_chrome::section_label(ui, "MES-lite Export Preview");
        egui::ScrollArea::horizontal()
            .id_salt("process_flow_mes_export_scroll")
            .show(ui, |ui| {
                egui::Grid::new("process_flow_mes_export_grid")
                    .striped(true)
                    .min_col_width(80.0)
                    .show(ui, |ui| {
                        ui.strong("Seq");
                        ui.strong("Step");
                        ui.strong("Area");
                        ui.strong("Class");
                        ui.strong("Recipe");
                        ui.strong("Tools");
                        ui.strong("Controls");
                        ui.end_row();
                        for step in &mes_route.steps {
                            let mut controls = Vec::new();
                            if step.signoff_required {
                                controls.push("signoff");
                            }
                            if step.rework_allowed {
                                controls.push("rework");
                            }
                            ui.label(step.sequence.to_string());
                            ui.label(&step.name);
                            ui.label(&step.area);
                            ui.label(step.required_tool_class.label());
                            ui.label(step.required_recipe.as_str());
                            ui.label(
                                step.eligible_tools
                                    .iter()
                                    .map(|tool| tool.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            );
                            ui.label(if controls.is_empty() {
                                "standard".to_string()
                            } else {
                                controls.join(", ")
                            });
                            ui.end_row();
                        }
                    });
            });
    }

    fn visible_nodes(&self) -> Vec<(usize, ProcessFlowNode)> {
        self.model
            .route
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| self.node_matches_filter(self.node_filter, node))
            .map(|(index, node)| (index, node.clone()))
            .collect()
    }

    fn count_nodes_for_filter(&self, filter: ProcessFlowNodeFilter) -> usize {
        self.model
            .route
            .nodes
            .iter()
            .filter(|node| self.node_matches_filter(filter, node))
            .count()
    }

    fn node_matches_filter(&self, filter: ProcessFlowNodeFilter, node: &ProcessFlowNode) -> bool {
        match filter {
            ProcessFlowNodeFilter::All => true,
            ProcessFlowNodeFilter::RecipeSteps => {
                node.kind.requires_recipe() || node.recipe.is_some()
            }
            ProcessFlowNodeFilter::Metrology => {
                node.kind == ProcessFlowNodeKind::Measurement
                    || node.measurement_checkpoint.is_some()
            }
            ProcessFlowNodeFilter::Holds => {
                node.hold_point || node.kind == ProcessFlowNodeKind::Hold
            }
            ProcessFlowNodeFilter::Rework => self.model.route.edges.iter().any(|edge| {
                edge.kind == ProcessFlowEdgeKind::Rework
                    && (edge.from == node.id || edge.to == node.id)
            }),
        }
    }

    fn incoming_edges(&self, node_id: &ProcessFlowNodeId) -> Vec<&ProcessFlowEdge> {
        self.model
            .route
            .edges
            .iter()
            .filter(|edge| &edge.to == node_id)
            .collect()
    }

    fn outgoing_edges(&self, node_id: &ProcessFlowNodeId) -> Vec<&ProcessFlowEdge> {
        self.model
            .route
            .edges
            .iter()
            .filter(|edge| &edge.from == node_id)
            .collect()
    }

    fn rework_edge_count_for_node(&self, node_id: &ProcessFlowNodeId) -> usize {
        self.model
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            .filter(|edge| &edge.from == node_id || &edge.to == node_id)
            .count()
    }

    fn node_findings<'a>(
        &self,
        node_id: &ProcessFlowNodeId,
        findings: &'a [ProcessFlowFinding],
    ) -> Vec<&'a ProcessFlowFinding> {
        findings
            .iter()
            .filter(|finding| finding.node_id.as_ref() == Some(node_id))
            .collect()
    }

    fn node_label(&self, node_id: &ProcessFlowNodeId) -> String {
        self.model
            .route
            .node(node_id)
            .map(|node| format!("{} {}", node.id, node.name))
            .unwrap_or_else(|| format!("{node_id} missing node"))
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

fn edge_color(kind: ProcessFlowEdgeKind) -> Color32 {
    match kind {
        ProcessFlowEdgeKind::Sequence => Tone::Info.color(),
        ProcessFlowEdgeKind::Branch => Tone::Warning.color(),
        ProcessFlowEdgeKind::Rework => Tone::Danger.color(),
    }
}

fn finding_color(severity: ProcessFlowFindingSeverity) -> Color32 {
    match severity {
        ProcessFlowFindingSeverity::Info => Tone::Info.color(),
        ProcessFlowFindingSeverity::Warning => Tone::Warning.color(),
        ProcessFlowFindingSeverity::Error => Tone::Danger.color(),
    }
}
