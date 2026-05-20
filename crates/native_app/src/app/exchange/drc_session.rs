#![allow(unused_imports)]
use super::*;

pub(crate) const LAYOUT_DRC_REPORT_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_DRC_REPORT_DATABASE_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_DRC_DECK_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_EXTRACTED_NETLIST_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_TRACE_STATE_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_L2N_DATABASE_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_LAYER_SET_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_VIEW_BOOKMARK_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_REFERENCE_IMAGE_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const LAYOUT_LIBRARY_CATALOG_EXCHANGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const APP_SESSION_EXCHANGE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Default)]
pub(crate) struct DrcReportCacheValue {
    pub(crate) findings: Vec<DrcValidationFinding>,
    pub(crate) violations: Vec<DrcViolation>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcReportExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) layout_revision: u64,
    pub(crate) findings: Vec<DrcValidationFinding>,
    pub(crate) violations: Vec<DrcViolation>,
    pub(crate) marker_states: BTreeMap<String, MarkerState>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcReportDatabaseEntryExchange {
    pub(crate) label: String,
    #[serde(default)]
    pub(crate) source: Option<String>,
    pub(crate) layout_revision: u64,
    pub(crate) findings: Vec<DrcValidationFinding>,
    pub(crate) violations: Vec<DrcViolation>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcReportDatabaseExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) layout_revision: u64,
    pub(crate) active_report_index: Option<usize>,
    pub(crate) reports: Vec<LayoutDrcReportDatabaseEntryExchange>,
    pub(crate) marker_states: BTreeMap<String, MarkerState>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcLayerRuleExchange {
    pub(crate) layer: String,
    pub(crate) value: Coord,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcDerivedLayerRuleExchange {
    pub(crate) name: String,
    pub(crate) operation: DerivedLayerOperation,
    pub(crate) a: String,
    pub(crate) b: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcEnclosureRuleExchange {
    pub(crate) via: String,
    pub(crate) enclosure: String,
    pub(crate) required: Coord,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcForbiddenOverlapRuleExchange {
    pub(crate) a: String,
    pub(crate) b: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcDerivedForbiddenOverlapRuleExchange {
    pub(crate) derived: String,
    pub(crate) layer: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutDrcDeckExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) layout_revision: u64,
    pub(crate) grid: Coord,
    #[serde(default)]
    pub(crate) derived_layers: Vec<LayoutDrcDerivedLayerRuleExchange>,
    #[serde(default)]
    pub(crate) derived_min_width: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) derived_max_width: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) derived_min_area: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) derived_max_area: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) derived_min_spacing: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) derived_min_edge_spacing: Vec<LayoutDrcLayerRuleExchange>,
    pub(crate) min_width: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) max_width: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) min_area: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) max_area: Vec<LayoutDrcLayerRuleExchange>,
    pub(crate) min_spacing: Vec<LayoutDrcLayerRuleExchange>,
    #[serde(default)]
    pub(crate) min_edge_spacing: Vec<LayoutDrcLayerRuleExchange>,
    pub(crate) via_enclosure: Vec<LayoutDrcEnclosureRuleExchange>,
    pub(crate) forbidden_overlaps: Vec<LayoutDrcForbiddenOverlapRuleExchange>,
    #[serde(default)]
    pub(crate) derived_forbidden_overlaps: Vec<LayoutDrcDerivedForbiddenOverlapRuleExchange>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct AppSessionExchange {
    pub(crate) schema_version: u32,
    pub(crate) workspace: WorkspaceDataset,
    pub(crate) app_options: AppOptions,
    pub(crate) current_layout_view: options::LayoutViewBookmarkOptions,
    #[serde(default)]
    pub(crate) drc_reports: Vec<LayoutDrcReportDatabaseEntryExchange>,
    #[serde(default)]
    pub(crate) drc_active_report_index: Option<usize>,
    #[serde(default)]
    pub(crate) drc_custom_deck: Option<LayoutDrcDeckExchange>,
    #[serde(default)]
    pub(crate) drc_custom_deck_label: Option<String>,
}

pub(crate) struct AppSessionParts {
    pub(crate) workspace: WorkspaceDataset,
    pub(crate) app_options: AppOptions,
    pub(crate) current_layout_view: LayoutViewState,
    pub(crate) drc_reports: Vec<LayoutDrcReportDatabaseEntryExchange>,
    pub(crate) drc_active_report_index: Option<usize>,
    pub(crate) drc_custom_deck: Option<RuleDeck>,
    pub(crate) drc_custom_deck_label: Option<String>,
}

