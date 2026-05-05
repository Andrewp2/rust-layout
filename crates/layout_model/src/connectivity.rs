use std::collections::{BTreeMap, BTreeSet};

use geometry_core::{Point, Rect};
use rstar::{AABB, RTree, RTreeObject};

use crate::{
    Document, LayerId, NetId, Shape, ShapeKind, ShapeOccurrenceId, TechnologyError, TechnologyFile,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectivityReport {
    pub components: Vec<NetComponent>,
    pub shape_to_component: BTreeMap<ShapeOccurrenceId, usize>,
    pub shorts: Vec<NetShort>,
    pub opens: Vec<NetOpen>,
    pub skipped: Option<String>,
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

#[derive(Clone, Debug, PartialEq)]
pub struct NetOpen {
    pub name: String,
    pub components: Vec<usize>,
    pub bounds: Rect,
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
) -> Result<(BTreeSet<LayerId>, BTreeSet<(LayerId, LayerId)>), TechnologyError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProcessLayer, ShapeKind, default_technology};
    use geometry_core::{Point, Rect};

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
