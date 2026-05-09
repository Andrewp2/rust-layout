use std::collections::BTreeMap;

use eframe::egui::{self, Color32, RichText};
use layout_model::recipe::{
    ApprovalAction, ApprovalState, Recipe, RecipeCatalog, RecipeDiffKind, RecipeId,
    RecipeParameterType, RecipeParameterValue, RecipeUnit, RecipeValidationIssue, RecipeVersion,
    RecipeVersionNumber, ValidationSeverity, diff_recipe_versions,
};
use web_time::Instant;

pub(crate) struct RecipeManagerPanel {
    catalog: RecipeCatalog,
    selected_recipe: Option<RecipeId>,
    selected_version: Option<RecipeVersionNumber>,
    compare_version: Option<RecipeVersionNumber>,
    draft_key: Option<(RecipeId, RecipeVersionNumber)>,
    draft_parameters: BTreeMap<String, RecipeParameterValue>,
    actor: String,
    session_started: Instant,
}

impl RecipeManagerPanel {
    pub(crate) fn from_catalog(catalog: RecipeCatalog) -> Self {
        let mut panel = Self {
            catalog,
            selected_recipe: None,
            selected_version: None,
            compare_version: None,
            draft_key: None,
            draft_parameters: BTreeMap::new(),
            actor: "process.engineer".to_string(),
            session_started: Instant::now(),
        };
        if panel
            .catalog
            .recipe(&RecipeId::from("SPIN_PR_3000"))
            .is_some()
        {
            panel.select_recipe(RecipeId::from("SPIN_PR_3000"));
        } else if let Some(recipe_id) = panel
            .catalog
            .sorted_recipes()
            .first()
            .map(|recipe| recipe.id.clone())
        {
            panel.select_recipe(recipe_id);
        }
        panel
    }

    pub(crate) fn catalog(&self) -> &RecipeCatalog {
        &self.catalog
    }

    pub(crate) fn select_recipe_id(&mut self, recipe_id: &str) -> bool {
        let recipe_id = RecipeId::from(recipe_id.to_string());
        if self.catalog.recipe(&recipe_id).is_none() {
            return false;
        }
        self.select_recipe(recipe_id);
        true
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        egui::CollapsingHeader::new("Recipe Manager")
            .default_open(false)
            .show(ui, |ui| {
                if self.catalog.recipes.is_empty() {
                    ui.label("No recipes loaded");
                    return;
                }

                if self.recipe_picker(ui) {
                    *status = "recipe selected".to_string();
                }
                self.ensure_selection();

                let Some(recipe_id) = self.selected_recipe.clone() else {
                    ui.label("No recipe selected");
                    return;
                };
                let Some(recipe) = self.catalog.recipe(&recipe_id).cloned() else {
                    ui.label("Selected recipe is missing");
                    return;
                };

                if self.version_picker(ui, &recipe) {
                    *status = "recipe version selected".to_string();
                }
                self.ensure_draft();

                let Some(version_number) = self.selected_version else {
                    ui.label("No version selected");
                    return;
                };
                let Some(version) = recipe.version(version_number).cloned() else {
                    ui.label("Selected version is missing");
                    return;
                };

                ui.separator();
                self.recipe_summary(ui, &recipe, &version);

                let mut candidate = version.clone();
                candidate.parameters = self.draft_parameters.clone();
                let validation_issues = recipe.validate_version(&candidate);
                let draft_dirty = self.draft_parameters != version.parameters;

                ui.separator();
                self.parameter_editor(ui, &recipe, &version, &validation_issues, status);
                self.validation_summary(ui, &validation_issues);

                ui.separator();
                self.approval_controls(
                    ui,
                    version.approval_state,
                    draft_dirty,
                    &validation_issues,
                    status,
                );

                ui.separator();
                self.usage_links(ui, &recipe, version_number);

                ui.separator();
                self.diff_ui(ui, &recipe, &version);
            });
    }

