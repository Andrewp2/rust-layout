#![allow(unused_imports)]
use super::*;

pub(crate) fn check_edge_spacing(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    if rules.min_edge_spacing.is_empty() {
        return;
    }
    let edges = drc_shapes(document)
        .iter()
        .flat_map(shape_boundary_edges)
        .collect::<Vec<_>>();
    for left_index in 0..edges.len() {
        let left = &edges[left_index];
        let Some(required) = rules.min_edge_spacing.get(&left.layer).copied() else {
            continue;
        };
        if required <= 0 {
            continue;
        }
        for right in &edges[left_index + 1..] {
            if left.layer != right.layer
                || left.axis != right.axis
                || left.occurrence == right.occurrence
            {
                continue;
            }
            let distance = (left.fixed - right.fixed).abs();
            if distance == 0 || distance >= required {
                continue;
            }
            let overlap_start = left.start.max(right.start);
            let overlap_end = left.end.min(right.end);
            if overlap_end <= overlap_start {
                continue;
            }
            violations.push(DrcViolation {
                id: 0,
                rule: "min_edge_spacing".to_string(),
                message: format!(
                    "parallel edge spacing is {distance} dbu, below required {required} dbu"
                ),
                shape_ids: vec![left.shape_id, right.shape_id],
                occurrence_ids: vec![left.occurrence.clone(), right.occurrence.clone()],
                bounds: edge_pair_marker_bounds(left, right, overlap_start, overlap_end, required),
                required,
                actual: distance as f64,
            });
        }
    }
}

pub(crate) fn shape_boundary_edges(shape: &DrcShape) -> Vec<DrcBoundaryEdge> {
    match &shape.shape.kind {
        ShapeKind::Rectangle(rect) => rect_boundary_edges(shape, *rect),
        ShapeKind::Polygon(poly) => poly
            .points
            .iter()
            .copied()
            .zip(poly.points.iter().copied().cycle().skip(1))
            .take(poly.points.len())
            .filter_map(|(a, b)| boundary_edge_from_points(shape, a, b))
            .collect(),
        ShapeKind::Via { .. } => rect_boundary_edges(shape, shape.bounds()),
        ShapeKind::Path { .. } | ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {
            Vec::new()
        }
    }
}

pub(crate) fn rect_boundary_edges(shape: &DrcShape, rect: Rect) -> Vec<DrcBoundaryEdge> {
    let corners = rect.corners();
    [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ]
    .into_iter()
    .filter_map(|(a, b)| boundary_edge_from_points(shape, a, b))
    .collect()
}

pub(crate) fn boundary_edge_from_points(
    shape: &DrcShape,
    a: Point,
    b: Point,
) -> Option<DrcBoundaryEdge> {
    if a == b {
        return None;
    }
    let (axis, fixed, start, end) = if a.y == b.y {
        (EdgeAxis::Horizontal, a.y, a.x.min(b.x), a.x.max(b.x))
    } else if a.x == b.x {
        (EdgeAxis::Vertical, a.x, a.y.min(b.y), a.y.max(b.y))
    } else {
        return None;
    };
    (end > start).then(|| DrcBoundaryEdge {
        layer: shape.layer(),
        shape_id: shape.source_shape_id(),
        occurrence: shape.occurrence.clone(),
        axis,
        fixed,
        start,
        end,
    })
}

pub(crate) fn edge_pair_marker_bounds(
    left: &DrcBoundaryEdge,
    right: &DrcBoundaryEdge,
    overlap_start: Coord,
    overlap_end: Coord,
    required: Coord,
) -> Rect {
    let raw = match left.axis {
        EdgeAxis::Horizontal => Rect::new(
            Point::new(overlap_start, left.fixed),
            Point::new(overlap_end, right.fixed),
        ),
        EdgeAxis::Vertical => Rect::new(
            Point::new(left.fixed, overlap_start),
            Point::new(right.fixed, overlap_end),
        ),
    };
    raw.expanded(required.max(1))
}

pub(crate) fn check_spacing(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    let shapes = drc_shapes(document);
    for left_index in 0..shapes.len() {
        let left = &shapes[left_index];
        let Some(required) = rules.min_spacing.get(&left.layer()).copied() else {
            continue;
        };
        if required <= 0 {
            continue;
        }
        let left_bounds = left.bounds();
        for right in &shapes[left_index + 1..] {
            if left.layer() != right.layer() {
                continue;
            }
            let right_bounds = right.bounds();
            if left_bounds.intersects(right_bounds) {
                continue;
            }
            let actual = left_bounds.distance_to_rect(right_bounds);
            if actual < required as f64 {
                violations.push(DrcViolation {
                    id: 0,
                    rule: "min_spacing".to_string(),
                    message: format!(
                        "same-layer spacing is {actual:.1} dbu, below required {required} dbu"
                    ),
                    shape_ids: vec![left.source_shape_id(), right.source_shape_id()],
                    occurrence_ids: vec![left.occurrence.clone(), right.occurrence.clone()],
                    bounds: left_bounds.union(right_bounds).expanded(required),
                    required,
                    actual,
                });
            }
        }
    }
}

