use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, vec2,
};
use layout_model::cross_section::{
    CrossSectionProcess, CrossSectionSegment, CrossSectionSnapshot, MaskOpening, MaterialId,
    ProcessStepKind,
};

use crate::{
    operad_sidecar::{SidecarRow, SidecarSection, render_sidecar_interactive},
    ui_chrome::{self, Tone},
};

const OPERAD_ACTION_SELECT_STEP: &str = "cross_section.action.select_step.";

pub(crate) struct CrossSectionPanel {
    process: CrossSectionProcess,
    snapshots: Vec<CrossSectionSnapshot>,
    selected_step: usize,
    selected_material: Option<MaterialId>,
    show_mask_overlay: bool,
    show_dimension_guides: bool,
    show_risk_cues: bool,
}

impl CrossSectionPanel {
    pub(crate) fn sample() -> Self {
        Self::from_process(CrossSectionProcess::sample_sequence())
    }

    pub(crate) fn from_process(process: CrossSectionProcess) -> Self {
        let snapshots = process.simulate();
        let selected_material = Some(process.substrate_material.clone());
        Self {
            process,
            snapshots,
            selected_step: 0,
            selected_material,
            show_mask_overlay: true,
            show_dimension_guides: true,
            show_risk_cues: true,
        }
    }

