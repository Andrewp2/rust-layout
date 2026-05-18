#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_view_bookmarks_from_options(
    bookmarks: &[options::LayoutViewBookmarkOptions],
) -> BTreeMap<u8, LayoutViewState> {
    bookmarks
        .iter()
        .filter(|bookmark| LAYOUT_VIEW_BOOKMARK_SLOTS.contains(&bookmark.slot))
        .map(|bookmark| {
            (
                bookmark.slot,
                LayoutViewState::from_bookmark_options(bookmark),
            )
        })
        .collect()
}

pub(crate) fn layout_view_bookmark_names_from_options(
    bookmarks: &[options::LayoutViewBookmarkOptions],
) -> BTreeMap<u8, String> {
    bookmarks
        .iter()
        .filter(|bookmark| LAYOUT_VIEW_BOOKMARK_SLOTS.contains(&bookmark.slot))
        .map(|bookmark| (bookmark.slot, bookmark.name.trim().to_string()))
        .collect()
}

pub(crate) fn layout_view_bookmark_default_name(slot: u8) -> String {
    format!("View {slot}")
}

pub(crate) fn layout_layer_set_names_from_options(
    layer_sets: &[options::LayoutLayerSetOptions],
) -> BTreeMap<u8, String> {
    layer_sets
        .iter()
        .filter(|layer_set| LAYOUT_LAYER_SET_SLOTS.contains(&layer_set.slot))
        .map(|layer_set| {
            (
                layer_set.slot,
                layout_layer_set_name_or_default(layer_set.slot, &layer_set.name),
            )
        })
        .collect()
}

pub(crate) fn layout_layer_set_default_name(slot: u8) -> String {
    format!("Layer Set {slot}")
}

pub(crate) fn layout_layer_set_name_or_default(slot: u8, name: &str) -> String {
    name.trim()
        .to_string()
        .if_empty_then(|| layout_layer_set_default_name(slot))
}

pub(crate) fn layout_view_bookmark_generated_name(
    document: &Document,
    slot: u8,
    state: LayoutViewState,
) -> String {
    let cell = document
        .cell(state.top_cell)
        .map(|cell| compact_button_label(&cell.name, 14))
        .unwrap_or_else(|| format!("Cell {}", state.top_cell.0));
    compact_button_label(&format!("{cell} {:.3}x", state.zoom), 32)
        .trim()
        .to_string()
        .if_empty_then(|| layout_view_bookmark_default_name(slot))
}

pub(crate) trait EmptyStringFallback {
    fn if_empty_then(self, fallback: impl FnOnce() -> String) -> String;
}

impl EmptyStringFallback for String {
    fn if_empty_then(self, fallback: impl FnOnce() -> String) -> String {
        if self.trim().is_empty() {
            fallback()
        } else {
            self
        }
    }
}

pub(crate) fn layout_layer_set_state_from_parts(
    document: &Document,
    visible_layers: impl IntoIterator<Item = u32>,
    layer_group_filter: &str,
    layer_usage_filter: &str,
    layer_depths: &[options::LayoutLayerDepthOptions],
) -> LayoutLayerSetState {
    LayoutLayerSetState {
        visible_layers: visible_layers.into_iter().map(LayerId).collect(),
        layer_group_filter: LayoutLayerGroupFilter::from_slug(layer_group_filter)
            .unwrap_or(LayoutLayerGroupFilter::All),
        layer_usage_filter: LayoutLayerUsageFilter::from_slug(layer_usage_filter)
            .unwrap_or(LayoutLayerUsageFilter::All),
        layer_depth_overrides: layout_layer_depth_overrides_from_options(document, layer_depths),
    }
}

pub(crate) fn layout_layer_sets_from_options(
    document: &Document,
    layer_sets: &[options::LayoutLayerSetOptions],
) -> BTreeMap<u8, LayoutLayerSetState> {
    layer_sets
        .iter()
        .filter(|layer_set| LAYOUT_LAYER_SET_SLOTS.contains(&layer_set.slot))
        .map(|layer_set| {
            (
                layer_set.slot,
                layout_layer_set_state_from_parts(
                    document,
                    layer_set.visible_layers.iter().copied(),
                    &layer_set.layer_group_filter,
                    &layer_set.layer_usage_filter,
                    &layer_set.layer_depths,
                ),
            )
        })
        .collect()
}