pub(crate) fn check_derived_forbidden_overlaps(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    if rules.derived_forbidden_overlaps.is_empty() {
        return;
    }
    let physical_shapes = drc_shapes(document);
    for rule in &rules.derived_forbidden_overlaps {
        for derived in derived_drc_shapes(document, rules, &rule.derived) {
            for physical in physical_shapes
                .iter()
                .filter(|shape| shape.layer() == rule.layer)
            {
                if derived
                    .occurrence_ids
                    .iter()
                    .any(|occurrence| occurrence == &physical.occurrence)
                {
                    continue;
                }
                let Some(overlap) = derived.bounds.intersection(physical.bounds()) else {
                    continue;
                };
                if overlap.area() <= 0 {
                    continue;
                }
                let (shape_ids, occurrence_ids) =
                    merged_derived_physical_shape_refs(&derived, physical);
                violations.push(DrcViolation {
                    id: 0,
                    rule: rule.name.clone(),
                    message: format!(
                        "derived layer {:?} overlaps forbidden layer {:?}",
                        rule.derived, rule.layer
                    ),
                    shape_ids,
                    occurrence_ids,
                    bounds: overlap.expanded(rules.grid.max(1)),
                    required: 0,
                    actual: overlap.area() as f64,
                });
            }
        }
    }
}

pub(crate) fn merged_derived_physical_shape_refs(
    derived: &DerivedDrcShape,
    physical: &DrcShape,
) -> (Vec<ShapeId>, Vec<ShapeOccurrenceId>) {
    let mut seen = BTreeSet::new();
    let mut shape_ids = Vec::new();
    let mut occurrence_ids = Vec::new();
    for (shape_id, occurrence) in derived
        .shape_ids
        .iter()
        .copied()
        .zip(derived.occurrence_ids.iter().cloned())
        .chain(std::iter::once((
            physical.source_shape_id(),
            physical.occurrence.clone(),
        )))
    {
        if seen.insert(occurrence_stable_key(&occurrence)) {
            shape_ids.push(shape_id);
            occurrence_ids.push(occurrence);
        }
    }
    (shape_ids, occurrence_ids)
}

pub(crate) fn check_forbidden_overlaps(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    let shapes = drc_shapes(document);
    for rule in &rules.forbidden_overlaps {
        for left_index in 0..shapes.len() {
            let left = &shapes[left_index];
            if left.layer() != rule.a && left.layer() != rule.b {
                continue;
            }
            for right in &shapes[left_index + 1..] {
                let matching_layers = (left.layer() == rule.a && right.layer() == rule.b)
                    || (left.layer() == rule.b && right.layer() == rule.a);
                if !matching_layers {
                    continue;
                }
                let left_bounds = left.bounds();
                let right_bounds = right.bounds();
                if let Some(overlap) = left_bounds.intersection(right_bounds) {
                    violations.push(DrcViolation {
                        id: 0,
                        rule: rule.name.clone(),
                        message: "forbidden process layers overlap".to_string(),
                        shape_ids: vec![left.source_shape_id(), right.source_shape_id()],
                        occurrence_ids: vec![left.occurrence.clone(), right.occurrence.clone()],
                        bounds: overlap.expanded(80),
                        required: 0,
                        actual: overlap.area() as f64,
                    });
                }
            }
        }
    }
}

pub(crate) fn check_via_enclosure(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    let shapes = drc_shapes(document);
    for rule in &rules.via_enclosure {
        for via in shapes
            .iter()
            .filter(|shape| shape.layer() == rule.via_layer)
        {
            let via_bounds = via.bounds();
            let required_rect = via_bounds.expanded(rule.required);
            let enclosed = shapes
                .iter()
                .filter(|shape| shape.layer() == rule.enclosure_layer)
                .any(|shape| shape.bounds().contains_rect(required_rect));
            if !enclosed {
                violations.push(DrcViolation {
                    id: 0,
                    rule: "via_enclosure".to_string(),
                    message: format!(
                        "via requires {} dbu enclosure on layer {:?}",
                        rule.required, rule.enclosure_layer
                    ),
                    shape_ids: vec![via.source_shape_id()],
                    occurrence_ids: vec![via.occurrence.clone()],
                    bounds: required_rect.expanded(rule.required),
                    required: rule.required,
                    actual: 0.0,
                });
            }
        }
    }
}

pub(crate) fn approximate_width(shape: &Shape) -> f64 {
    match &shape.kind {
        ShapeKind::Rectangle(rect) => rect.width().min(rect.height()) as f64,
        ShapeKind::Polygon(poly) => {
            let bbox_width = poly
                .bounds()
                .map(|rect| rect.width().min(rect.height()) as f64)
                .unwrap_or(0.0);
            poly.min_edge_length().unwrap_or(bbox_width).min(bbox_width)
        }
        ShapeKind::Path { width, .. } => *width as f64,
        ShapeKind::Via { size, .. } => *size as f64,
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => 0.0,
    }
}

