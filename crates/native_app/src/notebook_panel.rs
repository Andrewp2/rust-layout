use std::collections::BTreeSet;

use eframe::egui::{self, Color32, RichText, TextEdit};
use layout_model::{
    mes::LotId,
    notebook::{
        LabNotebook, NotebookEntry, NotebookEntryId, NotebookFilter, NotebookLinkKind,
        NotebookLinks,
    },
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct LabNotebookPanel {
    notebook: LabNotebook,
    selected_entry: Option<NotebookEntryId>,
    filter: NotebookFilter,
    selected_tag: String,
    selected_link_kind: Option<NotebookLinkKind>,
    preview_mode: bool,
    show_followups_only: bool,
}

#[derive(Clone, Debug)]
struct LinkFocus {
    kind: NotebookLinkKind,
    query: String,
    label: String,
}

#[derive(Clone, Debug)]
struct LinkChipData {
    kind: NotebookLinkKind,
    prefix: String,
    value: String,
    detail: String,
    tone: Tone,
}

impl LinkChipData {
    fn focus(&self) -> LinkFocus {
        LinkFocus {
            kind: self.kind,
            query: self.value.clone(),
            label: format!("{} {}", self.prefix, self.value),
        }
    }
}

#[derive(Clone, Debug)]
struct RelatedEntryRow {
    id: NotebookEntryId,
    title: String,
    reason: String,
    score: usize,
}

#[derive(Clone, Debug, Default)]
struct NotebookMetrics {
    total_links: usize,
    lots: usize,
    wafers: usize,
    recipes: usize,
    tool_runs: usize,
    metrology: usize,
    images: usize,
    followups: usize,
}

#[derive(Clone, Copy, Debug)]
enum NotebookEntryAction {
    AddFollowUpPlan,
    InsertMetrologyReview,
    RequestImageEvidence,
    TagHandoff,
}

impl LabNotebookPanel {
    pub(crate) fn from_notebook(notebook: LabNotebook) -> Self {
        let selected_entry = notebook.entries.first().map(|entry| entry.id.clone());
        Self {
            notebook,
            selected_entry,
            filter: NotebookFilter::default(),
            selected_tag: "All tags".to_string(),
            selected_link_kind: None,
            preview_mode: true,
            show_followups_only: false,
        }
    }

    pub(crate) fn notebook(&self) -> &LabNotebook {
        &self.notebook
    }

    pub(crate) fn add_quick_lot_note(&mut self, lot_id: &str) -> NotebookEntryId {
        let mut sequence = self.notebook.entries.len() + 1;
        let mut entry_id = NotebookEntryId::new(format!("WF-{sequence:04}"));
        while self.notebook.entry(&entry_id).is_some() {
            sequence += 1;
            entry_id = NotebookEntryId::new(format!("WF-{sequence:04}"));
        }
        let entry = NotebookEntry {
            id: entry_id.clone(),
            title: format!("Workflow note for {lot_id}"),
            author: "workflow".to_string(),
            created_at: "2026-05-08".to_string(),
            updated_at: "2026-05-08".to_string(),
            body_markdown: format!("Workflow note linked to lot {lot_id}."),
            tags: vec!["workflow".to_string()],
            links: NotebookLinks {
                lots: vec![LotId::new(lot_id)],
                ..NotebookLinks::default()
            },
        };
        self.notebook.entries.push(entry);
        self.selected_entry = Some(entry_id.clone());
        self.clear_filters();
        entry_id
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.sync_filter_controls();
        let filtered_ids = self.filtered_entry_ids();
        self.ensure_selection(&filtered_ids);
        let metrics = self.notebook_metrics();

        egui::ScrollArea::vertical()
            .id_salt("lab_notebook_dashboard")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Process engineering",
                    "Lab Notebook",
                    "Markdown notes linked to lots, wafers, recipes, tool runs, metrology, and images",
                    |ui| {
                        ui_chrome::status_pill(
                            ui,
                            &format!("{} entries", self.notebook.entries.len()),
                            Tone::Info,
                        );
                        ui_chrome::status_pill(
                            ui,
                            &format!("{} linked objects", metrics.total_links),
                            Tone::Neutral,
                        );
                    },
                );

                if self.notebook.entries.is_empty() {
                    ui_chrome::empty_state(ui, "No notebook entries loaded");
                    let _ = status;
                    return;
                }

                self.summary_metrics_ui(ui, &filtered_ids, &metrics);
                self.filter_bar_ui(ui);
                ui.separator();

                let available_width = ui.available_width();
                if available_width < 760.0 {
                    self.entry_list_ui(ui, &filtered_ids);
                    ui.separator();
                    self.selected_entry_ui(ui, status);
                    ui.separator();
                    self.related_context_ui(ui);
                } else if available_width > 1160.0 {
                    ui.columns(3, |columns| {
                        columns[0].set_min_width(270.0);
                        columns[1].set_min_width(460.0);
                        columns[2].set_min_width(260.0);
                        self.entry_list_ui(&mut columns[0], &filtered_ids);
                        self.selected_entry_ui(&mut columns[1], status);
                        self.related_context_ui(&mut columns[2]);
                    });
                } else {
                    ui.columns(2, |columns| {
                        columns[0].set_min_width(270.0);
                        self.entry_list_ui(&mut columns[0], &filtered_ids);
                        self.selected_entry_ui(&mut columns[1], status);
                    });
                }
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.sync_filter_controls();
        let metrics = self.notebook_metrics();
        ui_chrome::section_label(ui, "Lab Notebook");
        ui.label(format!(
            "{} entries / {} tags / {} links",
            self.notebook.entries.len(),
            self.notebook.tags().len(),
            metrics.total_links
        ));
        ui.label(format!(
            "{} lots / {} wafers / {} recipes",
            metrics.lots, metrics.wafers, metrics.recipes
        ));
        ui.label(format!(
            "{} metrology / {} images / {} follow-ups",
            metrics.metrology, metrics.images, metrics.followups
        ));
        self.active_filter_summary_ui(ui);
        ui.separator();

        if let Some(entry) = self.selected_entry().cloned() {
            ui.strong(&entry.title);
            ui.small(format!("{}  {}", entry.id, entry.updated_at));
            ui.label(format!(
                "{} words / {} linked objects",
                word_count(&entry.body_markdown),
                entry.link_count()
            ));

            ui_chrome::section_label(ui, "Tags");
            ui.horizontal_wrapped(|ui| {
                for tag in &entry.tags {
                    ui_chrome::status_pill(ui, tag, Tone::Neutral);
                }
            });

            let actions = follow_up_actions(&entry);
            if !actions.is_empty() {
                ui_chrome::section_label(ui, "Follow-Up Signals");
                for action in actions.iter().take(4) {
                    ui.add(egui::Label::new(format!("- {action}")).wrap());
                }
            }

            ui_chrome::section_label(ui, "Structured Links");
            link_count_rows(ui, &entry.links);
            if let Some(focus) = compact_link_focus_ui(ui, &entry.links) {
                self.apply_link_focus(focus);
            }
            self.related_entries_ui(ui, &entry);
        } else {
            ui_chrome::empty_state(ui, "No entry selected");
        }
    }

    pub(crate) fn link_filter_ui(&mut self, ui: &mut egui::Ui) {
        self.sync_filter_controls();
        ui_chrome::section_label(ui, "Notebook Filters");
        ui.label("Search");
        ui.add(TextEdit::singleline(&mut self.filter.query).hint_text("entry, tag, link id"));
        ui.checkbox(&mut self.show_followups_only, "Follow-ups only");

        ui.label("Tag");
        self.tag_filter_combo(ui, "notebook_tag_filter");

        ui.label("Link type");
        self.link_kind_filter_combo(ui, "notebook_link_filter");

        if ui.button("Clear filters").clicked() {
            self.clear_filters();
        }

        self.sync_filter_controls();
        ui.separator();
        ui_chrome::metric_tile(
            ui,
            "Matches",
            self.filtered_entry_ids().len(),
            "current filter",
        );
        ui_chrome::section_label(ui, "Link Coverage");
        let metrics = self.notebook_metrics();
        egui::Grid::new("notebook_filter_link_coverage")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                link_metric_row(ui, "Lots", metrics.lots);
                link_metric_row(ui, "Wafers", metrics.wafers);
                link_metric_row(ui, "Recipes", metrics.recipes);
                link_metric_row(ui, "Tool runs", metrics.tool_runs);
                link_metric_row(ui, "Metrology", metrics.metrology);
                link_metric_row(ui, "Images", metrics.images);
            });
    }

    fn filter_bar_ui(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 520.0 {
            ui.label("Search");
            ui.add_sized(
                [ui.available_width().min(280.0), 22.0],
                TextEdit::singleline(&mut self.filter.query).hint_text("residue, RUN-ETCH, CD"),
            );
            self.tag_filter_combo(ui, "notebook_dashboard_tag");
            self.link_kind_filter_combo(ui, "notebook_dashboard_link_kind");
            ui.checkbox(&mut self.show_followups_only, "Follow-ups only");
            if self.has_active_filters() && ui.button("Clear filters").clicked() {
                self.clear_filters();
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label("Search");
                ui.add_sized(
                    [220.0, 22.0],
                    TextEdit::singleline(&mut self.filter.query).hint_text("residue, RUN-ETCH, CD"),
                );
                self.tag_filter_combo(ui, "notebook_dashboard_tag");
                self.link_kind_filter_combo(ui, "notebook_dashboard_link_kind");
                ui.checkbox(&mut self.show_followups_only, "Follow-ups only");
                if self.has_active_filters() && ui.button("Clear").clicked() {
                    self.clear_filters();
                }
            });
        }
        self.quick_filter_chips_ui(ui);
        self.sync_filter_controls();
    }

    fn entry_list_ui(&mut self, ui: &mut egui::Ui, filtered_ids: &[NotebookEntryId]) {
        ui.horizontal_wrapped(|ui| {
            ui_chrome::section_label(ui, "Timeline");
            ui_chrome::muted(ui, format!("{} shown", filtered_ids.len()));
        });
        if filtered_ids.is_empty() {
            ui_chrome::empty_state(ui, "No matching entries");
            return;
        }

        for entry_id in filtered_ids {
            if let Some(entry) = self.notebook.entry(entry_id).cloned() {
                let selected = self
                    .selected_entry
                    .as_ref()
                    .is_some_and(|selected_id| selected_id == entry_id);
                if entry_timeline_card(ui, &entry, selected).clicked() {
                    self.selected_entry = Some(entry.id.clone());
                }
            }
        }
    }

    fn selected_entry_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        let Some(entry_id) = self.selected_entry.clone() else {
            ui_chrome::empty_state(ui, "No entry selected");
            return;
        };

        let Some(entry_snapshot) = self.notebook.entry(&entry_id).cloned() else {
            ui_chrome::empty_state(ui, "Selected entry is unavailable");
            return;
        };

        ui.horizontal_wrapped(|ui| {
            ui.heading(&entry_snapshot.title);
            ui_chrome::status_pill(ui, &entry_snapshot.id.to_string(), Tone::Info);
        });
        ui_chrome::muted(
            ui,
            format!(
                "{} / created {} / updated {}",
                entry_snapshot.author, entry_snapshot.created_at, entry_snapshot.updated_at
            ),
        );

        ui.horizontal_wrapped(|ui| {
            for tag in &entry_snapshot.tags {
                ui_chrome::status_pill(ui, tag, Tone::Neutral);
            }
            ui_chrome::status_pill(
                ui,
                &format!("{} words", word_count(&entry_snapshot.body_markdown)),
                Tone::Neutral,
            );
        });

        ui.separator();
        let mut entry_action = None;
        ui.horizontal_wrapped(|ui| {
            if ui.selectable_label(self.preview_mode, "Preview").clicked() {
                self.preview_mode = true;
            }
            if ui.selectable_label(!self.preview_mode, "Edit").clicked() {
                self.preview_mode = false;
            }
            ui.separator();
            if ui.button("Add follow-up").clicked() {
                entry_action = Some(NotebookEntryAction::AddFollowUpPlan);
            }
            if ui.button("Insert metrology review").clicked() {
                entry_action = Some(NotebookEntryAction::InsertMetrologyReview);
            }
            if ui.button("Request image evidence").clicked() {
                entry_action = Some(NotebookEntryAction::RequestImageEvidence);
            }
            if ui.button("Tag handoff").clicked() {
                entry_action = Some(NotebookEntryAction::TagHandoff);
            }
        });

        if let Some(action) = entry_action {
            if let Some(message) = self.apply_entry_action(&entry_id, action) {
                *status = message;
            }
        }

        ui.separator();
        if self.preview_mode {
            ui.label(RichText::new("Readable Preview").strong());
            if let Some(entry) = self.notebook.entry(&entry_id) {
                markdown_preview_ui(ui, &entry.body_markdown);
            }
        } else if let Some(entry) = self.notebook.entry_mut(&entry_id) {
            ui.label(RichText::new("Markdown Editor").strong());
            let response = ui.add(
                TextEdit::multiline(&mut entry.body_markdown)
                    .desired_rows(14)
                    .desired_width(f32::INFINITY),
            );
            if response.changed() {
                *status = format!("edited notebook entry {}", entry.id);
            }
        }

        ui.separator();
        ui.label(RichText::new("Linked Data").strong());
        if let Some(entry) = self.notebook.entry(&entry_id).cloned() {
            if let Some(focus) = linked_data_ui(ui, &entry.links) {
                self.apply_link_focus(focus.clone());
                *status = format!("filtered notebook to {}", focus.label);
            }
        }
    }

    fn selected_entry(&self) -> Option<&NotebookEntry> {
        self.selected_entry
            .as_ref()
            .and_then(|id| self.notebook.entry(id))
    }

    fn filtered_entry_ids(&self) -> Vec<NotebookEntryId> {
        self.notebook
            .filtered_entries(&self.filter)
            .into_iter()
            .filter(|entry| !self.show_followups_only || !follow_up_actions(entry).is_empty())
            .map(|entry| entry.id.clone())
            .collect()
    }

    fn ensure_selection(&mut self, filtered_ids: &[NotebookEntryId]) {
        if self
            .selected_entry
            .as_ref()
            .is_some_and(|id| filtered_ids.iter().any(|filtered_id| filtered_id == id))
        {
            return;
        }
        self.selected_entry = filtered_ids.first().cloned();
    }

    fn sync_filter_controls(&mut self) {
        self.filter.tag = if self.selected_tag == "All tags" {
            None
        } else {
            Some(self.selected_tag.clone())
        };
        self.filter.link_kind = self.selected_link_kind;
    }

    fn clear_filters(&mut self) {
        self.filter = NotebookFilter::default();
        self.selected_tag = "All tags".to_string();
        self.selected_link_kind = None;
        self.show_followups_only = false;
    }

    fn has_active_filters(&self) -> bool {
        !self.filter.query.trim().is_empty()
            || self.selected_tag != "All tags"
            || self.selected_link_kind.is_some()
            || self.show_followups_only
    }

    fn tag_filter_combo(&mut self, ui: &mut egui::Ui, id_salt: &'static str) {
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(&self.selected_tag)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_tag, "All tags".to_string(), "All tags");
                for tag in self.notebook.tags() {
                    ui.selectable_value(&mut self.selected_tag, tag.clone(), tag);
                }
            });
    }

    fn link_kind_filter_combo(&mut self, ui: &mut egui::Ui, id_salt: &'static str) {
        let selected_link_label = self
            .selected_link_kind
            .map(|kind| kind.label())
            .unwrap_or("Any link");
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(selected_link_label)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_link_kind, None, "Any link");
                for kind in NotebookLinkKind::ALL {
                    ui.selectable_value(&mut self.selected_link_kind, Some(kind), kind.label());
                }
            });
    }

    fn quick_filter_chips_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Quick filters").small());
            if ui
                .selectable_label(self.selected_link_kind.is_none(), "Any link")
                .clicked()
            {
                self.selected_link_kind = None;
            }
            for kind in NotebookLinkKind::ALL {
                let count = self
                    .notebook
                    .entries
                    .iter()
                    .filter(|entry| entry.has_link_kind(kind))
                    .count();
                let label = format!("{} {count}", link_kind_short_label(kind));
                if ui
                    .selectable_label(self.selected_link_kind == Some(kind), label)
                    .clicked()
                {
                    self.selected_link_kind = Some(kind);
                }
            }
        });

        let tags = self.notebook.tags();
        if !tags.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Tags").small());
                if ui
                    .selectable_label(self.selected_tag == "All tags", "All")
                    .clicked()
                {
                    self.selected_tag = "All tags".to_string();
                }
                for tag in tags.into_iter().take(8) {
                    let selected = self.selected_tag == tag;
                    if ui.selectable_label(selected, format!("#{tag}")).clicked() {
                        self.selected_tag = tag;
                    }
                }
            });
        }
    }

    fn summary_metrics_ui(
        &self,
        ui: &mut egui::Ui,
        filtered_ids: &[NotebookEntryId],
        metrics: &NotebookMetrics,
    ) {
        let active = if self.has_active_filters() {
            "filtered"
        } else {
            "all entries"
        };
        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Matches",
                    filtered_ids.len().to_string(),
                    active,
                    Tone::Info,
                ),
                (
                    "Structured links",
                    metrics.total_links.to_string(),
                    "lots, wafers, recipes, runs",
                    Tone::Success,
                ),
                (
                    "Evidence",
                    format!(
                        "{} metrology / {} images",
                        metrics.metrology, metrics.images
                    ),
                    "measurement and visual proof",
                    Tone::Neutral,
                ),
                (
                    "Follow-ups",
                    metrics.followups.to_string(),
                    "entries with action signals",
                    Tone::Warning,
                ),
            ],
        );
    }

    fn notebook_metrics(&self) -> NotebookMetrics {
        let mut metrics = NotebookMetrics::default();
        for entry in &self.notebook.entries {
            metrics.total_links += entry.link_count();
            metrics.lots += entry.links.lots.len();
            metrics.wafers += entry.links.wafers.len();
            metrics.recipes += entry.links.recipes.len();
            metrics.tool_runs += entry.links.tool_runs.len();
            metrics.metrology += entry.links.metrology.len();
            metrics.images += entry.links.images.len();
            if !follow_up_actions(entry).is_empty() {
                metrics.followups += 1;
            }
        }
        metrics
    }

    fn active_filter_summary_ui(&self, ui: &mut egui::Ui) {
        if !self.has_active_filters() {
            ui_chrome::muted(ui, "No active notebook filter");
            return;
        }

        ui_chrome::section_label(ui, "Active Filter");
        if !self.filter.query.trim().is_empty() {
            ui.label(format!("Search: {}", self.filter.query.trim()));
        }
        if self.selected_tag != "All tags" {
            ui.label(format!("Tag: {}", self.selected_tag));
        }
        if let Some(kind) = self.selected_link_kind {
            ui.label(format!("Link type: {}", kind.label()));
        }
        if self.show_followups_only {
            ui.label("Follow-ups only");
        }
    }

    fn apply_link_focus(&mut self, focus: LinkFocus) {
        self.filter.query = focus.query;
        self.selected_link_kind = Some(focus.kind);
        self.selected_tag = "All tags".to_string();
        self.show_followups_only = false;
        self.sync_filter_controls();
        let filtered_ids = self.filtered_entry_ids();
        self.ensure_selection(&filtered_ids);
    }

    fn apply_entry_action(
        &mut self,
        entry_id: &NotebookEntryId,
        action: NotebookEntryAction,
    ) -> Option<String> {
        let entry = self.notebook.entry_mut(entry_id)?;
        let message = match action {
            NotebookEntryAction::AddFollowUpPlan => {
                add_tag_once(entry, "follow-up");
                let section = follow_up_plan_markdown(entry);
                append_section_once(&mut entry.body_markdown, "## Follow-up", &section);
                format!("added follow-up plan to notebook entry {}", entry.id)
            }
            NotebookEntryAction::InsertMetrologyReview => {
                add_tag_once(entry, "metrology-review");
                let section = metrology_review_markdown(entry);
                append_section_once(&mut entry.body_markdown, "## Metrology Review", &section);
                format!("inserted metrology review for notebook entry {}", entry.id)
            }
            NotebookEntryAction::RequestImageEvidence => {
                add_tag_once(entry, "image-needed");
                let section = image_evidence_markdown(entry);
                append_section_once(&mut entry.body_markdown, "## Image Evidence", &section);
                format!("requested image evidence for notebook entry {}", entry.id)
            }
            NotebookEntryAction::TagHandoff => {
                add_tag_once(entry, "handoff");
                append_section_once(
                    &mut entry.body_markdown,
                    "## Shift Handoff",
                    "- [ ] Summarize the open disposition, owner, and next lot to inspect.",
                );
                format!("tagged notebook entry {} for handoff", entry.id)
            }
        };
        Some(message)
    }

    fn related_context_ui(&mut self, ui: &mut egui::Ui) {
        let Some(entry) = self.selected_entry().cloned() else {
            ui_chrome::empty_state(ui, "No entry selected");
            return;
        };

        ui_chrome::section_label(ui, "Context");
        ui.label(format!(
            "{} words / {} links / {} tags",
            word_count(&entry.body_markdown),
            entry.link_count(),
            entry.tags.len()
        ));
        let actions = follow_up_actions(&entry);
        if actions.is_empty() {
            ui_chrome::muted(ui, "No follow-up signals detected");
        } else {
            ui_chrome::section_label(ui, "Follow-Up Actions");
            for action in actions.iter().take(5) {
                ui.add(egui::Label::new(format!("- {action}")).wrap());
            }
        }

        ui_chrome::section_label(ui, "Focus Links");
        if let Some(focus) = compact_link_focus_ui(ui, &entry.links) {
            self.apply_link_focus(focus);
        }
        self.related_entries_ui(ui, &entry);
    }

    fn related_entries_ui(&mut self, ui: &mut egui::Ui, entry: &NotebookEntry) {
        ui_chrome::section_label(ui, "Related Entries");
        let related = self.related_entries(entry);
        if related.is_empty() {
            ui_chrome::empty_state(ui, "No related notebook entries");
            return;
        }

        for row in related.iter().take(5) {
            let label = format!("{}  {}", row.id, row.title);
            if ui
                .selectable_label(false, label)
                .on_hover_text("Open related notebook entry")
                .clicked()
            {
                self.selected_entry = Some(row.id.clone());
            }
            ui.add(
                egui::Label::new(
                    RichText::new(format!("{} / score {}", row.reason, row.score))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                )
                .wrap(),
            );
        }
    }

    fn related_entries(&self, entry: &NotebookEntry) -> Vec<RelatedEntryRow> {
        let source_tags = entry.tags.iter().cloned().collect::<BTreeSet<_>>();
        let source_links = link_tokens(&entry.links);
        let mut rows = Vec::new();

        for candidate in &self.notebook.entries {
            if candidate.id == entry.id {
                continue;
            }

            let shared_tags = candidate
                .tags
                .iter()
                .filter(|tag| source_tags.contains(*tag))
                .cloned()
                .collect::<Vec<_>>();
            let candidate_links = link_tokens(&candidate.links);
            let shared_links = candidate_links
                .intersection(&source_links)
                .cloned()
                .collect::<Vec<_>>();

            let score = shared_tags.len() + shared_links.len() * 2;
            if score == 0 {
                continue;
            }

            rows.push(RelatedEntryRow {
                id: candidate.id.clone(),
                title: candidate.title.clone(),
                reason: related_reason(&shared_tags, &shared_links),
                score,
            });
        }

        rows.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.title.cmp(&right.title))
        });
        rows
    }
}

