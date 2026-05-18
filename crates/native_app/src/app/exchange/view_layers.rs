#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutViewBookmarkExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) bookmarks: Vec<options::LayoutViewBookmarkOptions>,
}

impl LayoutViewBookmarkExchange {
    pub(crate) fn from_app(app: &GlassworksApp) -> Self {
        Self {
            schema_version: LAYOUT_VIEW_BOOKMARK_EXCHANGE_SCHEMA_VERSION,
            document_id: app.workspace.document.id.to_string(),
            document_name: app.workspace.document.name.clone(),
            bookmarks: app
                .layout_view_bookmarks
                .iter()
                .map(|(slot, state)| {
                    let mut bookmark = state.to_bookmark_options(*slot);
                    bookmark.name = app.layout_view_bookmark_name(*slot);
                    bookmark
                })
                .collect(),
        }
    }

    pub(crate) fn into_bookmarks(
        self,
        document: &Document,
    ) -> Result<(BTreeMap<u8, LayoutViewState>, BTreeMap<u8, String>), String> {
        if self.schema_version != LAYOUT_VIEW_BOOKMARK_EXCHANGE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported view bookmark schema version {}",
                self.schema_version
            ));
        }
        let mut bookmarks = BTreeMap::new();
        let mut bookmark_names = BTreeMap::new();
        for bookmark in self.bookmarks {
            if !LAYOUT_VIEW_BOOKMARK_SLOTS.contains(&bookmark.slot) {
                return Err(format!("invalid view bookmark slot {}", bookmark.slot));
            }
            if bookmarks.contains_key(&bookmark.slot) {
                return Err(format!("duplicate view bookmark slot {}", bookmark.slot));
            }
            let state = LayoutViewState::from_bookmark_options(&bookmark);
            if document.cell(state.top_cell).is_none() {
                return Err(format!(
                    "view bookmark {} references missing cell {}",
                    bookmark.slot, state.top_cell.0
                ));
            }
            bookmark_names.insert(
                bookmark.slot,
                bookmark
                    .name
                    .trim()
                    .to_string()
                    .if_empty_then(|| layout_view_bookmark_default_name(bookmark.slot)),
            );
            bookmarks.insert(bookmark.slot, state);
        }
        Ok((bookmarks, bookmark_names))
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutLayerSetExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) layer_sets: Vec<LayoutLayerSetExchangeSlot>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutLayerSetExchangeSlot {
    pub(crate) slot: u8,
    #[serde(default)]
    pub(crate) name: String,
    pub(crate) visible_layers: Vec<u32>,
    #[serde(default)]
    pub(crate) layer_group_filter: String,
    #[serde(default)]
    pub(crate) layer_usage_filter: String,
    #[serde(default)]
    pub(crate) layer_depths: Vec<options::LayoutLayerDepthOptions>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutReferenceImageExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) reference_images: Vec<ReferenceImageOverlay>,
}

impl LayoutReferenceImageExchange {
    pub(crate) fn from_app(app: &GlassworksApp) -> Self {
        Self {
            schema_version: LAYOUT_REFERENCE_IMAGE_EXCHANGE_SCHEMA_VERSION,
            document_id: app.workspace.document.id.to_string(),
            document_name: app.workspace.document.name.clone(),
            reference_images: app.workspace.document.reference_images.clone(),
        }
    }

    pub(crate) fn into_reference_images(self) -> Result<Vec<ReferenceImageOverlay>, String> {
        if self.schema_version != LAYOUT_REFERENCE_IMAGE_EXCHANGE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported reference image schema version {}",
                self.schema_version
            ));
        }
        GlassworksApp::validate_reference_image_overlays(&self.reference_images)?;
        Ok(self.reference_images)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LayoutLibraryCatalogExchange {
    pub(crate) schema_version: u32,
    pub(crate) document_id: String,
    pub(crate) document_name: String,
    pub(crate) presets: Vec<options::LayoutLibraryPresetOptions>,
}