pub(crate) fn approximate_max_width(shape: &Shape) -> f64 {
    match &shape.kind {
        ShapeKind::Rectangle(rect) => rect.width().max(rect.height()) as f64,
        ShapeKind::Polygon(poly) => poly
            .bounds()
            .map(|rect| rect.width().max(rect.height()) as f64)
            .unwrap_or(0.0),
        ShapeKind::Path { width, .. } => *width as f64,
        ShapeKind::Via { size, .. } => *size as f64,
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => 0.0,
    }
}

pub(crate) fn approximate_area(shape: &Shape) -> f64 {
    match &shape.kind {
        ShapeKind::Rectangle(rect) => rect.area() as f64,
        ShapeKind::Polygon(poly) => poly.area(),
        ShapeKind::Path { points, width } => {
            if *width <= 0 || points.len() < 2 {
                return 0.0;
            }
            points
                .windows(2)
                .map(|pair| pair[0].distance_to(pair[1]))
                .sum::<f64>()
                * *width as f64
        }
        ShapeKind::Via { size, .. } => (*size as f64) * (*size as f64),
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => 0.0,
    }
}

pub(crate) fn validate_drc_record(
    record: &DrcIssueRecord,
    index: usize,
    seen_keys: &mut BTreeSet<String>,
    findings: &mut Vec<DrcValidationFinding>,
) {
    let expected_key = record.violation.stable_key();
    if record.key.trim().is_empty() {
        findings.push(DrcValidationFinding::error(format!(
            "DRC record at index {index} has an empty stable issue key"
        )));
    } else if !seen_keys.insert(record.key.clone()) {
        findings.push(DrcValidationFinding::error(format!(
            "DRC stable issue key {:?} is duplicated",
            record.key
        )));
    }
    if record.key != expected_key {
        findings.push(DrcValidationFinding::error(format!(
            "DRC record key {:?} does not match violation stable key {:?}",
            record.key, expected_key
        )));
    }

    validate_violation(&record.violation, index, findings);
}

pub(crate) fn validate_violation(
    violation: &DrcViolation,
    index: usize,
    findings: &mut Vec<DrcValidationFinding>,
) {
    let expected_id = index + 1;
    if violation.id == 0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation at index {index} has id 0"
        )));
    } else if violation.id != expected_id {
        findings.push(DrcValidationFinding::warning(format!(
            "DRC violation at index {index} has id {}, expected {}",
            violation.id, expected_id
        )));
    }
    if violation.rule.trim().is_empty() {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has an empty rule",
            violation.id
        )));
    }
    if violation.message.trim().is_empty() {
        findings.push(DrcValidationFinding::warning(format!(
            "DRC violation {} has an empty message",
            violation.id
        )));
    }
    if violation.shape_ids.is_empty() && !drc_violation_allows_external_geometry(violation) {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has no shape references",
            violation.id
        )));
    }
    let mut seen_shapes = BTreeSet::new();
    for shape_id in &violation.shape_ids {
        if shape_id.0 == 0 {
            findings.push(DrcValidationFinding::error(format!(
                "DRC violation {} references invalid shape id 0",
                violation.id
            )));
        }
        if violation.occurrence_ids.is_empty() && !seen_shapes.insert(*shape_id) {
            findings.push(DrcValidationFinding::error(format!(
                "DRC violation {} repeats shape id {:?}",
                violation.id, shape_id
            )));
        }
    }
    if !violation.occurrence_ids.is_empty() {
        if violation.occurrence_ids.len() != violation.shape_ids.len() {
            findings.push(DrcValidationFinding::error(format!(
                "DRC violation {} has {} occurrence references for {} shape references",
                violation.id,
                violation.occurrence_ids.len(),
                violation.shape_ids.len()
            )));
        }
        let mut seen_occurrences = BTreeSet::new();
        for occurrence in &violation.occurrence_ids {
            if occurrence.shape.0 == 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC violation {} references invalid occurrence shape id 0",
                    violation.id
                )));
            }
            let key = occurrence_stable_key(occurrence);
            if !seen_occurrences.insert(key) {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC violation {} repeats occurrence {:?}",
                    violation.id, occurrence
                )));
            }
        }
    }
    if violation.bounds.width() < 0 || violation.bounds.height() < 0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has inverted bounds {:?}",
            violation.id, violation.bounds
        )));
    }
    if violation.required < 0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has a negative required value {}",
            violation.id, violation.required
        )));
    }
    if !violation.actual.is_finite() {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has a non-finite actual value {}",
            violation.id, violation.actual
        )));
    } else if violation.actual < 0.0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has a negative actual value {}",
            violation.id, violation.actual
        )));
    }
}

pub(crate) fn drc_violation_allows_external_geometry(violation: &DrcViolation) -> bool {
    violation.rule.starts_with("calibre.")
        || violation.rule.starts_with("klayout.")
        || violation.rule.starts_with("external.")
}

pub fn violation_marker(point: Point, size: Coord) -> Rect {
    Rect::new(
        Point::new(point.x - size, point.y - size),
        Point::new(point.x + size, point.y + size),
    )
}