fn link_count_rows(ui: &mut egui::Ui, links: &NotebookLinks) {
    ui.label(format!("Lots: {}", links.lots.len()));
    ui.label(format!("Wafers: {}", links.wafers.len()));
    ui.label(format!("Recipes: {}", links.recipes.len()));
    ui.label(format!("Tool runs: {}", links.tool_runs.len()));
    ui.label(format!("Metrology: {}", links.metrology.len()));
    ui.label(format!("Images: {}", links.images.len()));
}

fn link_metric_row(ui: &mut egui::Ui, label: &str, value: usize) {
    ui.label(label);
    ui.strong(value.to_string());
    ui.end_row();
}

fn entry_timeline_card(ui: &mut egui::Ui, entry: &NotebookEntry, selected: bool) -> egui::Response {
    let fill = if selected {
        ui.visuals().selection.bg_fill
    } else {
        ui.visuals().faint_bg_color
    };
    let stroke = if selected {
        egui::Stroke::new(1.5, Tone::Info.color())
    } else {
        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    };

    let inner = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Label::new(RichText::new(&entry.title).strong()).wrap());
                ui_chrome::status_pill(ui, &entry.id.to_string(), Tone::Info);
                if !follow_up_actions(entry).is_empty() {
                    ui_chrome::status_pill(ui, "follow-up", Tone::Warning);
                }
            });
            ui_chrome::muted(
                ui,
                format!(
                    "{} / updated {} / {} links",
                    entry.author,
                    entry.updated_at,
                    entry.link_count()
                ),
            );
            ui.add(egui::Label::new(markdown_excerpt(&entry.body_markdown, 150)).wrap());
            ui.horizontal_wrapped(|ui| {
                for tag in entry.tags.iter().take(5) {
                    ui.label(RichText::new(format!("#{tag}")).small());
                }
            });
            link_summary_pills(ui, &entry.links);
        });

    inner
        .response
        .interact(egui::Sense::click())
        .on_hover_text("Open notebook entry")
}

