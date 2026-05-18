use std::{collections::BTreeMap, error::Error, fmt};

use geometry_core::{Coord, Point, Polygon, Rect};

use crate::{Document, LayerId, ProcessLayer, ShapeKind};

#[derive(Debug)]
pub enum DefError {
    InvalidNumber { token: String, context: String },
    InvalidSyntax(String),
    UnexpectedEof { context: String },
}

impl fmt::Display for DefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber { token, context } => {
                write!(formatter, "invalid DEF number {token:?} in {context}")
            }
            Self::InvalidSyntax(message) => write!(formatter, "invalid DEF syntax: {message}"),
            Self::UnexpectedEof { context } => {
                write!(formatter, "unexpected end of DEF while reading {context}")
            }
        }
    }
}

impl Error for DefError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DefExportReport {
    pub layer_count: usize,
    pub shape_count: usize,
    pub fill_count: usize,
    pub route_count: usize,
    pub pin_count: usize,
    pub warnings: Vec<String>,
    pub skipped_shapes: Vec<String>,
}

impl DefExportReport {
    pub fn object_count(&self) -> usize {
        self.fill_count + self.route_count + self.pin_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefExportResult {
    pub text: String,
    pub report: DefExportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefGeneratedLayer {
    pub name: String,
    pub layer_id: LayerId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DefImportReport {
    pub section_count: usize,
    pub shape_count: usize,
    pub layer_count: usize,
    pub fill_count: usize,
    pub route_count: usize,
    pub pin_count: usize,
    pub generated_layers: Vec<DefGeneratedLayer>,
    pub skipped_items: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DefImportResult {
    pub document: Document,
    pub report: DefImportReport,
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

    fn next_required(&mut self, context: &str) -> Result<String, DefError> {
        self.next().ok_or_else(|| DefError::UnexpectedEof {
            context: context.to_string(),
        })
    }

    fn eat(&mut self, token: &str) -> bool {
        if self.peek() == Some(token) {
            self.index += 1;
            true
        } else {
            false
        }
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

pub fn export_def(document: &Document) -> Result<String, DefError> {
    Ok(export_def_with_report(document)?.text)
}

pub fn export_def_with_report(document: &Document) -> Result<DefExportResult, DefError> {
    let mut report = DefExportReport::default();
    let mut layer_names = BTreeMap::new();
    for layer in document.layers.values() {
        layer_names.insert(
            layer.id,
            sanitize_def_identifier(&layer.name, layer.process.as_technology_name()),
        );
    }
    report.layer_count = layer_names.len();

    let mut fills = Vec::new();
    let mut routes = Vec::new();
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
                fills.push(DefFill::Rect {
                    layer_name,
                    rect: *rect,
                });
                report.shape_count += 1;
                report.fill_count += 1;
            }
            ShapeKind::Polygon(poly) => {
                let points = cleanup_def_points(poly.points.clone());
                if points.len() < 3 {
                    report
                        .skipped_shapes
                        .push(format!("polygon #{} has fewer than 3 points", shape.id.0));
                    continue;
                }
                fills.push(DefFill::Polygon { layer_name, points });
                report.shape_count += 1;
                report.fill_count += 1;
            }
            ShapeKind::Path { points, width } => {
                let points = cleanup_def_points(points.clone());
                if points.len() < 2 {
                    report
                        .skipped_shapes
                        .push(format!("path #{} has fewer than 2 points", shape.id.0));
                    continue;
                }
                routes.push(DefRoute {
                    layer_name,
                    name: format!("glassworks_route_{}", shape.id.0),
                    width: width.abs().max(1),
                    points,
                });
                report.shape_count += 1;
                report.route_count += 1;
            }
            ShapeKind::Via { center, size, .. } => {
                let size = size.abs().max(1);
                let min = Point::new(center.x - size / 2, center.y - size / 2);
                fills.push(DefFill::Rect {
                    layer_name,
                    rect: Rect::from_min_size(min, size, size),
                });
                report.shape_count += 1;
                report.fill_count += 1;
                report.warnings.push(format!(
                    "via #{} exported as a DEF fill rectangle",
                    shape.id.0
                ));
            }
            ShapeKind::Label { position, text } => {
                pins.push(DefPin {
                    layer_name,
                    name: sanitize_def_identifier(text, &format!("label_{}", shape.id.0)),
                    position: *position,
                });
                report.shape_count += 1;
                report.pin_count += 1;
            }
            ShapeKind::Measurement { a, b, label, .. } => {
                routes.push(DefRoute {
                    layer_name: layer_name.clone(),
                    name: format!("glassworks_measurement_{}", shape.id.0),
                    width: document.grid.max(1),
                    points: vec![*a, *b],
                });
                pins.push(DefPin {
                    layer_name,
                    name: sanitize_def_identifier(label, &format!("measurement_{}", shape.id.0)),
                    position: *b,
                });
                report.shape_count += 1;
                report.route_count += 1;
                report.pin_count += 1;
                report.warnings.push(format!(
                    "measurement #{} exported as DEF route and pin",
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
    let mut text = String::new();
    text.push_str("VERSION 5.8 ;\n");
    text.push_str("DIVIDERCHAR \"/\" ;\n");
    text.push_str("BUSBITCHARS \"[]\" ;\n");
    text.push_str(&format!(
        "DESIGN {} ;\n",
        sanitize_def_identifier(&document.name, "glassworks")
    ));
    text.push_str("UNITS DISTANCE MICRONS 1000 ;\n");
    text.push_str(&format!(
        "DIEAREA ( {} {} ) ( {} {} ) ;\n",
        bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y
    ));
    write_def_fills(&mut text, &fills);
    write_def_specialnets(&mut text, &routes);
    write_def_pins(&mut text, &pins, document.grid);
    text.push_str("END DESIGN\n");
    Ok(DefExportResult { text, report })
}

pub fn import_def(input: &str) -> Result<Document, DefError> {
    Ok(import_def_with_report(input)?.document)
}

pub fn import_def_with_report(input: &str) -> Result<DefImportResult, DefError> {
    let mut cursor = TokenCursor::new(tokenize_def(input));
    let mut document = Document::new("DEF import");
    let mut report = DefImportReport::default();
    let mut layer_map = existing_def_layers(&document);

    while let Some(token) = cursor.next() {
        match token.to_ascii_uppercase().as_str() {
            "DESIGN" => {
                if let Some(name) = cursor.next() {
                    document.name = name;
                }
                cursor.skip_statement();
            }
            "FILLS" => {
                report.section_count += 1;
                parse_fills_section(&mut cursor, &mut document, &mut layer_map, &mut report)?;
            }
            "SPECIALNETS" => {
                report.section_count += 1;
                parse_nets_section(
                    &mut cursor,
                    &mut document,
                    &mut layer_map,
                    &mut report,
                    true,
                )?;
            }
            "NETS" => {
                report.section_count += 1;
                parse_nets_section(
                    &mut cursor,
                    &mut document,
                    &mut layer_map,
                    &mut report,
                    false,
                )?;
            }
            "PINS" => {
                report.section_count += 1;
                parse_pins_section(&mut cursor, &mut document, &mut layer_map, &mut report)?;
            }
            "END" if cursor.eat_keyword("DESIGN") => break,
            _ => {}
        }
    }
    report.layer_count = document.layers.len();
    document.ensure_hierarchy();
    Ok(DefImportResult { document, report })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DefFill {
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
struct DefRoute {
    layer_name: String,
    name: String,
    width: Coord,
    points: Vec<Point>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DefPin {
    layer_name: String,
    name: String,
    position: Point,
}

fn write_def_fills(output: &mut String, fills: &[DefFill]) {
    output.push_str(&format!("FILLS {} ;\n", fills.len()));
    for fill in fills {
        match fill {
            DefFill::Rect { layer_name, rect } => {
                output.push_str(&format!(
                    "  - LAYER {layer_name} + RECT ( {} {} ) ( {} {} ) ;\n",
                    rect.min.x, rect.min.y, rect.max.x, rect.max.y
                ));
            }
            DefFill::Polygon { layer_name, points } => {
                output.push_str(&format!("  - LAYER {layer_name} + POLYGON"));
                for point in points {
                    output.push_str(&format!(" ( {} {} )", point.x, point.y));
                }
                output.push_str(" ;\n");
            }
        }
    }
    output.push_str("END FILLS\n");
}

fn write_def_specialnets(output: &mut String, routes: &[DefRoute]) {
    output.push_str(&format!("SPECIALNETS {} ;\n", routes.len()));
    for route in routes {
        output.push_str(&format!(
            "  - {}\n    + ROUTED {} {}",
            route.name, route.layer_name, route.width
        ));
        for point in &route.points {
            output.push_str(&format!(" ( {} {} )", point.x, point.y));
        }
        output.push_str(" ;\n");
    }
    output.push_str("END SPECIALNETS\n");
}

fn write_def_pins(output: &mut String, pins: &[DefPin], grid: Coord) {
    output.push_str(&format!("PINS {} ;\n", pins.len()));
    let marker = grid.max(1);
    for pin in pins {
        output.push_str(&format!(
            "  - {} + NET {} + DIRECTION INPUT + USE SIGNAL\n",
            pin.name, pin.name
        ));
        output.push_str(&format!(
            "    + LAYER {} ( 0 0 ) ( {} {} )\n",
            pin.layer_name, marker, marker
        ));
        output.push_str(&format!(
            "    + PLACED ( {} {} ) N ;\n",
            pin.position.x, pin.position.y
        ));
    }
    output.push_str("END PINS\n");
}

fn parse_fills_section(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DefImportReport,
) -> Result<(), DefError> {
    cursor.skip_statement();
    while let Some(token) = cursor.peek() {
        if token.eq_ignore_ascii_case("END") {
            cursor.next();
            cursor.eat_keyword("FILLS");
            break;
        }
        if !cursor.eat("-") {
            cursor.next();
            continue;
        }
        if !cursor.eat_keyword("LAYER") {
            report
                .skipped_items
                .push("ignored FILL entry without LAYER".to_string());
            cursor.skip_statement();
            continue;
        }
        let layer_name = cursor.next_required("FILL layer name")?;
        let layer = layer_for_def_name(document, layer_map, &layer_name, report);
        parse_fill_statement(cursor, document, layer, report)?;
    }
    Ok(())
}

fn parse_fill_statement(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer: LayerId,
    report: &mut DefImportReport,
) -> Result<(), DefError> {
    while let Some(token) = cursor.peek() {
        if token == ";" {
            cursor.next();
            break;
        }
        if cursor.eat("+") {
            continue;
        }
        if cursor.eat_keyword("RECT") {
            let a = parse_def_point(cursor, "FILL RECT", None)?;
            let b = parse_def_point(cursor, "FILL RECT", None)?;
            let rect = Rect::new(a, b);
            if rect.width() <= 0 || rect.height() <= 0 {
                report
                    .skipped_items
                    .push("ignored degenerate DEF RECT".to_string());
                continue;
            }
            document.insert_shape(layer, ShapeKind::Rectangle(rect));
            report.fill_count += 1;
            report.shape_count += 1;
        } else if cursor.eat_keyword("POLYGON") {
            let mut points = Vec::new();
            while cursor.peek() == Some("(") {
                points.push(parse_def_point(cursor, "FILL POLYGON", None)?);
            }
            let points = cleanup_def_points(points);
            if points.len() < 3 {
                report
                    .skipped_items
                    .push("ignored degenerate DEF POLYGON".to_string());
                continue;
            }
            document.insert_shape(layer, ShapeKind::Polygon(Polygon::new(points)));
            report.fill_count += 1;
            report.shape_count += 1;
        } else {
            cursor.next();
        }
    }
    Ok(())
}

fn parse_nets_section(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DefImportReport,
    special: bool,
) -> Result<(), DefError> {
    cursor.skip_statement();
    let section_name = if special { "SPECIALNETS" } else { "NETS" };
    while let Some(token) = cursor.peek() {
        if token.eq_ignore_ascii_case("END") {
            cursor.next();
            cursor.eat_keyword(section_name);
            break;
        }
        if !cursor.eat("-") {
            cursor.next();
            continue;
        }
        let _net_name = cursor.next_required("DEF net name")?;
        parse_net_statement(cursor, document, layer_map, report)?;
    }
    Ok(())
}

fn parse_net_statement(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DefImportReport,
) -> Result<(), DefError> {
    while let Some(token) = cursor.peek() {
        if token == ";" {
            cursor.next();
            break;
        }
        if cursor.eat("+") {
            let keyword = cursor.next_required("DEF net attribute")?;
            match keyword.to_ascii_uppercase().as_str() {
                "ROUTED" | "FIXED" | "COVER" => {
                    parse_route_statement(cursor, document, layer_map, report)?
                }
                _ => {}
            }
        } else {
            cursor.next();
        }
    }
    Ok(())
}

fn parse_route_statement(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DefImportReport,
) -> Result<(), DefError> {
    let mut layer_name = cursor.next_required("DEF route layer")?;
    let mut layer = layer_for_def_name(document, layer_map, &layer_name, report);
    let mut width = optional_route_width(cursor)?.unwrap_or(document.grid.max(1));
    let mut points = Vec::new();
    let mut previous = None;
    while let Some(token) = cursor.peek() {
        if token == ";" || token == "+" {
            break;
        }
        if cursor.eat_keyword("NEW") {
            insert_def_route(document, layer, width, &mut points, report);
            layer_name = cursor.next_required("DEF NEW route layer")?;
            layer = layer_for_def_name(document, layer_map, &layer_name, report);
            width = optional_route_width(cursor)?.unwrap_or(width);
            previous = None;
        } else if cursor.peek() == Some("(") {
            let point = parse_def_point(cursor, "DEF route point", previous)?;
            previous = Some(point);
            points.push(point);
        } else {
            cursor.next();
        }
    }
    insert_def_route(document, layer, width, &mut points, report);
    Ok(())
}

fn insert_def_route(
    document: &mut Document,
    layer: LayerId,
    width: Coord,
    points: &mut Vec<Point>,
    report: &mut DefImportReport,
) {
    let cleaned = cleanup_def_points(std::mem::take(points));
    if cleaned.len() < 2 {
        if !cleaned.is_empty() {
            report
                .skipped_items
                .push("ignored short DEF route".to_string());
        }
        return;
    }
    document.insert_shape(
        layer,
        ShapeKind::Path {
            points: cleaned,
            width: width.abs().max(1),
        },
    );
    report.route_count += 1;
    report.shape_count += 1;
}

fn parse_pins_section(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DefImportReport,
) -> Result<(), DefError> {
    cursor.skip_statement();
    while let Some(token) = cursor.peek() {
        if token.eq_ignore_ascii_case("END") {
            cursor.next();
            cursor.eat_keyword("PINS");
            break;
        }
        if !cursor.eat("-") {
            cursor.next();
            continue;
        }
        let pin_name = cursor.next_required("DEF pin name")?;
        parse_pin_statement(cursor, document, layer_map, report, pin_name)?;
    }
    Ok(())
}

fn parse_pin_statement(
    cursor: &mut TokenCursor,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DefImportReport,
    pin_name: String,
) -> Result<(), DefError> {
    let mut layer = document.layer_by_process(ProcessLayer::Annotation);
    let mut position = None;
    while let Some(token) = cursor.peek() {
        if token == ";" {
            cursor.next();
            break;
        }
        if cursor.eat("+") {
            let keyword = cursor.next_required("DEF pin attribute")?;
            match keyword.to_ascii_uppercase().as_str() {
                "LAYER" => {
                    let layer_name = cursor.next_required("DEF pin layer")?;
                    layer = Some(layer_for_def_name(document, layer_map, &layer_name, report));
                }
                "PLACED" | "FIXED" | "COVER" => {
                    position = Some(parse_def_point(cursor, "DEF pin placement", None)?);
                    if cursor
                        .peek()
                        .is_some_and(|token| token != ";" && token != "+")
                    {
                        cursor.next();
                    }
                }
                _ => {}
            }
        } else {
            cursor.next();
        }
    }
    let Some(layer) = layer else {
        report
            .skipped_items
            .push(format!("ignored DEF pin {pin_name} without layer"));
        return Ok(());
    };
    let Some(position) = position else {
        report
            .skipped_items
            .push(format!("ignored DEF pin {pin_name} without placement"));
        return Ok(());
    };
    document.insert_shape(
        layer,
        ShapeKind::Label {
            position,
            text: pin_name,
        },
    );
    report.pin_count += 1;
    report.shape_count += 1;
    Ok(())
}

fn optional_route_width(cursor: &mut TokenCursor) -> Result<Option<Coord>, DefError> {
    let Some(token) = cursor.peek() else {
        return Ok(None);
    };
    if token == "(" || token == ";" || token == "+" || token.eq_ignore_ascii_case("NEW") {
        return Ok(None);
    }
    if token.parse::<f64>().is_err() {
        return Ok(None);
    }
    parse_coord(
        &cursor
            .next()
            .expect("peeked route width token should be present"),
        "DEF route width",
    )
    .map(Some)
}

fn parse_def_point(
    cursor: &mut TokenCursor,
    context: &str,
    previous: Option<Point>,
) -> Result<Point, DefError> {
    if !cursor.eat("(") {
        return Err(DefError::InvalidSyntax(format!(
            "expected point while reading {context}"
        )));
    }
    let x_token = cursor.next_required(context)?;
    let y_token = cursor.next_required(context)?;
    let x = parse_coord_or_repeat(&x_token, context, previous.map(|point| point.x))?;
    let y = parse_coord_or_repeat(&y_token, context, previous.map(|point| point.y))?;
    while let Some(token) = cursor.next() {
        if token == ")" {
            break;
        }
    }
    Ok(Point::new(x, y))
}

fn parse_coord_or_repeat(
    token: &str,
    context: &str,
    previous: Option<Coord>,
) -> Result<Coord, DefError> {
    if token == "*" {
        previous.ok_or_else(|| {
            DefError::InvalidSyntax(format!("DEF repeat coordinate appears first in {context}"))
        })
    } else {
        parse_coord(token, context)
    }
}

fn parse_coord(token: &str, context: &str) -> Result<Coord, DefError> {
    let value = token.parse::<f64>().map_err(|_| DefError::InvalidNumber {
        token: token.to_string(),
        context: context.to_string(),
    })?;
    if !value.is_finite() || value < Coord::MIN as f64 || value > Coord::MAX as f64 {
        return Err(DefError::InvalidNumber {
            token: token.to_string(),
            context: context.to_string(),
        });
    }
    Ok(value.round() as Coord)
}

fn existing_def_layers(document: &Document) -> BTreeMap<String, LayerId> {
    document
        .layers
        .values()
        .map(|layer| (normalize_def_identifier(&layer.name), layer.id))
        .collect()
}

fn layer_for_def_name(
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    raw_name: &str,
    report: &mut DefImportReport,
) -> LayerId {
    let key = normalize_def_identifier(raw_name);
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
    let layer_name = sanitize_def_identifier(raw_name, "def_layer");
    let layer = document.create_layer(layer_name.clone(), process, [0.35, 0.7, 0.45, 1.0]);
    layer_map.insert(key, layer);
    report.generated_layers.push(DefGeneratedLayer {
        name: layer_name,
        layer_id: layer,
    });
    layer
}

fn cleanup_def_points(mut points: Vec<Point>) -> Vec<Point> {
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

fn sanitize_def_identifier(value: &str, fallback: &str) -> String {
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

fn normalize_def_identifier(value: &str) -> String {
    sanitize_def_identifier(value, "layer").to_ascii_lowercase()
}

fn tokenize_def(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for line in input.lines() {
        let mut current = String::new();
        for ch in line.chars() {
            if ch == '#' {
                break;
            }
            if ch.is_whitespace() {
                push_def_token(&mut tokens, &mut current);
            } else if matches!(ch, '(' | ')' | ';') {
                push_def_token(&mut tokens, &mut current);
                tokens.push(ch.to_string());
            } else {
                current.push(ch);
            }
        }
        push_def_token(&mut tokens, &mut current);
    }
    tokens
}

fn push_def_token(tokens: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    tokens.push(std::mem::take(current));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn def_export_import_round_trips_basic_flat_geometry() {
        let mut document = Document::new("def round trip");
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

        let exported = export_def_with_report(&document).expect("DEF export should succeed");
        assert!(exported.text.contains("FILLS 2 ;"));
        assert!(exported.text.contains("SPECIALNETS 1 ;"));
        assert!(exported.text.contains("PINS 1 ;"));
        assert_eq!(exported.report.fill_count, 2);
        assert_eq!(exported.report.route_count, 1);
        assert_eq!(exported.report.pin_count, 1);

        let imported = import_def_with_report(&exported.text).expect("DEF import should succeed");
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
            |shape| matches!(&shape.kind, ShapeKind::Label { text, .. } if text == "pad_label")
        ));
    }

    #[test]
    fn def_import_generates_unknown_layers_and_reads_routes() {
        let input = "\
VERSION 5.8 ;
DESIGN custom ;
UNITS DISTANCE MICRONS 1000 ;
FILLS 1 ;
  - LAYER CUSTOM + RECT ( 0 0 ) ( 100 50 ) ;
END FILLS
SPECIALNETS 1 ;
  - route_a + ROUTED CUSTOM 12 ( 0 0 ) ( 100 0 ) ( * 50 ) ;
END SPECIALNETS
END DESIGN
";
        let imported =
            import_def_with_report(input).expect("DEF import should parse simple custom layers");
        assert_eq!(imported.report.generated_layers.len(), 1);
        assert_eq!(imported.report.fill_count, 1);
        assert_eq!(imported.report.route_count, 1);
        assert_eq!(imported.report.shape_count, 2);
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
                .any(|shape| matches!(shape.kind, ShapeKind::Path { width: 12, .. }))
        );
    }

    #[test]
    fn def_import_reads_pin_labels_and_skips_bad_geometry() {
        let input = "\
VERSION 5.8 ;
DESIGN pins ;
FILLS 1 ;
  - LAYER metal1 + RECT ( 0 0 ) ( 0 0 ) ;
END FILLS
PINS 1 ;
  - PAD_A + NET PAD_A + DIRECTION INPUT + USE SIGNAL
    + LAYER metal1 ( 0 0 ) ( 10 10 )
    + PLACED ( 40 50 ) N ;
END PINS
END DESIGN
";
        let imported = import_def_with_report(input).expect("DEF pin import should succeed");
        assert_eq!(imported.report.pin_count, 1);
        assert_eq!(imported.report.fill_count, 0);
        assert_eq!(imported.report.skipped_items.len(), 1);
        assert!(imported.document.shapes.values().any(
            |shape| matches!(&shape.kind, ShapeKind::Label { position, text }
                if *position == Point::new(40, 50) && text == "PAD_A")
        ));
    }
}
