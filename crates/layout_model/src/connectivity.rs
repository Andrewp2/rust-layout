use std::collections::{BTreeMap, BTreeSet};

use geometry_core::{Point, Rect};
use rstar::{AABB, RTree, RTreeObject};

use crate::{
    Document, LayerId, NetId, Shape, ShapeKind, ShapeOccurrenceId, TechnologyError, TechnologyFile,
};

type ConnectivityLayerSet = BTreeSet<LayerId>;
type ConnectivityLayerPairs = BTreeSet<(LayerId, LayerId)>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectivityReport {
    pub components: Vec<NetComponent>,
    pub shape_to_component: BTreeMap<ShapeOccurrenceId, usize>,
    pub shorts: Vec<NetShort>,
    pub opens: Vec<NetOpen>,
    pub skipped: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityHealth {
    Skipped,
    Clean,
    Shorts,
    Opens,
    ShortsAndOpens,
}

impl ConnectivityHealth {
    pub fn label(self) -> &'static str {
        match self {
            Self::Skipped => "skipped",
            Self::Clean => "clean",
            Self::Shorts => "shorts",
            Self::Opens => "opens",
            Self::ShortsAndOpens => "shorts and opens",
        }
    }
}

impl Default for ConnectivityHealth {
    fn default() -> Self {
        Self::Clean
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectivitySummary {
    pub health: ConnectivityHealth,
    pub component_count: usize,
    pub labeled_component_count: usize,
    pub short_count: usize,
    pub open_count: usize,
    pub largest_component_shape_count: usize,
    pub displayed_short_count: usize,
    pub displayed_open_count: usize,
    pub omitted_short_count: usize,
    pub omitted_open_count: usize,
    pub skipped: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityIssueKind {
    Short,
    Open,
}

impl ConnectivityIssueKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Open => "open",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectivityIssue {
    Short(NetShort),
    Open(NetOpen),
}

impl ConnectivityIssue {
    pub fn kind(&self) -> ConnectivityIssueKind {
        match self {
            Self::Short(_) => ConnectivityIssueKind::Short,
            Self::Open(_) => ConnectivityIssueKind::Open,
        }
    }

    pub fn bounds(&self) -> Rect {
        match self {
            Self::Short(short) => short.bounds,
            Self::Open(open) => open.bounds,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectivityIssueRecord {
    pub key: String,
    pub issue: ConnectivityIssue,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectivityIssueSummary {
    pub total_count: usize,
    pub displayed_count: usize,
    pub omitted_count: usize,
    pub short_count: usize,
    pub open_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectivityIssueStore {
    pub records: Vec<ConnectivityIssueRecord>,
    key_index: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectivityValidationFinding {
    pub severity: ConnectivityValidationSeverity,
    pub message: String,
}

impl ConnectivityValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: ConnectivityValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: ConnectivityValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

impl ConnectivityReport {
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            skipped: Some(reason.into()),
            ..Default::default()
        }
    }

    pub fn component(&self, id: usize) -> Option<&NetComponent> {
        id.checked_sub(1)
            .and_then(|index| self.components.get(index))
    }

    pub fn component_for_occurrence(&self, occurrence: &ShapeOccurrenceId) -> Option<usize> {
        self.shape_to_component.get(occurrence).copied()
    }

    pub fn summary(&self, max_issue_rows: usize) -> ConnectivitySummary {
        let displayed_short_count = self.shorts.len().min(max_issue_rows);
        let displayed_open_count = self.opens.len().min(max_issue_rows);
        ConnectivitySummary {
            health: if self.skipped.is_some() {
                ConnectivityHealth::Skipped
            } else if self.shorts.is_empty() && self.opens.is_empty() {
                ConnectivityHealth::Clean
            } else if self.shorts.is_empty() {
                ConnectivityHealth::Opens
            } else if self.opens.is_empty() {
                ConnectivityHealth::Shorts
            } else {
                ConnectivityHealth::ShortsAndOpens
            },
            component_count: self.components.len(),
            labeled_component_count: self
                .components
                .iter()
                .filter(|component| component.net_name.is_some())
                .count(),
            short_count: self.shorts.len(),
            open_count: self.opens.len(),
            largest_component_shape_count: self
                .components
                .iter()
                .map(|component| component.shapes.len())
                .max()
                .unwrap_or(0),
            displayed_short_count,
            displayed_open_count,
            omitted_short_count: self.shorts.len().saturating_sub(displayed_short_count),
            omitted_open_count: self.opens.len().saturating_sub(displayed_open_count),
            skipped: self.skipped.clone(),
        }
    }

    pub fn issue_store(&self) -> ConnectivityIssueStore {
        ConnectivityIssueStore::from_report(self)
    }

    pub fn validate(&self) -> Vec<ConnectivityValidationFinding> {
        let mut findings = Vec::new();
        if self
            .skipped
            .as_ref()
            .is_some_and(|reason| reason.trim().is_empty())
        {
            findings.push(ConnectivityValidationFinding::error(
                "connectivity report has an empty skipped reason",
            ));
        }
        if self.skipped.is_some()
            && (!self.components.is_empty()
                || !self.shape_to_component.is_empty()
                || !self.shorts.is_empty()
                || !self.opens.is_empty())
        {
            findings.push(ConnectivityValidationFinding::warning(
                "connectivity report is marked skipped but also contains extracted data",
            ));
        }

        let mut component_ids = BTreeSet::new();
        let mut component_shapes = BTreeMap::new();
        let mut component_shape_pairs = BTreeSet::new();
        for (index, component) in self.components.iter().enumerate() {
            let expected_id = index + 1;
            if component.id != expected_id {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "connectivity component at index {index} has id {}, expected {expected_id}",
                    component.id
                )));
            }
            if !component_ids.insert(component.id) {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "connectivity component id {} is duplicated",
                    component.id
                )));
            }
            if component.shapes.is_empty() {
                findings.push(ConnectivityValidationFinding::warning(format!(
                    "connectivity component {} has no shapes",
                    component.id
                )));
            }
            let mut seen_shapes = BTreeSet::new();
            for occurrence in &component.shapes {
                if !seen_shapes.insert(occurrence.clone()) {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "connectivity component {} repeats shape occurrence {:?}",
                        component.id, occurrence
                    )));
                }
                if let Some(mapped_component) = self.shape_to_component.get(occurrence) {
                    if *mapped_component != component.id {
                        findings.push(ConnectivityValidationFinding::error(format!(
                            "connectivity component {} shape {:?} maps to component {}",
                            component.id, occurrence, mapped_component
                        )));
                    }
                } else {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "connectivity component {} shape {:?} is missing from shape_to_component",
                        component.id, occurrence
                    )));
                }
                if let Some(previous_component) =
                    component_shapes.insert(occurrence.clone(), component.id)
                {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "connectivity shape {:?} appears in components {} and {}",
                        occurrence, previous_component, component.id
                    )));
                }
                component_shape_pairs.insert((occurrence.clone(), component.id));
            }
            validate_component_names(component, &mut findings);
        }

        for (occurrence, component_id) in &self.shape_to_component {
            if !component_ids.contains(component_id) {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "connectivity shape {occurrence:?} maps to missing component {component_id}"
                )));
            }
            if !component_shape_pairs.contains(&(occurrence.clone(), *component_id)) {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "connectivity shape {occurrence:?} maps to component {component_id}, but no matching component shape entry exists"
                )));
            }
        }

        let mut issue_keys = BTreeSet::new();
        for short in &self.shorts {
            validate_short(short, &component_ids, &mut issue_keys, &mut findings);
        }
        for open in &self.opens {
            validate_open(open, &component_ids, &mut issue_keys, &mut findings);
        }

        findings
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetComponent {
    pub id: usize,
    pub shapes: Vec<ShapeOccurrenceId>,
    pub bounds: Rect,
    pub labels: Vec<NetLabel>,
    pub explicit_nets: Vec<NetId>,
    pub net_name: Option<String>,
    pub net_id: Option<NetId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetLabel {
    pub occurrence: ShapeOccurrenceId,
    pub text: String,
    pub position: Point,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetShort {
    pub component: usize,
    pub names: Vec<String>,
    pub bounds: Rect,
}

impl NetShort {
    pub fn stable_key(&self) -> String {
        let mut names = self.names.clone();
        names.sort();
        format!(
            "short|{}|{},{},{},{}",
            names.join(","),
            self.bounds.min.x,
            self.bounds.min.y,
            self.bounds.max.x,
            self.bounds.max.y
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetOpen {
    pub name: String,
    pub components: Vec<usize>,
    pub bounds: Rect,
}

impl NetOpen {
    pub fn stable_key(&self) -> String {
        format!(
            "open|{}|{},{},{},{}",
            self.name, self.bounds.min.x, self.bounds.min.y, self.bounds.max.x, self.bounds.max.y
        )
    }
}

impl ConnectivityIssueRecord {
    pub fn kind(&self) -> ConnectivityIssueKind {
        self.issue.kind()
    }

    pub fn bounds(&self) -> Rect {
        self.issue.bounds()
    }
}

impl ConnectivityIssueStore {
    pub fn from_report(report: &ConnectivityReport) -> Self {
        let mut records = Vec::with_capacity(report.shorts.len() + report.opens.len());
        records.extend(
            report
                .shorts
                .iter()
                .cloned()
                .map(|short| ConnectivityIssueRecord {
                    key: short.stable_key(),
                    issue: ConnectivityIssue::Short(short),
                }),
        );
        records.extend(
            report
                .opens
                .iter()
                .cloned()
                .map(|open| ConnectivityIssueRecord {
                    key: open.stable_key(),
                    issue: ConnectivityIssue::Open(open),
                }),
        );

        let mut key_index = BTreeMap::new();
        for (index, record) in records.iter().enumerate() {
            key_index.entry(record.key.clone()).or_insert(index);
        }
        Self { records, key_index }
    }

    pub fn get(&self, key: &str) -> Option<&ConnectivityIssueRecord> {
        self.key_index
            .get(key)
            .and_then(|index| self.records.get(*index))
    }

    pub fn displayed_records(&self, max_issue_rows: usize) -> &[ConnectivityIssueRecord] {
        let displayed_count = self.records.len().min(max_issue_rows);
        &self.records[..displayed_count]
    }

    pub fn summary(&self, max_issue_rows: usize) -> ConnectivityIssueSummary {
        let displayed_count = self.records.len().min(max_issue_rows);
        let short_count = self
            .records
            .iter()
            .filter(|record| record.kind() == ConnectivityIssueKind::Short)
            .count();
        let open_count = self.records.len().saturating_sub(short_count);
        ConnectivityIssueSummary {
            total_count: self.records.len(),
            displayed_count,
            omitted_count: self.records.len().saturating_sub(displayed_count),
            short_count,
            open_count,
        }
    }
}

#[derive(Clone, Debug)]
struct ConnectiveShape {
    index: usize,
    occurrence: ShapeOccurrenceId,
    shape: Shape,
    bounds: Rect,
    layers: BTreeSet<LayerId>,
}

#[derive(Clone, Debug)]
struct LabelShape {
    occurrence: ShapeOccurrenceId,
    layer: LayerId,
    text: String,
    position: Point,
}

#[derive(Clone, Debug)]
struct IndexedConnectiveShape {
    index: usize,
    bounds: Rect,
}

impl RTreeObject for IndexedConnectiveShape {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.bounds.min.x as f64, self.bounds.min.y as f64],
            [self.bounds.max.x as f64, self.bounds.max.y as f64],
        )
    }
}

#[derive(Clone, Debug)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            rank: vec![0; len],
        }
    }

    fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            let root = self.find(self.parent[value]);
            self.parent[value] = root;
        }
        self.parent[value]
    }

    fn union(&mut self, left: usize, right: usize) {
        let left = self.find(left);
        let right = self.find(right);
        if left == right {
            return;
        }
        if self.rank[left] < self.rank[right] {
            self.parent[left] = right;
        } else if self.rank[left] > self.rank[right] {
            self.parent[right] = left;
        } else {
            self.parent[right] = left;
            self.rank[left] += 1;
        }
    }
}

