use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::{
    mes::{LotId, WaferId},
    metrology::MeasurementKind,
    recipe::{RecipeId, ToolRunId},
};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

string_id!(NotebookEntryId);
string_id!(MetrologyLinkId);
string_id!(ImageLinkId);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabNotebook {
    #[serde(default)]
    pub entries: Vec<NotebookEntry>,
}

impl LabNotebook {
    pub fn sample() -> Self {
        sample_lab_notebook()
    }

    pub fn filtered_entries(&self, filter: &NotebookFilter) -> Vec<&NotebookEntry> {
        self.entries
            .iter()
            .filter(|entry| filter.matches(entry))
            .collect()
    }

    pub fn tags(&self) -> Vec<String> {
        let mut tags = self
            .entries
            .iter()
            .flat_map(|entry| entry.tags.iter().cloned())
            .collect::<Vec<_>>();
        tags.sort();
        tags.dedup();
        tags
    }

    pub fn entry(&self, id: &NotebookEntryId) -> Option<&NotebookEntry> {
        self.entries.iter().find(|entry| &entry.id == id)
    }

    pub fn entry_mut(&mut self, id: &NotebookEntryId) -> Option<&mut NotebookEntry> {
        self.entries.iter_mut().find(|entry| &entry.id == id)
    }

    pub fn validate(&self) -> Vec<NotebookValidationFinding> {
        let mut findings = Vec::new();
        let mut entry_ids = BTreeSet::new();
        for entry in &self.entries {
            validate_entry(entry, &mut entry_ids, &mut findings);
        }
        findings
    }

    pub fn validate_with_context(
        &self,
        context: &NotebookValidationContext,
    ) -> Vec<NotebookValidationFinding> {
        let mut findings = self.validate();
        for entry in &self.entries {
            validate_entry_links_with_context(entry, context, &mut findings);
        }
        findings
    }