    pub(crate) fn process(&self) -> &CrossSectionProcess {
        &self.process
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.clamp_selection();
        if self.operad_context_ui(ui).is_err() {
            self.egui_context_ui(ui);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        self.clamp_selection();
        let Some(snapshot) = self.selected_snapshot().cloned() else {
            return render_sidecar_interactive(
                ui,
                "cross_section.context",
                &[
                    SidecarSection::new("Process Cross-Section")
                        .empty("No process sequence loaded"),
                ],
            )
            .map(|_| ());
        };

        let risks = self.risk_findings(&snapshot);
        let surface = surface_summary(&self.process, &snapshot);
        let selected_material = self
            .selected_material_id(&snapshot)
            .map(|material| self.material_name(&material))
            .unwrap_or_else(|| "n/a".to_string());
        let mut sections = Vec::new();
        sections.push(
            SidecarSection::new("Process Cross-Section")
                .row(SidecarRow::new(
                    format!("Step {}: {}", snapshot.step_index, snapshot.title),
                    snapshot.detail.clone(),
                    Tone::Info,
                ))
                .row(SidecarRow::new(
                    self.step_kind_label(snapshot.step_index),
                    risk_summary_label(&risks),
                    risk_summary_tone(&risks),
                ))
                .row(SidecarRow::new(
                    "Surface",
                    format!(
                        "{} avg / {} range",
                        format_um(surface.average_um),
                        format_um(surface.range_um)
                    ),
                    surface_range_tone(surface.range_um),
                )),
        );

        let mut sequence = SidecarSection::new("Step Sequence").empty("No process steps");
        for step_index in 0..self.snapshots.len() {
            let title = if step_index == 0 {
                "Starting substrate".to_string()
            } else {
                self.process
                    .steps
                    .get(step_index - 1)
                    .map(|step| step.name.clone())
                    .unwrap_or_else(|| format!("Step {step_index}"))
            };
            sequence = sequence.row(
                SidecarRow::new(
                    format!("{step_index}. {title}"),
                    if step_index == self.selected_step {
                        "selected step"
                    } else {
                        "inspect step"
                    },
                    if step_index == self.selected_step {
                        Tone::Info
                    } else {
                        Tone::Neutral
                    },
                )
                .selected(step_index == self.selected_step)
                .action(format!("{OPERAD_ACTION_SELECT_STEP}{step_index}|context")),
            );
        }
        sections.push(sequence);

        sections.push(
            SidecarSection::new("Selected Material")
                .row(SidecarRow::new(
                    "Material",
                    selected_material,
                    Tone::Neutral,
                ))
                .row(SidecarRow::new(
                    "Mask openings",
                    format!("{} active openings", snapshot.active_mask.len()),
                    if snapshot.active_mask.is_empty() {
                        Tone::Warning
                    } else {
                        Tone::Neutral
                    },
                ))
                .row(SidecarRow::new(
                    "Segments",
                    format!("{} rendered bands", snapshot.segments.len()),
                    Tone::Neutral,
                )),
        );

        let mut risk_section =
            SidecarSection::new("Defects / Risks").empty("No significant cross-section risk cues");
        for risk in risks.iter().take(5) {
            risk_section = risk_section.row(SidecarRow::new(
                risk.title.clone(),
                risk.detail.clone(),
                risk.tone,
            ));
        }
        sections.push(risk_section);

        if let Some(action) = render_sidecar_interactive(ui, "cross_section.context", &sections)?
            && let Some(step) = action
                .strip_prefix(OPERAD_ACTION_SELECT_STEP)
                .and_then(|value| value.split_once('|').map(|(step, _)| step).or(Some(value)))
                .and_then(|value| value.parse::<usize>().ok())
        {
            self.select_step(step);
        }
        Ok(())
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui) {
        self.clamp_selection();
        ui_chrome::section_label(ui, "Process Cross-Section");
        let Some(snapshot) = self.selected_snapshot().cloned() else {
            ui_chrome::empty_state(ui, "No process sequence loaded");
            return;
        };

        let risks = self.risk_findings(&snapshot);
        ui.label(
            RichText::new(format!("Step {}: {}", snapshot.step_index, snapshot.title)).strong(),
        );
        ui.add(egui::Label::new(&snapshot.detail).wrap());
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, self.step_kind_label(snapshot.step_index), Tone::Info);
            ui_chrome::status_pill(ui, risk_summary_label(&risks), risk_summary_tone(&risks));
        });

        ui.separator();
        ui_chrome::section_label(ui, "Step Sequence");
        egui::ScrollArea::vertical()
            .id_salt("cross_section_step_list")
            .max_height(240.0)
            .auto_shrink([false, false])
            .show(ui, |ui| self.render_step_sequence(ui));

        ui.separator();
        self.render_step_detail(ui, &snapshot);
        ui.separator();
        self.render_material_detail(ui, &snapshot);
        ui.separator();
        self.render_risk_list(ui, &risks, true);
    }

    pub(crate) fn layer_stack_ui(&mut self, ui: &mut egui::Ui) {
        self.clamp_selection();
        let Some(snapshot) = self.selected_snapshot().cloned() else {
            ui_chrome::empty_state(ui, "No cross-section snapshots loaded");
            return;
        };

        ui_chrome::section_label(ui, "Cross-Section Controls");
        self.render_step_controls(ui);
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.show_mask_overlay, "Mask");
            ui.checkbox(&mut self.show_dimension_guides, "Dimensions");
            ui.checkbox(&mut self.show_risk_cues, "Risks");
        });

        ui.separator();
        self.render_layer_stack(ui, &snapshot, true);
        ui.separator();
        self.render_material_legend(ui, &snapshot);
        ui.separator();
        self.render_mask_openings(ui, &snapshot);
        ui.separator();
        self.render_process_parameters(ui, &snapshot);
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.clamp_selection();
        egui::ScrollArea::vertical()
            .id_salt("cross_section_dashboard_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut step_changed = false;
                ui_chrome::module_header(
                    ui,
                    "Process integration",
                    "3D/2D Process Cross-Section",
                    "Inspect simulated layer build-up, mask windows, film thickness, and process risk cues.",
                    |ui| {
                        step_changed = self.render_step_controls(ui);
                    },
                );

                if step_changed {
                    if let Some(snapshot) = self.selected_snapshot() {
                        *status = format!(
                            "cross-section step {}: {}",
                            snapshot.step_index, snapshot.title
                        );
                    }
                }

                let Some(snapshot) = self.selected_snapshot().cloned() else {
                    ui_chrome::empty_state(ui, "No cross-section snapshots loaded");
                    return;
                };

                let surface = surface_summary(&self.process, &snapshot);
                let risks = self.risk_findings(&snapshot);
                let mask_coverage = mask_coverage_um(&self.process, &snapshot)
                    / self.process.width_um.max(0.1);
                ui_chrome::metric_tiles(
                    ui,
                    &[
                        (
                            "Active step",
                            format!("{} / {}", snapshot.step_index, self.snapshots.len() - 1),
                            self.step_kind_label(snapshot.step_index),
                            Tone::Info,
                        ),
                        (
                            "Stack height",
                            format_um(surface.max_um),
                            "surface max",
                            Tone::Info,
                        ),
                        (
                            "Surface range",
                            format_um(surface.range_um),
                            "min-to-max",
                            surface_range_tone(surface.range_um),
                        ),
                        (
                            "Mask open",
                            format!("{:.0}%", mask_coverage.clamp(0.0, 1.0) * 100.0),
                            "lateral window",
                            Tone::Info,
                        ),
                        (
                            "Risk cues",
                            risks.len().to_string(),
                            risk_summary_label(&risks),
                            risk_summary_tone(&risks),
                        ),
                    ],
                );

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&snapshot.title).strong());
                    ui_chrome::status_pill(
                        ui,
                        &self.material_focus_label(&snapshot),
                        Tone::Neutral,
                    );
                });
                ui.add(egui::Label::new(&snapshot.detail).wrap());
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.show_mask_overlay, "Mask overlay");
                    ui.checkbox(&mut self.show_dimension_guides, "Dimension guides");
                    ui.checkbox(&mut self.show_risk_cues, "Risk markers");
                });
                ui.add_space(6.0);

                let selected_material = self.selected_material_id(&snapshot);
                draw_cross_section(
                    ui,
                    &self.process,
                    &snapshot,
                    selected_material.as_ref(),
                    CrossSectionDrawOptions {
                        show_mask_overlay: self.show_mask_overlay,
                        show_dimension_guides: self.show_dimension_guides,
                        show_risk_cues: self.show_risk_cues,
                    },
                );

                ui.add_space(8.0);
                self.render_dashboard_sections(ui, &snapshot, &risks);
            });
    }

    fn render_dashboard_sections(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &CrossSectionSnapshot,
        risks: &[RiskFinding],
    ) {
        let available = ui.available_width();
        if available >= 980.0 {
            ui.columns(3, |columns| {
                self.render_layer_stack(&mut columns[0], snapshot, false);
                self.render_material_detail(&mut columns[1], snapshot);
                self.render_risk_list(&mut columns[2], risks, false);
            });
        } else if available >= 660.0 {
            ui.columns(2, |columns| {
                self.render_layer_stack(&mut columns[0], snapshot, false);
                self.render_material_detail(&mut columns[1], snapshot);
            });
            ui.separator();
            self.render_risk_list(ui, risks, false);
        } else {
            self.render_layer_stack(ui, snapshot, true);
            ui.separator();
            self.render_material_detail(ui, snapshot);
            ui.separator();
            self.render_risk_list(ui, risks, true);
        }
    }

    fn render_step_controls(&mut self, ui: &mut egui::Ui) -> bool {
        let last = self.snapshots.len().saturating_sub(1);
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.selected_step > 0, egui::Button::new("<"))
                .on_hover_text("Previous process step")
                .clicked()
            {
                self.select_step(self.selected_step.saturating_sub(1));
                changed = true;
            }

            ui.label("Step");
            let mut step = self.selected_step;
            if ui
                .add(egui::Slider::new(&mut step, 0..=last).show_value(true))
                .changed()
            {
                self.select_step(step);
                changed = true;
            }

            if ui
                .add_enabled(self.selected_step < last, egui::Button::new(">"))
                .on_hover_text("Next process step")
                .clicked()
            {
                self.select_step(self.selected_step + 1);
                changed = true;
            }
        });
        changed
    }

    fn render_step_sequence(&mut self, ui: &mut egui::Ui) {
        for step_index in 0..self.snapshots.len() {
            let title = if step_index == 0 {
                "Starting substrate".to_string()
            } else {
                self.process
                    .steps
                    .get(step_index - 1)
                    .map(|step| step.name.clone())
                    .unwrap_or_else(|| format!("Step {step_index}"))
            };
            let selected = self.selected_step == step_index;
            let response = ui.selectable_label(selected, format!("{step_index}. {title}"));
            if response.clicked() {
                self.select_step(step_index);
            }
        }
    }

    fn render_step_detail(&self, ui: &mut egui::Ui, snapshot: &CrossSectionSnapshot) {
        ui_chrome::section_label(ui, "Selected Step");
        if snapshot.step_index == 0 {
            detail_grid(
                ui,
                "cross_section_initial_step_grid",
                &[
                    ("Operation", "Initial substrate stack".to_string()),
                    (
                        "Substrate",
                        self.material_name(&self.process.substrate_material),
                    ),
                    ("Thickness", format_um(self.process.substrate_thickness_um)),
                ],
            );
            return;
        }

        let Some(step) = self.process.steps.get(snapshot.step_index - 1) else {
            ui_chrome::empty_state(ui, "Selected process step is unavailable");
            return;
        };

        ui.add(egui::Label::new(&step.detail).wrap());
        match &step.kind {
            ProcessStepKind::Deposit {
                material,
                thickness_um,
            } => detail_grid(
                ui,
                "cross_section_deposit_step_grid",
                &[
                    ("Operation", "Deposit".to_string()),
                    ("Material", self.material_name(material)),
                    ("Target thickness", format_um(*thickness_um)),
                    ("Mask dependency", "Blanket film".to_string()),
                ],
            ),
            ProcessStepKind::Etch { material, depth_um } => detail_grid(
                ui,
                "cross_section_etch_step_grid",
                &[
                    ("Operation", "Etch".to_string()),
                    ("Target material", self.material_name(material)),
                    ("Etch depth", format_um(*depth_um)),
                    (
                        "Open mask span",
                        format_um(mask_coverage_um(&self.process, snapshot)),
                    ),
                ],
            ),
            ProcessStepKind::Pattern { openings } => {
                detail_grid(
                    ui,
                    "cross_section_pattern_step_grid",
                    &[
                        ("Operation", "Pattern mask".to_string()),
                        ("Openings", openings.len().to_string()),
                        (
                            "Open span",
                            format_um(opening_coverage_um(&self.process, openings)),
                        ),
                        (
                            "Minimum opening",
                            min_opening_width(openings)
                                .map(format_um)
                                .unwrap_or_else(|| "none".to_string()),
                        ),
                    ],
                );
            }
        }
    }

    fn render_layer_stack(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &CrossSectionSnapshot,
        compact: bool,
    ) {
        ui_chrome::section_label(ui, "Layer Stack");
        let bands = layer_bands(snapshot);
        if bands.is_empty() {
            ui_chrome::empty_state(ui, "No simulated layer geometry");
            return;
        }

        let selected_material = self.selected_material_id(snapshot);
        for (index, band) in bands.iter().enumerate() {
            let selected = selected_material.as_ref() == Some(&band.material);
            let color = self.material_color(&band.material);
            let name = self.material_name(&band.material);
            let coverage = (band.coverage_um / self.process.width_um.max(0.1)).clamp(0.0, 1.0);
            let fill = if selected {
                Color32::from_rgba_unmultiplied(43, 94, 137, 95)
            } else {
                ui.visuals().faint_bg_color
            };
            let stroke = if selected {
                Stroke::new(1.5, Tone::Info.color())
            } else {
                Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
            };
            let inner = egui::Frame::new()
                .fill(fill)
                .stroke(stroke)
                .corner_radius(6)
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        material_swatch(ui, color, vec2(16.0, 16.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).strong());
                            ui_chrome::muted(
                                ui,
                                format!(
                                    "{} thick, z {}-{}",
                                    format_um(band.thickness_um()),
                                    format_um(band.y0_um),
                                    format_um(band.y1_um)
                                ),
                            );
                        });
                    });
                    if !compact {
                        ui.add(
                            egui::ProgressBar::new(coverage)
                                .desired_width(ui.available_width())
                                .text(format!("{:.0}% lateral coverage", coverage * 100.0)),
                        );
                    } else {
                        ui_chrome::muted(
                            ui,
                            format!(
                                "{:.0}% coverage, {} segment(s)",
                                coverage * 100.0,
                                band.segment_count
                            ),
                        );
                    }
                });
            let response = ui.interact(
                inner.response.rect,
                ui.make_persistent_id(("cross_section_layer_band", index, band.material.as_str())),
                Sense::click(),
            );
            if response.clicked() {
                self.selected_material = Some(band.material.clone());
            }
        }
    }

    fn render_material_legend(&mut self, ui: &mut egui::Ui, snapshot: &CrossSectionSnapshot) {
        ui_chrome::section_label(ui, "Materials");
        let selected_material = self.selected_material_id(snapshot);
        for index in 0..self.process.materials.len() {
            let material = self.process.materials[index].clone();
            let selected = selected_material.as_ref() == Some(&material.id);
            ui.horizontal(|ui| {
                material_swatch(ui, material_color_rgb(material.color_rgb), vec2(16.0, 16.0));
                if ui.selectable_label(selected, &material.name).clicked() {
                    self.selected_material = Some(material.id.clone());
                }
            });
        }
    }

    fn render_material_detail(&self, ui: &mut egui::Ui, snapshot: &CrossSectionSnapshot) {
        ui_chrome::section_label(ui, "Selected Material");
        let Some(material_id) = self.selected_material_id(snapshot) else {
            ui_chrome::empty_state(ui, "Select a layer or material");
            return;
        };

        let color = self.material_color(&material_id);
        ui.horizontal(|ui| {
            material_swatch(ui, color, vec2(18.0, 18.0));
            ui.label(RichText::new(self.material_name(&material_id)).strong());
            ui_chrome::status_pill(
                ui,
                &self.material_role(&material_id, snapshot),
                Tone::Neutral,
            );
        });

        let stats = material_stats(&self.process, snapshot)
            .into_iter()
            .find(|stats| stats.material == material_id);
        if let Some(stats) = stats {
            detail_grid(
                ui,
                "cross_section_material_detail_grid",
                &[
                    ("Material ID", material_id.as_str().to_string()),
                    ("Area", format!("{:.2} um2", stats.area_um2)),
                    (
                        "Lateral coverage",
                        format!(
                            "{:.0}% ({})",
                            (stats.coverage_um / self.process.width_um.max(0.1)).clamp(0.0, 1.0)
                                * 100.0,
                            format_um(stats.coverage_um)
                        ),
                    ),
                    (
                        "Film thickness",
                        format!(
                            "{} to {}",
                            format_um(stats.min_thickness_um),
                            format_um(stats.max_thickness_um)
                        ),
                    ),
                    ("Segments", stats.segment_count.to_string()),
                ],
            );
        } else {
            ui_chrome::empty_state(ui, "Material is not present after the selected step");
        }
    }

    fn render_mask_openings(&self, ui: &mut egui::Ui, snapshot: &CrossSectionSnapshot) {
        ui_chrome::section_label(ui, "Mask Openings");
        if snapshot.active_mask.is_empty() {
            ui_chrome::empty_state(ui, "No active openings");
            return;
        }

        let full_width = self.process.width_um.max(0.1);
        for (index, opening) in snapshot.active_mask.iter().enumerate() {
            let width = (opening.end_um - opening.start_um).max(0.0);
            ui.horizontal(|ui| {
                ui.label(format!("#{index}"));
                ui.add(
                    egui::ProgressBar::new((width / full_width).clamp(0.0, 1.0))
                        .desired_width(ui.available_width().max(80.0))
                        .text(format!(
                            "{} to {} ({})",
                            format_um(opening.start_um),
                            format_um(opening.end_um),
                            format_um(width)
                        )),
                );
            });
        }
    }

    fn render_process_parameters(&self, ui: &mut egui::Ui, snapshot: &CrossSectionSnapshot) {
        ui_chrome::section_label(ui, "Process Parameters");
        let surface = surface_summary(&self.process, snapshot);
        let column_pitch = self.process.width_um / self.process.columns.max(1) as f32;
        detail_grid(
            ui,
            "cross_section_process_parameter_grid",
            &[
                ("Window width", format_um(self.process.width_um)),
                ("Simulation columns", self.process.columns.to_string()),
                ("Column pitch", format_um(column_pitch)),
                (
                    "Substrate",
                    self.material_name(&self.process.substrate_material),
                ),
                (
                    "Substrate thickness",
                    format_um(self.process.substrate_thickness_um),
                ),
                ("Minimum surface", format_um(surface.min_um)),
                ("Average surface", format_um(surface.average_um)),
            ],
        );
    }

    fn render_risk_list(&self, ui: &mut egui::Ui, risks: &[RiskFinding], compact: bool) {
        ui_chrome::section_label(ui, "Defects / Risks");
        if risks.is_empty() {
            ui_chrome::empty_state(ui, "No significant cross-section risk cues");
            return;
        }

        for risk in risks {
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(Stroke::new(1.0, risk.tone.color()))
                .corner_radius(6)
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui_chrome::status_pill(ui, risk.tone.label(), risk.tone);
                        ui.label(RichText::new(&risk.title).strong());
                    });
                    if !compact {
                        ui.add(egui::Label::new(&risk.detail).wrap());
                    } else {
                        ui_chrome::muted(ui, &risk.detail);
                    }
                });
        }
    }

    fn select_step(&mut self, step_index: usize) {
        self.selected_step = step_index.min(self.snapshots.len().saturating_sub(1));
        self.sync_selected_material_to_step();
    }

    fn sync_selected_material_to_step(&mut self) {
        if let Some(material) = self.step_material_id(self.selected_step) {
            self.selected_material = Some(material);
        } else if self.selected_material.is_none() {
            self.selected_material = Some(self.process.substrate_material.clone());
        }
    }

    fn selected_snapshot(&self) -> Option<&CrossSectionSnapshot> {
        self.snapshots.get(self.selected_step)
    }

    fn clamp_selection(&mut self) {
        self.selected_step = self
            .selected_step
            .min(self.snapshots.len().saturating_sub(1));
        if self.selected_material.is_none() {
            self.sync_selected_material_to_step();
        }
    }

    fn selected_material_id(&self, snapshot: &CrossSectionSnapshot) -> Option<MaterialId> {
        if let Some(material) = &self.selected_material
            && (self.process.material(material).is_some()
                || snapshot
                    .segments
                    .iter()
                    .any(|segment| segment.material == *material))
        {
            return Some(material.clone());
        }
        self.step_material_id(snapshot.step_index)
            .or_else(|| dominant_material(snapshot))
    }

    fn step_material_id(&self, step_index: usize) -> Option<MaterialId> {
        if step_index == 0 {
            return Some(self.process.substrate_material.clone());
        }
        match &self.process.steps.get(step_index - 1)?.kind {
            ProcessStepKind::Deposit { material, .. } | ProcessStepKind::Etch { material, .. } => {
                Some(material.clone())
            }
            ProcessStepKind::Pattern { .. } => None,
        }
    }

    fn material_focus_label(&self, snapshot: &CrossSectionSnapshot) -> String {
        self.selected_material_id(snapshot)
            .map(|material| format!("Focus: {}", self.material_name(&material)))
            .unwrap_or_else(|| "No material focus".to_string())
    }

    fn step_kind_label(&self, step_index: usize) -> &'static str {
        if step_index == 0 {
            return "Substrate";
        }
        match self
            .process
            .steps
            .get(step_index - 1)
            .map(|step| &step.kind)
        {
            Some(ProcessStepKind::Deposit { .. }) => "Deposit",
            Some(ProcessStepKind::Etch { .. }) => "Etch",
            Some(ProcessStepKind::Pattern { .. }) => "Pattern",
            None => "Step",
        }
    }

    fn material_role(&self, id: &MaterialId, snapshot: &CrossSectionSnapshot) -> String {
        if *id == self.process.substrate_material {
            return "Substrate".to_string();
        }
        if let Some(step_id) = self.step_material_id(snapshot.step_index)
            && step_id == *id
        {
            return match self
                .process
                .steps
                .get(snapshot.step_index.saturating_sub(1))
                .map(|step| &step.kind)
            {
                Some(ProcessStepKind::Deposit { .. }) => "Deposition film".to_string(),
                Some(ProcessStepKind::Etch { .. }) => "Etch target".to_string(),
                _ => "Step material".to_string(),
            };
        }
        if snapshot
            .segments
            .iter()
            .any(|segment| segment.material == *id)
        {
            "Existing film".to_string()
        } else {
            "Not present".to_string()
        }
    }

    fn risk_findings(&self, snapshot: &CrossSectionSnapshot) -> Vec<RiskFinding> {
        let mut findings = Vec::new();
        let surface = surface_summary(&self.process, snapshot);
        if surface.range_um > 0.35 {
            findings.push(RiskFinding::new(
                "Large surface step height",
                format!(
                    "Current topography is {}, which can challenge coat uniformity and etch loading.",
                    format_um(surface.range_um)
                ),
                Tone::Danger,
            ));
        } else if surface.range_um > 0.12 {
            findings.push(RiskFinding::new(
                "Surface topography forming",
                format!(
                    "Surface range is {}; watch subsequent coverage over edges.",
                    format_um(surface.range_um)
                ),
                Tone::Warning,
            ));
        }

        if snapshot.step_index > 0
            && let Some(step) = self.process.steps.get(snapshot.step_index - 1)
            && let ProcessStepKind::Etch { material, depth_um } = &step.kind
            && let Some(previous) = self.snapshots.get(snapshot.step_index - 1)
        {
            let available = max_material_thickness_in_openings(&self.process, previous, material);
            if available <= f32::EPSILON {
                findings.push(RiskFinding::new(
                    "Etch target absent",
                    format!(
                        "{} is not exposed in the active mask before this etch.",
                        self.material_name(material)
                    ),
                    Tone::Danger,
                ));
            } else if *depth_um > available + 0.02 {
                findings.push(RiskFinding::new(
                    "Etch budget exceeds film",
                    format!(
                        "{} etch depth is {} against {} available film in the openings.",
                        self.material_name(material),
                        format_um(*depth_um),
                        format_um(available)
                    ),
                    Tone::Danger,
                ));
            } else if *depth_um > available * 0.9 {
                findings.push(RiskFinding::new(
                    "Low etch margin",
                    format!(
                        "{} etch depth is close to the available {} film thickness.",
                        format_um(*depth_um),
                        self.material_name(material)
                    ),
                    Tone::Warning,
                ));
            }
        }

        if let Some(min_opening) = min_opening_width(&snapshot.active_mask)
            && min_opening < 0.75
        {
            findings.push(RiskFinding::new(
                "Narrow mask opening",
                format!(
                    "Minimum active opening is {}; verify lithography and etch bias margin.",
                    format_um(min_opening)
                ),
                Tone::Warning,
            ));
        }

        let mask_edge_count = snapshot.active_mask.len() * 2;
        if mask_edge_count >= 4 && surface.range_um > 0.1 {
            findings.push(RiskFinding::new(
                "Multiple patterned edges",
                format!(
                    "{mask_edge_count} mask edges coincide with {} topography.",
                    format_um(surface.range_um)
                ),
                Tone::Warning,
            ));
        }

        if let Some(material) = self.selected_material_id(snapshot) {
            let segment_count = snapshot
                .segments
                .iter()
                .filter(|segment| segment.material == material)
                .count();
            if segment_count > 1 {
                findings.push(RiskFinding::new(
                    "Disconnected material regions",
                    format!(
                        "{} appears in {segment_count} separated cross-section regions.",
                        self.material_name(&material)
                    ),
                    Tone::Info,
                ));
            }
        }

        findings
    }

    fn material_name(&self, id: &MaterialId) -> String {
        self.process
            .material(id)
            .map(|material| material.name.clone())
            .unwrap_or_else(|| id.as_str().to_string())
    }

    fn material_color(&self, id: &MaterialId) -> Color32 {
        self.process
            .material(id)
            .map(|material| material_color_rgb(material.color_rgb))
            .unwrap_or(Color32::LIGHT_GRAY)
    }
}