fn link_summary_pills(ui: &mut egui::Ui, links: &NotebookLinks) {
    ui.horizontal_wrapped(|ui| {
        link_count_pill(ui, "lots", links.lots.len(), Tone::Info);
        link_count_pill(ui, "wafers", links.wafers.len(), Tone::Info);
        link_count_pill(ui, "recipes", links.recipes.len(), Tone::Success);
        link_count_pill(ui, "runs", links.tool_runs.len(), Tone::Warning);
        link_count_pill(ui, "metrology", links.metrology.len(), Tone::Neutral);
        link_count_pill(ui, "images", links.images.len(), Tone::Neutral);
    });
}

fn link_count_pill(ui: &mut egui::Ui, label: &str, count: usize, tone: Tone) {
    if count > 0 {
        ui_chrome::status_pill(ui, &format!("{count} {label}"), tone);
    }
}

fn linked_data_ui(ui: &mut egui::Ui, links: &NotebookLinks) -> Option<LinkFocus> {
    let chips = link_chip_data(links);
    if chips.is_empty() {
        ui_chrome::empty_state(ui, "No linked data on this entry");
        return None;
    }

    let mut focus = None;
    let narrow = ui.available_width() < 680.0;
    if narrow {
        ui.vertical(|ui| {
            for kind in NotebookLinkKind::ALL {
                let kind_chips = chips
                    .iter()
                    .filter(|chip| chip.kind == kind)
                    .collect::<Vec<_>>();
                set_focus(
                    &mut focus,
                    link_group_ui(ui, link_kind_group_title(kind), &kind_chips),
                );
            }
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            for kind in NotebookLinkKind::ALL {
                let kind_chips = chips
                    .iter()
                    .filter(|chip| chip.kind == kind)
                    .collect::<Vec<_>>();
                set_focus(
                    &mut focus,
                    link_group_ui(ui, link_kind_group_title(kind), &kind_chips),
                );
            }
        });
    }
    focus
}