    pub fn is_valid(&self) -> bool {
        self.validate()
            .iter()
            .all(|finding| finding.severity != NotebookValidationSeverity::Error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotebookValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotebookValidationFinding {
    pub severity: NotebookValidationSeverity,
    pub message: String,
}

impl NotebookValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: NotebookValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: NotebookValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NotebookValidationContext {
    pub lot_ids: BTreeSet<String>,
    pub wafer_ids: BTreeSet<String>,
    pub wafer_ids_by_lot: BTreeMap<String, BTreeSet<String>>,
    pub recipe_ids: BTreeSet<String>,
    pub metrology_ids: BTreeSet<String>,
}

impl NotebookValidationContext {
    pub fn from_mes_recipes_and_metrology<I, J>(
        mes: &crate::mes::FabMesData,
        recipe_ids: I,
        metrology_ids: J,
    ) -> Self
    where
        I: IntoIterator<Item = String>,
        J: IntoIterator<Item = String>,
    {
        let mut context = Self {
            recipe_ids: recipe_ids.into_iter().collect(),
            metrology_ids: metrology_ids.into_iter().collect(),
            ..Self::default()
        };
        for (lot_id, lot) in &mes.lots {
            let lot_id = lot_id.as_str().to_string();
            context.lot_ids.insert(lot_id.clone());
            let wafer_ids = context.wafer_ids_by_lot.entry(lot_id).or_default();
            for wafer in &lot.wafers {
                let wafer_id = wafer.id.as_str().to_string();
                context.wafer_ids.insert(wafer_id.clone());
                wafer_ids.insert(wafer_id);
                wafer_ids.insert(format!("W{:02}", wafer.slot));
            }
        }
        context
    }

    fn contains_lot(&self, lot_id: &LotId) -> bool {
        self.lot_ids.contains(lot_id.as_str())
    }

    fn contains_wafer(&self, wafer_id: &WaferId) -> bool {
        self.wafer_ids.contains(wafer_id.as_str())
    }

    fn contains_wafer_in_lot(&self, lot_id: &LotId, wafer_id: &WaferId) -> bool {
        self.wafer_ids_by_lot
            .get(lot_id.as_str())
            .is_some_and(|wafer_ids| wafer_ids.contains(wafer_id.as_str()))
    }

    fn contains_recipe(&self, recipe_id: &RecipeId) -> bool {
        self.recipe_ids.contains(recipe_id.as_str())
    }

    fn contains_metrology(&self, metrology_id: &MetrologyLinkId) -> bool {
        self.metrology_ids.contains(metrology_id.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookEntry {
    pub id: NotebookEntryId,
    pub title: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
    pub body_markdown: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub links: NotebookLinks,
}

impl NotebookEntry {
    pub fn has_link_kind(&self, kind: NotebookLinkKind) -> bool {
        self.links.has_kind(kind)
    }

    pub fn link_count(&self) -> usize {
        self.links.count()
    }

    fn matches_query(&self, normalized_query: &str) -> bool {
        if normalized_query.is_empty() {
            return true;
        }

        let mut haystack = format!(
            "{}\n{}\n{}\n{}\n{}",
            self.id, self.title, self.author, self.body_markdown, self.updated_at
        );
        for tag in &self.tags {
            haystack.push('\n');
            haystack.push_str(tag);
        }
        for label in self.links.search_labels() {
            haystack.push('\n');
            haystack.push_str(&label);
        }
        haystack.to_lowercase().contains(normalized_query)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookLinks {
    #[serde(default)]
    pub lots: Vec<LotId>,
    #[serde(default)]
    pub wafers: Vec<WaferId>,
    #[serde(default)]
    pub recipes: Vec<RecipeId>,
    #[serde(default)]
    pub tool_runs: Vec<ToolRunId>,
    #[serde(default)]
    pub metrology: Vec<MetrologyLink>,
    #[serde(default)]
    pub images: Vec<ImageLink>,
}

impl NotebookLinks {
    pub fn count(&self) -> usize {
        self.lots.len()
            + self.wafers.len()
            + self.recipes.len()
            + self.tool_runs.len()
            + self.metrology.len()
            + self.images.len()
    }

    pub fn has_kind(&self, kind: NotebookLinkKind) -> bool {
        match kind {
            NotebookLinkKind::Lot => !self.lots.is_empty(),
            NotebookLinkKind::Wafer => !self.wafers.is_empty(),
            NotebookLinkKind::Recipe => !self.recipes.is_empty(),
            NotebookLinkKind::ToolRun => !self.tool_runs.is_empty(),
            NotebookLinkKind::Metrology => !self.metrology.is_empty(),
            NotebookLinkKind::Image => !self.images.is_empty(),
        }
    }

    pub fn search_labels(&self) -> Vec<String> {
        let mut labels = Vec::new();
        labels.extend(self.lots.iter().map(ToString::to_string));
        labels.extend(self.wafers.iter().map(ToString::to_string));
        labels.extend(self.recipes.iter().map(ToString::to_string));
        labels.extend(self.tool_runs.iter().map(ToString::to_string));
        labels.extend(self.metrology.iter().map(MetrologyLink::search_label));
        labels.extend(self.images.iter().map(ImageLink::search_label));
        labels
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetrologyLink {
    pub id: MetrologyLinkId,
    pub kind: MeasurementKind,
    pub lot_id: Option<LotId>,
    pub wafer_id: Option<WaferId>,
    pub summary: String,
}

impl MetrologyLink {
    fn search_label(&self) -> String {
        format!(
            "{} {} {} {}",
            self.id,
            self.kind.label(),
            self.lot_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            self.wafer_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageLink {
    pub id: ImageLinkId,
    pub label: String,
    pub uri: String,
}

impl ImageLink {
    fn search_label(&self) -> String {
        format!("{} {} {}", self.id, self.label, self.uri)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotebookLinkKind {
    Lot,
    Wafer,
    Recipe,
    ToolRun,
    Metrology,
    Image,
}

impl NotebookLinkKind {
    pub const ALL: [Self; 6] = [
        Self::Lot,
        Self::Wafer,
        Self::Recipe,
        Self::ToolRun,
        Self::Metrology,
        Self::Image,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Lot => "lots",
            Self::Wafer => "wafers",
            Self::Recipe => "recipes",
            Self::ToolRun => "tool runs",
            Self::Metrology => "metrology",
            Self::Image => "images",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookFilter {
    pub query: String,
    pub tag: Option<String>,
    pub link_kind: Option<NotebookLinkKind>,
}

impl NotebookFilter {
    pub fn matches(&self, entry: &NotebookEntry) -> bool {
        let normalized_query = self.query.trim().to_lowercase();
        entry.matches_query(&normalized_query)
            && self
                .tag
                .as_ref()
                .is_none_or(|tag| entry.tags.iter().any(|entry_tag| entry_tag == tag))
            && self.link_kind.is_none_or(|kind| entry.has_link_kind(kind))
    }
}

fn validate_entry(
    entry: &NotebookEntry,
    entry_ids: &mut BTreeSet<NotebookEntryId>,
    findings: &mut Vec<NotebookValidationFinding>,
) {
    if entry.id.as_str().trim().is_empty() {
        findings.push(NotebookValidationFinding::error(
            "notebook entry id cannot be empty",
        ));
    } else if !entry_ids.insert(entry.id.clone()) {
        findings.push(NotebookValidationFinding::error(format!(
            "notebook contains duplicate entry {}",
            entry.id
        )));
    }
    if entry.title.trim().is_empty() {
        findings.push(NotebookValidationFinding::warning(format!(
            "notebook entry {} has no title",
            entry.id
        )));
    }
    if entry.author.trim().is_empty() {
        findings.push(NotebookValidationFinding::warning(format!(
            "notebook entry {} has no author",
            entry.id
        )));
    }
    if entry.created_at.trim().is_empty() {
        findings.push(NotebookValidationFinding::warning(format!(
            "notebook entry {} has no created timestamp",
            entry.id
        )));
    }
    if entry.updated_at.trim().is_empty() {
        findings.push(NotebookValidationFinding::warning(format!(
            "notebook entry {} has no updated timestamp",
            entry.id
        )));
    }
    if !entry.created_at.trim().is_empty()
        && !entry.updated_at.trim().is_empty()
        && entry.updated_at < entry.created_at
    {
        findings.push(NotebookValidationFinding::error(format!(
            "notebook entry {} updated timestamp is before created timestamp",
            entry.id
        )));
    }
    if entry.body_markdown.trim().is_empty() {
        findings.push(NotebookValidationFinding::warning(format!(
            "notebook entry {} has no body",
            entry.id
        )));
    }

    let mut tags = BTreeSet::new();
    for tag in &entry.tags {
        if tag.trim().is_empty() {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} has an empty tag",
                entry.id
            )));
        } else if !tags.insert(tag.clone()) {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} repeats tag {tag}",
                entry.id
            )));
        }
    }
    validate_links(entry, findings);
}

fn validate_links(entry: &NotebookEntry, findings: &mut Vec<NotebookValidationFinding>) {
    warn_duplicate_ids(
        &entry.id,
        "lot",
        entry.links.lots.iter().map(LotId::as_str),
        findings,
    );
    warn_duplicate_ids(
        &entry.id,
        "wafer",
        entry.links.wafers.iter().map(WaferId::as_str),
        findings,
    );
    warn_duplicate_ids(
        &entry.id,
        "recipe",
        entry.links.recipes.iter().map(RecipeId::as_str),
        findings,
    );
    warn_duplicate_ids(
        &entry.id,
        "tool run",
        entry
            .links
            .tool_runs
            .iter()
            .map(|tool_run| tool_run.0.as_str()),
        findings,
    );

    let mut metrology_ids = BTreeSet::new();
    for link in &entry.links.metrology {
        if link.id.as_str().trim().is_empty() {
            findings.push(NotebookValidationFinding::error(format!(
                "notebook entry {} has metrology link with empty id",
                entry.id
            )));
        } else if !metrology_ids.insert(link.id.clone()) {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} repeats metrology link {}",
                entry.id, link.id
            )));
        }
        if link.summary.trim().is_empty() {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} metrology link {} has no summary",
                entry.id, link.id
            )));
        }
        if link.wafer_id.is_some() && link.lot_id.is_none() {
            findings.push(NotebookValidationFinding::error(format!(
                "notebook entry {} metrology link {} references a wafer without a lot",
                entry.id, link.id
            )));
        }
    }

    let mut image_ids = BTreeSet::new();
    for link in &entry.links.images {
        if link.id.as_str().trim().is_empty() {
            findings.push(NotebookValidationFinding::error(format!(
                "notebook entry {} has image link with empty id",
                entry.id
            )));
        } else if !image_ids.insert(link.id.clone()) {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} repeats image link {}",
                entry.id, link.id
            )));
        }
        if link.label.trim().is_empty() {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} image link {} has no label",
                entry.id, link.id
            )));
        }
        if link.uri.trim().is_empty() {
            findings.push(NotebookValidationFinding::error(format!(
                "notebook entry {} image link {} has no URI",
                entry.id, link.id
            )));
        }
    }
}

