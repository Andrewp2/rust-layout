#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug)]
pub(crate) struct LayoutIndexCacheValue {
    pub(crate) layout_revision: u64,
    pub(crate) view_revision: u64,
    pub(crate) root_cell: CellId,
    pub(crate) min_depth: usize,
    pub(crate) max_depth: Option<usize>,
    pub(crate) index: LayoutIndex,
}

#[derive(Clone, Debug)]
pub(crate) struct ConnectivityReportCacheValue {
    pub(crate) revision: u64,
    pub(crate) report: Result<ConnectivityReport, String>,
}

#[derive(Clone, Debug)]
pub(crate) struct DrcReportCacheEntry {
    pub(crate) revision: u64,
    pub(crate) value: DrcReportCacheValue,
}

#[derive(Clone, Debug)]
pub(crate) struct DrcReportHistoryEntry {
    pub(crate) id: u64,
    pub(crate) revision: u64,
    pub(crate) label: String,
    pub(crate) value: DrcReportCacheValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutHierarchyDepth {
    Full,
    Top,
    One,
    Two,
    Three,
    Numeric(u8),
    Boxes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutImportLayerPolicy {
    Id,
    Name,
    Copy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LayoutLayerGroupFilter {
    All,
    FrontEnd,
    FrontEndDiffusion,
    FrontEndGate,
    FrontEndDielectric,
    Routing,
    RoutingContact,
    RoutingMetal,
    RoutingVia,
    Annotation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutLayerUsageFilter {
    All,
    Used,
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutLayerSetState {
    pub(crate) visible_layers: BTreeSet<LayerId>,
    pub(crate) layer_group_filter: LayoutLayerGroupFilter,
    pub(crate) layer_usage_filter: LayoutLayerUsageFilter,
    pub(crate) layer_depth_overrides: BTreeMap<LayerId, LayoutHierarchyDepth>,
}

impl LayoutImportLayerPolicy {
    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "id" | "ids" | "number" | "numbers" => Some(Self::Id),
            "name" | "names" => Some(Self::Name),
            "copy" | "copies" | "new" => Some(Self::Copy),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Name => "name",
            Self::Copy => "copy",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Id => "By ID",
            Self::Name => "By Name",
            Self::Copy => "Copy Layers",
        }
    }

    pub(crate) fn status_label(self) -> &'static str {
        match self {
            Self::Id => "by layer ID",
            Self::Name => "by layer name",
            Self::Copy => "copy every source layer",
        }
    }
}

impl LayoutLayerGroupFilter {
    pub(crate) const ALL: [Self; 10] = [
        Self::All,
        Self::FrontEnd,
        Self::FrontEndDiffusion,
        Self::FrontEndGate,
        Self::FrontEndDielectric,
        Self::Routing,
        Self::RoutingContact,
        Self::RoutingMetal,
        Self::RoutingVia,
        Self::Annotation,
    ];
    pub(crate) const PRIMARY: [Self; 4] =
        [Self::All, Self::FrontEnd, Self::Routing, Self::Annotation];
    pub(crate) const FRONT_END_CHILDREN: [Self; 3] = [
        Self::FrontEndDiffusion,
        Self::FrontEndGate,
        Self::FrontEndDielectric,
    ];
    pub(crate) const ROUTING_CHILDREN: [Self; 3] =
        [Self::RoutingContact, Self::RoutingMetal, Self::RoutingVia];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "all" => Some(Self::All),
            "front_end" | "front-end" | "feol" | "process" => Some(Self::FrontEnd),
            "feol_diffusion"
            | "feol-diffusion"
            | "front_end_diffusion"
            | "front-end-diffusion"
            | "active"
            | "diffusion" => Some(Self::FrontEndDiffusion),
            "feol_gate" | "feol-gate" | "front_end_gate" | "front-end-gate" | "gate" | "poly"
            | "polysilicon" => Some(Self::FrontEndGate),
            "feol_dielectric"
            | "feol-dielectric"
            | "front_end_dielectric"
            | "front-end-dielectric"
            | "dielectric"
            | "oxide" => Some(Self::FrontEndDielectric),
            "routing" | "route" | "interconnect" | "beol" => Some(Self::Routing),
            "routing_contact" | "routing-contact" | "contact" | "contacts" | "cont" => {
                Some(Self::RoutingContact)
            }
            "routing_metal" | "routing-metal" | "metal" | "metals" => Some(Self::RoutingMetal),
            "routing_via" | "routing-via" | "via" | "vias" => Some(Self::RoutingVia),
            "annotation" | "text" | "labels" => Some(Self::Annotation),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::FrontEnd => "front_end",
            Self::FrontEndDiffusion => "feol_diffusion",
            Self::FrontEndGate => "feol_gate",
            Self::FrontEndDielectric => "feol_dielectric",
            Self::Routing => "routing",
            Self::RoutingContact => "routing_contact",
            Self::RoutingMetal => "routing_metal",
            Self::RoutingVia => "routing_via",
            Self::Annotation => "annotation",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::FrontEnd => "FEOL",
            Self::FrontEndDiffusion => "Diffusion",
            Self::FrontEndGate => "Gate",
            Self::FrontEndDielectric => "Oxide",
            Self::Routing => "Routing",
            Self::RoutingContact => "Contacts",
            Self::RoutingMetal => "Metals",
            Self::RoutingVia => "Vias",
            Self::Annotation => "Text",
        }
    }