impl LayoutLibraryCatalogExchange {
    pub(crate) fn from_app(app: &GlassworksApp) -> Self {
        Self {
            schema_version: LAYOUT_LIBRARY_CATALOG_EXCHANGE_SCHEMA_VERSION,
            document_id: app.workspace.document.id.to_string(),
            document_name: app.workspace.document.name.clone(),
            presets: layout_library_preset_options_from_via_array_presets(
                &app.layout_library_via_array_presets,
            ),
        }
    }

    pub(crate) fn into_presets(self) -> Result<BTreeMap<u8, LayoutViaArrayLibraryPreset>, String> {
        if self.schema_version != LAYOUT_LIBRARY_CATALOG_EXCHANGE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported library catalog schema version {}",
                self.schema_version
            ));
        }
        layout_via_array_library_presets_from_catalog_options(&self.presets)
    }
}

pub(crate) fn parse_layout_reference_image_exchange(
    contents: &str,
) -> Result<LayoutReferenceImageExchange, String> {
    match serde_json::from_str::<LayoutReferenceImageExchange>(contents) {
        Ok(exchange) => Ok(exchange),
        Err(exchange_error) => {
            let reference_images = serde_json::from_str::<Vec<ReferenceImageOverlay>>(contents)
                .map_err(|error| {
                    format!(
                        "parse reference image exchange: {exchange_error}; array fallback: {error}"
                    )
                })?;
            Ok(LayoutReferenceImageExchange {
                schema_version: LAYOUT_REFERENCE_IMAGE_EXCHANGE_SCHEMA_VERSION,
                document_id: String::new(),
                document_name: String::new(),
                reference_images,
            })
        }
    }
}

impl LayoutLayerSetExchange {
    pub(crate) fn from_app(app: &GlassworksApp) -> Self {
        Self {
            schema_version: LAYOUT_LAYER_SET_EXCHANGE_SCHEMA_VERSION,
            document_id: app.workspace.document.id.to_string(),
            document_name: app.workspace.document.name.clone(),
            layer_sets: app
                .layout_layer_sets
                .iter()
                .map(|(slot, state)| LayoutLayerSetExchangeSlot {
                    slot: *slot,
                    name: app.layout_layer_set_name(*slot),
                    visible_layers: state.visible_layers.iter().map(|layer| layer.0).collect(),
                    layer_group_filter: state.layer_group_filter.slug().to_string(),
                    layer_usage_filter: state.layer_usage_filter.slug().to_string(),
                    layer_depths: state
                        .layer_depth_overrides
                        .iter()
                        .map(|(layer, depth)| options::LayoutLayerDepthOptions {
                            layer: layer.0,
                            max_depth: depth.slug(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub(crate) fn into_layer_sets(
        self,
        document: &Document,
    ) -> Result<(BTreeMap<u8, LayoutLayerSetState>, BTreeMap<u8, String>), String> {
        if self.schema_version != LAYOUT_LAYER_SET_EXCHANGE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported layer set schema version {}",
                self.schema_version
            ));
        }
        let mut layer_sets = BTreeMap::new();
        let mut layer_set_names = BTreeMap::new();
        for layer_set in self.layer_sets {
            if !LAYOUT_LAYER_SET_SLOTS.contains(&layer_set.slot) {
                return Err(format!("invalid layer set slot {}", layer_set.slot));
            }
            if layer_sets.contains_key(&layer_set.slot) {
                return Err(format!("duplicate layer set slot {}", layer_set.slot));
            }
            layer_set_names.insert(
                layer_set.slot,
                layout_layer_set_name_or_default(layer_set.slot, &layer_set.name),
            );
            layer_sets.insert(
                layer_set.slot,
                layout_layer_set_state_from_parts(
                    document,
                    layer_set.visible_layers.iter().copied(),
                    &layer_set.layer_group_filter,
                    &layer_set.layer_usage_filter,
                    &layer_set.layer_depths,
                ),
            );
        }
        Ok((layer_sets, layer_set_names))
    }
}
