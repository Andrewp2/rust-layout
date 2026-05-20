use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use geometry_core::{Coord, Point, Polygon, Rect, Vector};

use crate::{CellId, Document, LayerId, ProcessLayer, Shape, ShapeKind, Transform};

#[derive(Debug)]
pub enum CifError {
    InvalidCommand(String),
    InvalidNumber { token: String, command: String },
}

impl fmt::Display for CifError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand(message) => write!(formatter, "invalid CIF command: {message}"),
            Self::InvalidNumber { token, command } => {
                write!(formatter, "invalid CIF number {token:?} in {command:?}")
            }
        }
    }
}

impl Error for CifError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CifExportReport {
    pub layer_count: usize,
    pub shape_count: usize,
    pub box_count: usize,
    pub polygon_count: usize,
    pub wire_count: usize,
    pub label_count: usize,
    pub warnings: Vec<String>,
    pub skipped_shapes: Vec<String>,
}

impl CifExportReport {
    pub fn element_count(&self) -> usize {
        self.box_count + self.polygon_count + self.wire_count + self.label_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CifExportResult {
    pub text: String,
    pub report: CifExportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CifGeneratedLayer {
    pub name: String,
    pub layer_id: LayerId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CifImportReport {
    pub command_count: usize,
    pub shape_count: usize,
    pub layer_count: usize,
    pub generated_layers: Vec<CifGeneratedLayer>,
    pub skipped_commands: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CifImportResult {
    pub document: Document,
    pub report: CifImportReport,
}

pub fn export_cif(document: &Document) -> Result<String, CifError> {
    Ok(export_cif_with_report(document)?.text)
}

pub fn export_cif_with_report(document: &Document) -> Result<CifExportResult, CifError> {
    if !supports_hierarchical_cif_export(document) {
        let mut result = export_cif_flat_with_report(document)?;
        result.report.warnings.push(
            "non-translation instance transform exported through flat CIF fallback".to_string(),
        );
        return Ok(result);
    }
    export_cif_hierarchical_with_report(document)
}

fn export_cif_flat_with_report(document: &Document) -> Result<CifExportResult, CifError> {
    let mut report = CifExportReport::default();
    let mut text = String::new();
    let mut layer_names = BTreeMap::new();
    for layer in document.layers.values() {
        layer_names.insert(
            layer.id,
            sanitize_cif_identifier(&layer.name, layer.process.as_technology_name()),
        );
    }
    report.layer_count = layer_names.len();

    text.push_str("( Glassworks CIF subset export );\n");
    text.push_str("DS 1 1 1;\n");
    text.push_str(&format!(
        "9 {};\n",
        sanitize_cif_identifier(&document.name, "GLASSWORKS")
    ));

    let mut current_layer = None;
    for flattened in document.flattened_shapes() {
        let shape = flattened.transformed_shape();
        write_cif_shape(
            &mut text,
            &mut report,
            &layer_names,
            &mut current_layer,
            &shape,
            document.grid,
        );
    }

    text.push_str("DF;\n");
    text.push_str("E;\n");
    Ok(CifExportResult { text, report })
}

fn export_cif_hierarchical_with_report(document: &Document) -> Result<CifExportResult, CifError> {
    let mut report = CifExportReport::default();
    let mut text = String::new();
    let mut layer_names = BTreeMap::new();
    for layer in document.layers.values() {
        layer_names.insert(
            layer.id,
            sanitize_cif_identifier(&layer.name, layer.process.as_technology_name()),
        );
    }
    report.layer_count = layer_names.len();

    text.push_str("( Glassworks CIF subset export );\n");
    for cell_id in reachable_cif_cells(document) {
        let Some(cell) = document.cell(cell_id) else {
            continue;
        };
        text.push_str(&format!("DS {} 1 1;\n", cell_id.0));
        let cell_name = if cell_id == document.top_cell {
            sanitize_cif_identifier(&document.name, "GLASSWORKS")
        } else {
            sanitize_cif_identifier(&cell.name, &format!("cell_{}", cell_id.0))
        };
        text.push_str(&format!("9 {cell_name};\n"));

        let mut current_layer = None;
        if cell_id == document.top_cell {
            for shape in document.shapes.values() {
                write_cif_shape(
                    &mut text,
                    &mut report,
                    &layer_names,
                    &mut current_layer,
                    &shape,
                    document.grid,
                );
            }
        }
        for shape in cell.shapes.values() {
            write_cif_shape(
                &mut text,
                &mut report,
                &layer_names,
                &mut current_layer,
                &shape,
                document.grid,
            );
        }
        for instance in cell.instances.values() {
            write_cif_instance_calls(&mut text, instance);
        }
        text.push_str("DF;\n");
    }
    text.push_str("E;\n");
    Ok(CifExportResult { text, report })
}

fn write_cif_shape(
    text: &mut String,
    report: &mut CifExportReport,
    layer_names: &BTreeMap<LayerId, String>,
    current_layer: &mut Option<LayerId>,
    shape: &Shape,
    grid: Coord,
) {
    let Some(layer_name) = layer_names.get(&shape.layer) else {
        report.skipped_shapes.push(format!(
            "shape #{} references missing layer {}",
            shape.id.0, shape.layer.0
        ));
        return;
    };
    if *current_layer != Some(shape.layer) {
        text.push_str(&format!("L {layer_name};\n"));
        *current_layer = Some(shape.layer);
    }
    report.shape_count += 1;
    match &shape.kind {
        ShapeKind::Rectangle(rect) => {
            write_cif_box(text, *rect);
            report.box_count += 1;
        }
        ShapeKind::Polygon(poly) => {
            let points = cleanup_cif_points(poly.points.clone());
            if points.len() < 3 {
                report
                    .skipped_shapes
                    .push(format!("polygon #{} has fewer than 3 points", shape.id.0));
                return;
            }
            write_cif_polygon(text, &points);
            report.polygon_count += 1;
        }
        ShapeKind::Path { points, width } => {
            let points = cleanup_cif_points(points.clone());
            if points.len() < 2 {
                report
                    .skipped_shapes
                    .push(format!("path #{} has fewer than 2 points", shape.id.0));
                return;
            }
            write_cif_wire(text, *width, &points);
            report.wire_count += 1;
        }
        ShapeKind::Via { center, size, .. } => {
            write_cif_box(text, Rect::from_min_size(*center, 0, 0).expanded(*size / 2));
            report.box_count += 1;
            report
                .warnings
                .push(format!("via #{} exported as a CIF box", shape.id.0));
        }
        ShapeKind::Label {
            position,
            text: label,
        } => {
            write_cif_label(text, label, *position);
            report.label_count += 1;
        }
        ShapeKind::Measurement { a, b, label, .. } => {
            write_cif_wire(text, grid.max(1), &[*a, *b]);
            write_cif_label(text, label, *b);
            report.wire_count += 1;
            report.label_count += 1;
            report.warnings.push(format!(
                "measurement #{} exported as CIF wire and label",
                shape.id.0
            ));
        }
    }
}

fn supports_hierarchical_cif_export(document: &Document) -> bool {
    document
        .cells
        .values()
        .flat_map(|cell| cell.instances.values())
        .all(|instance| instance.transform.matrix == Transform::IDENTITY.matrix)
}

fn reachable_cif_cells(document: &Document) -> Vec<CellId> {
    let mut visited = BTreeSet::new();
    let mut ordered = Vec::new();
    collect_reachable_cif_cells(document, document.top_cell, &mut visited, &mut ordered);
    ordered
}

fn collect_reachable_cif_cells(
    document: &Document,
    cell_id: CellId,
    visited: &mut BTreeSet<CellId>,
    ordered: &mut Vec<CellId>,
) {
    if !visited.insert(cell_id) || document.cell(cell_id).is_none() {
        return;
    }
    ordered.push(cell_id);
    let Some(cell) = document.cell(cell_id) else {
        return;
    };
    for instance in cell.instances.values() {
        collect_reachable_cif_cells(document, instance.cell, visited, ordered);
    }
}

fn write_cif_instance_calls(text: &mut String, instance: crate::CellInstance) {
    let array = instance.array.normalized();
    for row in 0..array.rows {
        for column in 0..array.columns {
            let offset = array.element_offset(column, row);
            let translation = Vector::new(
                instance.transform.translation.dx + offset.dx,
                instance.transform.translation.dy + offset.dy,
            );
            text.push_str(&format!(
                "C {} T {} {};\n",
                instance.cell.0, translation.dx, translation.dy
            ));
        }
    }
}

pub fn import_cif(input: &str) -> Result<Document, CifError> {
    Ok(import_cif_with_report(input)?.document)
}

pub fn import_cif_with_report(input: &str) -> Result<CifImportResult, CifError> {
    let mut document = Document::new("CIF import");
    let mut report = CifImportReport::default();
    let mut layer_map = existing_cif_layers(&document);
    let mut current_layer = document.layer_by_process(ProcessLayer::Annotation);
    let mut current_cell = document.top_cell;
    let mut structure_cells = BTreeMap::<String, CellId>::new();

    for raw_command in input.split(';') {
        let command = raw_command.trim();
        if command.is_empty() || command.starts_with('(') {
            continue;
        }
        report.command_count += 1;
        let tokens = command.split_whitespace().collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }
        match tokens[0] {
            "E" => {}
            "DF" => {
                current_cell = document.top_cell;
                current_layer = document.layer_by_process(ProcessLayer::Annotation);
            }
            "DS" => {
                let Some(structure_name) = tokens.get(1) else {
                    return Err(CifError::InvalidCommand(command.to_string()));
                };
                current_cell = cif_cell_for_structure(
                    &mut document,
                    &mut structure_cells,
                    structure_name,
                    true,
                );
                current_layer = document.layer_by_process(ProcessLayer::Annotation);
            }
            "9" => {
                if tokens.len() > 1 {
                    let name = tokens[1..].join(" ");
                    if let Some(cell) = document.cell_mut(current_cell) {
                        cell.name =
                            sanitize_cif_identifier(&name, &format!("cell_{}", current_cell.0));
                    }
                    if current_cell == document.top_cell {
                        document.name = sanitize_cif_identifier(&name, "CIF_import");
                    }
                }
            }
            "L" => {
                let Some(name) = tokens.get(1) else {
                    return Err(CifError::InvalidCommand(command.to_string()));
                };
                let layer = layer_for_cif_name(&mut document, &mut layer_map, name, &mut report);
                current_layer = Some(layer);
            }
            "B" => {
                let layer = current_layer.ok_or_else(|| {
                    CifError::InvalidCommand("CIF box appears before a layer command".to_string())
                })?;
                if tokens.len() < 5 {
                    return Err(CifError::InvalidCommand(command.to_string()));
                }
                let width = parse_coord(tokens[1], command)?;
                let height = parse_coord(tokens[2], command)?;
                let center = Point::new(
                    parse_coord(tokens[3], command)?,
                    parse_coord(tokens[4], command)?,
                );
                if width <= 0 || height <= 0 {
                    report
                        .skipped_commands
                        .push(format!("ignored nonpositive box: {command}"));
                    continue;
                }
                let min = Point::new(center.x - width / 2, center.y - height / 2);
                document.insert_shape_in_cell(
                    current_cell,
                    layer,
                    ShapeKind::Rectangle(Rect::from_min_size(min, width, height)),
                );
                report.shape_count += 1;
            }
            "P" => {
                let layer = current_layer.ok_or_else(|| {
                    CifError::InvalidCommand(
                        "CIF polygon appears before a layer command".to_string(),
                    )
                })?;
                let points = parse_point_tokens(&tokens[1..], command)?;
                let points = cleanup_cif_points(points);
                if points.len() < 3 {
                    report
                        .skipped_commands
                        .push(format!("ignored degenerate polygon: {command}"));
                    continue;
                }
                document.insert_shape_in_cell(
                    current_cell,
                    layer,
                    ShapeKind::Polygon(Polygon::new(points)),
                );
                report.shape_count += 1;
            }
            "W" => {
                let layer = current_layer.ok_or_else(|| {
                    CifError::InvalidCommand("CIF wire appears before a layer command".to_string())
                })?;
                if tokens.len() < 6 {
                    return Err(CifError::InvalidCommand(command.to_string()));
                }
                let width = parse_coord(tokens[1], command)?.abs().max(1);
                let points = parse_point_tokens(&tokens[2..], command)?;
                if points.len() < 2 {
                    report
                        .skipped_commands
                        .push(format!("ignored short wire: {command}"));
                    continue;
                }
                document.insert_shape_in_cell(
                    current_cell,
                    layer,
                    ShapeKind::Path { points, width },
                );
                report.shape_count += 1;
            }
            "94" => {
                let layer = current_layer.ok_or_else(|| {
                    CifError::InvalidCommand("CIF label appears before a layer command".to_string())
                })?;
                if tokens.len() < 4 {
                    return Err(CifError::InvalidCommand(command.to_string()));
                }
                let x_index = tokens.len() - 2;
                let position = Point::new(
                    parse_coord(tokens[x_index], command)?,
                    parse_coord(tokens[x_index + 1], command)?,
                );
                let label = tokens[1..x_index].join(" ");
                document.insert_shape_in_cell(
                    current_cell,
                    layer,
                    ShapeKind::Label {
                        position,
                        text: label,
                    },
                );
                report.shape_count += 1;
            }
            "C" => {
                let Some(structure_name) = tokens.get(1) else {
                    return Err(CifError::InvalidCommand(command.to_string()));
                };
                let target_cell = cif_cell_for_structure(
                    &mut document,
                    &mut structure_cells,
                    structure_name,
                    false,
                );
                if target_cell == current_cell {
                    report
                        .skipped_commands
                        .push(format!("ignored recursive CIF call: {command}"));
                    continue;
                }
                let Some(transform) = parse_cif_call_transform(&tokens[2..], command, &mut report)?
                else {
                    continue;
                };
                document.insert_instance(current_cell, target_cell, transform);
            }
            _ => {
                report
                    .skipped_commands
                    .push(format!("ignored unsupported CIF command: {command}"));
            }
        }
    }
    report.layer_count = document.layers.len();
    document.ensure_hierarchy();
    Ok(CifImportResult { document, report })
}

fn write_cif_box(output: &mut String, rect: Rect) {
    output.push_str(&format!(
        "B {} {} {} {};\n",
        rect.width().max(1),
        rect.height().max(1),
        rect.center().x,
        rect.center().y
    ));
}

fn write_cif_polygon(output: &mut String, points: &[Point]) {
    output.push_str("P");
    for point in points {
        output.push_str(&format!(" {} {}", point.x, point.y));
    }
    output.push_str(";\n");
}

fn write_cif_wire(output: &mut String, width: Coord, points: &[Point]) {
    output.push_str(&format!("W {}", width.abs().max(1)));
    for point in points {
        output.push_str(&format!(" {} {}", point.x, point.y));
    }
    output.push_str(";\n");
}

fn write_cif_label(output: &mut String, label: &str, position: Point) {
    output.push_str(&format!(
        "94 {} {} {};\n",
        sanitize_cif_label(label),
        position.x,
        position.y
    ));
}

fn parse_coord(token: &str, command: &str) -> Result<Coord, CifError> {
    token.parse::<Coord>().map_err(|_| CifError::InvalidNumber {
        token: token.to_string(),
        command: command.to_string(),
    })
}

fn cif_cell_for_structure(
    document: &mut Document,
    structure_cells: &mut BTreeMap<String, CellId>,
    raw_name: &str,
    definition: bool,
) -> CellId {
    let key = sanitize_cif_identifier(raw_name, "1");
    if let Some(cell) = structure_cells.get(&key) {
        return *cell;
    }
    let cell = if structure_cells.is_empty() && definition {
        document.top_cell
    } else {
        document.create_cell(format!("cif_cell_{key}"))
    };
    structure_cells.insert(key, cell);
    cell
}

fn parse_cif_call_transform(
    tokens: &[&str],
    command: &str,
    report: &mut CifImportReport,
) -> Result<Option<Transform>, CifError> {
    let mut translation = Vector::ZERO;
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index] {
            "T" => {
                if index + 2 >= tokens.len() {
                    return Err(CifError::InvalidCommand(command.to_string()));
                }
                translation.dx = parse_coord(tokens[index + 1], command)?;
                translation.dy = parse_coord(tokens[index + 2], command)?;
                index += 3;
            }
            token => {
                report.skipped_commands.push(format!(
                    "ignored CIF call with unsupported transform {token}: {command}"
                ));
                return Ok(None);
            }
        }
    }
    Ok(Some(Transform::from_translation(translation)))
}

fn parse_point_tokens(tokens: &[&str], command: &str) -> Result<Vec<Point>, CifError> {
    if tokens.len() % 2 != 0 {
        return Err(CifError::InvalidCommand(command.to_string()));
    }
    let mut points = Vec::with_capacity(tokens.len() / 2);
    for chunk in tokens.chunks(2) {
        points.push(Point::new(
            parse_coord(chunk[0], command)?,
            parse_coord(chunk[1], command)?,
        ));
    }
    Ok(points)
}

fn existing_cif_layers(document: &Document) -> BTreeMap<String, LayerId> {
    document
        .layers
        .values()
        .map(|layer| (normalize_cif_identifier(&layer.name), layer.id))
        .collect()
}

fn layer_for_cif_name(
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    raw_name: &str,
    report: &mut CifImportReport,
) -> LayerId {
    let key = normalize_cif_identifier(raw_name);
    if let Some(layer) = layer_map.get(&key) {
        return *layer;
    }
    if let Some(process) = ProcessLayer::from_technology_name(raw_name) {
        if let Some(layer) = document.layer_by_process(process) {
            layer_map.insert(key, layer);
            return layer;
        }
    }
    let process = ProcessLayer::from_technology_name(raw_name).unwrap_or(ProcessLayer::Annotation);
    let layer_name = sanitize_cif_identifier(raw_name, "cif_layer");
    let layer = document.create_layer(layer_name.clone(), process, [0.75, 0.55, 0.25, 1.0]);
    layer_map.insert(key, layer);
    report.generated_layers.push(CifGeneratedLayer {
        name: layer_name,
        layer_id: layer,
    });
    layer
}

fn cleanup_cif_points(mut points: Vec<Point>) -> Vec<Point> {
    while points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    let mut cleaned = Vec::with_capacity(points.len());
    for point in points {
        if cleaned.last().copied() != Some(point) {
            cleaned.push(point);
        }
    }
    cleaned
}

fn sanitize_cif_identifier(value: &str, fallback: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            previous_underscore = false;
        } else if !previous_underscore {
            output.push('_');
            previous_underscore = true;
        }
    }
    let output = output.trim_matches('_').to_string();
    if output.is_empty() {
        fallback.to_string()
    } else {
        output
    }
}