    pub(crate) fn path_label(self) -> String {
        self.parent()
            .map(|parent| format!("{} / {}", parent.label(), self.label()))
            .unwrap_or_else(|| self.label().to_string())
    }

    pub(crate) fn parent(self) -> Option<Self> {
        match self {
            Self::FrontEndDiffusion | Self::FrontEndGate | Self::FrontEndDielectric => {
                Some(Self::FrontEnd)
            }
            Self::RoutingContact | Self::RoutingMetal | Self::RoutingVia => Some(Self::Routing),
            Self::All | Self::FrontEnd | Self::Routing | Self::Annotation => None,
        }
    }

    pub(crate) fn child_groups(self) -> &'static [Self] {
        match self {
            Self::FrontEnd => &Self::FRONT_END_CHILDREN,
            Self::Routing => &Self::ROUTING_CHILDREN,
            _ => &[],
        }
    }

    pub(crate) fn matches(self, process: ProcessLayer) -> bool {
        match self {
            Self::All => true,
            Self::FrontEnd => matches!(
                process,
                ProcessLayer::Diffusion | ProcessLayer::Poly | ProcessLayer::Oxide
            ),
            Self::FrontEndDiffusion => matches!(process, ProcessLayer::Diffusion),
            Self::FrontEndGate => matches!(process, ProcessLayer::Poly),
            Self::FrontEndDielectric => matches!(process, ProcessLayer::Oxide),
            Self::Routing => matches!(
                process,
                ProcessLayer::Contact
                    | ProcessLayer::Metal1
                    | ProcessLayer::Via1
                    | ProcessLayer::Metal2
            ),
            Self::RoutingContact => matches!(process, ProcessLayer::Contact),
            Self::RoutingMetal => matches!(process, ProcessLayer::Metal1 | ProcessLayer::Metal2),
            Self::RoutingVia => matches!(process, ProcessLayer::Via1),
            Self::Annotation => matches!(process, ProcessLayer::Annotation),
        }
    }
}

impl LayoutLayerUsageFilter {
    pub(crate) const ALL: [Self; 3] = [Self::All, Self::Used, Self::Empty];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "all" => Some(Self::All),
            "used" | "populated" | "non_empty" | "non-empty" => Some(Self::Used),
            "empty" | "unused" => Some(Self::Empty),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Used => "used",
            Self::Empty => "empty",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Used => "Used",
            Self::Empty => "Empty",
        }
    }

    pub(crate) fn matches(self, usage_count: usize) -> bool {
        match self {
            Self::All => true,
            Self::Used => usage_count > 0,
            Self::Empty => usage_count == 0,
        }
    }
}

pub(crate) fn layout_layer_group_for_process(process: ProcessLayer) -> LayoutLayerGroupFilter {
    match process {
        ProcessLayer::Diffusion => LayoutLayerGroupFilter::FrontEndDiffusion,
        ProcessLayer::Poly => LayoutLayerGroupFilter::FrontEndGate,
        ProcessLayer::Oxide => LayoutLayerGroupFilter::FrontEndDielectric,
        ProcessLayer::Contact => LayoutLayerGroupFilter::RoutingContact,
        ProcessLayer::Metal1 | ProcessLayer::Metal2 => LayoutLayerGroupFilter::RoutingMetal,
        ProcessLayer::Via1 => LayoutLayerGroupFilter::RoutingVia,
        ProcessLayer::Annotation => LayoutLayerGroupFilter::Annotation,
    }
}