fn compact_link_focus_ui(ui: &mut egui::Ui, links: &NotebookLinks) -> Option<LinkFocus> {
    let chips = link_chip_data(links);
    if chips.is_empty() {
        ui_chrome::muted(ui, "No structured links");
        return None;
    }

    let mut focus = None;
    ui.horizontal_wrapped(|ui| {
        for chip in &chips {
            if link_focus_chip(ui, chip).clicked() {
                focus = Some(chip.focus());
            }
        }
    });
    focus
}

fn link_group_ui(ui: &mut egui::Ui, title: &str, chips: &[&LinkChipData]) -> Option<LinkFocus> {
    if chips.is_empty() {
        return None;
    }

    let inner = egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width().clamp(180.0, 280.0).min(240.0));
            ui.set_max_width(300.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(title).strong());
                ui_chrome::status_pill(ui, &chips.len().to_string(), Tone::Neutral);
            });

            let mut focus = None;
            for chip in chips {
                if link_focus_chip(ui, chip).clicked() {
                    focus = Some(chip.focus());
                }
            }
            focus
        });
    inner.inner
}

fn link_focus_chip(ui: &mut egui::Ui, chip: &LinkChipData) -> egui::Response {
    let color = link_chip_color(ui, chip.tone);
    let text_color = ui_chrome::readable_text_color(color);
    let inner = egui::Frame::new()
        .fill(color)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(7, 4))
        .show(ui, |ui| {
            ui.set_max_width(ui.available_width().clamp(130.0, 300.0));
            ui.add(
                egui::Label::new(
                    RichText::new(format!("{}: {}", chip.prefix, chip.value))
                        .small()
                        .strong()
                        .color(text_color),
                )
                .wrap(),
            );
            if !chip.detail.is_empty() {
                ui.add(
                    egui::Label::new(
                        RichText::new(truncate_chars(&chip.detail, 92))
                            .small()
                            .color(text_color),
                    )
                    .wrap(),
                );
            }
        });
    inner
        .response
        .interact(egui::Sense::click())
        .on_hover_text("Filter notebook entries by this linked object")
}