fn normalize_cif_identifier(value: &str) -> String {
    sanitize_cif_identifier(value, "layer").to_ascii_lowercase()
}

fn sanitize_cif_label(value: &str) -> String {
    let mut output = value
        .chars()
        .map(|ch| {
            if ch == ';' || ch.is_control() {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>();
    output = output.split_whitespace().collect::<Vec<_>>().join(" ");
    if output.is_empty() {
        "label".to_string()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cif_export_import_round_trips_basic_flat_geometry() {
        let mut document = Document::new("cif round trip");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let annotation = document.layer_by_process(ProcessLayer::Annotation).unwrap();
        document.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 600))),
        );
        document.insert_shape(
            metal1,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(2_000, 0),
                Point::new(2_600, 0),
                Point::new(2_400, 400),
            ])),
        );
        document.insert_shape(
            metal1,
            ShapeKind::Path {
                points: vec![Point::new(0, 2_000), Point::new(1_000, 2_000)],
                width: 120,
            },
        );
        document.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(0, 3_000),
                text: "pad label".to_string(),
            },
        );

        let exported = export_cif_with_report(&document).expect("CIF export should succeed");
        assert!(exported.text.contains("DS 1 1 1;"));
        assert!(exported.text.contains("L metal1;"));
        assert_eq!(exported.report.box_count, 1);
        assert_eq!(exported.report.polygon_count, 1);
        assert_eq!(exported.report.wire_count, 1);
        assert_eq!(exported.report.label_count, 1);

        let imported = import_cif_with_report(&exported.text).expect("CIF import should succeed");
        assert_eq!(imported.report.generated_layers.len(), 0);
        assert!(imported.document.shapes.values().any(|shape| {
            matches!(shape.kind, ShapeKind::Rectangle(rect) if rect.width() == 1_000 && rect.height() == 600)
        }));
        assert!(imported
            .document
            .shapes
            .values()
            .any(|shape| matches!(shape.kind, ShapeKind::Polygon(_))));
        assert!(imported
            .document
            .shapes
            .values()
            .any(|shape| matches!(shape.kind, ShapeKind::Path { width: 120, .. })));
        assert!(imported.document.shapes.values().any(
            |shape| matches!(&shape.kind, ShapeKind::Label { text, .. } if text == "pad label")
        ));
    }

    #[test]
    fn cif_import_generates_unknown_layers_and_forward_call_placeholders() {
        let imported = import_cif_with_report("DS 1 1 1; L CUSTOM; B 10 20 5 10; C 2; DF; E;")
            .expect("CIF import should parse simple custom layers");
        assert_eq!(imported.report.generated_layers.len(), 1);
        assert_eq!(imported.report.shape_count, 1);
        assert_eq!(imported.report.skipped_commands.len(), 0);
        assert_eq!(
            imported
                .document
                .cell(imported.document.top_cell)
                .expect("top cell should exist")
                .instances
                .len(),
            1
        );
        assert!(imported
            .document
            .layers
            .values()
            .any(|layer| layer.name == "CUSTOM"));
    }

    #[test]
    fn cif_import_preserves_basic_hierarchy_calls() {
        let imported = import_cif_with_report(
            "DS 1 1 1; 9 TOP; C 2 T 1000 2000; DF;\
             DS 2 1 1; 9 UNIT; L metal1; B 100 80 50 40; DF; E;",
        )
        .expect("CIF import should parse hierarchy calls");

        assert_eq!(imported.report.shape_count, 1);
        assert_eq!(imported.report.skipped_commands.len(), 0);
        assert_eq!(imported.document.name, "TOP");
        assert_eq!(imported.document.cells.len(), 2);
        let top = imported
            .document
            .cell(imported.document.top_cell)
            .expect("top cell should exist");
        assert_eq!(top.instances.len(), 1);
        let child_id = top
            .instances
            .values()
            .next()
            .expect("top should place child")
            .cell;
        assert_eq!(
            imported
                .document
                .cell(child_id)
                .expect("child cell should exist")
                .name,
            "UNIT"
        );
        let flattened = imported.document.visible_flattened_shapes();
        assert_eq!(flattened.len(), 1);
        assert_eq!(
            flattened[0].bounds,
            Rect::new(Point::new(1_000, 2_000), Point::new(1_100, 2_080))
        );
    }

    #[test]
    fn cif_export_preserves_basic_translated_hierarchy() {
        let mut document = Document::new("cif hierarchy export");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = document.create_cell("unit");
        document
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
            )
            .expect("child shape should be inserted");
        document
            .insert_instance_in_top(child, Transform::translate(1_000, 2_000))
            .expect("child instance should be inserted");

        let exported = export_cif_with_report(&document).expect("CIF export should succeed");
        assert!(exported.text.contains(&format!("DS {} 1 1;", child.0)));
        assert!(exported
            .text
            .contains(&format!("C {} T 1000 2000;", child.0)));
        assert_eq!(exported.report.box_count, 1);

        let imported = import_cif_with_report(&exported.text).expect("CIF import should succeed");
        assert_eq!(imported.document.cells.len(), 2);
        assert_eq!(
            imported.document.visible_flattened_shapes()[0].bounds,
            Rect::new(Point::new(1_000, 2_000), Point::new(1_100, 2_080))
        );
    }

    #[test]
    fn cif_export_falls_back_to_flat_for_rotated_instances() {
        let mut document = Document::new("cif rotated export");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = document.create_cell("unit");
        document
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
            )
            .expect("child shape should be inserted");
        document
            .insert_instance_in_top(
                child,
                Transform::translate(1_000, 2_000).compose(Transform::rotate_cw90()),
            )
            .expect("child instance should be inserted");

        let exported = export_cif_with_report(&document).expect("CIF export should succeed");
        assert!(exported.text.contains("DS 1 1 1;"));
        assert!(!exported.text.contains(&format!("C {}", child.0)));
        assert!(exported
            .report
            .warnings
            .iter()
            .any(|warning| warning.contains("flat CIF fallback")));
        let imported = import_cif_with_report(&exported.text).expect("CIF import should succeed");
        assert_eq!(imported.document.visible_flattened_shapes().len(), 1);
    }
}
