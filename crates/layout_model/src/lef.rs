use std::{collections::BTreeMap, error::Error, fmt};

use geometry_core::{Coord, DBU_PER_MICRON, Point, Polygon, Rect};

use crate::{Document, LayerId, ProcessLayer, ShapeKind};

#[derive(Debug)]
pub enum LefError {
    InvalidNumber { token: String, context: String },
    InvalidSyntax(String),
    UnexpectedEof { context: String },
}

impl fmt::Display for LefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber { token, context } => {
                write!(formatter, "invalid LEF number {token:?} in {context}")
            }
            Self::InvalidSyntax(message) => write!(formatter, "invalid LEF syntax: {message}"),
            Self::UnexpectedEof { context } => {
                write!(formatter, "unexpected end of LEF while reading {context}")
            }
        }
    }
}

impl Error for LefError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LefExportReport {
    pub layer_count: usize,
    pub shape_count: usize,
    pub obstruction_count: usize,
    pub pin_count: usize,
    pub warnings: Vec<String>,
    pub skipped_shapes: Vec<String>,
}

impl LefExportReport {
    pub fn object_count(&self) -> usize {
        self.obstruction_count + self.pin_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LefExportResult {
    pub text: String,
    pub report: LefExportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LefGeneratedLayer {
    pub name: String,
    pub layer_id: LayerId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LefImportReport {
    pub macro_count: usize,
    pub shape_count: usize,
    pub layer_count: usize,
    pub obstruction_count: usize,
    pub pin_count: usize,
    pub generated_layers: Vec<LefGeneratedLayer>,
    pub skipped_items: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct LefImportResult {
    pub document: Document,
    pub report: LefImportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TokenCursor {
    tokens: Vec<String>,
    index: usize,
}

impl TokenCursor {
    fn new(tokens: Vec<String>) -> Self {
        Self { tokens, index: 0 }
    }

    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.index).map(String::as_str)
    }

    fn next(&mut self) -> Option<String> {
        let token = self.tokens.get(self.index).cloned()?;
        self.index += 1;
        Some(token)
    }

    fn next_required(&mut self, context: &str) -> Result<String, LefError> {
        self.next().ok_or_else(|| LefError::UnexpectedEof {
            context: context.to_string(),
        })
    }

    fn eat_keyword(&mut self, keyword: &str) -> bool {
        if self
            .peek()
            .is_some_and(|token| token.eq_ignore_ascii_case(keyword))
        {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn skip_statement(&mut self) {
        while let Some(token) = self.next() {
            if token == ";" {
                break;
            }
        }
    }
}

pub fn export_lef(document: &Document) -> Result<String, LefError> {
    Ok(export_lef_with_report(document)?.text)
}

pub fn export_lef_with_report(document: &Document) -> Result<LefExportResult, LefError> {
    let mut report = LefExportReport::default();
    let mut layer_names = BTreeMap::new();
    for layer in document.layers.values() {
        layer_names.insert(
            layer.id,
            sanitize_lef_identifier(&layer.name, layer.process.as_technology_name()),
        );
    }
    report.layer_count = layer_names.len();

    let mut obstructions = Vec::new();
    let mut pins = Vec::new();
    let mut all_bounds = Vec::new();
    for flattened in document.flattened_shapes() {
        let mut shape = flattened.transformed_shape();
        let Some(layer_name) = layer_names.get(&shape.layer).cloned() else {
            report.skipped_shapes.push(format!(
                "shape #{} references missing layer {}",
                shape.id.0, shape.layer.0
            ));
            continue;
        };
        all_bounds.push(shape.kind.bounds());
        match &mut shape.kind {
            ShapeKind::Rectangle(rect) => {
                obstructions.push(LefGeometry::Rect {
                    layer_name,
                    rect: *rect,
                });
                report.shape_count += 1;
                report.obstruction_count += 1;
            }
            ShapeKind::Polygon(poly) => {
                let points = cleanup_lef_points(poly.points.clone());
                if points.len() < 3 {
                    report
                        .skipped_shapes
                        .push(format!("polygon #{} has fewer than 3 points", shape.id.0));
                    continue;
                }
                obstructions.push(LefGeometry::Polygon { layer_name, points });
                report.shape_count += 1;
                report.obstruction_count += 1;
            }
            ShapeKind::Path { points, width } => {
                let points = cleanup_lef_points(points.clone());
                if points.len() < 2 {
                    report
                        .skipped_shapes
                        .push(format!("path #{} has fewer than 2 points", shape.id.0));
                    continue;
                }
                let rects = path_segment_rects(&points, *width);
                if rects.is_empty() {
                    report.skipped_shapes.push(format!(
                        "path #{} has no horizontal or vertical LEF obstruction segments",
                        shape.id.0
                    ));
                    continue;
                }
                for rect in rects {
                    obstructions.push(LefGeometry::Rect {
                        layer_name: layer_name.clone(),
                        rect,
                    });
                    report.obstruction_count += 1;
                }
                report.shape_count += 1;
                report.warnings.push(format!(
                    "path #{} exported as LEF obstruction rectangle(s)",
                    shape.id.0
                ));
            }
            ShapeKind::Via { center, size, .. } => {
                let size = size.abs().max(1);
                let min = Point::new(center.x - size / 2, center.y - size / 2);
                obstructions.push(LefGeometry::Rect {
                    layer_name,
                    rect: Rect::from_min_size(min, size, size),
                });
                report.shape_count += 1;
                report.obstruction_count += 1;
                report.warnings.push(format!(
                    "via #{} exported as a LEF obstruction rectangle",
                    shape.id.0
                ));
            }
            ShapeKind::Label { position, text } => {
                pins.push(LefPin {
                    layer_name,
                    name: sanitize_lef_identifier(text, &format!("label_{}", shape.id.0)),
                    rect: pin_marker_rect(*position, document.grid),
                });
                report.shape_count += 1;
                report.pin_count += 1;
            }
            ShapeKind::Measurement { a, b, label, .. } => {
                for rect in path_segment_rects(&[*a, *b], document.grid.max(1)) {
                    obstructions.push(LefGeometry::Rect {
                        layer_name: layer_name.clone(),
                        rect,
                    });
                    report.obstruction_count += 1;
                }
                pins.push(LefPin {
                    layer_name,
                    name: sanitize_lef_identifier(label, &format!("measurement_{}", shape.id.0)),
                    rect: pin_marker_rect(*b, document.grid),
                });
                report.shape_count += 1;
                report.pin_count += 1;
                report.warnings.push(format!(
                    "measurement #{} exported as LEF obstruction and pin",
                    shape.id.0
                ));
            }
        }
    }

    let bounds = all_bounds
        .into_iter()
        .reduce(|left, right| left.union(right))
        .unwrap_or_else(|| {
            Rect::from_min_size(Point::ZERO, document.grid.max(1), document.grid.max(1))
        });
    let macro_name = sanitize_lef_identifier(&document.name, "glassworks");
    let mut text = String::new();
    text.push_str("VERSION 5.8 ;\n");
    text.push_str("BUSBITCHARS \"[]\" ;\n");
    text.push_str("DIVIDERCHAR \"/\" ;\n");
    text.push_str("UNITS\n");
    text.push_str(&format!("  DATABASE MICRONS {} ;\n", DBU_PER_MICRON));
    text.push_str("END UNITS\n");
    text.push_str(&format!("MACRO {macro_name}\n"));
    text.push_str("  CLASS BLOCK ;\n");
    text.push_str("  ORIGIN 0 0 ;\n");
    text.push_str(&format!("  FOREIGN {macro_name} 0 0 ;\n"));
    text.push_str(&format!(
        "  SIZE {} BY {} ;\n",
        format_lef_coord(bounds.width().max(1)),
        format_lef_coord(bounds.height().max(1))
    ));
    write_lef_obstructions(&mut text, &obstructions);
    write_lef_pins(&mut text, &pins);
    text.push_str(&format!("END {macro_name}\n"));
    text.push_str("END LIBRARY\n");
    Ok(LefExportResult { text, report })
}

pub fn import_lef(input: &str) -> Result<Document, LefError> {
    Ok(import_lef_with_report(input)?.document)
}

pub fn import_lef_with_report(input: &str) -> Result<LefImportResult, LefError> {
    let mut cursor = TokenCursor::new(tokenize_lef(input));
    let mut document = Document::new("LEF import");
    let mut report = LefImportReport::default();
    let mut layer_map = existing_lef_layers(&document);
    let mut database_microns = DBU_PER_MICRON;

    while let Some(token) = cursor.next() {
        match token.to_ascii_uppercase().as_str() {
            "UNITS" => parse_units_block(&mut cursor, &mut database_microns)?,
            "MACRO" => {
                let macro_name = cursor.next_required("LEF macro name")?;
                if report.macro_count == 0 {
                    document.name = macro_name.clone();
                }
                report.macro_count += 1;
                parse_macro_block(
                    &mut cursor,
                    &mut document,
                    &mut layer_map,
                    &mut report,
                    &macro_name,
                    database_microns,
                )?;
            }
            "END" if cursor.eat_keyword("LIBRARY") => break,
            _ => {}
        }
    }
    report.layer_count = document.layers.len();
    document.ensure_hierarchy();
    Ok(LefImportResult { document, report })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LefGeometry {
    Rect {
        layer_name: String,
        rect: Rect,
    },
    Polygon {
        layer_name: String,
        points: Vec<Point>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LefPin {
    layer_name: String,
    name: String,
    rect: Rect,
}

fn write_lef_obstructions(output: &mut String, obstructions: &[LefGeometry]) {
    output.push_str("  OBS\n");
    let mut current_layer: Option<&str> = None;
    for obstruction in obstructions {
        let layer_name = match obstruction {
            LefGeometry::Rect { layer_name, .. } | LefGeometry::Polygon { layer_name, .. } => {
                layer_name
            }
        };
        if current_layer != Some(layer_name.as_str()) {
            output.push_str(&format!("    LAYER {layer_name} ;\n"));
            current_layer = Some(layer_name);
        }
        match obstruction {
            LefGeometry::Rect { rect, .. } => {
                output.push_str(&format!(
                    "      RECT {} {} {} {} ;\n",
                    format_lef_coord(rect.min.x),
                    format_lef_coord(rect.min.y),
                    format_lef_coord(rect.max.x),
                    format_lef_coord(rect.max.y)
                ));
            }
            LefGeometry::Polygon { points, .. } => {
                output.push_str("      POLYGON");
                for point in points {
                    output.push_str(&format!(
                        " {} {}",
                        format_lef_coord(point.x),
                        format_lef_coord(point.y)
                    ));
                }
                output.push_str(" ;\n");
            }
        }
    }
    output.push_str("  END\n");
}

fn write_lef_pins(output: &mut String, pins: &[LefPin]) {
    for pin in pins {
        output.push_str(&format!("  PIN {}\n", pin.name));
        output.push_str("    DIRECTION INPUT ;\n");
        output.push_str("    USE SIGNAL ;\n");
        output.push_str("    PORT\n");
        output.push_str(&format!("      LAYER {} ;\n", pin.layer_name));
        output.push_str(&format!(
            "        RECT {} {} {} {} ;\n",
            format_lef_coord(pin.rect.min.x),
            format_lef_coord(pin.rect.min.y),
            format_lef_coord(pin.rect.max.x),
            format_lef_coord(pin.rect.max.y)
        ));
        output.push_str("    END\n");
        output.push_str(&format!("  END {}\n", pin.name));
    }
}

fn parse_units_block(
    cursor: &mut TokenCursor,
    database_microns: &mut Coord,
) -> Result<(), LefError> {
    while let Some(token) = cursor.next() {
        match token.to_ascii_uppercase().as_str() {
            "DATABASE" => {
                cursor.eat_keyword("MICRONS");
                let token = cursor.next_required("LEF DATABASE MICRONS")?;
                *database_microns = parse_lef_units(&token)?;
                cursor.skip_statement();
            }
            "END" if cursor.eat_keyword("UNITS") => break,
            _ => {
                if token == ";" {
                    continue;
                }
                cursor.skip_statement();
            }
        }
    }
    Ok(())
}

fn parse_macro_block(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut LefImportReport,
    macro_name: &str,
    database_microns: Coord,
) -> Result<(), LefError> {
    while let Some(token) = cursor.next() {
        match token.to_ascii_uppercase().as_str() {
            "OBS" => {
                parse_geometry_block(cursor, document, layer_map, report, database_microns, true)?
            }
            "PIN" => {
                let pin_name = cursor.next_required("LEF pin name")?;
                parse_pin_block(
                    cursor,
                    document,
                    layer_map,
                    report,
                    &pin_name,
                    database_microns,
                )?;
            }
            "END" => {
                if cursor
                    .peek()
                    .is_some_and(|token| token.eq_ignore_ascii_case(macro_name))
                {
                    cursor.next();
                }
                break;
            }
            _ => {
                if token != ";" {
                    cursor.skip_statement();
                }
            }
        }
    }
    Ok(())
}

fn parse_geometry_block(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut LefImportReport,
    database_microns: Coord,
    insert_obstructions: bool,
) -> Result<(), LefError> {
    let mut current_layer = document.layer_by_process(ProcessLayer::Annotation);
    while let Some(token) = cursor.next() {
        match token.to_ascii_uppercase().as_str() {
            "END" => break,
            "LAYER" => {
                let layer_name = cursor.next_required("LEF geometry layer")?;
                current_layer = Some(layer_for_lef_name(document, layer_map, &layer_name, report));
                cursor.skip_statement();
            }
            "RECT" => {
                let Some(layer) = current_layer else {
                    report
                        .skipped_items
                        .push("ignored LEF RECT without a layer".to_string());
                    cursor.skip_statement();
                    continue;
                };
                let rect = parse_lef_rect(cursor, database_microns, "LEF RECT")?;
                if rect.width() <= 0 || rect.height() <= 0 {
                    report
                        .skipped_items
                        .push("ignored degenerate LEF RECT".to_string());
                    continue;
                }
                if insert_obstructions {
                    document.insert_shape(layer, ShapeKind::Rectangle(rect));
                    report.obstruction_count += 1;
                    report.shape_count += 1;
                }
            }
            "POLYGON" => {
                let Some(layer) = current_layer else {
                    report
                        .skipped_items
                        .push("ignored LEF POLYGON without a layer".to_string());
                    cursor.skip_statement();
                    continue;
                };
                let points =
                    parse_lef_points_until_statement(cursor, database_microns, "LEF POLYGON")?;
                let points = cleanup_lef_points(points);
                if points.len() < 3 {
                    report
                        .skipped_items
                        .push("ignored degenerate LEF POLYGON".to_string());
                    continue;
                }
                if insert_obstructions {
                    document.insert_shape(layer, ShapeKind::Polygon(Polygon::new(points)));
                    report.obstruction_count += 1;
                    report.shape_count += 1;
                }
            }
            "PATH" => {
                let Some(layer) = current_layer else {
                    report
                        .skipped_items
                        .push("ignored LEF PATH without a layer".to_string());
                    cursor.skip_statement();
                    continue;
                };
                let points =
                    parse_lef_points_until_statement(cursor, database_microns, "LEF PATH")?;
                let points = cleanup_lef_points(points);
                if points.len() < 2 {
                    report
                        .skipped_items
                        .push("ignored short LEF PATH".to_string());
                    continue;
                }
                if insert_obstructions {
                    document.insert_shape(
                        layer,
                        ShapeKind::Path {
                            points,
                            width: document.grid.max(1),
                        },
                    );
                    report.obstruction_count += 1;
                    report.shape_count += 1;
                }
            }
            _ => {
                if token != ";" {
                    cursor.skip_statement();
                }
            }
        }
    }
    Ok(())
}

fn parse_pin_block(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut LefImportReport,
    pin_name: &str,
    database_microns: Coord,
) -> Result<(), LefError> {
    let mut current_layer = document.layer_by_process(ProcessLayer::Annotation);
    let mut inserted = 0usize;
    let mut inside_port = false;
    while let Some(token) = cursor.next() {
        match token.to_ascii_uppercase().as_str() {
            "PORT" => inside_port = true,
            "END" => {
                if cursor
                    .peek()
                    .is_some_and(|token| token.eq_ignore_ascii_case(pin_name))
                {
                    cursor.next();
                    break;
                }
                if inside_port {
                    inside_port = false;
                } else {
                    break;
                }
            }
            "LAYER" => {
                let layer_name = cursor.next_required("LEF pin layer")?;
                current_layer = Some(layer_for_lef_name(document, layer_map, &layer_name, report));
                cursor.skip_statement();
            }
            "RECT" => {
                let Some(layer) = current_layer else {
                    report
                        .skipped_items
                        .push(format!("ignored LEF pin {pin_name} RECT without a layer"));
                    cursor.skip_statement();
                    continue;
                };
                let rect = parse_lef_rect(cursor, database_microns, "LEF pin RECT")?;
                if rect.width() <= 0 || rect.height() <= 0 {
                    report
                        .skipped_items
                        .push(format!("ignored degenerate LEF pin {pin_name} RECT"));
                    continue;
                }
                insert_lef_pin_label(document, layer, pin_name, rect.center(), report);
                inserted += 1;
            }
            "POLYGON" => {
                let Some(layer) = current_layer else {
                    report.skipped_items.push(format!(
                        "ignored LEF pin {pin_name} POLYGON without a layer"
                    ));
                    cursor.skip_statement();
                    continue;
                };
                let points =
                    parse_lef_points_until_statement(cursor, database_microns, "LEF pin POLYGON")?;
                if let Some(bounds) = Rect::from_points(&points) {
                    insert_lef_pin_label(document, layer, pin_name, bounds.center(), report);
                    inserted += 1;
                } else {
                    report
                        .skipped_items
                        .push(format!("ignored empty LEF pin {pin_name} POLYGON"));
                }
            }
            "PATH" => {
                let Some(layer) = current_layer else {
                    report
                        .skipped_items
                        .push(format!("ignored LEF pin {pin_name} PATH without a layer"));
                    cursor.skip_statement();
                    continue;
                };
                let points =
                    parse_lef_points_until_statement(cursor, database_microns, "LEF pin PATH")?;
                if let Some(bounds) = Rect::from_points(&points) {
                    insert_lef_pin_label(document, layer, pin_name, bounds.center(), report);
                    inserted += 1;
                } else {
                    report
                        .skipped_items
                        .push(format!("ignored empty LEF pin {pin_name} PATH"));
                }
            }
            _ => {
                if token != ";" {
                    cursor.skip_statement();
                }
            }
        }
    }
    if inserted == 0 {
        report
            .skipped_items
            .push(format!("ignored LEF pin {pin_name} without port geometry"));
    }
    Ok(())
}

fn insert_lef_pin_label(
    document: &mut Document,
    layer: LayerId,
    pin_name: &str,
    position: Point,
    report: &mut LefImportReport,
) {
    document.insert_shape(
        layer,
        ShapeKind::Label {
            position,
            text: pin_name.to_string(),
        },
    );
    report.pin_count += 1;
    report.shape_count += 1;
}

fn parse_lef_rect(
    cursor: &mut TokenCursor,
    database_microns: Coord,
    context: &str,
) -> Result<Rect, LefError> {
    let x1 = parse_lef_coord(&cursor.next_required(context)?, database_microns, context)?;
    let y1 = parse_lef_coord(&cursor.next_required(context)?, database_microns, context)?;
    let x2 = parse_lef_coord(&cursor.next_required(context)?, database_microns, context)?;
    let y2 = parse_lef_coord(&cursor.next_required(context)?, database_microns, context)?;
    cursor.skip_statement();
    Ok(Rect::new(Point::new(x1, y1), Point::new(x2, y2)))
}

fn parse_lef_points_until_statement(
    cursor: &mut TokenCursor,
    database_microns: Coord,
    context: &str,
) -> Result<Vec<Point>, LefError> {
    let mut coordinates = Vec::new();
    while let Some(token) = cursor.next() {
        if token == ";" {
            break;
        }
        coordinates.push(parse_lef_coord(&token, database_microns, context)?);
    }
    if coordinates.len() % 2 != 0 {
        return Err(LefError::InvalidSyntax(format!(
            "{context} has an odd number of coordinates"
        )));
    }
    Ok(coordinates
        .chunks_exact(2)
        .map(|chunk| Point::new(chunk[0], chunk[1]))
        .collect())
}

fn parse_lef_units(token: &str) -> Result<Coord, LefError> {
    let value = token.parse::<f64>().map_err(|_| LefError::InvalidNumber {
        token: token.to_string(),
        context: "LEF DATABASE MICRONS".to_string(),
    })?;
    if !value.is_finite() || value <= 0.0 || value > Coord::MAX as f64 {
        return Err(LefError::InvalidNumber {
            token: token.to_string(),
            context: "LEF DATABASE MICRONS".to_string(),
        });
    }
    Ok(value.round() as Coord)
}

fn parse_lef_coord(token: &str, database_microns: Coord, context: &str) -> Result<Coord, LefError> {
    let value = token.parse::<f64>().map_err(|_| LefError::InvalidNumber {
        token: token.to_string(),
        context: context.to_string(),
    })?;
    let scaled = value * database_microns as f64;
    if !scaled.is_finite() || scaled < Coord::MIN as f64 || scaled > Coord::MAX as f64 {
        return Err(LefError::InvalidNumber {
            token: token.to_string(),
            context: context.to_string(),
        });
    }
    Ok(scaled.round() as Coord)
}

fn path_segment_rects(points: &[Point], width: Coord) -> Vec<Rect> {
    let half = width.abs().max(1) / 2;
    let mut rects = Vec::new();
    for segment in points.windows(2) {
        let a = segment[0];
        let b = segment[1];
        if a == b {
            continue;
        }
        if a.y == b.y {
            rects.push(Rect::new(
                Point::new(a.x.min(b.x), a.y - half),
                Point::new(a.x.max(b.x), a.y + half),
            ));
        } else if a.x == b.x {
            rects.push(Rect::new(
                Point::new(a.x - half, a.y.min(b.y)),
                Point::new(a.x + half, a.y.max(b.y)),
            ));
        }
    }
    rects
}

fn pin_marker_rect(position: Point, grid: Coord) -> Rect {
    let size = grid.max(1);
    let half = size / 2;
    Rect::new(
        Point::new(position.x - half, position.y - half),
        Point::new(position.x + size - half, position.y + size - half),
    )
}

fn existing_lef_layers(document: &Document) -> BTreeMap<String, LayerId> {
    document
        .layers
        .values()
        .map(|layer| (normalize_lef_identifier(&layer.name), layer.id))
        .collect()
}

fn layer_for_lef_name(
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    raw_name: &str,
    report: &mut LefImportReport,
) -> LayerId {
    let key = normalize_lef_identifier(raw_name);
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
    let layer_name = sanitize_lef_identifier(raw_name, "lef_layer");
    let layer = document.create_layer(layer_name.clone(), process, [0.55, 0.68, 0.4, 1.0]);
    layer_map.insert(key, layer);
    report.generated_layers.push(LefGeneratedLayer {
        name: layer_name,
        layer_id: layer,
    });
    layer
}

fn cleanup_lef_points(mut points: Vec<Point>) -> Vec<Point> {
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

fn sanitize_lef_identifier(value: &str, fallback: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
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

fn normalize_lef_identifier(value: &str) -> String {
    sanitize_lef_identifier(value, "layer").to_ascii_lowercase()
}

fn format_lef_coord(value: Coord) -> String {
    if value % DBU_PER_MICRON == 0 {
        return (value / DBU_PER_MICRON).to_string();
    }
    let mut text = format!("{:.6}", value as f64 / DBU_PER_MICRON as f64);
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" { "0".to_string() } else { text }
}

fn tokenize_lef(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for line in input.lines() {
        let mut current = String::new();
        let mut in_quote = false;
        for ch in line.chars() {
            if ch == '"' {
                if in_quote {
                    current.push(ch);
                    push_lef_token(&mut tokens, &mut current);
                    in_quote = false;
                } else {
                    push_lef_token(&mut tokens, &mut current);
                    current.push(ch);
                    in_quote = true;
                }
                continue;
            }
            if in_quote {
                current.push(ch);
                continue;
            }
            if ch == '#' {
                break;
            }
            if ch.is_whitespace() {
                push_lef_token(&mut tokens, &mut current);
            } else if ch == ';' {
                push_lef_token(&mut tokens, &mut current);
                tokens.push(ch.to_string());
            } else {
                current.push(ch);
            }
        }
        push_lef_token(&mut tokens, &mut current);
    }
    tokens
}

fn push_lef_token(tokens: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    tokens.push(std::mem::take(current));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lef_export_import_round_trips_basic_flat_geometry() {
        let mut document = Document::new("lef round trip");
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
                text: "pad_label".to_string(),
            },
        );

        let exported = export_lef_with_report(&document).expect("LEF export should succeed");
        assert!(exported.text.contains("MACRO lef_round_trip"));
        assert!(exported.text.contains("OBS"));
        assert!(exported.text.contains("PIN pad_label"));
        assert!(exported.text.contains("RECT 0 0 1 0.6 ;"));
        assert_eq!(exported.report.obstruction_count, 3);
        assert_eq!(exported.report.pin_count, 1);

        let imported = import_lef_with_report(&exported.text).expect("LEF import should succeed");
        assert_eq!(imported.report.generated_layers.len(), 0);
        assert_eq!(imported.report.obstruction_count, 3);
        assert_eq!(imported.report.pin_count, 1);
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
        assert!(imported.document.shapes.values().any(
            |shape| matches!(&shape.kind, ShapeKind::Label { text, .. } if text == "pad_label")
        ));
    }

    #[test]
    fn lef_import_generates_unknown_layers_and_reads_pin_ports() {
        let input = "\
VERSION 5.8 ;
UNITS
  DATABASE MICRONS 1000 ;
END UNITS
MACRO custom
  SIZE 2 BY 2 ;
  OBS
    LAYER CUSTOM ;
      RECT 0 0 0.1 0.05 ;
  END
  PIN PAD_A
    PORT
      LAYER metal1 ;
        RECT 0.04 0.05 0.06 0.07 ;
    END
  END PAD_A
END custom
END LIBRARY
";
        let imported =
            import_lef_with_report(input).expect("LEF import should parse simple custom layers");
        assert_eq!(imported.report.macro_count, 1);
        assert_eq!(imported.report.generated_layers.len(), 1);
        assert_eq!(imported.report.obstruction_count, 1);
        assert_eq!(imported.report.pin_count, 1);
        assert!(
            imported
                .document
                .layers
                .values()
                .any(|layer| layer.name == "CUSTOM")
        );
        assert!(
            imported
                .document
                .shapes
                .values()
                .any(|shape| matches!(shape.kind, ShapeKind::Rectangle(rect)
                if rect.width() == 100 && rect.height() == 50))
        );
        assert!(imported.document.shapes.values().any(
            |shape| matches!(&shape.kind, ShapeKind::Label { position, text }
                if *position == Point::new(50, 60) && text == "PAD_A")
        ));
    }

    #[test]
    fn lef_import_skips_bad_geometry_and_empty_pins() {
        let input = "\
MACRO bad
  OBS
    LAYER metal1 ;
      RECT 0 0 0 0 ;
      POLYGON 0 0 1 0 ;
  END
  PIN EMPTY
    DIRECTION INPUT ;
  END EMPTY
END bad
END LIBRARY
";
        let imported = import_lef_with_report(input).expect("LEF import should skip bad geometry");
        assert_eq!(imported.report.obstruction_count, 0);
        assert_eq!(imported.report.pin_count, 0);
        assert_eq!(imported.report.skipped_items.len(), 3);
    }
}