#[derive(Clone, Copy)]
struct CrossSectionDrawOptions {
    show_mask_overlay: bool,
    show_dimension_guides: bool,
    show_risk_cues: bool,
}

#[derive(Clone, Copy)]
struct SurfaceSummary {
    min_um: f32,
    max_um: f32,
    average_um: f32,
    range_um: f32,
}

struct LayerBand {
    material: MaterialId,
    y0_um: f32,
    y1_um: f32,
    coverage_um: f32,
    segment_count: usize,
}

impl LayerBand {
    fn thickness_um(&self) -> f32 {
        (self.y1_um - self.y0_um).max(0.0)
    }
}

struct MaterialStats {
    material: MaterialId,
    area_um2: f32,
    coverage_um: f32,
    min_thickness_um: f32,
    max_thickness_um: f32,
    segment_count: usize,
}

struct RiskFinding {
    title: String,
    detail: String,
    tone: Tone,
}

impl RiskFinding {
    fn new(title: impl Into<String>, detail: impl Into<String>, tone: Tone) -> Self {
        Self {
            title: title.into(),
            detail: detail.into(),
            tone,
        }
    }
}

trait ToneLabel {
    fn label(self) -> &'static str;
}

impl ToneLabel for Tone {
    fn label(self) -> &'static str {
        match self {
            Tone::Neutral => "Note",
            Tone::Info => "Info",
            Tone::Success => "OK",
            Tone::Warning => "Warn",
            Tone::Danger => "Risk",
        }
    }
}