fn warn_duplicate_ids<'a>(
    entry_id: &NotebookEntryId,
    label: &str,
    ids: impl Iterator<Item = &'a str>,
    findings: &mut Vec<NotebookValidationFinding>,
) {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() {
            findings.push(NotebookValidationFinding::error(format!(
                "notebook entry {entry_id} has empty {label} link"
            )));
        } else if !seen.insert(id.to_string()) {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {entry_id} repeats {label} link {id}"
            )));
        }
    }
}

fn validate_entry_links_with_context(
    entry: &NotebookEntry,
    context: &NotebookValidationContext,
    findings: &mut Vec<NotebookValidationFinding>,
) {
    for lot_id in &entry.links.lots {
        if !context.contains_lot(lot_id) {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} references external lot {lot_id}",
                entry.id
            )));
        }
    }
    for wafer_id in &entry.links.wafers {
        if !context.contains_wafer(wafer_id) {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} references external wafer {wafer_id}",
                entry.id
            )));
        }
    }
    for recipe_id in &entry.links.recipes {
        if !context.contains_recipe(recipe_id) {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} references external recipe {recipe_id}",
                entry.id
            )));
        }
    }
    for link in &entry.links.metrology {
        if !context.contains_metrology(&link.id) {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} references external metrology {}",
                entry.id, link.id
            )));
        }
        if let Some(lot_id) = &link.lot_id
            && !context.contains_lot(lot_id)
        {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} metrology link {} references external lot {lot_id}",
                entry.id, link.id
            )));
        }
        if let Some(wafer_id) = &link.wafer_id
            && !context.contains_wafer(wafer_id)
        {
            findings.push(NotebookValidationFinding::warning(format!(
                "notebook entry {} metrology link {} references external wafer {wafer_id}",
                entry.id, link.id
            )));
        }
        if let (Some(lot_id), Some(wafer_id)) = (&link.lot_id, &link.wafer_id)
            && context.contains_lot(lot_id)
            && context.contains_wafer(wafer_id)
            && !context.contains_wafer_in_lot(lot_id, wafer_id)
        {
            findings.push(NotebookValidationFinding::error(format!(
                "notebook entry {} metrology link {} references wafer {wafer_id} outside lot {lot_id}",
                entry.id, link.id
            )));
        }
    }
}

