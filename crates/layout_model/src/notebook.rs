use std::fmt;

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