pub fn extract_connectivity(
    document: &Document,
    technology: &TechnologyFile,
) -> Result<ConnectivityReport, TechnologyError> {
    technology.validate()?;
    let (conductive_layers, connected_pairs) = connectivity_layers(document, technology)?;
    let (connective_shapes, labels) = materialize_shapes(document, &conductive_layers);
    if connective_shapes.is_empty() {
        return Ok(ConnectivityReport::default());
    }

    let entries: Vec<_> = connective_shapes
        .iter()
        .map(|shape| IndexedConnectiveShape {
            index: shape.index,
            bounds: shape.bounds,
        })
        .collect();
    let tree = RTree::bulk_load(entries);
    let mut union_find = UnionFind::new(connective_shapes.len());

    for shape in &connective_shapes {
        let envelope = AABB::from_corners(
            [shape.bounds.min.x as f64, shape.bounds.min.y as f64],
            [shape.bounds.max.x as f64, shape.bounds.max.y as f64],
        );
        for candidate in tree.locate_in_envelope_intersecting(&envelope) {
            if candidate.index <= shape.index {
                continue;
            }
            let other = &connective_shapes[candidate.index];
            if shape.bounds.intersects(other.bounds)
                && layers_are_connected(&shape.layers, &other.layers, &connected_pairs)
            {
                union_find.union(shape.index, other.index);
            }
        }
    }

    let mut roots = BTreeMap::<usize, Vec<usize>>::new();
    for shape in &connective_shapes {
        let root = union_find.find(shape.index);
        roots.entry(root).or_default().push(shape.index);
    }

    let mut root_to_component = BTreeMap::new();
    let mut components = Vec::new();
    let mut shape_to_component = BTreeMap::new();
    for (root, indexes) in roots {
        let id = components.len() + 1;
        root_to_component.insert(root, id);
        let mut shapes = Vec::new();
        let mut bounds = connective_shapes[indexes[0]].bounds;
        let mut explicit_nets = BTreeSet::new();
        for index in indexes {
            let shape = &connective_shapes[index];
            shapes.push(shape.occurrence.clone());
            bounds = bounds.union(shape.bounds);
            if let Some(net) = shape.shape.net {
                explicit_nets.insert(net);
            }
            shape_to_component.insert(shape.occurrence.clone(), id);
        }
        components.push(NetComponent {
            id,
            shapes,
            bounds,
            labels: Vec::new(),
            explicit_nets: explicit_nets.into_iter().collect(),
            net_name: None,
            net_id: None,
        });
    }

    assign_labels(
        &mut components,
        &connective_shapes,
        &labels,
        &mut union_find,
        &root_to_component,
    );
    let shorts = summarize_component_names(&mut components);
    let opens = find_open_nets(&components);

    Ok(ConnectivityReport {
        components,
        shape_to_component,
        shorts,
        opens,
        skipped: None,
    })
}