fn draw_cross_section(
    ui: &mut egui::Ui,
    process: &CrossSectionProcess,
    snapshot: &CrossSectionSnapshot,
    selected_material: Option<&MaterialId>,
    options: CrossSectionDrawOptions,
) {
    let width = ui.available_width().max(260.0);
    let height = if width < 520.0 { 340.0 } else { 430.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 5.0, Color32::from_rgb(20, 24, 28));
    painter.rect_stroke(
        rect,
        5.0,
        Stroke::new(1.0, Color32::from_rgb(68, 76, 84)),
        StrokeKind::Outside,
    );

    let bounds = content_bounds(rect);
    let max_y = surface_summary(process, snapshot)
        .max_um
        .max(process.substrate_thickness_um)
        .max(1.0);
    if options.show_dimension_guides {
        draw_grid(&painter, bounds, process.width_um, max_y);
    }
    if options.show_mask_overlay {
        draw_mask_overlay(&painter, process, snapshot, bounds, max_y);
    }
    for segment in &snapshot.segments {
        draw_segment(
            &painter,
            process,
            segment,
            bounds,
            max_y,
            selected_material == Some(&segment.material),
            options.show_dimension_guides,
        );
    }
    if options.show_risk_cues {
        draw_risk_markers(&painter, process, snapshot, bounds, max_y);
    }
    draw_axes(
        &painter,
        bounds,
        process.width_um,
        max_y,
        options.show_dimension_guides,
    );
}