pub(crate) fn layout_layer_depth_overrides_from_options(
    document: &Document,
    layer_depths: &[options::LayoutLayerDepthOptions],
) -> BTreeMap<LayerId, LayoutHierarchyDepth> {
    layer_depths
        .iter()
        .filter_map(|layer_depth| {
            let layer = LayerId(layer_depth.layer);
            document.layers.contains_key(&layer).then(|| {
                (
                    layer,
                    LayoutHierarchyDepth::from_slug(&layer_depth.max_depth)
                        .unwrap_or(LayoutHierarchyDepth::Full),
                )
            })
        })
        .filter(|(_, depth)| !depth.shows_instance_boxes())
        .collect()
}

pub(crate) fn layout_via_array_library_presets_from_options(
    presets: &[options::LayoutLibraryPresetOptions],
) -> BTreeMap<u8, LayoutViaArrayLibraryPreset> {
    presets
        .iter()
        .filter(|preset| {
            LAYOUT_LIBRARY_PRESET_SLOTS.contains(&preset.slot)
                && preset.macro_name.as_str() == "via_array"
        })
        .map(|preset| {
            (
                preset.slot,
                LayoutViaArrayLibraryPreset {
                    name: preset.name.trim().to_string(),
                    columns: preset.columns.clamp(
                        LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
                        LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
                    ),
                    rows: preset.rows.clamp(
                        LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
                        LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
                    ),
                    size_grids: preset.size_grids.clamp(
                        LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS,
                        LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS,
                    ),
                    pitch_grids: clamp_layout_library_via_array_pitch_grids(
                        preset.size_grids.clamp(
                            LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS,
                            LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS,
                        ),
                        preset.pitch_grids,
                    ),
                },
            )
        })
        .collect()
}

pub(crate) fn layout_library_preset_options_from_via_array_presets(
    presets: &BTreeMap<u8, LayoutViaArrayLibraryPreset>,
) -> Vec<options::LayoutLibraryPresetOptions> {
    presets
        .iter()
        .map(|(slot, preset)| options::LayoutLibraryPresetOptions {
            slot: *slot,
            name: preset.name.clone(),
            macro_name: "via_array".to_string(),
            columns: preset.columns,
            rows: preset.rows,
            size_grids: preset.size_grids,
            pitch_grids: preset.pitch_grids,
        })
        .collect()
}

pub(crate) fn layout_via_array_library_presets_from_catalog_options(
    presets: &[options::LayoutLibraryPresetOptions],
) -> Result<BTreeMap<u8, LayoutViaArrayLibraryPreset>, String> {
    let mut catalog = BTreeMap::new();
    for preset in presets {
        if !LAYOUT_LIBRARY_PRESET_SLOTS.contains(&preset.slot) {
            return Err(format!("invalid library preset slot {}", preset.slot));
        }
        if catalog.contains_key(&preset.slot) {
            return Err(format!("duplicate library preset slot {}", preset.slot));
        }
        let macro_name = preset.macro_name.trim();
        if !macro_name.is_empty() && macro_name != "via_array" {
            return Err(format!(
                "unsupported library preset macro {}",
                preset.macro_name
            ));
        }
        let size_grids = preset.size_grids.clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS,
        );
        let name = preset.name.trim();
        catalog.insert(
            preset.slot,
            LayoutViaArrayLibraryPreset {
                name: if name.is_empty() {
                    format!("Via Array {}", preset.slot)
                } else {
                    name.chars().take(32).collect()
                },
                columns: preset.columns.clamp(
                    LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
                    LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
                ),
                rows: preset.rows.clamp(
                    LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
                    LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
                ),
                size_grids,
                pitch_grids: clamp_layout_library_via_array_pitch_grids(
                    size_grids,
                    preset.pitch_grids,
                ),
            },
        );
    }
    Ok(catalog)
}

