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
        self.filter = NotebookFilter::default();
        self.selected_tag = "All tags".to_string();
        self.selected_link_kind = None;
        entry_id
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.sync_filter_controls();
        let filtered_ids = self.filtered_entry_ids();
        self.ensure_selection(&filtered_ids);

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
                    },
                );

                if self.notebook.entries.is_empty() {
                    ui_chrome::empty_state(ui, "No notebook entries loaded");
                    let _ = status;
                    return;
                }

                self.filter_bar_ui(ui);
                ui.separator();

                if ui.available_width() < 760.0 {
                    self.entry_list_ui(ui, &filtered_ids);
                    ui.separator();
                    self.selected_entry_ui(ui, status);
                } else {
                    ui.columns(2, |columns| {
                        columns[0].set_min_width(250.0);
                        self.entry_list_ui(&mut columns[0], &filtered_ids);
                        self.selected_entry_ui(&mut columns[1], status);
                    });
                }
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.sync_filter_controls();
        ui_chrome::section_label(ui, "Lab Notebook");
        ui.label(format!("{} entries", self.notebook.entries.len()));
        ui.label(format!("{} tags", self.notebook.tags().len()));
        ui.separator();

        if let Some(entry) = self.selected_entry() {
            ui.strong(&entry.title);
            ui.small(format!("{}  {}", entry.id, entry.updated_at));
            ui_chrome::section_label(ui, "Tags");
            ui.horizontal_wrapped(|ui| {
                for tag in &entry.tags {
                    ui_chrome::status_pill(ui, tag, Tone::Neutral);
                }
            });
            ui_chrome::section_label(ui, "Structured Links");
            link_count_rows(ui, &entry.links);
        } else {
            ui_chrome::empty_state(ui, "No entry selected");
        }
    }

    pub(crate) fn link_filter_ui(&mut self, ui: &mut egui::Ui) {
        self.sync_filter_controls();
        ui_chrome::section_label(ui, "Notebook Filters");
        ui.label("Search");
        ui.add(TextEdit::singleline(&mut self.filter.query).hint_text("entry, tag, link id"));

        ui.label("Tag");
        egui::ComboBox::from_id_salt("notebook_tag_filter")
            .selected_text(&self.selected_tag)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_tag, "All tags".to_string(), "All tags");
                for tag in self.notebook.tags() {
                    ui.selectable_value(&mut self.selected_tag, tag.clone(), tag);
                }
            });

        ui.label("Link type");
        let selected_link_label = self
            .selected_link_kind
            .map(|kind| kind.label())
            .unwrap_or("Any link");
        egui::ComboBox::from_id_salt("notebook_link_filter")
            .selected_text(selected_link_label)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_link_kind, None, "Any link");
                for kind in NotebookLinkKind::ALL {
                    ui.selectable_value(&mut self.selected_link_kind, Some(kind), kind.label());
                }
            });

        if ui.button("Clear filters").clicked() {
            self.filter = NotebookFilter::default();
            self.selected_tag = "All tags".to_string();
            self.selected_link_kind = None;
        }

        self.sync_filter_controls();
        ui.separator();
        let matches = self.notebook.filtered_entries(&self.filter).len();
        ui_chrome::metric_tile(ui, "Matches", matches, "current filter");
    }

    fn filter_bar_ui(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 520.0 {
            ui.label("Search");
            ui.add_sized(
                [ui.available_width().min(280.0), 22.0],
                TextEdit::singleline(&mut self.filter.query).hint_text("residue, RUN-ETCH, CD"),
            );
            egui::ComboBox::from_id_salt("notebook_dashboard_tag")
                .selected_text(&self.selected_tag)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_tag, "All tags".to_string(), "All tags");
                    for tag in self.notebook.tags() {
                        ui.selectable_value(&mut self.selected_tag, tag.clone(), tag);
                    }
                });

            let selected_link_label = self
                .selected_link_kind
                .map(|kind| kind.label())
                .unwrap_or("Any link");
            egui::ComboBox::from_id_salt("notebook_dashboard_link_kind")
                .selected_text(selected_link_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_link_kind, None, "Any link");
                    for kind in NotebookLinkKind::ALL {
                        ui.selectable_value(&mut self.selected_link_kind, Some(kind), kind.label());
                    }
                });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label("Search");
                ui.add_sized(
                    [220.0, 22.0],
                    TextEdit::singleline(&mut self.filter.query).hint_text("residue, RUN-ETCH, CD"),
                );

                egui::ComboBox::from_id_salt("notebook_dashboard_tag")
                    .selected_text(&self.selected_tag)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.selected_tag,
                            "All tags".to_string(),
                            "All tags",
                        );
                        for tag in self.notebook.tags() {
                            ui.selectable_value(&mut self.selected_tag, tag.clone(), tag);
                        }
                    });

                let selected_link_label = self
                    .selected_link_kind
                    .map(|kind| kind.label())
                    .unwrap_or("Any link");
                egui::ComboBox::from_id_salt("notebook_dashboard_link_kind")
                    .selected_text(selected_link_label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.selected_link_kind, None, "Any link");
                        for kind in NotebookLinkKind::ALL {
                            ui.selectable_value(
                                &mut self.selected_link_kind,
                                Some(kind),
                                kind.label(),
                            );
                        }
                    });
            });
        }
        self.sync_filter_controls();
    }

    fn entry_list_ui(&mut self, ui: &mut egui::Ui, filtered_ids: &[NotebookEntryId]) {
        ui_chrome::section_label(ui, "Timeline");
        if filtered_ids.is_empty() {
            ui_chrome::empty_state(ui, "No matching entries");
            return;
        }

        for entry_id in filtered_ids {
            if let Some(entry) = self.notebook.entry(entry_id) {
                let selected = self
                    .selected_entry
                    .as_ref()
                    .is_some_and(|selected_id| selected_id == entry_id);
                let label = format!(
                    "{}\n{} links  {}",
                    entry.title,
                    entry.link_count(),
                    entry.updated_at
                );
                if ui
                    .selectable_label(selected, RichText::new(label).line_height(Some(17.0)))
                    .clicked()
                {
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

        let Some(entry) = self.notebook.entry_mut(&entry_id) else {
            ui_chrome::empty_state(ui, "Selected entry is unavailable");
            return;
        };

        ui.horizontal_wrapped(|ui| {
            ui.heading(&entry.title);
            ui_chrome::status_pill(ui, &entry.id.to_string(), Tone::Info);
        });
        ui_chrome::muted(
            ui,
            format!(
                "{} / created {} / updated {}",
                entry.author, entry.created_at, entry.updated_at
            ),
        );

        ui.horizontal_wrapped(|ui| {
            for tag in &entry.tags {
                ui_chrome::status_pill(ui, tag, Tone::Neutral);
            }
        });

        ui.separator();
        ui.label(RichText::new("Markdown").strong());
        let response = ui.add(
            TextEdit::multiline(&mut entry.body_markdown)
                .desired_rows(12)
                .desired_width(f32::INFINITY),
        );
        if response.changed() {
            *status = format!("edited notebook entry {}", entry.id);
        }

        ui.separator();
        ui.label(RichText::new("Linked Data").strong());
        link_chips_ui(ui, &entry.links);
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
}

fn link_count_rows(ui: &mut egui::Ui, links: &NotebookLinks) {
    ui.label(format!("Lots: {}", links.lots.len()));
    ui.label(format!("Wafers: {}", links.wafers.len()));
    ui.label(format!("Recipes: {}", links.recipes.len()));
    ui.label(format!("Tool runs: {}", links.tool_runs.len()));
    ui.label(format!("Metrology: {}", links.metrology.len()));
    ui.label(format!("Images: {}", links.images.len()));
}

fn link_chips_ui(ui: &mut egui::Ui, links: &NotebookLinks) {
    if ui.available_width() < 760.0 {
        ui.vertical(|ui| link_chips_contents(ui, links));
    } else {
        ui.horizontal_wrapped(|ui| link_chips_contents(ui, links));
    }
}

fn link_chips_contents(ui: &mut egui::Ui, links: &NotebookLinks) {
    for lot in &links.lots {
        link_chip(ui, "lot", &lot.to_string(), Tone::Info);
    }
    for wafer in &links.wafers {
        link_chip(ui, "wafer", &wafer.to_string(), Tone::Info);
    }
    for recipe in &links.recipes {
        link_chip(ui, "recipe", &recipe.to_string(), Tone::Success);
    }
    for run in &links.tool_runs {
        link_chip(ui, "run", &run.to_string(), Tone::Warning);
    }
    for metrology in &links.metrology {
        link_chip(
            ui,
            metrology.kind.label(),
            &format!("{} {}", metrology.id, metrology.summary),
            Tone::Neutral,
        );
    }
    for image in &links.images {
        link_chip(
            ui,
            "image",
            &format!("{} {}", image.id, image.label),
            Tone::Neutral,
        );
    }
}

fn link_chip(ui: &mut egui::Ui, prefix: &str, value: &str, tone: Tone) {
    let color = match tone {
        Tone::Info => Color32::from_rgb(52, 112, 170),
        Tone::Success => Color32::from_rgb(54, 130, 88),
        Tone::Warning => Color32::from_rgb(146, 111, 38),
        Tone::Danger => Color32::from_rgb(150, 62, 62),
        Tone::Neutral => ui.visuals().faint_bg_color,
    };
    egui::Frame::new()
        .fill(color)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.set_max_width(ui.available_width().clamp(96.0, 360.0));
            let text_color = ui_chrome::readable_text_color(color);
            ui.add(
                egui::Label::new(
                    RichText::new(format!("{prefix}: {value}"))
                        .small()
                        .color(text_color),
                )
                .wrap(),
            );
        });
}