fn content_bounds(rect: Rect) -> Rect {
    Rect::from_min_max(rect.min + vec2(48.0, 40.0), rect.max - vec2(18.0, 42.0))
}

fn draw_segment(
    painter: &egui::Painter,
    process: &CrossSectionProcess,
    segment: &CrossSectionSegment,
    bounds: Rect,
    max_y: f32,
    selected: bool,
    show_label: bool,
) {
    let rect = segment_rect(process, segment, bounds, max_y);
    let color = process
        .material(&segment.material)
        .map(|material| material_color_rgb(material.color_rgb))
        .unwrap_or(Color32::LIGHT_GRAY);
    painter.rect_filled(rect, 0.0, color);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(0.5, Color32::from_black_alpha(90)),
        StrokeKind::Inside,
    );

    if selected {
        painter.rect_stroke(
            rect.shrink(0.75),
            0.0,
            Stroke::new(2.0, Color32::WHITE),
            StrokeKind::Inside,
        );
    }

    if show_label && rect.width() > 54.0 && rect.height() > 18.0 {
        let label = process
            .material(&segment.material)
            .map(|material| material.name.as_str())
            .unwrap_or_else(|| segment.material.as_str());
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            readable_text_on(color),
        );
    }
}

fn draw_mask_overlay(
    painter: &egui::Painter,
    process: &CrossSectionProcess,
    snapshot: &CrossSectionSnapshot,
    bounds: Rect,
    max_y: f32,
) {
    if snapshot.active_mask.is_empty() {
        return;
    }

    let top = y_to_screen(bounds, max_y, max_y);
    for opening in &snapshot.active_mask {
        let x0 = x_to_screen(bounds, process.width_um, opening.start_um);
        let x1 = x_to_screen(bounds, process.width_um, opening.end_um);
        let rect = Rect::from_min_max(Pos2::new(x0, top - 18.0), Pos2::new(x1, top - 6.0));
        painter.rect_filled(
            rect,
            1.0,
            Color32::from_rgba_unmultiplied(250, 210, 80, 135),
        );
    }

    let label = if is_full_width_mask(process, snapshot) {
        "blanket exposure window"
    } else {
        "active mask openings"
    };
    painter.text(
        Pos2::new(bounds.left(), top - 22.0),
        Align2::LEFT_BOTTOM,
        label,
        FontId::proportional(12.0),
        Color32::from_rgb(214, 206, 178),
    );
}