fn link_chip_color(ui: &egui::Ui, tone: Tone) -> Color32 {
    match tone {
        Tone::Info => Color32::from_rgb(52, 112, 170),
        Tone::Success => Color32::from_rgb(54, 130, 88),
        Tone::Warning => Color32::from_rgb(146, 111, 38),
        Tone::Danger => Color32::from_rgb(150, 62, 62),
        Tone::Neutral => ui.visuals().extreme_bg_color,
    }
}

fn set_focus(current: &mut Option<LinkFocus>, next: Option<LinkFocus>) {
    if next.is_some() {
        *current = next;
    }
}

fn link_chip_data(links: &NotebookLinks) -> Vec<LinkChipData> {
    let mut chips = Vec::new();
    chips.extend(links.lots.iter().map(|lot| LinkChipData {
        kind: NotebookLinkKind::Lot,
        prefix: "lot".to_string(),
        value: lot.to_string(),
        detail: "MES lot history".to_string(),
        tone: Tone::Info,
    }));
    chips.extend(links.wafers.iter().map(|wafer| LinkChipData {
        kind: NotebookLinkKind::Wafer,
        prefix: "wafer".to_string(),
        value: wafer.to_string(),
        detail: "slot-level evidence".to_string(),
        tone: Tone::Info,
    }));
    chips.extend(links.recipes.iter().map(|recipe| LinkChipData {
        kind: NotebookLinkKind::Recipe,
        prefix: "recipe".to_string(),
        value: recipe.to_string(),
        detail: "process recipe".to_string(),
        tone: Tone::Success,
    }));
    chips.extend(links.tool_runs.iter().map(|run| LinkChipData {
        kind: NotebookLinkKind::ToolRun,
        prefix: "run".to_string(),
        value: run.to_string(),
        detail: "equipment run".to_string(),
        tone: Tone::Warning,
    }));
    chips.extend(links.metrology.iter().map(|metrology| {
        let target = metrology
            .wafer_id
            .as_ref()
            .map(ToString::to_string)
            .or_else(|| metrology.lot_id.as_ref().map(ToString::to_string))
            .unwrap_or_else(|| "unscoped".to_string());
        LinkChipData {
            kind: NotebookLinkKind::Metrology,
            prefix: metrology.kind.label().to_string(),
            value: metrology.id.to_string(),
            detail: format!("{target} / {}", metrology.summary),
            tone: Tone::Neutral,
        }
    }));
    chips.extend(links.images.iter().map(|image| LinkChipData {
        kind: NotebookLinkKind::Image,
        prefix: "image".to_string(),
        value: image.id.to_string(),
        detail: format!("{} / {}", image.label, image.uri),
        tone: Tone::Neutral,
    }));
    chips
}