    fn recipe_picker(&mut self, ui: &mut egui::Ui) -> bool {
        let mut selected = self.selected_recipe.clone();
        let selected_text = selected
            .as_ref()
            .and_then(|id| self.catalog.recipe(id))
            .map(recipe_picker_label)
            .unwrap_or_else(|| "Select recipe".to_string());

        egui::ComboBox::from_id_salt("recipe_manager_recipe_picker")
            .selected_text(selected_text)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for recipe in self.catalog.sorted_recipes() {
                    ui.selectable_value(
                        &mut selected,
                        Some(recipe.id.clone()),
                        recipe_picker_label(recipe),
                    );
                }
            });

        if selected != self.selected_recipe {
            if let Some(recipe_id) = selected {
                self.select_recipe(recipe_id);
                return true;
            }
        }
        false
    }

    fn version_picker(&mut self, ui: &mut egui::Ui, recipe: &Recipe) -> bool {
        let mut selected = self.selected_version;
        let selected_text = selected
            .map(|version| version.to_string())
            .unwrap_or_else(|| "Select version".to_string());

        ui.horizontal(|ui| {
            ui.label("Version");
            egui::ComboBox::from_id_salt("recipe_manager_version_picker")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    let mut versions: Vec<_> = recipe.versions.iter().collect();
                    versions.sort_by_key(|version| std::cmp::Reverse(version.version));
                    for version in versions {
                        ui.selectable_value(
                            &mut selected,
                            Some(version.version),
                            format!("{} - {}", version.version, version.approval_state.label()),
                        );
                    }
                });
        });

        if selected != self.selected_version {
            self.selected_version = selected;
            if let Some(version) = selected {
                self.compare_version = preferred_compare_version(recipe, version);
            }
            self.draft_key = None;
            return true;
        }
        false
    }

    fn recipe_summary(&self, ui: &mut egui::Ui, recipe: &Recipe, version: &RecipeVersion) {
        ui.label(RichText::new(&recipe.name).strong());
        ui.label(format!("{} - {}", recipe.id, recipe.tool_class));
        ui.label(format!("Owner: {}", recipe.owner));
        ui.label(format!("{} - {}", version.version, version.change_summary));
        ui.label(format!("State: {}", version.approval_state));
        if let Some(approved_by) = &version.approved_by {
            ui.label(format!("Approved by: {approved_by}"));
        }
    }

    fn parameter_editor(
        &mut self,
        ui: &mut egui::Ui,
        recipe: &Recipe,
        version: &RecipeVersion,
        validation_issues: &[RecipeValidationIssue],
        status: &mut String,
    ) {
        ui.label(RichText::new("Parameters").strong());
        for spec in recipe.sorted_parameter_specs() {
            let key = spec.key.clone();
            let fallback = version
                .parameters
                .get(&key)
                .cloned()
                .unwrap_or_else(|| spec.fallback_value());
            let value = self.draft_parameters.entry(key.clone()).or_insert(fallback);
            let issue_summary = parameter_issue_summary(validation_issues, &key);

            ui.push_id(("recipe_parameter", &key), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(&spec.label).on_hover_text(&spec.description);
                    edit_parameter_value(ui, &spec.value_type, spec.unit, value);
                });
                if let Some((severity, message)) = issue_summary {
                    let color = match severity {
                        ValidationSeverity::Error => Color32::from_rgb(220, 70, 70),
                        ValidationSeverity::Warning => Color32::from_rgb(210, 150, 45),
                    };
                    ui.colored_label(color, message);
                }
            });
        }

        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Reset draft").clicked() {
                self.reset_draft_from_version(version);
                *status = "recipe parameter draft reset".to_string();
            }
            let locked = matches!(
                version.approval_state,
                ApprovalState::Approved | ApprovalState::Retired
            );
            if ui
                .add_enabled(!locked, egui::Button::new("Apply draft"))
                .on_disabled_hover_text("Approved and retired versions are locked")
                .clicked()
            {
                self.apply_draft_to_selected(status);
            }
        });
    }

    fn validation_summary(&self, ui: &mut egui::Ui, issues: &[RecipeValidationIssue]) {
        let errors = issues
            .iter()
            .filter(|issue| issue.severity == ValidationSeverity::Error)
            .count();
        let warnings = issues
            .iter()
            .filter(|issue| issue.severity == ValidationSeverity::Warning)
            .count();

        if errors == 0 && warnings == 0 {
            ui.colored_label(Color32::from_rgb(90, 170, 120), "Validation: clean");
            return;
        }

        ui.colored_label(
            if errors > 0 {
                Color32::from_rgb(220, 70, 70)
            } else {
                Color32::from_rgb(210, 150, 45)
            },
            format!("Validation: {errors} error(s), {warnings} warning(s)"),
        );
        for issue in issues.iter().take(4) {
            let prefix = issue
                .parameter_key
                .as_deref()
                .map(|key| format!("{key}: "))
                .unwrap_or_default();
            ui.label(format!("{prefix}{}", issue.message));
        }
    }

    fn approval_controls(
        &mut self,
        ui: &mut egui::Ui,
        state: ApprovalState,
        draft_dirty: bool,
        validation_issues: &[RecipeValidationIssue],
        status: &mut String,
    ) {
        ui.label(RichText::new("Approval").strong());
        if draft_dirty {
            ui.colored_label(
                Color32::from_rgb(210, 150, 45),
                "Unsaved parameter draft; apply before approval.",
            );
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    state == ApprovalState::Draft && !draft_dirty,
                    egui::Button::new("Submit"),
                )
                .clicked()
            {
                self.apply_approval_action(
                    ApprovalAction::SubmitForReview,
                    validation_issues,
                    status,
                );
            }
            if ui
                .add_enabled(
                    state == ApprovalState::InReview && !draft_dirty,
                    egui::Button::new("Approve"),
                )
                .clicked()
            {
                self.apply_approval_action(ApprovalAction::Approve, validation_issues, status);
            }
            if ui
                .add_enabled(
                    state == ApprovalState::InReview && !draft_dirty,
                    egui::Button::new("Changes"),
                )
                .clicked()
            {
                self.apply_approval_action(
                    ApprovalAction::RequestChanges,
                    validation_issues,
                    status,
                );
            }
            if ui
                .add_enabled(
                    state == ApprovalState::Approved && !draft_dirty,
                    egui::Button::new("Retire"),
                )
                .clicked()
            {
                self.apply_approval_action(ApprovalAction::Retire, validation_issues, status);
            }
        });
    }

    fn usage_links(&self, ui: &mut egui::Ui, recipe: &Recipe, version: RecipeVersionNumber) {
        ui.label(RichText::new("Use links").strong());
        let mut any = false;
        for usage in recipe
            .usage_references
            .iter()
            .filter(|usage| usage.binding.version == version)
        {
            any = true;
            ui.label(format!("{} - {}", usage.target.label(), usage.context));
        }

        for route in &self.catalog.process_routes {
            for step in &route.steps {
                if step.recipe.as_ref().is_some_and(|binding| {
                    binding.recipe_id == recipe.id && binding.version == version
                }) {
                    any = true;
                    ui.label(format!(
                        "traveler {} v{} #{:02}: {}",
                        route.id, route.version, step.sequence, step.name
                    ));
                }
            }
        }

        for run in &self.catalog.tool_runs {
            if run.recipe.recipe_id == recipe.id && run.recipe.version == version {
                any = true;
                ui.label(format!(
                    "run {} on {} - {:?}",
                    run.id, run.tool_id, run.state
                ));
            }
        }

        if !any {
            ui.label("No route steps or tool runs reference this version.");
        }
    }

    fn diff_ui(&mut self, ui: &mut egui::Ui, recipe: &Recipe, version: &RecipeVersion) {
        ui.label(RichText::new("Version diff").strong());
        let other_versions: Vec<_> = recipe
            .versions
            .iter()
            .filter(|candidate| candidate.version != version.version)
            .map(|candidate| candidate.version)
            .collect();
        if other_versions.is_empty() {
            ui.label("No other version to compare.");
            return;
        }
        if self
            .compare_version
            .is_none_or(|version| !other_versions.contains(&version))
        {
            self.compare_version = preferred_compare_version(recipe, version.version);
        }

        let mut compare_version = self.compare_version;
        ui.horizontal(|ui| {
            ui.label("From");
            egui::ComboBox::from_id_salt("recipe_manager_diff_picker")
                .selected_text(
                    compare_version
                        .map(|version| version.to_string())
                        .unwrap_or_else(|| "Select".to_string()),
                )
                .show_ui(ui, |ui| {
                    for candidate in &other_versions {
                        ui.selectable_value(
                            &mut compare_version,
                            Some(*candidate),
                            candidate.to_string(),
                        );
                    }
                });
            ui.label(format!("to {}", version.version));
        });
        self.compare_version = compare_version;

        let Some(from_version) = compare_version.and_then(|number| recipe.version(number)) else {
            ui.label("Select a version to compare.");
            return;
        };
        let diff = diff_recipe_versions(recipe, from_version, version);
        if diff.is_empty() {
            ui.label("No parameter, approval, or dependency changes.");
            return;
        }

        for entry in diff.entries.iter().take(8) {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(diff_color(entry.kind), diff_kind_label(entry.kind));
                ui.label(&entry.label);
                let before = entry.before.as_deref().unwrap_or("-");
                let after = entry.after.as_deref().unwrap_or("-");
                ui.label(format!("{before} -> {after}"));
            });
        }
    }

    fn select_recipe(&mut self, recipe_id: RecipeId) {
        self.selected_recipe = Some(recipe_id.clone());
        let Some(recipe) = self.catalog.recipe(&recipe_id) else {
            self.selected_version = None;
            self.compare_version = None;
            self.draft_key = None;
            self.draft_parameters.clear();
            return;
        };
        let selected_version = recipe
            .latest_version()
            .map(|version| version.version)
            .or_else(|| recipe.versions.first().map(|version| version.version));
        self.selected_version = selected_version;
        self.compare_version = selected_version.and_then(|version| {
            self.catalog
                .recipe(&recipe_id)
                .and_then(|recipe| preferred_compare_version(recipe, version))
        });
        self.draft_key = None;
    }

    fn ensure_selection(&mut self) {
        let selected_exists = self
            .selected_recipe
            .as_ref()
            .is_some_and(|id| self.catalog.recipe(id).is_some());
        if !selected_exists {
            if let Some(recipe_id) = self
                .catalog
                .sorted_recipes()
                .first()
                .map(|recipe| recipe.id.clone())
            {
                self.select_recipe(recipe_id);
            }
        }

        let Some(recipe_id) = self.selected_recipe.clone() else {
            return;
        };
        let Some(recipe) = self.catalog.recipe(&recipe_id) else {
            return;
        };
        let version_exists = self
            .selected_version
            .is_some_and(|version| recipe.version(version).is_some());
        if !version_exists {
            self.selected_version = recipe.latest_version().map(|version| version.version);
            self.compare_version = self
                .selected_version
                .and_then(|version| preferred_compare_version(recipe, version));
            self.draft_key = None;
        }
    }

    fn ensure_draft(&mut self) {
        let Some(recipe_id) = self.selected_recipe.clone() else {
            self.draft_parameters.clear();
            self.draft_key = None;
            return;
        };
        let Some(version_number) = self.selected_version else {
            self.draft_parameters.clear();
            self.draft_key = None;
            return;
        };
        let key = (recipe_id.clone(), version_number);
        if self.draft_key.as_ref() == Some(&key) {
            return;
        }
        let parameters = self
            .catalog
            .recipe(&recipe_id)
            .and_then(|recipe| recipe.version(version_number))
            .map(|version| version.parameters.clone())
            .unwrap_or_default();
        self.draft_key = Some(key);
        self.draft_parameters = parameters;
    }

    fn reset_draft_from_version(&mut self, version: &RecipeVersion) {
        self.draft_parameters = version.parameters.clone();
        self.draft_key = Some((version.recipe_id.clone(), version.version));
    }

    fn apply_draft_to_selected(&mut self, status: &mut String) {
        let Some(recipe_id) = self.selected_recipe.clone() else {
            return;
        };
        let Some(version_number) = self.selected_version else {
            return;
        };
        let Some(recipe) = self.catalog.recipe_mut(&recipe_id) else {
            return;
        };
        let Some(version) = recipe.version_mut(version_number) else {
            return;
        };
        if matches!(
            version.approval_state,
            ApprovalState::Approved | ApprovalState::Retired
        ) {
            *status = "approved and retired recipe versions are locked".to_string();
            return;
        }
        version.parameters = self.draft_parameters.clone();
        *status = format!("applied draft to {} {}", recipe_id, version_number);
    }

    fn apply_approval_action(
        &mut self,
        action: ApprovalAction,
        validation_issues: &[RecipeValidationIssue],
        status: &mut String,
    ) {
        let Some(recipe_id) = self.selected_recipe.clone() else {
            return;
        };
        let Some(version_number) = self.selected_version else {
            return;
        };
        let timestamp = self.session_timestamp();
        let actor = self.actor.clone();
        let Some(recipe) = self.catalog.recipe_mut(&recipe_id) else {
            return;
        };
        let Some(version) = recipe.version_mut(version_number) else {
            return;
        };
        match version.apply_approval_action(
            action,
            actor,
            timestamp,
            action.label(),
            validation_issues,
        ) {
            Ok(()) => *status = format!("{} {}", action.label(), version_number),
            Err(error) => *status = error.to_string(),
        }
    }

    fn session_timestamp(&self) -> String {
        format!("session+{}s", self.session_started.elapsed().as_secs())
    }
}