impl LayoutHierarchyDepth {
    pub(crate) const ALL: [Self; 8] = [
        Self::Top,
        Self::One,
        Self::Two,
        Self::Three,
        Self::Numeric(4),
        Self::Numeric(5),
        Self::Boxes,
        Self::Full,
    ];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        match value.as_str() {
            "full" => Some(Self::Full),
            "top" | "0" | "depth_0" | "depth-0" | "max_0" | "max-0" => Some(Self::Top),
            "one" | "1" | "depth_1" | "depth-1" | "max_1" | "max-1" => Some(Self::One),
            "two" | "2" | "depth_2" | "depth-2" | "max_2" | "max-2" => Some(Self::Two),
            "three" | "3" | "depth_3" | "depth-3" | "max_3" | "max-3" => Some(Self::Three),
            "boxes" | "box" => Some(Self::Boxes),
            _ => parse_numeric_hierarchy_depth(&value).map(Self::Numeric),
        }
    }

    pub(crate) fn slug(self) -> String {
        match self {
            Self::Full => "full".to_string(),
            Self::Top => "top".to_string(),
            Self::One => "one".to_string(),
            Self::Two => "two".to_string(),
            Self::Three => "three".to_string(),
            Self::Numeric(depth) => format!("depth_{depth}"),
            Self::Boxes => "boxes".to_string(),
        }
    }

    pub(crate) fn label(self) -> String {
        match self {
            Self::Full => "Full".to_string(),
            Self::Top => "Top".to_string(),
            Self::One => "1".to_string(),
            Self::Two => "2".to_string(),
            Self::Three => "3".to_string(),
            Self::Numeric(depth) => depth.to_string(),
            Self::Boxes => "Box".to_string(),
        }
    }

    pub(crate) fn detail_label(self) -> String {
        match self {
            Self::Full => "full hierarchy".to_string(),
            Self::Top => "top cell only".to_string(),
            Self::One => "one hierarchy level".to_string(),
            Self::Two => "two hierarchy levels".to_string(),
            Self::Three => "three hierarchy levels".to_string(),
            Self::Numeric(depth) => format!("{depth} hierarchy levels"),
            Self::Boxes => "child cell boxes".to_string(),
        }
    }

    pub(crate) fn max_depth(self) -> Option<usize> {
        match self {
            Self::Full => None,
            Self::Top => Some(0),
            Self::One => Some(1),
            Self::Two => Some(2),
            Self::Three => Some(3),
            Self::Numeric(depth) => Some(depth as usize),
            Self::Boxes => Some(0),
        }
    }

    pub(crate) fn for_minimum_depth(depth: usize) -> Self {
        match depth {
            0 => Self::Top,
            1 => Self::One,
            2 => Self::Two,
            3 => Self::Three,
            _ => Self::Numeric(
                depth
                    .min(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH as usize)
                    .max(4) as u8,
            ),
        }
    }

    pub(crate) fn shows_instance_boxes(self) -> bool {
        matches!(self, Self::Boxes)
    }

    pub(crate) fn deeper(self) -> Self {
        match self {
            Self::Top => Self::One,
            Self::One => Self::Two,
            Self::Two => Self::Three,
            Self::Three => Self::Numeric(4),
            Self::Numeric(depth) if depth < LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH => {
                Self::Numeric(depth + 1)
            }
            Self::Numeric(_) | Self::Full => Self::Full,
            Self::Boxes => Self::One,
        }
    }

    pub(crate) fn shallower(self) -> Self {
        match self {
            Self::Full => Self::Numeric(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH),
            Self::Numeric(depth) if depth > 4 => Self::Numeric(depth - 1),
            Self::Numeric(_) => Self::Three,
            Self::Three => Self::Two,
            Self::Two => Self::One,
            Self::One | Self::Top => Self::Top,
            Self::Boxes => Self::Top,
        }
    }
}

pub(crate) fn parse_numeric_hierarchy_depth(value: &str) -> Option<u8> {
    let candidate = value
        .strip_prefix("depth_")
        .or_else(|| value.strip_prefix("depth-"))
        .or_else(|| value.strip_prefix("max_"))
        .or_else(|| value.strip_prefix("max-"))
        .unwrap_or(value);
    let depth = candidate.parse::<u8>().ok()?;
    (4..=LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH)
        .contains(&depth)
        .then_some(depth)
}

pub(crate) fn clamp_layout_hierarchy_min_depth_for_max(
    min_depth: u8,
    max_depth: LayoutHierarchyDepth,
) -> u8 {
    let min_depth = min_depth.min(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH);
    if let Some(max_depth) = max_depth.max_depth() {
        min_depth.min(max_depth.min(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH as usize) as u8)
    } else {
        min_depth
    }
}