fn draw_grid(painter: &egui::Painter, bounds: Rect, width_um: f32, max_y: f32) {
    let stroke = Stroke::new(0.5, Color32::from_rgba_unmultiplied(180, 190, 200, 55));
    for fraction in [0.25, 0.5, 0.75] {
        let x = bounds.left() + bounds.width() * fraction;
        painter.line_segment(
            [Pos2::new(x, bounds.top()), Pos2::new(x, bounds.bottom())],
            stroke,
        );
        let y = bounds.bottom() - bounds.height() * fraction;
        painter.line_segment(
            [Pos2::new(bounds.left(), y), Pos2::new(bounds.right(), y)],
            stroke,
        );
        painter.text(
            Pos2::new(x, bounds.bottom() + 18.0),
            Align2::CENTER_CENTER,
            format!("{:.1}", width_um * fraction),
            FontId::proportional(10.0),
            Color32::from_rgb(150, 158, 166),
        );
        painter.text(
            Pos2::new(bounds.left() - 8.0, y),
            Align2::RIGHT_CENTER,
            format!("{:.1}", max_y * fraction),
            FontId::proportional(10.0),
            Color32::from_rgb(150, 158, 166),
        );
    }
}

fn draw_risk_markers(
    painter: &egui::Painter,
    process: &CrossSectionProcess,
    snapshot: &CrossSectionSnapshot,
    bounds: Rect,
    max_y: f32,
) {
    let heights = surface_heights(process, snapshot);
    if heights.len() < 2 {
        return;
    }

    let column_width = process.width_um.max(0.1) / heights.len() as f32;
    let mut last_marker_x = f32::NEG_INFINITY;
    for index in 1..heights.len() {
        let delta = (heights[index] - heights[index - 1]).abs();
        if delta < 0.08 {
            continue;
        }
        let x_um = index as f32 * column_width;
        let x = x_to_screen(bounds, process.width_um, x_um);
        if (x - last_marker_x).abs() < 12.0 {
            continue;
        }
        last_marker_x = x;
        let y = y_to_screen(bounds, heights[index].max(heights[index - 1]), max_y);
        let color = if delta > 0.25 {
            Tone::Danger.color()
        } else {
            Tone::Warning.color()
        };
        painter.line_segment(
            [Pos2::new(x, bounds.top()), Pos2::new(x, bounds.bottom())],
            Stroke::new(1.0, color),
        );
        painter.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(x, y - 10.0),
                Pos2::new(x - 5.0, y - 1.0),
                Pos2::new(x + 5.0, y - 1.0),
            ],
            color,
            Stroke::new(1.0, Color32::from_black_alpha(120)),
        ));
    }
}