fn recipe_picker_label(recipe: &Recipe) -> String {
    format!("{} - {}", recipe.id, recipe.name)
}

fn preferred_compare_version(
    recipe: &Recipe,
    selected: RecipeVersionNumber,
) -> Option<RecipeVersionNumber> {
    let mut versions: Vec<_> = recipe
        .versions
        .iter()
        .map(|version| version.version)
        .collect();
    versions.sort();
    versions
        .iter()
        .rev()
        .copied()
        .find(|version| *version < selected)
        .or_else(|| versions.into_iter().find(|version| *version != selected))
}

fn edit_parameter_value(
    ui: &mut egui::Ui,
    value_type: &RecipeParameterType,
    unit: Option<RecipeUnit>,
    value: &mut RecipeParameterValue,
) {
    match (value_type, value) {
        (RecipeParameterType::Decimal, RecipeParameterValue::Decimal(number)) => {
            ui.add(
                egui::DragValue::new(number)
                    .speed(0.1)
                    .suffix(unit_suffix(unit)),
            );
        }
        (RecipeParameterType::Integer, RecipeParameterValue::Integer(number)) => {
            ui.add(
                egui::DragValue::new(number)
                    .speed(1)
                    .suffix(unit_suffix(unit)),
            );
        }
        (RecipeParameterType::Boolean, RecipeParameterValue::Boolean(enabled)) => {
            ui.checkbox(enabled, "");
        }
        (RecipeParameterType::Text, RecipeParameterValue::Text(text)) => {
            ui.text_edit_singleline(text);
        }
        (RecipeParameterType::Choice { options }, RecipeParameterValue::Choice(selected)) => {
            egui::ComboBox::from_id_salt("choice")
                .selected_text(selected.as_str())
                .show_ui(ui, |ui| {
                    for option in options {
                        ui.selectable_value(selected, option.clone(), option);
                    }
                });
        }
        (_, value) => {
            ui.colored_label(Color32::from_rgb(220, 70, 70), value.kind_label());
        }
    }
}