pub(crate) fn parse_layout_hierarchy_min_depth(value: &str) -> Option<u8> {
    let value = value.trim().to_ascii_lowercase();
    match value.as_str() {
        "top" | "0" | "depth_0" | "depth-0" | "min_0" | "min-0" => Some(0),
        "one" | "1" | "depth_1" | "depth-1" | "min_1" | "min-1" => Some(1),
        "two" | "2" | "depth_2" | "depth-2" | "min_2" | "min-2" => Some(2),
        "three" | "3" | "depth_3" | "depth-3" | "min_3" | "min-3" => Some(3),
        _ => {
            let candidate = value
                .strip_prefix("depth_")
                .or_else(|| value.strip_prefix("depth-"))
                .or_else(|| value.strip_prefix("min_"))
                .or_else(|| value.strip_prefix("min-"))
                .unwrap_or(&value);
            let depth = candidate.parse::<u8>().ok()?;
            (depth <= LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH).then_some(depth)
        }
    }
}

pub(crate) fn layout_hierarchy_min_depth_label(min_depth: u8) -> String {
    match min_depth {
        0 => "top level and below".to_string(),
        1 => "from one hierarchy level".to_string(),
        2 => "from two hierarchy levels".to_string(),
        3 => "from three hierarchy levels".to_string(),
        depth => format!("from {depth} hierarchy levels"),
    }
}

pub(crate) fn layout_hierarchy_display_label(
    max_depth: LayoutHierarchyDepth,
    min_depth: u8,
) -> String {
    if min_depth == 0 {
        max_depth.detail_label()
    } else {
        format!(
            "{} to {}",
            layout_hierarchy_min_depth_label(min_depth),
            max_depth.detail_label()
        )
    }
}

pub(crate) fn layout_layer_depth_override_label(depth: Option<LayoutHierarchyDepth>) -> String {
    depth
        .map(|depth| depth.detail_label())
        .unwrap_or_else(|| "inherited".to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutFlattenDepth {
    OneLevel,
    Deep,
}

impl LayoutFlattenDepth {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OneLevel => "one level",
            Self::Deep => "deep",
        }
    }
}

#[derive(Default)]
pub(crate) struct LayoutFlattenReplacements {
    pub(crate) shapes: Vec<Shape>,
    pub(crate) instances: Vec<CellInstance>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LayoutViewState {
    pub(crate) zoom: f32,
    pub(crate) pan: [f32; 2],
    pub(crate) top_cell: CellId,
    pub(crate) hierarchy_depth: LayoutHierarchyDepth,
    pub(crate) hierarchy_min_depth: u8,
}

impl LayoutViewState {
    pub(crate) fn from_bookmark_options(bookmark: &options::LayoutViewBookmarkOptions) -> Self {
        let hierarchy_depth = LayoutHierarchyDepth::from_slug(&bookmark.hierarchy_depth)
            .unwrap_or(LayoutHierarchyDepth::Full);
        Self {
            zoom: bookmark.zoom.clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM),
            pan: bookmark.pan,
            top_cell: CellId(bookmark.top_cell),
            hierarchy_depth,
            hierarchy_min_depth: clamp_layout_hierarchy_min_depth_for_max(
                bookmark.hierarchy_min_depth,
                hierarchy_depth,
            ),
        }
    }