fn connectivity_layers(
    document: &Document,
    technology: &TechnologyFile,
) -> Result<(ConnectivityLayerSet, ConnectivityLayerPairs), TechnologyError> {
    let mut conductive_layers = BTreeSet::new();
    let mut connected_pairs = BTreeSet::new();
    for connection in &technology.connectivity {
        let from = document_layer_id(document, &connection.from)?;
        let through = document_layer_id(document, &connection.through)?;
        let to = document_layer_id(document, &connection.to)?;
        conductive_layers.insert(from);
        conductive_layers.insert(through);
        conductive_layers.insert(to);
        insert_layer_pair(&mut connected_pairs, from, through);
        insert_layer_pair(&mut connected_pairs, through, to);
    }
    Ok((conductive_layers, connected_pairs))
}

fn materialize_shapes(
    document: &Document,
    conductive_layers: &BTreeSet<LayerId>,
) -> (Vec<ConnectiveShape>, Vec<LabelShape>) {
    let mut connective_shapes = Vec::new();
    let mut labels = Vec::new();
    for flattened in document.visible_flattened_shapes() {
        let shape = flattened.transformed_shape();
        match &shape.kind {
            ShapeKind::Label { position, text } => {
                let text = text.trim();
                if !text.is_empty() {
                    labels.push(LabelShape {
                        occurrence: flattened.id,
                        layer: shape.layer,
                        text: text.to_string(),
                        position: *position,
                    });
                }
            }
            ShapeKind::Measurement { .. } => {}
            ShapeKind::Via { lower, upper, .. } => {
                let mut layers = BTreeSet::new();
                layers.insert(shape.layer);
                layers.insert(*lower);
                layers.insert(*upper);
                connective_shapes.push(ConnectiveShape {
                    index: connective_shapes.len(),
                    occurrence: flattened.id,
                    bounds: shape.kind.bounds(),
                    shape,
                    layers,
                });
            }
            _ if conductive_layers.contains(&shape.layer) => {
                let mut layers = BTreeSet::new();
                layers.insert(shape.layer);
                connective_shapes.push(ConnectiveShape {
                    index: connective_shapes.len(),
                    occurrence: flattened.id,
                    bounds: shape.kind.bounds(),
                    shape,
                    layers,
                });
            }
            _ => {}
        }
    }
    (connective_shapes, labels)
}