fn draw_axes(
    painter: &egui::Painter,
    bounds: Rect,
    width_um: f32,
    max_y: f32,
    show_dimensions: bool,
) {
    painter.line_segment(
        [bounds.left_bottom(), bounds.right_bottom()],
        Stroke::new(1.0, Color32::from_rgb(160, 168, 176)),
    );
    painter.line_segment(
        [bounds.left_bottom(), bounds.left_top()],
        Stroke::new(1.0, Color32::from_rgb(160, 168, 176)),
    );
    painter.text(
        bounds.left_bottom() + Vec2::new(0.0, 18.0),
        Align2::LEFT_CENTER,
        "0 um",
        FontId::proportional(12.0),
        Color32::from_rgb(190, 196, 204),
    );
    painter.text(
        bounds.right_bottom() + Vec2::new(0.0, 18.0),
        Align2::RIGHT_CENTER,
        format!("{width_um:.1} um"),
        FontId::proportional(12.0),
        Color32::from_rgb(190, 196, 204),
    );
    painter.text(
        bounds.left_top() + Vec2::new(-8.0, 0.0),
        Align2::RIGHT_CENTER,
        format!("{max_y:.1} um"),
        FontId::proportional(12.0),
        Color32::from_rgb(190, 196, 204),
    );
    if show_dimensions {
        painter.text(
            bounds.center_top() + Vec2::new(0.0, -18.0),
            Align2::CENTER_CENTER,
            "lateral position (um) / film height (um)",
            FontId::proportional(12.0),
            Color32::from_rgb(190, 196, 204),
        );
    }
}

fn segment_rect(
    process: &CrossSectionProcess,
    segment: &CrossSectionSegment,
    bounds: Rect,
    max_y: f32,
) -> Rect {
    let x0 = x_to_screen(bounds, process.width_um, segment.x0_um);
    let x1 = x_to_screen(bounds, process.width_um, segment.x1_um);
    let y0 = y_to_screen(bounds, segment.y0_um, max_y);
    let y1 = y_to_screen(bounds, segment.y1_um, max_y);
    Rect::from_min_max(Pos2::new(x0, y1), Pos2::new(x1, y0))
}

fn x_to_screen(bounds: Rect, width_um: f32, value_um: f32) -> f32 {
    bounds.left() + bounds.width() * value_um / width_um.max(0.1)
}

fn y_to_screen(bounds: Rect, value_um: f32, max_y: f32) -> f32 {
    bounds.bottom() - bounds.height() * value_um / max_y.max(0.1)
}

fn detail_grid(ui: &mut egui::Ui, id: &'static str, rows: &[(&str, String)]) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([10.0, 4.0])
        .striped(false)
        .show(ui, |ui| {
            for (label, value) in rows {
                ui_chrome::muted(ui, *label);
                ui.add(egui::Label::new(value).wrap());
                ui.end_row();
            }
        });
}

fn material_swatch(ui: &mut egui::Ui, color: Color32, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color);
    ui.painter().rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, Color32::from_black_alpha(90)),
        StrokeKind::Inside,
    );
}

