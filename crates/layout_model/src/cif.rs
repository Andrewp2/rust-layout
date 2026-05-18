use std::{collections::BTreeMap, error::Error, fmt};

use geometry_core::{Coord, Point, Polygon, Rect};

use crate::{Document, LayerId, ProcessLayer, ShapeKind};

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
        let mut shape = flattened.transformed_shape();
        let Some(layer_name) = layer_names.get(&shape.layer) else {
            report.skipped_shapes.push(format!(
                "shape #{} references missing layer {}",
                shape.id.0, shape.layer.0
            ));
            continue;
        };
        if current_layer != Some(shape.layer) {
            text.push_str(&format!("L {layer_name};\n"));
            current_layer = Some(shape.layer);
        }
        report.shape_count += 1;
        match &mut shape.kind {
            ShapeKind::Rectangle(rect) => {
                write_cif_box(&mut text, *rect);
                report.box_count += 1;
            }
            ShapeKind::Polygon(poly) => {
                let points = cleanup_cif_points(poly.points.clone());
                if points.len() < 3 {
                    report
                        .skipped_shapes
                        .push(format!("polygon #{} has fewer than 3 points", shape.id.0));
                    continue;
                }
                write_cif_polygon(&mut text, &points);
                report.polygon_count += 1;
            }
            ShapeKind::Path { points, width } => {
                let points = cleanup_cif_points(points.clone());
                if points.len() < 2 {
                    report
                        .skipped_shapes
                        .push(format!("path #{} has fewer than 2 points", shape.id.0));
                    continue;
                }
                write_cif_wire(&mut text, *width, &points);
                report.wire_count += 1;
            }
            ShapeKind::Via { center, size, .. } => {
                write_cif_box(
                    &mut text,
                    Rect::from_min_size(*center, 0, 0).expanded(*size / 2),
                );
                report.box_count += 1;
                report
                    .warnings
                    .push(format!("via #{} exported as a CIF box", shape.id.0));
            }
            ShapeKind::Label {
                position,
                text: label,
            } => {
                write_cif_label(&mut text, label, *position);
                report.label_count += 1;
            }
            ShapeKind::Measurement { a, b, label, .. } => {
                write_cif_wire(&mut text, document.grid.max(1), &[*a, *b]);
                write_cif_label(&mut text, label, *b);
                report.wire_count += 1;
                report.label_count += 1;
                report.warnings.push(format!(
                    "measurement #{} exported as CIF wire and label",
                    shape.id.0
                ));
            }
        }
    }

    text.push_str("DF;\n");
    text.push_str("E;\n");
    Ok(CifExportResult { text, report })
}

pub fn import_cif(input: &str) -> Result<Document, CifError> {
    Ok(import_cif_with_report(input)?.document)
}

pub fn import_cif_with_report(input: &str) -> Result<CifImportResult, CifError> {
    let mut document = Document::new("CIF import");
    let mut report = CifImportReport::default();
    let mut layer_map = existing_cif_layers(&document);
    let mut current_layer = document.layer_by_process(ProcessLayer::Annotation);

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
            "E" | "DF" | "DS" | "9" => {}
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
                document.insert_shape(
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
                document.insert_shape(layer, ShapeKind::Polygon(Polygon::new(points)));
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
                document.insert_shape(layer, ShapeKind::Path { points, width });
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
                document.insert_shape(
                    layer,
                    ShapeKind::Label {
                        position,
                        text: label,
                    },
                );
                report.shape_count += 1;
            }
            "C" => {
                report
                    .skipped_commands
                    .push(format!("ignored unsupported CIF call command: {command}"));
            }
            _ => {
                report
                    .skipped_commands
                    .push(format!("ignored unsupported CIF command: {command}"));
            }
        }
    }
    report.layer_count = document.layers.len();
    document.name = "CIF import".to_string();
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
        assert!(
            imported
                .document
                .shapes
                .values()
                .any(|shape| matches!(shape.kind, ShapeKind::Polygon(_)))
        );
        assert!(
            imported
                .document
                .shapes
                .values()
                .any(|shape| matches!(shape.kind, ShapeKind::Path { width: 120, .. }))
        );
        assert!(imported.document.shapes.values().any(
            |shape| matches!(&shape.kind, ShapeKind::Label { text, .. } if text == "pad label")
        ));
    }

    #[test]
    fn cif_import_generates_unknown_layers_and_skips_calls() {
        let imported = import_cif_with_report("DS 1 1 1; L CUSTOM; B 10 20 5 10; C 2; DF; E;")
            .expect("CIF import should parse simple custom layers");
        assert_eq!(imported.report.generated_layers.len(), 1);
        assert_eq!(imported.report.shape_count, 1);
        assert_eq!(imported.report.skipped_commands.len(), 1);
        assert!(
            imported
                .document
                .layers
                .values()
                .any(|layer| layer.name == "CUSTOM")
        );
    }
}