fn assign_labels(
    components: &mut [NetComponent],
    connective_shapes: &[ConnectiveShape],
    labels: &[LabelShape],
    union_find: &mut UnionFind,
    root_to_component: &BTreeMap<usize, usize>,
) {
    for label in labels {
        let mut component_ids = BTreeSet::new();
        let preferred = connective_shapes
            .iter()
            .filter(|shape| {
                shape.layers.contains(&label.layer) && shape.bounds.contains_point(label.position)
            })
            .map(|shape| shape.index)
            .collect::<Vec<_>>();
        let candidates: Vec<_> = if preferred.is_empty() {
            connective_shapes
                .iter()
                .filter(|shape| shape.bounds.contains_point(label.position))
                .map(|shape| shape.index)
                .collect()
        } else {
            preferred
        };
        for index in candidates {
            let root = union_find.find(index);
            if let Some(component) = root_to_component.get(&root) {
                component_ids.insert(*component);
            }
        }
        for component_id in component_ids {
            if let Some(component) = components.get_mut(component_id - 1) {
                component.labels.push(NetLabel {
                    occurrence: label.occurrence.clone(),
                    text: label.text.clone(),
                    position: label.position,
                });
            }
        }
    }
}

fn summarize_component_names(components: &mut [NetComponent]) -> Vec<NetShort> {
    let mut shorts = Vec::new();
    for component in components {
        let label_names = component
            .labels
            .iter()
            .map(|label| normalize_net_name(&label.text))
            .filter(|name| !name.is_empty())
            .collect::<BTreeSet<_>>();
        let explicit_nets = component
            .explicit_nets
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();

        if label_names.len() == 1 {
            component.net_name = label_names.iter().next().cloned();
        } else if label_names.is_empty() && explicit_nets.len() == 1 {
            component.net_name = explicit_nets.iter().next().map(|net| net_label(*net));
        }
        if explicit_nets.len() == 1 {
            component.net_id = explicit_nets.iter().next().copied();
        }

        if label_names.len() > 1 || explicit_nets.len() > 1 {
            let mut names = label_names.into_iter().collect::<Vec<_>>();
            names.extend(explicit_nets.into_iter().map(net_label));
            names.sort();
            names.dedup();
            shorts.push(NetShort {
                component: component.id,
                names,
                bounds: component.bounds,
            });
        }
    }
    shorts
}

fn find_open_nets(components: &[NetComponent]) -> Vec<NetOpen> {
    let mut by_name = BTreeMap::<String, Vec<&NetComponent>>::new();
    for component in components {
        let names = component
            .labels
            .iter()
            .map(|label| normalize_net_name(&label.text))
            .filter(|name| !name.is_empty())
            .chain(component.explicit_nets.iter().copied().map(net_label))
            .collect::<BTreeSet<_>>();
        for name in names {
            by_name.entry(name).or_default().push(component);
        }
    }

    by_name
        .into_iter()
        .filter_map(|(name, components)| {
            if components.len() <= 1 {
                return None;
            }
            let mut bounds = components[0].bounds;
            let ids = components
                .iter()
                .map(|component| {
                    bounds = bounds.union(component.bounds);
                    component.id
                })
                .collect();
            Some(NetOpen {
                name,
                components: ids,
                bounds,
            })
        })
        .collect()
}

fn layers_are_connected(
    left: &BTreeSet<LayerId>,
    right: &BTreeSet<LayerId>,
    connected_pairs: &BTreeSet<(LayerId, LayerId)>,
) -> bool {
    for left in left {
        for right in right {
            if left == right || connected_pairs.contains(&ordered_pair(*left, *right)) {
                return true;
            }
        }
    }
    false
}

fn document_layer_id(document: &Document, reference: &str) -> Result<LayerId, TechnologyError> {
    let normalized = normalize_layer_ref(reference);
    document
        .layers
        .values()
        .find(|layer| normalize_layer_ref(&layer.name) == normalized)
        .map(|layer| layer.id)
        .ok_or_else(|| {
            TechnologyError::Invalid(format!(
                "technology connectivity references missing document layer {reference:?}"
            ))
        })
}

fn insert_layer_pair(pairs: &mut BTreeSet<(LayerId, LayerId)>, left: LayerId, right: LayerId) {
    pairs.insert(ordered_pair(left, right));
}