fn markdown_preview_ui(ui: &mut egui::Ui, markdown: &str) {
    if markdown.trim().is_empty() {
        ui_chrome::empty_state(ui, "No note body");
        return;
    }

    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_height(180.0);
            for line in markdown.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    ui.add_space(6.0);
                    continue;
                }

                let heading_level = trimmed
                    .chars()
                    .take_while(|character| *character == '#')
                    .count();
                if (1..=3).contains(&heading_level) {
                    let heading = trimmed[heading_level..].trim();
                    let size = match heading_level {
                        1 => 18.0,
                        2 => 16.0,
                        _ => 14.0,
                    };
                    ui.label(RichText::new(heading).strong().size(size));
                    continue;
                }

                if let Some(text) = trimmed
                    .strip_prefix("- [ ] ")
                    .or_else(|| trimmed.strip_prefix("* [ ] "))
                {
                    let mut checked = false;
                    ui.add_enabled(false, egui::Checkbox::new(&mut checked, text));
                    continue;
                }

                if let Some(text) = trimmed
                    .strip_prefix("- [x] ")
                    .or_else(|| trimmed.strip_prefix("- [X] "))
                    .or_else(|| trimmed.strip_prefix("* [x] "))
                    .or_else(|| trimmed.strip_prefix("* [X] "))
                {
                    let mut checked = true;
                    ui.add_enabled(false, egui::Checkbox::new(&mut checked, text));
                    continue;
                }

                if let Some(text) = trimmed
                    .strip_prefix("- ")
                    .or_else(|| trimmed.strip_prefix("* "))
                {
                    ui.add(egui::Label::new(format!("- {text}")).wrap());
                    continue;
                }

                ui.add(egui::Label::new(clean_markdown_line(trimmed)).wrap());
            }
        });
}