impl LayoutDrcDeckExchange {
    pub(crate) fn from_app(app: &GlassworksApp, deck: &RuleDeck) -> Self {
        Self {
            schema_version: LAYOUT_DRC_DECK_EXCHANGE_SCHEMA_VERSION,
            document_id: app.workspace.document.id.to_string(),
            document_name: app.workspace.document.name.clone(),
            layout_revision: app.layout_revision,
            grid: deck.grid,
            derived_layers: deck
                .derived_layers
                .iter()
                .map(|rule| LayoutDrcDerivedLayerRuleExchange {
                    name: rule.name.clone(),
                    operation: rule.operation,
                    a: layout_drc_deck_layer_reference(&app.workspace.document, rule.a),
                    b: layout_drc_deck_layer_reference(&app.workspace.document, rule.b),
                })
                .collect(),
            derived_min_width: deck
                .derived_min_width
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layer.clone(),
                    value: *value,
                })
                .collect(),
            derived_max_width: deck
                .derived_max_width
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layer.clone(),
                    value: *value,
                })
                .collect(),
            derived_min_area: deck
                .derived_min_area
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layer.clone(),
                    value: *value,
                })
                .collect(),
            derived_max_area: deck
                .derived_max_area
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layer.clone(),
                    value: *value,
                })
                .collect(),
            derived_min_spacing: deck
                .derived_min_spacing
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layer.clone(),
                    value: *value,
                })
                .collect(),
            derived_min_edge_spacing: deck
                .derived_min_edge_spacing
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layer.clone(),
                    value: *value,
                })
                .collect(),
            min_width: deck
                .min_width
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layout_drc_deck_layer_reference(&app.workspace.document, *layer),
                    value: *value,
                })
                .collect(),
            max_width: deck
                .max_width
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layout_drc_deck_layer_reference(&app.workspace.document, *layer),
                    value: *value,
                })
                .collect(),
            min_area: deck
                .min_area
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layout_drc_deck_layer_reference(&app.workspace.document, *layer),
                    value: *value,
                })
                .collect(),
            max_area: deck
                .max_area
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layout_drc_deck_layer_reference(&app.workspace.document, *layer),
                    value: *value,
                })
                .collect(),
            min_spacing: deck
                .min_spacing
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layout_drc_deck_layer_reference(&app.workspace.document, *layer),
                    value: *value,
                })
                .collect(),
            min_edge_spacing: deck
                .min_edge_spacing
                .iter()
                .map(|(layer, value)| LayoutDrcLayerRuleExchange {
                    layer: layout_drc_deck_layer_reference(&app.workspace.document, *layer),
                    value: *value,
                })
                .collect(),
            via_enclosure: deck
                .via_enclosure
                .iter()
                .map(|rule| LayoutDrcEnclosureRuleExchange {
                    via: layout_drc_deck_layer_reference(&app.workspace.document, rule.via_layer),
                    enclosure: layout_drc_deck_layer_reference(
                        &app.workspace.document,
                        rule.enclosure_layer,
                    ),
                    required: rule.required,
                })
                .collect(),
            forbidden_overlaps: deck
                .forbidden_overlaps
                .iter()
                .map(|rule| LayoutDrcForbiddenOverlapRuleExchange {
                    a: layout_drc_deck_layer_reference(&app.workspace.document, rule.a),
                    b: layout_drc_deck_layer_reference(&app.workspace.document, rule.b),
                    name: rule.name.clone(),
                })
                .collect(),
            derived_forbidden_overlaps: deck
                .derived_forbidden_overlaps
                .iter()
                .map(|rule| LayoutDrcDerivedForbiddenOverlapRuleExchange {
                    derived: rule.derived.clone(),
                    layer: layout_drc_deck_layer_reference(&app.workspace.document, rule.layer),
                    name: rule.name.clone(),
                })
                .collect(),
        }
    }

    pub(crate) fn into_rule_deck(self, document: &Document) -> Result<RuleDeck, String> {
        if self.schema_version != LAYOUT_DRC_DECK_EXCHANGE_SCHEMA_VERSION {
            return Err(format!(
                "deck schema {} is unsupported; expected {}",
                self.schema_version, LAYOUT_DRC_DECK_EXCHANGE_SCHEMA_VERSION
            ));
        }
        let mut derived_layers = Vec::new();
        let mut derived_layer_names = BTreeSet::new();
        for rule in self.derived_layers {
            let normalized = rule.name.trim().to_ascii_lowercase();
            if !derived_layer_names.insert(normalized) {
                return Err(format!(
                    "derived layer rule for {} is duplicated",
                    rule.name
                ));
            }
            derived_layers.push(DerivedLayerRule {
                name: rule.name,
                operation: rule.operation,
                a: resolve_layout_drc_deck_layer(document, &rule.a, "derived-layer")?,
                b: resolve_layout_drc_deck_layer(document, &rule.b, "derived-layer")?,
            });
        }
        let mut derived_min_width = BTreeMap::new();
        for rule in self.derived_min_width {
            if derived_min_width
                .insert(rule.layer.clone(), rule.value)
                .is_some()
            {
                return Err(format!(
                    "derived min-width rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut derived_max_width = BTreeMap::new();
        for rule in self.derived_max_width {
            if derived_max_width
                .insert(rule.layer.clone(), rule.value)
                .is_some()
            {
                return Err(format!(
                    "derived max-width rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut derived_min_area = BTreeMap::new();
        for rule in self.derived_min_area {
            if derived_min_area
                .insert(rule.layer.clone(), rule.value)
                .is_some()
            {
                return Err(format!(
                    "derived min-area rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut derived_max_area = BTreeMap::new();
        for rule in self.derived_max_area {
            if derived_max_area
                .insert(rule.layer.clone(), rule.value)
                .is_some()
            {
                return Err(format!(
                    "derived max-area rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut derived_min_spacing = BTreeMap::new();
        for rule in self.derived_min_spacing {
            if derived_min_spacing
                .insert(rule.layer.clone(), rule.value)
                .is_some()
            {
                return Err(format!(
                    "derived min-spacing rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut derived_min_edge_spacing = BTreeMap::new();
        for rule in self.derived_min_edge_spacing {
            if derived_min_edge_spacing
                .insert(rule.layer.clone(), rule.value)
                .is_some()
            {
                return Err(format!(
                    "derived min-edge-spacing rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut min_width = BTreeMap::new();
        for rule in self.min_width {
            let layer = resolve_layout_drc_deck_layer(document, &rule.layer, "min-width")?;
            if min_width.insert(layer, rule.value).is_some() {
                return Err(format!(
                    "min-width rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut max_width = BTreeMap::new();
        for rule in self.max_width {
            let layer = resolve_layout_drc_deck_layer(document, &rule.layer, "max-width")?;
            if max_width.insert(layer, rule.value).is_some() {
                return Err(format!(
                    "max-width rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut min_area = BTreeMap::new();
        for rule in self.min_area {
            let layer = resolve_layout_drc_deck_layer(document, &rule.layer, "min-area")?;
            if min_area.insert(layer, rule.value).is_some() {
                return Err(format!(
                    "min-area rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut max_area = BTreeMap::new();
        for rule in self.max_area {
            let layer = resolve_layout_drc_deck_layer(document, &rule.layer, "max-area")?;
            if max_area.insert(layer, rule.value).is_some() {
                return Err(format!(
                    "max-area rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut min_spacing = BTreeMap::new();
        for rule in self.min_spacing {
            let layer = resolve_layout_drc_deck_layer(document, &rule.layer, "min-spacing")?;
            if min_spacing.insert(layer, rule.value).is_some() {
                return Err(format!(
                    "min-spacing rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut min_edge_spacing = BTreeMap::new();
        for rule in self.min_edge_spacing {
            let layer = resolve_layout_drc_deck_layer(document, &rule.layer, "min-edge-spacing")?;
            if min_edge_spacing.insert(layer, rule.value).is_some() {
                return Err(format!(
                    "min-edge-spacing rule for layer {} is duplicated",
                    rule.layer
                ));
            }
        }
        let mut via_enclosure = Vec::new();
        for rule in self.via_enclosure {
            via_enclosure.push(EnclosureRule {
                via_layer: resolve_layout_drc_deck_layer(document, &rule.via, "via-enclosure")?,
                enclosure_layer: resolve_layout_drc_deck_layer(
                    document,
                    &rule.enclosure,
                    "via-enclosure",
                )?,
                required: rule.required,
            });
        }
        let mut forbidden_overlaps = Vec::new();
        for rule in self.forbidden_overlaps {
            forbidden_overlaps.push(ForbiddenOverlapRule {
                a: resolve_layout_drc_deck_layer(document, &rule.a, "forbidden-overlap")?,
                b: resolve_layout_drc_deck_layer(document, &rule.b, "forbidden-overlap")?,
                name: rule.name,
            });
        }
        let mut derived_forbidden_overlaps = Vec::new();
        for rule in self.derived_forbidden_overlaps {
            derived_forbidden_overlaps.push(DerivedForbiddenOverlapRule {
                derived: rule.derived,
                layer: resolve_layout_drc_deck_layer(document, &rule.layer, "derived-overlap")?,
                name: rule.name,
            });
        }
        let deck = RuleDeck {
            grid: self.grid,
            derived_layers,
            derived_min_width,
            derived_max_width,
            derived_min_area,
            derived_max_area,
            derived_min_spacing,
            derived_min_edge_spacing,
            min_width,
            max_width,
            min_area,
            max_area,
            min_spacing,
            min_edge_spacing,
            via_enclosure,
            forbidden_overlaps,
            derived_forbidden_overlaps,
        };
        let errors = deck
            .validate_for_document(document)
            .into_iter()
            .filter(|finding| finding.severity == DrcValidationSeverity::Error)
            .map(|finding| finding.message)
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(errors.into_iter().take(3).collect::<Vec<_>>().join("; "));
        }
        Ok(deck)
    }
}

impl AppSessionExchange {
    pub(crate) fn from_app(app: &GlassworksApp) -> Self {
        let mut workspace = app.workspace.clone();
        workspace.schema_version = WORKSPACE_DATASET_SCHEMA_VERSION;
        workspace.metadata = WorkspaceSnapshotMetadata::current(workspace.document.schema_version);
        let mut current_layout_view = app.current_layout_view_state().to_bookmark_options(1);
        current_layout_view.name = "Current View".to_string();
        let drc_reports = if app.layout_drc_report_history.is_empty() {
            app.drc_report()
                .map(|report| {
                    vec![LayoutDrcReportDatabaseEntryExchange {
                        label: "Active DRC Report".to_string(),
                        source: None,
                        layout_revision: app.layout_revision,
                        findings: report.findings,
                        violations: report.violations,
                    }]
                })
                .unwrap_or_default()
        } else {
            app.layout_drc_report_history
                .iter()
                .map(|entry| LayoutDrcReportDatabaseEntryExchange {
                    label: entry.label.clone(),
                    source: entry.source.clone(),
                    layout_revision: entry.revision,
                    findings: entry.value.findings.clone(),
                    violations: entry.value.violations.clone(),
                })
                .collect()
        };
        let drc_active_report_index = app
            .layout_selected_drc_report_id
            .and_then(|selected_id| {
                app.layout_drc_report_history
                    .iter()
                    .position(|entry| entry.id == selected_id)
            })
            .or_else(|| (!drc_reports.is_empty()).then_some(0));
        let drc_custom_deck = app
            .layout_custom_drc_deck
            .as_ref()
            .map(|deck| LayoutDrcDeckExchange::from_app(app, deck));
        Self {
            schema_version: APP_SESSION_EXCHANGE_SCHEMA_VERSION,
            workspace,
            app_options: app.app_options_from_state(),
            current_layout_view,
            drc_reports,
            drc_active_report_index,
            drc_custom_deck,
            drc_custom_deck_label: app.layout_custom_drc_deck_label.clone(),
        }
    }

    pub(crate) fn into_parts(self) -> Result<AppSessionParts, String> {
        if self.schema_version != APP_SESSION_EXCHANGE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported session schema version {}; expected {}",
                self.schema_version, APP_SESSION_EXCHANGE_SCHEMA_VERSION
            ));
        }
        let workspace = self.workspace;
        let validation = workspace.validate();
        if !validation.is_valid() {
            return Err(validation.error_summary());
        }
        let options = self.app_options.normalized();
        let current_layout_view = LayoutViewState::from_bookmark_options(&self.current_layout_view);
        if workspace
            .document
            .cell(current_layout_view.top_cell)
            .is_none()
        {
            return Err(format!(
                "current layout view references missing cell {}",
                current_layout_view.top_cell.0
            ));
        }
        for report in &self.drc_reports {
            let issue_store = DrcIssueStore::from_violations(report.violations.clone());
            let errors = issue_store
                .validate()
                .into_iter()
                .filter(|finding| finding.severity == DrcValidationSeverity::Error)
                .map(|finding| finding.message)
                .collect::<Vec<_>>();
            if !errors.is_empty() {
                let label = report.label.trim();
                let label = if label.is_empty() {
                    "unnamed report"
                } else {
                    label
                };
                return Err(format!(
                    "session DRC report {label} failed validation: {}",
                    errors
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }
        let drc_custom_deck = match self.drc_custom_deck {
            Some(deck) => Some(deck.into_rule_deck(&workspace.document)?),
            None => None,
        };
        Ok(AppSessionParts {
            workspace,
            app_options: options,
            current_layout_view,
            drc_reports: self.drc_reports,
            drc_active_report_index: self.drc_active_report_index,
            drc_custom_deck,
            drc_custom_deck_label: self.drc_custom_deck_label,
        })
    }
}