fn ordered_pair(left: LayerId, right: LayerId) -> (LayerId, LayerId) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn normalize_layer_ref(reference: &str) -> String {
    reference
        .trim()
        .chars()
        .filter(|character| *character != '_' && *character != '-' && !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_net_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

fn net_label(net: NetId) -> String {
    format!("NET{}", net.0)
}

fn validate_component_names(
    component: &NetComponent,
    findings: &mut Vec<ConnectivityValidationFinding>,
) {
    let mut explicit_nets = BTreeSet::new();
    for net in &component.explicit_nets {
        if !explicit_nets.insert(*net) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} repeats explicit net {:?}",
                component.id, net
            )));
        }
    }

    if let Some(net_id) = component.net_id
        && !explicit_nets.contains(&net_id)
    {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity component {} net_id {:?} is not present in explicit_nets",
            component.id, net_id
        )));
    }

    let mut derived_names = BTreeSet::new();
    for label in &component.labels {
        let normalized = normalize_net_name(&label.text);
        if normalized.is_empty() {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} has an empty net label",
                component.id
            )));
        } else {
            derived_names.insert(normalized);
        }
    }
    derived_names.extend(explicit_nets.into_iter().map(net_label));

    if let Some(net_name) = &component.net_name {
        let normalized = normalize_net_name(net_name);
        if normalized.is_empty() {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} has an empty net_name",
                component.id
            )));
        } else if derived_names.is_empty() {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} net_name {:?} has no label or explicit net source",
                component.id, net_name
            )));
        } else if !derived_names.contains(&normalized) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} net_name {:?} is not derived from labels or explicit nets",
                component.id, net_name
            )));
        }
    }
}

fn validate_short(
    short: &NetShort,
    component_ids: &BTreeSet<usize>,
    issue_keys: &mut BTreeSet<String>,
    findings: &mut Vec<ConnectivityValidationFinding>,
) {
    if !component_ids.contains(&short.component) {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity short {:?} references missing component {}",
            short.names, short.component
        )));
    }

    let mut normalized_names = BTreeSet::new();
    for name in &short.names {
        let normalized = normalize_net_name(name);
        if normalized.is_empty() {
            findings.push(ConnectivityValidationFinding::error(
                "connectivity short has an empty net name",
            ));
        } else if normalized != *name {
            findings.push(ConnectivityValidationFinding::warning(format!(
                "connectivity short net name {name:?} is not normalized"
            )));
        }
        if !normalized.is_empty() && !normalized_names.insert(normalized) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity short repeats net name {name:?}"
            )));
        }
    }
    if normalized_names.len() < 2 {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity short on component {} has fewer than two distinct net names",
            short.component
        )));
    }

    validate_issue_key("short", short.stable_key(), issue_keys, findings);
}

fn validate_open(
    open: &NetOpen,
    component_ids: &BTreeSet<usize>,
    issue_keys: &mut BTreeSet<String>,
    findings: &mut Vec<ConnectivityValidationFinding>,
) {
    let normalized_name = normalize_net_name(&open.name);
    if normalized_name.is_empty() {
        findings.push(ConnectivityValidationFinding::error(
            "connectivity open has an empty net name",
        ));
    } else if normalized_name != open.name {
        findings.push(ConnectivityValidationFinding::warning(format!(
            "connectivity open net name {:?} is not normalized",
            open.name
        )));
    }

    let mut open_components = BTreeSet::new();
    for component_id in &open.components {
        if !component_ids.contains(component_id) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity open {:?} references missing component {}",
                open.name, component_id
            )));
        }
        if !open_components.insert(*component_id) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity open {:?} repeats component {}",
                open.name, component_id
            )));
        }
    }
    if open_components.len() < 2 {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity open {:?} has fewer than two distinct components",
            open.name
        )));
    }

    validate_issue_key("open", open.stable_key(), issue_keys, findings);
}