pub(crate) fn clamp_layout_library_via_array_pitch_grids(
    size_grids: Coord,
    pitch_grids: Coord,
) -> Coord {
    let minimum_pitch = (size_grids + LAYOUT_LIBRARY_VIA_ARRAY_MIN_SPACING_GRIDS)
        .min(LAYOUT_LIBRARY_VIA_ARRAY_MAX_PITCH_GRIDS);
    pitch_grids.clamp(minimum_pitch, LAYOUT_LIBRARY_VIA_ARRAY_MAX_PITCH_GRIDS)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn remap_layout_json_import_layer(
    layer: LayerId,
    layer_map: &BTreeMap<LayerId, LayerId>,
) -> Result<LayerId, String> {
    layer_map
        .get(&layer)
        .copied()
        .ok_or_else(|| format!("imported shape references missing layer L{}", layer.0))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn layout_import_layer_name_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

pub(crate) fn parse_layout_library_preset_slot(value: &str) -> Option<u8> {
    value
        .parse::<u8>()
        .ok()
        .filter(|slot| LAYOUT_LIBRARY_PRESET_SLOTS.contains(slot))
}

pub(crate) fn parse_layout_layer_set_slot(value: &str) -> Option<u8> {
    value
        .parse::<u8>()
        .ok()
        .filter(|slot| LAYOUT_LAYER_SET_SLOTS.contains(slot))
}

pub(crate) fn parse_layout_view_bookmark_slot(value: &str) -> Option<u8> {
    value
        .parse::<u8>()
        .ok()
        .filter(|slot| LAYOUT_VIEW_BOOKMARK_SLOTS.contains(slot))
}

pub(crate) const LAYOUT_BUILTIN_TECHNOLOGY_SLUGS: [&str; 2] =
    ["glassworks_demo", "glassworks_high_density"];

pub(crate) fn layout_builtin_technology_slug(index: usize) -> &'static str {
    LAYOUT_BUILTIN_TECHNOLOGY_SLUGS
        .get(index)
        .copied()
        .unwrap_or(LAYOUT_BUILTIN_TECHNOLOGY_SLUGS[0])
}

pub(crate) fn normalize_layout_technology_slug(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| match ch {
            '-' | ' ' => '_',
            _ => ch,
        })
        .collect()
}

pub(crate) fn layout_builtin_technology_from_slug(value: &str) -> (usize, TechnologyFile) {
    let slug = normalize_layout_technology_slug(value);
    let index = LAYOUT_BUILTIN_TECHNOLOGY_SLUGS
        .iter()
        .position(|candidate| *candidate == slug)
        .unwrap_or(0);
    let technology = layout_model::builtin_technologies()
        .into_iter()
        .nth(index)
        .unwrap_or_else(layout_model::default_technology);
    (index, technology)
}

pub(crate) fn layout_builtin_technology_by_index(index: usize) -> Option<(usize, TechnologyFile)> {
    layout_model::builtin_technologies()
        .into_iter()
        .nth(index)
        .map(|technology| (index, technology))
}

pub(crate) fn apply_layout_technology_preserving_extra_layers(
    document: &mut Document,
    technology: &TechnologyFile,
) -> Result<usize, TechnologyError> {
    let existing_layers = document.layers.values().cloned().collect::<Vec<_>>();
    document.apply_technology(technology)?;
    let mut preserved = 0usize;
    for layer in existing_layers {
        if document.layers.contains_key(&layer.id) {
            continue;
        }
        let next_after_id = layer.id.0.checked_add(1).ok_or_else(|| {
            TechnologyError::Invalid(
                "unable to preserve extra layer; layer id space is exhausted".to_string(),
            )
        })?;
        document.next_layer_id = document.next_layer_id.max(next_after_id);
        document.layers.insert(layer.id, layer);
        preserved += 1;
    }
    Ok(preserved)
}