fn follow_up_actions(entry: &NotebookEntry) -> Vec<String> {
    let mut actions = Vec::new();
    let body = entry.body_markdown.to_lowercase();

    if body.contains("follow-up")
        || body.contains("needs")
        || entry.tags.iter().any(|tag| tag == "follow-up")
    {
        push_action_once(
            &mut actions,
            "Resolve the stated follow-up before releasing the split.",
        );
    }
    if body.contains("re-check")
        || body.contains("recheck")
        || body.contains("watch")
        || body.contains("next lot")
    {
        push_action_once(
            &mut actions,
            "Schedule a repeat check on the next linked lot or wafer.",
        );
    }
    if body.contains("attach") || entry.links.images.is_empty() && !entry.links.metrology.is_empty()
    {
        push_action_once(
            &mut actions,
            "Attach image, trace, or inspection evidence for the observation.",
        );
    }
    if !entry.links.metrology.is_empty()
        && (body.contains("cd") || body.contains("thickness") || body.contains("metrology"))
    {
        push_action_once(
            &mut actions,
            "Review linked metrology before changing recipe disposition.",
        );
    }
    if !entry.links.recipes.is_empty() && !entry.links.tool_runs.is_empty() {
        push_action_once(
            &mut actions,
            "Compare recipe settings against the linked tool run.",
        );
    }

    actions
}

fn push_action_once(actions: &mut Vec<String>, action: &str) {
    if !actions.iter().any(|existing| existing == action) {
        actions.push(action.to_string());
    }
}

fn add_tag_once(entry: &mut NotebookEntry, tag: &str) {
    if !entry.tags.iter().any(|existing| existing == tag) {
        entry.tags.push(tag.to_string());
    }
}

fn append_section_once(body: &mut String, heading: &str, section_body: &str) {
    if body.contains(heading) {
        return;
    }
    if !body.trim().is_empty() {
        body.push_str("\n\n");
    }
    body.push_str(heading);
    body.push('\n');
    body.push_str(section_body.trim());
}