fn validate_issue_key(
    kind: &str,
    key: String,
    issue_keys: &mut BTreeSet<String>,
    findings: &mut Vec<ConnectivityValidationFinding>,
) {
    if key.trim().is_empty() {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity {kind} produced an empty stable issue key"
        )));
    } else if !issue_keys.insert(key.clone()) {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity stable issue key {key:?} is duplicated"
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MarkerState, ProcessLayer, ShapeId, ShapeKind, default_technology};
    use geometry_core::{Coord, Point, Rect};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct ConnectivityGoldenFixture {
        name: String,
        shapes: Vec<ConnectivityFixtureShape>,
        expect: ConnectivityGoldenExpectation,
        #[serde(default)]
        issue_states: Vec<ConnectivityIssueStateFixture>,
    }

    #[derive(Debug, Deserialize)]
    struct ConnectivityGoldenExpectation {
        health: String,
        component_count: usize,
        labeled_component_count: usize,
        short_count: usize,
        short_names: Vec<String>,
        short_key: String,
        open_count: usize,
        open_name: String,
        open_key: String,
        open_component_count: usize,
        data_component_shape_count: usize,
    }

    #[derive(Debug, Deserialize)]
    struct ConnectivityIssueStateFixture {
        key: String,
        hidden: bool,
        waived: bool,
        note: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum ConnectivityFixtureShape {
        Rect {
            layer: String,
            x: Coord,
            y: Coord,
            w: Coord,
            h: Coord,
        },
        Label {
            layer: String,
            x: Coord,
            y: Coord,
            text: String,
        },
    }

    fn document_from_connectivity_fixture(fixture: &ConnectivityGoldenFixture) -> Document {
        let mut document = Document::new(&fixture.name);
        for shape in &fixture.shapes {
            match shape {
                ConnectivityFixtureShape::Rect { layer, x, y, w, h } => {
                    let layer_id = fixture_layer(&document, layer);
                    document.insert_shape(
                        layer_id,
                        ShapeKind::Rectangle(Rect::from_min_size(Point::new(*x, *y), *w, *h)),
                    );
                }
                ConnectivityFixtureShape::Label { layer, x, y, text } => {
                    let layer_id = fixture_layer(&document, layer);
                    document.insert_shape(
                        layer_id,
                        ShapeKind::Label {
                            position: Point::new(*x, *y),
                            text: text.clone(),
                        },
                    );
                }
            }
        }
        document
    }

    fn fixture_layer(document: &Document, layer: &str) -> LayerId {
        let process = ProcessLayer::from_technology_name(layer)
            .unwrap_or_else(|| panic!("unknown fixture layer {layer}"));
        document
            .layer_by_process(process)
            .unwrap_or_else(|| panic!("fixture layer {layer} missing from document"))
    }

    #[test]
    fn labels_propagate_across_via_stack() {
        let technology = default_technology();
        let mut doc = Document::new("connectivity");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let via1 = doc.layer_by_process(ProcessLayer::Via1).unwrap();
        let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
        let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();

        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 500, 500)),
        );
        doc.insert_shape(
            via1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 200), 100, 100)),
        );
        doc.insert_shape(
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(150, 150), 500, 500)),
        );
        doc.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(250, 250),
                text: "out".to_string(),
            },
        );

        let report = extract_connectivity(&doc, &technology).unwrap();

        assert_eq!(report.components.len(), 1);
        assert_eq!(report.components[0].shapes.len(), 3);
        assert_eq!(report.components[0].net_name.as_deref(), Some("OUT"));
        assert!(report.shorts.is_empty());
        assert!(report.opens.is_empty());
    }

    #[test]
    fn conflicting_labels_on_one_component_report_short() {
        let technology = default_technology();
        let mut doc = Document::new("short");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 800, 300)),
        );
        for (x, text) in [(100, "A"), (700, "B")] {
            doc.insert_shape(
                annotation,
                ShapeKind::Label {
                    position: Point::new(x, 100),
                    text: text.to_string(),
                },
            );
        }

        let report = extract_connectivity(&doc, &technology).unwrap();

        assert_eq!(report.shorts.len(), 1);
        assert_eq!(
            report.shorts[0].names,
            vec!["A".to_string(), "B".to_string()]
        );
    }

    #[test]
    fn repeated_label_on_disconnected_components_reports_open() {
        let technology = default_technology();
        let mut doc = Document::new("open");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
        for (x, label_x) in [(0, 100), (1_000, 1_100)] {
            doc.insert_shape(
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(x, 0), 300, 300)),
            );
            doc.insert_shape(
                annotation,
                ShapeKind::Label {
                    position: Point::new(label_x, 100),
                    text: "clk".to_string(),
                },
            );
        }

        let report = extract_connectivity(&doc, &technology).unwrap();

        assert_eq!(report.opens.len(), 1);
        assert_eq!(report.opens[0].name, "CLK");
        assert_eq!(report.opens[0].components.len(), 2);
    }

    #[test]
    fn connectivity_golden_fixture_reports_expected_shorts_and_opens() {
        let fixture: ConnectivityGoldenFixture = serde_json::from_str(include_str!(
            "../../../fixtures/quality/connectivity_golden.json"
        ))
        .unwrap();
        let technology = default_technology();
        let doc = document_from_connectivity_fixture(&fixture);

        let report = extract_connectivity(&doc, &technology).unwrap();
        let summary = report.summary(8);

        assert_eq!(report.validate(), Vec::new());
        assert_eq!(report.components.len(), fixture.expect.component_count);
        assert_eq!(summary.component_count, fixture.expect.component_count);
        assert_eq!(summary.health.label(), fixture.expect.health);
        assert_eq!(
            summary.labeled_component_count,
            fixture.expect.labeled_component_count
        );
        assert_eq!(report.shorts.len(), fixture.expect.short_count);
        assert_eq!(report.shorts[0].names, fixture.expect.short_names);
        assert_eq!(report.shorts[0].stable_key(), fixture.expect.short_key);
        assert_eq!(report.opens.len(), fixture.expect.open_count);
        assert_eq!(report.opens[0].name, fixture.expect.open_name);
        assert_eq!(report.opens[0].stable_key(), fixture.expect.open_key);
        assert_eq!(
            report.opens[0].components.len(),
            fixture.expect.open_component_count
        );
        assert!(
            report
                .components
                .iter()
                .any(|component| component.net_name.as_deref() == Some("DATA")
                    && component.shapes.len() == fixture.expect.data_component_shape_count)
        );
    }

    #[test]
    fn connectivity_report_validation_rejects_stale_component_and_issue_metadata() {
        let first = ShapeOccurrenceId::top_level(ShapeId(1));
        let second = ShapeOccurrenceId::top_level(ShapeId(2));
        let stale = ShapeOccurrenceId::top_level(ShapeId(99));
        let bounds = Rect::from_min_size(Point::new(0, 0), 100, 100);
        let mut shape_to_component = BTreeMap::new();
        shape_to_component.insert(first.clone(), 1);
        shape_to_component.insert(second.clone(), 1);
        shape_to_component.insert(stale, 9);
        let report = ConnectivityReport {
            components: vec![
                NetComponent {
                    id: 1,
                    shapes: vec![first.clone(), second.clone()],
                    bounds,
                    labels: Vec::new(),
                    explicit_nets: Vec::new(),
                    net_name: Some("DATA".to_string()),
                    net_id: Some(NetId(7)),
                },
                NetComponent {
                    id: 3,
                    shapes: vec![second],
                    bounds,
                    labels: Vec::new(),
                    explicit_nets: Vec::new(),
                    net_name: None,
                    net_id: None,
                },
            ],
            shape_to_component,
            shorts: vec![
                NetShort {
                    component: 42,
                    names: vec!["VDD".to_string(), "VSS".to_string()],
                    bounds,
                },
                NetShort {
                    component: 1,
                    names: vec!["VSS".to_string(), "VDD".to_string()],
                    bounds,
                },
                NetShort {
                    component: 1,
                    names: vec!["CLK".to_string(), "clk".to_string()],
                    bounds: Rect::from_min_size(Point::new(200, 0), 100, 100),
                },
            ],
            opens: vec![
                NetOpen {
                    name: "CLK".to_string(),
                    components: vec![1, 9, 1],
                    bounds,
                },
                NetOpen {
                    name: " ".to_string(),
                    components: vec![1, 3],
                    bounds: Rect::from_min_size(Point::new(400, 0), 100, 100),
                },
            ],
            skipped: Some(" ".to_string()),
        };

        let findings = report.validate();
        let messages = findings
            .iter()
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            findings
                .iter()
                .any(|finding| finding.severity == ConnectivityValidationSeverity::Error)
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("empty skipped reason"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("expected 2"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("appears in components 1 and 3"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("maps to missing component 9"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("net_id NetId(7) is not present"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("net_name \"DATA\" has no label"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("references missing component 42"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("stable issue key")
                    && message.contains("duplicated"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("fewer than two distinct net names"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("repeats component 1"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("open has an empty net name"))
        );
    }

    #[test]
    fn connectivity_golden_fixture_carries_issue_state_keys() {
        let fixture: ConnectivityGoldenFixture = serde_json::from_str(include_str!(
            "../../../fixtures/quality/connectivity_golden.json"
        ))
        .unwrap();
        let technology = default_technology();
        let doc = document_from_connectivity_fixture(&fixture);
        let report = extract_connectivity(&doc, &technology).unwrap();
        let store = report.issue_store();
        let states = fixture
            .issue_states
            .iter()
            .map(|state| {
                (
                    state.key.clone(),
                    MarkerState {
                        hidden: state.hidden,
                        waived: state.waived,
                        note: state.note.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        assert_eq!(states.len(), fixture.issue_states.len());
        for expected in &fixture.issue_states {
            let record = store
                .get(&expected.key)
                .unwrap_or_else(|| panic!("missing fixture issue key {}", expected.key));
            let state = states.get(&expected.key).unwrap();
            assert_eq!(state.hidden, expected.hidden);
            assert_eq!(state.waived, expected.waived);
            assert_eq!(state.note, expected.note);
            assert!(matches!(
                record.kind(),
                ConnectivityIssueKind::Short | ConnectivityIssueKind::Open
            ));
        }
    }

    #[test]
    fn connectivity_summary_captures_health_and_caps_issue_counts() {
        let technology = default_technology();
        let mut doc = Document::new("summary");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
        for (x, text) in [(0, "A"), (500, "B"), (1_000, "CLK"), (1_500, "CLK")] {
            doc.insert_shape(
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(x, 0), 300, 300)),
            );
            doc.insert_shape(
                annotation,
                ShapeKind::Label {
                    position: Point::new(x + 100, 100),
                    text: text.to_string(),
                },
            );
        }
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(2_000, 0), 500, 300)),
        );
        for (x, text) in [(2_050, "VDD"), (2_350, "VSS")] {
            doc.insert_shape(
                annotation,
                ShapeKind::Label {
                    position: Point::new(x, 100),
                    text: text.to_string(),
                },
            );
        }

        let report = extract_connectivity(&doc, &technology).unwrap();
        let summary = report.summary(1);

        assert_eq!(summary.health, ConnectivityHealth::ShortsAndOpens);
        assert_eq!(summary.short_count, 1);
        assert_eq!(summary.open_count, 1);
        assert_eq!(summary.displayed_short_count, 1);
        assert_eq!(summary.displayed_open_count, 1);
        assert_eq!(summary.omitted_short_count, 0);
        assert_eq!(summary.omitted_open_count, 0);
        assert_eq!(summary.largest_component_shape_count, 1);
        assert!(summary.labeled_component_count >= 4);
    }

    #[test]
    fn connectivity_summary_reports_skipped_and_omitted_rows() {
        let mut report = ConnectivityReport::skipped("too many shapes");
        report.shorts = vec![
            NetShort {
                component: 1,
                names: vec!["A".to_string(), "B".to_string()],
                bounds: Rect::from_min_size(Point::new(0, 0), 1, 1),
            },
            NetShort {
                component: 2,
                names: vec!["C".to_string(), "D".to_string()],
                bounds: Rect::from_min_size(Point::new(2, 0), 1, 1),
            },
        ];
        report.opens = vec![NetOpen {
            name: "CLK".to_string(),
            components: vec![3, 4],
            bounds: Rect::from_min_size(Point::new(4, 0), 1, 1),
        }];

        let summary = report.summary(1);

        assert_eq!(summary.health, ConnectivityHealth::Skipped);
        assert_eq!(summary.displayed_short_count, 1);
        assert_eq!(summary.omitted_short_count, 1);
        assert_eq!(summary.displayed_open_count, 1);
        assert_eq!(summary.omitted_open_count, 0);
        assert_eq!(summary.skipped.as_deref(), Some("too many shapes"));
    }

    #[test]
    fn connectivity_issue_keys_ignore_generated_component_ids_and_ordering() {
        let first_short = NetShort {
            component: 7,
            names: vec!["VSS".to_string(), "VDD".to_string()],
            bounds: Rect::from_min_size(Point::new(0, 0), 100, 100),
        };
        let second_short = NetShort {
            component: 99,
            names: vec!["VDD".to_string(), "VSS".to_string()],
            bounds: first_short.bounds,
        };
        assert_eq!(first_short.stable_key(), second_short.stable_key());

        let first_open = NetOpen {
            name: "CLK".to_string(),
            components: vec![9, 3, 5],
            bounds: Rect::from_min_size(Point::new(200, 0), 100, 100),
        };
        let second_open = NetOpen {
            name: "CLK".to_string(),
            components: vec![50, 90, 30],
            bounds: first_open.bounds,
        };
        assert_eq!(first_open.stable_key(), second_open.stable_key());
    }

    #[test]
    fn connectivity_issue_keys_survive_recomputed_component_numbering() {
        fn fixture(with_unrelated_component_first: bool) -> ConnectivityReport {
            let technology = default_technology();
            let mut doc = Document::new("connectivity key fixture");
            let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
            let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();

            if with_unrelated_component_first {
                doc.insert_shape(
                    metal1,
                    ShapeKind::Rectangle(Rect::from_min_size(Point::new(-2_000, 0), 100, 100)),
                );
            }

            doc.insert_shape(
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 500, 300)),
            );
            for (x, text) in [(50, "VDD"), (350, "VSS")] {
                doc.insert_shape(
                    annotation,
                    ShapeKind::Label {
                        position: Point::new(x, 100),
                        text: text.to_string(),
                    },
                );
            }

            for (x, label_x) in [(1_000, 1_100), (2_000, 2_100)] {
                doc.insert_shape(
                    metal1,
                    ShapeKind::Rectangle(Rect::from_min_size(Point::new(x, 0), 300, 300)),
                );
                doc.insert_shape(
                    annotation,
                    ShapeKind::Label {
                        position: Point::new(label_x, 100),
                        text: "CLK".to_string(),
                    },
                );
            }

            extract_connectivity(&doc, &technology).unwrap()
        }

        let baseline = fixture(false);
        let shifted = fixture(true);
        assert_eq!(baseline.shorts.len(), 1);
        assert_eq!(baseline.opens.len(), 1);
        assert_eq!(shifted.shorts.len(), 1);
        assert_eq!(shifted.opens.len(), 1);
        assert_ne!(baseline.shorts[0].component, shifted.shorts[0].component);
        assert_ne!(baseline.opens[0].components, shifted.opens[0].components);

        assert_eq!(
            baseline.shorts[0].stable_key(),
            shifted.shorts[0].stable_key()
        );
        assert_eq!(
            baseline.opens[0].stable_key(),
            shifted.opens[0].stable_key()
        );
        assert!(
            shifted
                .issue_store()
                .get(&baseline.shorts[0].stable_key())
                .is_some()
        );
        assert!(
            shifted
                .issue_store()
                .get(&baseline.opens[0].stable_key())
                .is_some()
        );
    }

    #[test]
    fn connectivity_issue_store_queries_and_caps_mixed_issues() {
        let short = NetShort {
            component: 1,
            names: vec!["A".to_string(), "B".to_string()],
            bounds: Rect::from_min_size(Point::new(0, 0), 1, 1),
        };
        let open = NetOpen {
            name: "CLK".to_string(),
            components: vec![3, 4],
            bounds: Rect::from_min_size(Point::new(4, 0), 1, 1),
        };
        let open_key = open.stable_key();
        let report = ConnectivityReport {
            shorts: vec![short],
            opens: vec![open],
            ..Default::default()
        };

        let store = report.issue_store();
        let summary = store.summary(1);

        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.displayed_count, 1);
        assert_eq!(summary.omitted_count, 1);
        assert_eq!(summary.short_count, 1);
        assert_eq!(summary.open_count, 1);
        assert_eq!(store.displayed_records(1).len(), 1);
        assert_eq!(
            store.get(&open_key).map(ConnectivityIssueRecord::kind),
            Some(ConnectivityIssueKind::Open)
        );
        assert_eq!(
            store.get(&open_key).map(|record| record.kind().label()),
            Some("open")
        );
    }

    #[test]
    fn explicit_net_ids_propagate_to_components() {
        let technology = default_technology();
        let mut doc = Document::new("explicit nets");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let id = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 300, 300)),
        );
        doc.shapes.get_mut(&id).unwrap().net = Some(NetId(42));

        let report = extract_connectivity(&doc, &technology).unwrap();

        assert_eq!(report.components[0].net_id, Some(NetId(42)));
        assert_eq!(report.components[0].net_name.as_deref(), Some("NET42"));
    }
}