pub fn sample_lab_notebook() -> LabNotebook {
    LabNotebook {
        entries: vec![
            NotebookEntry {
                id: NotebookEntryId::from("E-0042"),
                title: "Reduce poly etch residue".to_string(),
                author: "process.eng".to_string(),
                created_at: "2026-04-22T13:30:00Z".to_string(),
                updated_at: "2026-04-22T16:10:00Z".to_string(),
                body_markdown: "## Goal\nReduce post-etch poly residue without moving CD outside spec.\n\n## Result\nResidue reduced after switching from the baseline etch recipe. Line width shifted by 15 nm and needs follow-up metrology on the next split lot.".to_string(),
                tags: vec![
                    "poly".to_string(),
                    "etch".to_string(),
                    "residue".to_string(),
                ],
                links: NotebookLinks {
                    lots: vec![LotId::from("L-00100"), LotId::from("L-00101")],
                    wafers: vec![WaferId::from("L-00100-W07"), WaferId::from("L-00101-W11")],
                    recipes: vec![
                        RecipeId::from("ETCH_CF4_POLY_001"),
                        RecipeId::from("POLY_ETCH_004"),
                    ],
                    tool_runs: vec![ToolRunId::from("RUN-ETCH-0012")],
                    metrology: vec![MetrologyLink {
                        id: MetrologyLinkId::from("MET-CD-8841"),
                        kind: MeasurementKind::CriticalDimensionNm,
                        lot_id: Some(LotId::from("L-00101")),
                        wafer_id: Some(WaferId::from("L-00101-W11")),
                        summary: "CD mean shifted +15 nm after residue cleanup split".to_string(),
                    }],
                    images: vec![ImageLink {
                        id: ImageLinkId::from("IMG-ETCH-0042-A"),
                        label: "post-etch SEM residue comparison".to_string(),
                        uri: "images/sem/poly_residue_e0042.png".to_string(),
                    }],
                },
            },
            NotebookEntry {
                id: NotebookEntryId::from("E-0043"),
                title: "Photoresist coat uniformity check".to_string(),
                author: "litho.owner".to_string(),
                created_at: "2026-04-23T09:05:00Z".to_string(),
                updated_at: "2026-04-23T10:20:00Z".to_string(),
                body_markdown: "Thickness map shows a mild edge bead after the track bowl clean. Keep the new dispense delay and re-check after two production lots.".to_string(),
                tags: vec!["lithography".to_string(), "coat".to_string()],
                links: NotebookLinks {
                    lots: vec![LotId::from("L-00042")],
                    wafers: vec![WaferId::from("L-00042-W03")],
                    recipes: vec![RecipeId::from("SPIN_PR_3000")],
                    tool_runs: vec![ToolRunId::from("RUN-SPIN-0007")],
                    metrology: vec![MetrologyLink {
                        id: MetrologyLinkId::from("MET-THK-7719"),
                        kind: MeasurementKind::ThicknessNm,
                        lot_id: Some(LotId::from("L-00042")),
                        wafer_id: Some(WaferId::from("L-00042-W03")),
                        summary: "Edge thickness +2.1 percent versus center".to_string(),
                    }],
                    images: Vec::new(),
                },
            },
            NotebookEntry {
                id: NotebookEntryId::from("SHIFT-121"),
                title: "Night shift handoff".to_string(),
                author: "shift.lead".to_string(),
                created_at: "2026-04-24T06:00:00Z".to_string(),
                updated_at: "2026-04-24T06:05:00Z".to_string(),
                body_markdown: "ETCH-02 completed qualification wafers. Watch first product lot for endpoint drift and attach the FDC trace if alarm margin tightens.".to_string(),
                tags: vec!["handoff".to_string(), "qualification".to_string()],
                links: NotebookLinks {
                    tool_runs: vec![ToolRunId::from("RUN-ETCH-0012")],
                    ..NotebookLinks::default()
                },
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validation_context() -> NotebookValidationContext {
        NotebookValidationContext::from_mes_recipes_and_metrology(
            &crate::mes::FabMesData::sample(),
            crate::recipe::RecipeCatalog::sample()
                .recipes
                .keys()
                .map(|recipe_id| recipe_id.as_str().to_string()),
            std::iter::empty::<String>(),
        )
    }

    fn has_validation_error(findings: &[NotebookValidationFinding], needle: &str) -> bool {
        findings.iter().any(|finding| {
            finding.severity == NotebookValidationSeverity::Error
                && finding.message.contains(needle)
        })
    }

    fn has_validation_warning(findings: &[NotebookValidationFinding], needle: &str) -> bool {
        findings.iter().any(|finding| {
            finding.severity == NotebookValidationSeverity::Warning
                && finding.message.contains(needle)
        })
    }

    #[test]
    fn sample_notebook_validates_internally_and_allows_external_context_links() {
        let notebook = LabNotebook::sample();
        let findings = notebook.validate();

        assert!(findings.is_empty(), "{findings:?}");
        assert!(notebook.is_valid());

        let context_findings = notebook.validate_with_context(&validation_context());
        assert!(
            !context_findings
                .iter()
                .any(|finding| finding.severity == NotebookValidationSeverity::Error),
            "{context_findings:?}"
        );
        assert!(has_validation_warning(
            &context_findings,
            "notebook entry E-0042 references external lot L-00100"
        ));
    }

    #[test]
    fn notebook_validation_rejects_duplicate_entries_and_empty_required_links() {
        let mut notebook = LabNotebook::sample();
        notebook.entries.push(notebook.entries[0].clone());
        notebook.entries[0].updated_at = "2026-04-20T00:00:00Z".to_string();
        notebook.entries[0].links.images[0].uri.clear();
        notebook.entries[0].links.metrology.push(MetrologyLink {
            id: MetrologyLinkId::from(""),
            kind: MeasurementKind::ThicknessNm,
            lot_id: None,
            wafer_id: None,
            summary: String::new(),
        });

        let findings = notebook.validate();

        assert!(has_validation_error(
            &findings,
            "notebook contains duplicate entry E-0042"
        ));
        assert!(has_validation_error(
            &findings,
            "notebook entry E-0042 image link IMG-ETCH-0042-A has no URI"
        ));
        assert!(has_validation_error(
            &findings,
            "notebook entry E-0042 has metrology link with empty id"
        ));
        assert!(has_validation_error(
            &findings,
            "notebook entry E-0042 updated timestamp is before created timestamp"
        ));
        let mut wafer_without_lot = notebook.entries[0].links.metrology[0].clone();
        wafer_without_lot.id = MetrologyLinkId::from("MET-WAFER-WITHOUT-LOT");
        wafer_without_lot.lot_id = None;
        wafer_without_lot.wafer_id = Some(WaferId::from("L-00101-W11"));
        notebook.entries[0].links.metrology.push(wafer_without_lot);

        let findings = notebook.validate();
        assert!(has_validation_error(
            &findings,
            "metrology link MET-WAFER-WITHOUT-LOT references a wafer without a lot"
        ));
    }

    #[test]
    fn notebook_validation_warns_about_external_context_links() {
        let mut notebook = LabNotebook::sample();
        notebook.entries[1].links.metrology[0].id = MetrologyLinkId::from("MET-MISSING");

        let findings = notebook.validate_with_context(&validation_context());

        assert!(has_validation_warning(
            &findings,
            "notebook entry E-0043 references external metrology MET-MISSING"
        ));
        assert!(has_validation_warning(
            &findings,
            "notebook entry E-0042 references external wafer L-00100-W07"
        ));
    }

    #[test]
    fn notebook_validation_rejects_internal_lot_wafer_mismatch() {
        let mut notebook = LabNotebook::sample();
        notebook.entries[1].links.metrology[0].id = MetrologyLinkId::from("MET-MISMATCHED-PAIR");
        notebook.entries[1].links.metrology[0].lot_id = Some(LotId::from("L-00042"));
        notebook.entries[1].links.metrology[0].wafer_id = Some(WaferId::from("L-OTHER-W01"));
        let mut context = validation_context();
        context.lot_ids.insert("L-OTHER".to_string());
        context.wafer_ids.insert("L-OTHER-W01".to_string());
        context
            .wafer_ids_by_lot
            .entry("L-OTHER".to_string())
            .or_default()
            .insert("L-OTHER-W01".to_string());

        let findings = notebook.validate_with_context(&context);

        assert!(has_validation_error(
            &findings,
            "metrology link MET-MISMATCHED-PAIR references wafer L-OTHER-W01 outside lot L-00042"
        ));
    }

    #[test]
    fn search_matches_markdown_tags_and_link_targets() {
        let notebook = LabNotebook::sample();

        let by_body = notebook.filtered_entries(&NotebookFilter {
            query: "line width shifted".to_string(),
            ..NotebookFilter::default()
        });
        assert_eq!(by_body[0].id, NotebookEntryId::from("E-0042"));

        let by_tag = notebook.filtered_entries(&NotebookFilter {
            query: "handoff".to_string(),
            ..NotebookFilter::default()
        });
        assert_eq!(by_tag[0].id, NotebookEntryId::from("SHIFT-121"));

        let by_link = notebook.filtered_entries(&NotebookFilter {
            query: "RUN-SPIN-0007".to_string(),
            ..NotebookFilter::default()
        });
        assert_eq!(by_link[0].id, NotebookEntryId::from("E-0043"));
    }

    #[test]
    fn filters_can_target_link_kind_and_tag_together() {
        let notebook = LabNotebook::sample();

        let filter = NotebookFilter {
            query: "poly".to_string(),
            tag: Some("etch".to_string()),
            link_kind: Some(NotebookLinkKind::Metrology),
        };
        let matches = notebook.filtered_entries(&filter);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, NotebookEntryId::from("E-0042"));

        let no_match = notebook.filtered_entries(&NotebookFilter {
            tag: Some("etch".to_string()),
            link_kind: Some(NotebookLinkKind::Image),
            query: "coat".to_string(),
        });
        assert!(no_match.is_empty());
    }

    #[test]
    fn link_count_includes_every_structured_link_bucket() {
        let notebook = LabNotebook::sample();
        let entry = notebook
            .entry(&NotebookEntryId::from("E-0042"))
            .expect("sample entry exists");

        assert_eq!(entry.link_count(), 9);
        assert!(entry.has_link_kind(NotebookLinkKind::Lot));
        assert!(entry.has_link_kind(NotebookLinkKind::Wafer));
        assert!(entry.has_link_kind(NotebookLinkKind::Recipe));
        assert!(entry.has_link_kind(NotebookLinkKind::ToolRun));
        assert!(entry.has_link_kind(NotebookLinkKind::Metrology));
        assert!(entry.has_link_kind(NotebookLinkKind::Image));
    }
}