fn material_color_rgb(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

fn readable_text_on(background: Color32) -> Color32 {
    let luminance = 0.2126 * background.r() as f32
        + 0.7152 * background.g() as f32
        + 0.0722 * background.b() as f32;
    if luminance > 150.0 {
        Color32::from_rgb(20, 24, 28)
    } else {
        Color32::WHITE
    }
}

fn layer_bands(snapshot: &CrossSectionSnapshot) -> Vec<LayerBand> {
    let mut bands: Vec<LayerBand> = Vec::new();
    for segment in &snapshot.segments {
        let coverage = (segment.x1_um - segment.x0_um).max(0.0);
        if let Some(existing) = bands.iter_mut().find(|band| {
            band.material == segment.material
                && nearly_equal(band.y0_um, segment.y0_um)
                && nearly_equal(band.y1_um, segment.y1_um)
        }) {
            existing.coverage_um += coverage;
            existing.segment_count += 1;
        } else {
            bands.push(LayerBand {
                material: segment.material.clone(),
                y0_um: segment.y0_um,
                y1_um: segment.y1_um,
                coverage_um: coverage,
                segment_count: 1,
            });
        }
    }
    bands.sort_by(|a, b| {
        b.y1_um
            .total_cmp(&a.y1_um)
            .then_with(|| b.y0_um.total_cmp(&a.y0_um))
            .then_with(|| a.material.cmp(&b.material))
    });
    bands
}

fn material_stats(
    process: &CrossSectionProcess,
    snapshot: &CrossSectionSnapshot,
) -> Vec<MaterialStats> {
    let mut stats: Vec<MaterialStats> = Vec::new();
    for segment in &snapshot.segments {
        let width = (segment.x1_um - segment.x0_um).max(0.0);
        let thickness = (segment.y1_um - segment.y0_um).max(0.0);
        let area = width * thickness;
        if let Some(existing) = stats
            .iter_mut()
            .find(|stats| stats.material == segment.material)
        {
            existing.area_um2 += area;
            existing.coverage_um += width;
            existing.min_thickness_um = existing.min_thickness_um.min(thickness);
            existing.max_thickness_um = existing.max_thickness_um.max(thickness);
            existing.segment_count += 1;
        } else {
            stats.push(MaterialStats {
                material: segment.material.clone(),
                area_um2: area,
                coverage_um: width,
                min_thickness_um: thickness,
                max_thickness_um: thickness,
                segment_count: 1,
            });
        }
    }
    for stat in &mut stats {
        stat.coverage_um = stat.coverage_um.min(process.width_um.max(0.0));
    }
    stats.sort_by(|a, b| b.area_um2.total_cmp(&a.area_um2));
    stats
}

fn dominant_material(snapshot: &CrossSectionSnapshot) -> Option<MaterialId> {
    let mut best_material = None;
    let mut best_area = 0.0;
    for segment in &snapshot.segments {
        let area =
            (segment.x1_um - segment.x0_um).max(0.0) * (segment.y1_um - segment.y0_um).max(0.0);
        if area > best_area {
            best_area = area;
            best_material = Some(segment.material.clone());
        }
    }
    best_material
}

fn surface_summary(
    process: &CrossSectionProcess,
    snapshot: &CrossSectionSnapshot,
) -> SurfaceSummary {
    let heights = surface_heights(process, snapshot);
    if heights.is_empty() {
        return SurfaceSummary {
            min_um: 0.0,
            max_um: process.substrate_thickness_um,
            average_um: process.substrate_thickness_um,
            range_um: 0.0,
        };
    }

    let min_um = heights.iter().copied().fold(f32::INFINITY, f32::min);
    let max_um = heights.iter().copied().fold(0.0, f32::max);
    let average_um = heights.iter().sum::<f32>() / heights.len() as f32;
    SurfaceSummary {
        min_um,
        max_um,
        average_um,
        range_um: max_um - min_um,
    }
}

fn surface_heights(process: &CrossSectionProcess, snapshot: &CrossSectionSnapshot) -> Vec<f32> {
    let columns = process.columns.max(1);
    let column_width = process.width_um.max(0.1) / columns as f32;
    (0..columns)
        .map(|index| {
            let x_um = (index as f32 + 0.5) * column_width;
            snapshot
                .segments
                .iter()
                .filter(|segment| x_um >= segment.x0_um && x_um <= segment.x1_um)
                .map(|segment| segment.y1_um)
                .fold(0.0, f32::max)
        })
        .collect()
}

fn mask_coverage_um(process: &CrossSectionProcess, snapshot: &CrossSectionSnapshot) -> f32 {
    opening_coverage_um(process, &snapshot.active_mask)
}

fn opening_coverage_um(process: &CrossSectionProcess, openings: &[MaskOpening]) -> f32 {
    openings
        .iter()
        .map(|opening| {
            let start = opening.start_um.clamp(0.0, process.width_um);
            let end = opening.end_um.clamp(0.0, process.width_um);
            (end - start).max(0.0)
        })
        .sum::<f32>()
        .min(process.width_um.max(0.0))
}

fn min_opening_width(openings: &[MaskOpening]) -> Option<f32> {
    openings
        .iter()
        .map(|opening| (opening.end_um - opening.start_um).max(0.0))
        .min_by(|a, b| a.total_cmp(b))
}

fn max_material_thickness_in_openings(
    process: &CrossSectionProcess,
    snapshot: &CrossSectionSnapshot,
    material: &MaterialId,
) -> f32 {
    let openings = if snapshot.active_mask.is_empty() {
        vec![MaskOpening::new(0.0, process.width_um)]
    } else {
        snapshot.active_mask.clone()
    };
    snapshot
        .segments
        .iter()
        .filter(|segment| segment.material == *material)
        .filter(|segment| {
            openings
                .iter()
                .any(|opening| segment.x1_um > opening.start_um && segment.x0_um < opening.end_um)
        })
        .map(|segment| (segment.y1_um - segment.y0_um).max(0.0))
        .fold(0.0, f32::max)
}

fn is_full_width_mask(process: &CrossSectionProcess, snapshot: &CrossSectionSnapshot) -> bool {
    snapshot.active_mask.len() == 1
        && snapshot.active_mask[0].start_um <= 0.001
        && snapshot.active_mask[0].end_um >= process.width_um - 0.001
}

fn risk_summary_label(risks: &[RiskFinding]) -> &'static str {
    if risks.iter().any(|risk| risk.tone == Tone::Danger) {
        "high risk"
    } else if risks.iter().any(|risk| risk.tone == Tone::Warning) {
        "review"
    } else if risks.iter().any(|risk| risk.tone == Tone::Info) {
        "notes"
    } else {
        "nominal"
    }
}

fn risk_summary_tone(risks: &[RiskFinding]) -> Tone {
    if risks.iter().any(|risk| risk.tone == Tone::Danger) {
        Tone::Danger
    } else if risks.iter().any(|risk| risk.tone == Tone::Warning) {
        Tone::Warning
    } else if risks.iter().any(|risk| risk.tone == Tone::Info) {
        Tone::Info
    } else {
        Tone::Success
    }
}

fn surface_range_tone(range_um: f32) -> Tone {
    if range_um > 0.35 {
        Tone::Danger
    } else if range_um > 0.12 {
        Tone::Warning
    } else {
        Tone::Success
    }
}

fn format_um(value: f32) -> String {
    format!("{value:.2} um")
}

fn nearly_equal(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.0001
}