    pub(crate) fn to_bookmark_options(self, slot: u8) -> options::LayoutViewBookmarkOptions {
        options::LayoutViewBookmarkOptions {
            slot,
            name: String::new(),
            zoom: self.zoom,
            pan: self.pan,
            top_cell: self.top_cell.0,
            hierarchy_depth: self.hierarchy_depth.slug(),
            hierarchy_min_depth: self.hierarchy_min_depth,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutShapeBrowserFilter {
    All,
    ActiveLayer,
    Selected,
    SelectedNet,
    Properties,
    Rectangles,
    Polygons,
    Paths,
    Vias,
    Labels,
    Measurements,
}

impl LayoutShapeBrowserFilter {
    pub(crate) const ALL: [Self; 11] = [
        Self::All,
        Self::ActiveLayer,
        Self::Selected,
        Self::SelectedNet,
        Self::Properties,
        Self::Rectangles,
        Self::Polygons,
        Self::Paths,
        Self::Vias,
        Self::Labels,
        Self::Measurements,
    ];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "active_layer" | "active-layer" | "layer" => Some(Self::ActiveLayer),
            "selected" | "selection" | "selected_shape" | "selected-shape" => Some(Self::Selected),
            "selected_net" | "selected-net" | "net" => Some(Self::SelectedNet),
            "properties" | "property" | "props" | "named" | "with_properties" => {
                Some(Self::Properties)
            }
            "rectangles" | "rectangle" | "rects" | "rect" => Some(Self::Rectangles),
            "polygons" | "polygon" | "poly" => Some(Self::Polygons),
            "paths" | "path" => Some(Self::Paths),
            "vias" | "via" => Some(Self::Vias),
            "labels" | "label" | "text" => Some(Self::Labels),
            "measurements" | "measurement" | "rulers" | "ruler" => Some(Self::Measurements),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::ActiveLayer => "active_layer",
            Self::Selected => "selected",
            Self::SelectedNet => "selected_net",
            Self::Properties => "properties",
            Self::Rectangles => "rectangles",
            Self::Polygons => "polygons",
            Self::Paths => "paths",
            Self::Vias => "vias",
            Self::Labels => "labels",
            Self::Measurements => "measurements",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::ActiveLayer => "Active Layer",
            Self::Selected => "Selected",
            Self::SelectedNet => "Selected Net",
            Self::Properties => "Properties",
            Self::Rectangles => "Rectangles",
            Self::Polygons => "Polygons",
            Self::Paths => "Paths",
            Self::Vias => "Vias",
            Self::Labels => "Labels",
            Self::Measurements => "Measurements",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutShapeBrowserSort {
    Id,
    Layer,
    Cell,
    Kind,
}

impl LayoutShapeBrowserSort {
    pub(crate) const ALL: [Self; 4] = [Self::Id, Self::Layer, Self::Cell, Self::Kind];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "id" => Some(Self::Id),
            "layer" => Some(Self::Layer),
            "cell" | "source_cell" | "source-cell" => Some(Self::Cell),
            "kind" | "type" => Some(Self::Kind),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Layer => "layer",
            Self::Cell => "cell",
            Self::Kind => "kind",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Layer => "Layer",
            Self::Cell => "Cell",
            Self::Kind => "Kind",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutCellBrowserFilter {
    All,
    Current,
    Used,
    Unused,
    Library,
    Properties,
    Empty,
    Leaves,
    Branches,
    Parents,
    Children,
}

impl LayoutCellBrowserFilter {
    pub(crate) const ALL: [Self; 11] = [
        Self::All,
        Self::Current,
        Self::Used,
        Self::Unused,
        Self::Library,
        Self::Properties,
        Self::Empty,
        Self::Leaves,
        Self::Branches,
        Self::Parents,
        Self::Children,
    ];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "current" | "current_cell" | "current-cell" => Some(Self::Current),
            "used" | "referenced" => Some(Self::Used),
            "unused" | "unreferenced" => Some(Self::Unused),
            "library" | "macro" | "pcell" => Some(Self::Library),
            "properties" | "property" | "props" | "metadata" | "with_properties" => {
                Some(Self::Properties)
            }
            "empty" | "blank" | "no_geometry" | "no-geometry" => Some(Self::Empty),
            "leaves" | "leaf" => Some(Self::Leaves),
            "branches" | "branch" | "non_leaf" | "non-leaf" => Some(Self::Branches),
            "parents" | "parent" => Some(Self::Parents),
            "children" | "child" => Some(Self::Children),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Current => "current",
            Self::Used => "used",
            Self::Unused => "unused",
            Self::Library => "library",
            Self::Properties => "properties",
            Self::Empty => "empty",
            Self::Leaves => "leaves",
            Self::Branches => "branches",
            Self::Parents => "parents",
            Self::Children => "children",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Current => "Current",
            Self::Used => "Used",
            Self::Unused => "Unused",
            Self::Library => "Library",
            Self::Properties => "Properties",
            Self::Empty => "Empty",
            Self::Leaves => "Leaves",
            Self::Branches => "Branches",
            Self::Parents => "Parents",
            Self::Children => "Children",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutCellBrowserSort {
    Name,
    Id,
    ShapeCount,
    InstanceCount,
}

impl LayoutCellBrowserSort {
    pub(crate) const ALL: [Self; 4] = [Self::Name, Self::Id, Self::ShapeCount, Self::InstanceCount];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "name" => Some(Self::Name),
            "id" => Some(Self::Id),
            "shape_count" | "shape-count" | "shapes" => Some(Self::ShapeCount),
            "instance_count" | "instance-count" | "instances" => Some(Self::InstanceCount),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Id => "id",
            Self::ShapeCount => "shape_count",
            Self::InstanceCount => "instance_count",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Id => "ID",
            Self::ShapeCount => "Shapes",
            Self::InstanceCount => "Instances",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutNetBrowserFilter {
    All,
    Labeled,
    Unlabeled,
    Devices,
    History,
    SpiceExtra,
    Shorted,
    Open,
}

impl LayoutNetBrowserFilter {
    pub(crate) const ALL: [Self; 8] = [
        Self::All,
        Self::Labeled,
        Self::Unlabeled,
        Self::Devices,
        Self::History,
        Self::SpiceExtra,
        Self::Shorted,
        Self::Open,
    ];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "labeled" | "named" => Some(Self::Labeled),
            "unlabeled" | "unnamed" => Some(Self::Unlabeled),
            "devices" | "device" | "device_connected" | "device-connected" => Some(Self::Devices),
            "history" | "traced" | "trace_history" | "trace-history" => Some(Self::History),
            "spice_extra" | "spice-extra" | "extra_layout" | "extra-layout" | "lvs_extra"
            | "lvs-extra" => Some(Self::SpiceExtra),
            "shorted" | "short" | "shorts" => Some(Self::Shorted),
            "open" | "opens" => Some(Self::Open),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Labeled => "labeled",
            Self::Unlabeled => "unlabeled",
            Self::Devices => "devices",
            Self::History => "history",
            Self::SpiceExtra => "spice_extra",
            Self::Shorted => "shorted",
            Self::Open => "open",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Labeled => "Labeled",
            Self::Unlabeled => "Unlabeled",
            Self::Devices => "Devices",
            Self::History => "History",
            Self::SpiceExtra => "SPICE Extra",
            Self::Shorted => "Shorted",
            Self::Open => "Open",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutNetBrowserSort {
    Name,
    Size,
    Id,
}

impl LayoutNetBrowserSort {
    pub(crate) const ALL: [Self; 3] = [Self::Name, Self::Size, Self::Id];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "name" => Some(Self::Name),
            "size" | "shapes" => Some(Self::Size),
            "id" => Some(Self::Id),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Size => "size",
            Self::Id => "id",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Size => "Size",
            Self::Id => "ID",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutTraceHighlightMode {
    Selected,
    History,
    Off,
}

impl LayoutTraceHighlightMode {
    pub(crate) const ALL: [Self; 3] = [Self::Selected, Self::History, Self::Off];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "selected" | "selection" => Some(Self::Selected),
            "history" | "traces" => Some(Self::History),
            "off" | "none" => Some(Self::Off),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::History => "history",
            Self::Off => "off",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Selected => "Selected",
            Self::History => "History",
            Self::Off => "Off",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutInstanceBrowserScope {
    CurrentCell,
    Hierarchy,
}

impl LayoutInstanceBrowserScope {
    pub(crate) const ALL: [Self; 2] = [Self::CurrentCell, Self::Hierarchy];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "current" | "current_cell" | "current-cell" => Some(Self::CurrentCell),
            "hierarchy" | "tree" => Some(Self::Hierarchy),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::CurrentCell => "current_cell",
            Self::Hierarchy => "hierarchy",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::CurrentCell => "Current Cell",
            Self::Hierarchy => "Hierarchy",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutInstanceBrowserFilter {
    All,
    Named,
    Properties,
    Arrays,
    Identity,
    Transformed,
}

impl LayoutInstanceBrowserFilter {
    pub(crate) const ALL: [Self; 6] = [
        Self::All,
        Self::Named,
        Self::Properties,
        Self::Arrays,
        Self::Identity,
        Self::Transformed,
    ];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "named" | "name" => Some(Self::Named),
            "properties" | "property" | "props" | "metadata" | "with_properties" => {
                Some(Self::Properties)
            }
            "arrays" | "array" | "aref" => Some(Self::Arrays),
            "identity" | "untransformed" | "no_transform" | "no-transform" => Some(Self::Identity),
            "transformed" | "transform" | "placed" | "non_identity" | "non-identity" => {
                Some(Self::Transformed)
            }
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Named => "named",
            Self::Properties => "properties",
            Self::Arrays => "arrays",
            Self::Identity => "identity",
            Self::Transformed => "transformed",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Named => "Named",
            Self::Properties => "Properties",
            Self::Arrays => "Arrays",
            Self::Identity => "Identity",
            Self::Transformed => "Transformed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutInstanceBrowserSort {
    Id,
    Parent,
    Child,
    Array,
}

impl LayoutInstanceBrowserSort {
    pub(crate) const ALL: [Self; 4] = [Self::Id, Self::Parent, Self::Child, Self::Array];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "id" => Some(Self::Id),
            "parent" | "parent_cell" | "parent-cell" => Some(Self::Parent),
            "child" | "child_cell" | "child-cell" => Some(Self::Child),
            "array" => Some(Self::Array),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Parent => "parent",
            Self::Child => "child",
            Self::Array => "array",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Parent => "Parent",
            Self::Child => "Child",
            Self::Array => "Array",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutBrowserReplaceScope {
    Document,
    CurrentCell,
    VisibleHierarchy,
}

impl LayoutBrowserReplaceScope {
    pub(crate) const ALL: [Self; 3] = [Self::Document, Self::CurrentCell, Self::VisibleHierarchy];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "document" | "all" => Some(Self::Document),
            "current_cell" | "current-cell" | "cell" => Some(Self::CurrentCell),
            "visible_hierarchy" | "visible-hierarchy" | "visible" | "view" => {
                Some(Self::VisibleHierarchy)
            }
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::CurrentCell => "current_cell",
            Self::VisibleHierarchy => "visible_hierarchy",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Document => "Document",
            Self::CurrentCell => "Current Cell",
            Self::VisibleHierarchy => "Visible",
        }
    }

    pub(crate) fn status_label(self) -> &'static str {
        match self {
            Self::Document => "whole document",
            Self::CurrentCell => "current cell",
            Self::VisibleHierarchy => "visible hierarchy",
        }
    }
}

#[derive(Default)]
pub(crate) struct LayoutBrowserReplaceTargets {
    pub(crate) top_shapes: BTreeSet<ShapeId>,
    pub(crate) cell_shapes: BTreeMap<CellId, BTreeSet<ShapeId>>,
    pub(crate) cells: BTreeSet<CellId>,
    pub(crate) instances: BTreeMap<CellId, BTreeSet<InstanceId>>,
    pub(crate) include_layers: bool,
}

impl LayoutBrowserReplaceTargets {
    pub(crate) fn includes_top_shape(&self, shape: ShapeId) -> bool {
        self.top_shapes.contains(&shape)
    }

    pub(crate) fn includes_cell_shape(&self, cell: CellId, shape: ShapeId) -> bool {
        self.cell_shapes
            .get(&cell)
            .is_some_and(|shapes| shapes.contains(&shape))
    }

    pub(crate) fn includes_cell(&self, cell: CellId) -> bool {
        self.cells.contains(&cell)
    }

    pub(crate) fn includes_instance(&self, parent: CellId, instance: InstanceId) -> bool {
        self.instances
            .get(&parent)
            .is_some_and(|instances| instances.contains(&instance))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutMeasurementFilter {
    All,
    ActiveLayer,
    Direct,
    Horizontal,
    Vertical,
    Manhattan,
}

impl LayoutMeasurementFilter {
    pub(crate) const ALL: [Self; 6] = [
        Self::All,
        Self::ActiveLayer,
        Self::Direct,
        Self::Horizontal,
        Self::Vertical,
        Self::Manhattan,
    ];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "active_layer" | "active-layer" | "layer" => Some(Self::ActiveLayer),
            "direct" => Some(Self::Direct),
            "horizontal" | "x" => Some(Self::Horizontal),
            "vertical" | "y" => Some(Self::Vertical),
            "manhattan" | "orthogonal" | "xy" => Some(Self::Manhattan),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::ActiveLayer => "active_layer",
            Self::Direct => "direct",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Manhattan => "manhattan",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::ActiveLayer => "Active Layer",
            Self::Direct => "Direct",
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
            Self::Manhattan => "Manhattan",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutBrowserColumnSet {
    Summary,
    Geometry,
    Relations,
    All,
}

impl LayoutBrowserColumnSet {
    pub(crate) const ALL: [Self; 4] = [Self::Summary, Self::Geometry, Self::Relations, Self::All];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "summary" | "basic" => Some(Self::Summary),
            "geometry" | "geom" => Some(Self::Geometry),
            "relations" | "relation" | "connectivity" | "state" => Some(Self::Relations),
            "all" | "full" => Some(Self::All),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Geometry => "geometry",
            Self::Relations => "relations",
            Self::All => "all",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Geometry => "Geometry",
            Self::Relations => "Relations",
            Self::All => "All",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutDrcMarkerFilter {
    Active,
    All,
    Hidden,
    Waived,
    Visited,
    Important,
    Noted,
    Owned,
    SignedOff,
    Tagged,
    Snapshots,
}

impl LayoutDrcMarkerFilter {
    pub(crate) const ALL: [Self; 11] = [
        Self::Active,
        Self::All,
        Self::Hidden,
        Self::Waived,
        Self::Visited,
        Self::Important,
        Self::Noted,
        Self::Owned,
        Self::SignedOff,
        Self::Tagged,
        Self::Snapshots,
    ];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "all" => Some(Self::All),
            "hidden" => Some(Self::Hidden),
            "waived" => Some(Self::Waived),
            "visited" => Some(Self::Visited),
            "important" => Some(Self::Important),
            "noted" => Some(Self::Noted),
            "owned" => Some(Self::Owned),
            "signed_off" | "signed-off" => Some(Self::SignedOff),
            "tagged" => Some(Self::Tagged),
            "snapshots" | "snapshot" | "screenshots" | "screenshot" => Some(Self::Snapshots),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::All => "all",
            Self::Hidden => "hidden",
            Self::Waived => "waived",
            Self::Visited => "visited",
            Self::Important => "important",
            Self::Noted => "noted",
            Self::Owned => "owned",
            Self::SignedOff => "signed_off",
            Self::Tagged => "tagged",
            Self::Snapshots => "snapshots",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::All => "All",
            Self::Hidden => "Hidden",
            Self::Waived => "Waived",
            Self::Visited => "Visited",
            Self::Important => "Important",
            Self::Noted => "Noted",
            Self::Owned => "Owned",
            Self::SignedOff => "Signed Off",
            Self::Tagged => "Tagged",
            Self::Snapshots => "Snapshots",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutDrcMarkerSort {
    Id,
    Rule,
    State,
}

impl LayoutDrcMarkerSort {
    pub(crate) const ALL: [Self; 3] = [Self::Id, Self::Rule, Self::State];

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "id" => Some(Self::Id),
            "rule" => Some(Self::Rule),
            "state" => Some(Self::State),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Rule => "rule",
            Self::State => "state",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Rule => "Rule",
            Self::State => "State",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LayoutDrcMarkerCategory {
    Grid,
    Width,
    Area,
    Spacing,
    Enclosure,
    Overlap,
    Other,
}

impl LayoutDrcMarkerCategory {
    pub(crate) fn from_rule(rule: &str) -> Self {
        match rule {
            "off_grid" => Self::Grid,
            "min_width" | "max_width" => Self::Width,
            _ if rule.starts_with("derived_min_width.")
                || rule.starts_with("derived_max_width.") =>
            {
                Self::Width
            }
            "min_area" | "max_area" => Self::Area,
            _ if rule.starts_with("derived_min_area.") || rule.starts_with("derived_max_area.") => {
                Self::Area
            }
            "min_spacing" | "min_edge_spacing" => Self::Spacing,
            _ if rule.starts_with("derived_min_spacing.")
                || rule.starts_with("derived_min_edge_spacing.") =>
            {
                Self::Spacing
            }
            "via_enclosure" => Self::Enclosure,
            _ if rule.contains("overlap") => Self::Overlap,
            _ => Self::Other,
        }
    }

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "grid" => Some(Self::Grid),
            "width" => Some(Self::Width),
            "area" => Some(Self::Area),
            "spacing" => Some(Self::Spacing),
            "enclosure" => Some(Self::Enclosure),
            "overlap" => Some(Self::Overlap),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Grid => "grid",
            Self::Width => "width",
            Self::Area => "area",
            Self::Spacing => "spacing",
            Self::Enclosure => "enclosure",
            Self::Overlap => "overlap",
            Self::Other => "other",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::Width => "Width",
            Self::Area => "Area",
            Self::Spacing => "Spacing",
            Self::Enclosure => "Enclosure",
            Self::Overlap => "Overlap",
            Self::Other => "Other",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutDrcMarkerEntry {
    pub(crate) id: usize,
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) state_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutDrcMarkerCategoryEntry {
    pub(crate) category: LayoutDrcMarkerCategory,
    pub(crate) total: usize,
    pub(crate) active: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutDrcMarkerDirectoryEntry {
    pub(crate) path: String,
    pub(crate) total: usize,
    pub(crate) active: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutViaArrayLibraryPreset {
    pub(crate) name: String,
    pub(crate) columns: u8,
    pub(crate) rows: u8,
    pub(crate) size_grids: Coord,
    pub(crate) pitch_grids: Coord,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct LayoutJsonCellImportPlan {
    pub(crate) redo_operations: Vec<Operation>,
    pub(crate) undo_operations: Vec<Operation>,
    pub(crate) first_shape: Option<ShapeId>,
    pub(crate) instance_id: InstanceId,
    pub(crate) root_cell: CellId,
    pub(crate) root_cell_name: String,
    pub(crate) added_layers: usize,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct LayoutJsonMergeImportPlan {
    pub(crate) redo_operations: Vec<Operation>,
    pub(crate) undo_operations: Vec<Operation>,
    pub(crate) first_shape: Option<ShapeId>,
    pub(crate) imported_shapes: usize,
    pub(crate) added_layers: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutVertexHit {
    pub(crate) shape_id: ShapeId,
    pub(crate) vertex: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutEdgeHit {
    pub(crate) shape_id: ShapeId,
    pub(crate) edge: usize,
}