fn follow_up_plan_markdown(entry: &NotebookEntry) -> String {
    let mut lines = Vec::new();
    if !entry.links.lots.is_empty() {
        lines.push(format!(
            "- [ ] Confirm lot disposition for {}.",
            join_display(entry.links.lots.iter())
        ));
    }
    if !entry.links.wafers.is_empty() {
        lines.push(format!(
            "- [ ] Re-check wafer evidence for {}.",
            join_display(entry.links.wafers.iter())
        ));
    }
    if !entry.links.metrology.is_empty() {
        lines.push("- [ ] Review linked metrology and record pass/fail basis.".to_string());
    }
    if entry.links.images.is_empty() {
        lines.push("- [ ] Attach representative image or trace evidence.".to_string());
    }
    if !entry.links.recipes.is_empty() {
        lines.push("- [ ] Decide whether the linked recipe needs revision notes.".to_string());
    }
    if lines.is_empty() {
        lines.push("- [ ] Assign owner, due condition, and acceptance criteria.".to_string());
    }
    lines.join("\n")
}

fn metrology_review_markdown(entry: &NotebookEntry) -> String {
    if entry.links.metrology.is_empty() {
        return "- No metrology links are attached yet; add measurement IDs before review."
            .to_string();
    }

    let mut lines = Vec::new();
    for metrology in &entry.links.metrology {
        let target = metrology
            .wafer_id
            .as_ref()
            .map(ToString::to_string)
            .or_else(|| metrology.lot_id.as_ref().map(ToString::to_string))
            .unwrap_or_else(|| "unscoped".to_string());
        lines.push(format!(
            "- {} / {} / {}: {}",
            metrology.id,
            metrology.kind.label(),
            target,
            metrology.summary
        ));
    }
    lines.push("- [ ] Record whether data supports recipe, lot, or wafer action.".to_string());
    lines.join("\n")
}

fn image_evidence_markdown(entry: &NotebookEntry) -> String {
    if entry.links.images.is_empty() {
        return "- [ ] Attach SEM, optical, FDC trace, or map image and link the image ID here."
            .to_string();
    }

    let mut lines = entry
        .links
        .images
        .iter()
        .map(|image| format!("- {} / {} / {}", image.id, image.label, image.uri))
        .collect::<Vec<_>>();
    lines.push("- [ ] Verify image captures the failure mode and lot or wafer ID.".to_string());
    lines.join("\n")
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn markdown_excerpt(markdown: &str, max_chars: usize) -> String {
    let text = markdown
        .lines()
        .map(clean_markdown_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    truncate_chars(&text, max_chars)
}

fn clean_markdown_line(line: &str) -> String {
    let mut text = line.trim();
    while let Some(stripped) = text.strip_prefix('#') {
        text = stripped.trim_start();
    }
    for prefix in [
        "- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "- ", "* ",
    ] {
        if let Some(stripped) = text.strip_prefix(prefix) {
            text = stripped;
            break;
        }
    }
    text.replace("**", "").replace('`', "").trim().to_string()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let take = max_chars.saturating_sub(3);
    let mut truncated = text.chars().take(take).collect::<String>();
    truncated.truncate(truncated.trim_end().len());
    truncated.push_str("...");
    truncated
}

fn join_display<'a, T>(values: impl Iterator<Item = &'a T>) -> String
where
    T: ToString + 'a,
{
    values
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn link_tokens(links: &NotebookLinks) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    tokens.extend(links.lots.iter().map(ToString::to_string));
    tokens.extend(links.wafers.iter().map(ToString::to_string));
    tokens.extend(links.recipes.iter().map(ToString::to_string));
    tokens.extend(links.tool_runs.iter().map(ToString::to_string));
    for metrology in &links.metrology {
        tokens.insert(metrology.id.to_string());
        if let Some(lot_id) = &metrology.lot_id {
            tokens.insert(lot_id.to_string());
        }
        if let Some(wafer_id) = &metrology.wafer_id {
            tokens.insert(wafer_id.to_string());
        }
    }
    for image in &links.images {
        tokens.insert(image.id.to_string());
        tokens.insert(image.uri.clone());
    }
    tokens
}

fn related_reason(shared_tags: &[String], shared_links: &[String]) -> String {
    let mut parts = Vec::new();
    if !shared_links.is_empty() {
        parts.push(format!(
            "links {}",
            truncate_list(shared_links.iter().map(String::as_str), 3)
        ));
    }
    if !shared_tags.is_empty() {
        parts.push(format!(
            "tags {}",
            truncate_list(shared_tags.iter().map(String::as_str), 3)
        ));
    }
    parts.join(" / ")
}

fn truncate_list<'a>(values: impl Iterator<Item = &'a str>, limit: usize) -> String {
    let values = values.collect::<Vec<_>>();
    if values.len() <= limit {
        return values.join(", ");
    }
    format!("{} +{}", values[..limit].join(", "), values.len() - limit)
}

fn link_kind_group_title(kind: NotebookLinkKind) -> &'static str {
    match kind {
        NotebookLinkKind::Lot => "Lots",
        NotebookLinkKind::Wafer => "Wafers",
        NotebookLinkKind::Recipe => "Recipes",
        NotebookLinkKind::ToolRun => "Tool Runs",
        NotebookLinkKind::Metrology => "Metrology",
        NotebookLinkKind::Image => "Images",
    }
}

fn link_kind_short_label(kind: NotebookLinkKind) -> &'static str {
    match kind {
        NotebookLinkKind::Lot => "lots",
        NotebookLinkKind::Wafer => "wafers",
        NotebookLinkKind::Recipe => "recipes",
        NotebookLinkKind::ToolRun => "runs",
        NotebookLinkKind::Metrology => "metrology",
        NotebookLinkKind::Image => "images",
    }
}