fn unit_suffix(unit: Option<RecipeUnit>) -> String {
    unit.map(|unit| format!(" {}", unit.symbol()))
        .unwrap_or_default()
}

fn parameter_issue_summary(
    issues: &[RecipeValidationIssue],
    key: &str,
) -> Option<(ValidationSeverity, String)> {
    let mut matches = issues
        .iter()
        .filter(|issue| issue.parameter_key.as_deref() == Some(key));
    let first = matches.next()?;
    let mut message = first.message.clone();
    for issue in matches {
        message.push_str("; ");
        message.push_str(&issue.message);
    }
    Some((first.severity, message))
}

fn diff_kind_label(kind: RecipeDiffKind) -> &'static str {
    match kind {
        RecipeDiffKind::ParameterAdded => "added",
        RecipeDiffKind::ParameterRemoved => "removed",
        RecipeDiffKind::ParameterChanged => "changed",
        RecipeDiffKind::ApprovalChanged => "state",
        RecipeDiffKind::DependencyChanged => "deps",
    }
}

fn diff_color(kind: RecipeDiffKind) -> Color32 {
    match kind {
        RecipeDiffKind::ParameterAdded => Color32::from_rgb(90, 170, 120),
        RecipeDiffKind::ParameterRemoved => Color32::from_rgb(220, 90, 80),
        RecipeDiffKind::ParameterChanged => Color32::from_rgb(95, 150, 210),
        RecipeDiffKind::ApprovalChanged => Color32::from_rgb(210, 150, 45),
        RecipeDiffKind::DependencyChanged => Color32::from_rgb(150, 120, 210),
    }
}
