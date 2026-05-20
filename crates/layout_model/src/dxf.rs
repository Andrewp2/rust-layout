use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use geometry_core::{Coord, Point, Polygon, Rect, Vector};

use crate::{CellId, Document, LayerId, ProcessLayer, Shape, ShapeKind, Transform};

const DXF_CURVE_SEGMENT_DEGREES: f64 = 15.0;
const DXF_MAX_CURVE_SEGMENTS: usize = 96;

#[derive(Debug)]
pub enum DxfError {
    InvalidGroupCode { line: usize, value: String },
    MissingGroupValue { line: usize },
    InvalidNumber { token: String, context: String },
}

impl fmt::Display for DxfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGroupCode { line, value } => {
                write!(formatter, "invalid DXF group code {value:?} on line {line}")
            }
            Self::MissingGroupValue { line } => {
                write!(formatter, "missing DXF group value after line {line}")
            }
            Self::InvalidNumber { token, context } => {
                write!(formatter, "invalid DXF number {token:?} in {context}")
            }
        }
    }
}

impl Error for DxfError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DxfExportReport {
    pub layer_count: usize,
    pub shape_count: usize,
    pub block_count: usize,
    pub insert_count: usize,
    pub polyline_count: usize,
    pub text_count: usize,
    pub warnings: Vec<String>,
    pub skipped_shapes: Vec<String>,
}

impl DxfExportReport {
    pub fn entity_count(&self) -> usize {
        self.insert_count + self.polyline_count + self.text_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxfExportResult {
    pub text: String,
    pub report: DxfExportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxfGeneratedLayer {
    pub name: String,
    pub layer_id: LayerId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DxfImportReport {
    pub entity_count: usize,
    pub shape_count: usize,
    pub block_count: usize,
    pub insert_count: usize,
    pub layer_count: usize,
    pub polyline_count: usize,
    pub line_count: usize,
    pub curve_count: usize,
    pub text_count: usize,
    pub generated_layers: Vec<DxfGeneratedLayer>,
    pub skipped_entities: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DxfImportResult {
    pub document: Document,
    pub report: DxfImportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DxfPair {
    code: i32,
    value: String,
    line: usize,
}

pub fn export_dxf(document: &Document) -> Result<String, DxfError> {
    Ok(export_dxf_with_report(document)?.text)
}

pub fn export_dxf_with_report(document: &Document) -> Result<DxfExportResult, DxfError> {
    let mut report = DxfExportReport::default();
    let mut text = String::new();
    let mut layer_names = BTreeMap::new();
    for layer in document.layers.values() {
        layer_names.insert(
            layer.id,
            sanitize_dxf_layer_name(&layer.name, layer.process.as_technology_name()),
        );
    }
    report.layer_count = layer_names.len();

    write_dxf_pair(&mut text, 0, "SECTION");
    write_dxf_pair(&mut text, 2, "HEADER");
    write_dxf_pair(&mut text, 9, "$ACADVER");
    write_dxf_pair(&mut text, 1, "AC1015");
    write_dxf_pair(&mut text, 0, "ENDSEC");
    write_dxf_pair(&mut text, 0, "SECTION");
    write_dxf_pair(&mut text, 2, "TABLES");
    write_dxf_pair(&mut text, 0, "TABLE");
    write_dxf_pair(&mut text, 2, "LAYER");
    write_dxf_pair(&mut text, 70, layer_names.len());
    for layer_name in layer_names.values() {
        write_dxf_pair(&mut text, 0, "LAYER");
        write_dxf_pair(&mut text, 2, layer_name);
        write_dxf_pair(&mut text, 70, 0);
        write_dxf_pair(&mut text, 62, 7);
        write_dxf_pair(&mut text, 6, "CONTINUOUS");
    }
    write_dxf_pair(&mut text, 0, "ENDTAB");
    write_dxf_pair(&mut text, 0, "ENDSEC");

    let cell_names = dxf_cell_names(document);
    write_dxf_pair(&mut text, 0, "SECTION");
    write_dxf_pair(&mut text, 2, "BLOCKS");
    for cell in document.cells.values() {
        if cell.id == document.top_cell {
            continue;
        }
        let block_name = cell_names
            .get(&cell.id)
            .cloned()
            .unwrap_or_else(|| sanitize_dxf_layer_name(&cell.name, &format!("cell_{}", cell.id.0)));
        write_dxf_pair(&mut text, 0, "BLOCK");
        write_dxf_pair(&mut text, 8, "0");
        write_dxf_pair(&mut text, 2, &block_name);
        write_dxf_pair(&mut text, 70, 0);
        write_dxf_pair(&mut text, 10, 0);
        write_dxf_pair(&mut text, 20, 0);
        for shape in cell.shapes.values() {
            write_dxf_shape(&mut text, &shape, &layer_names, &mut report, document.grid);
        }
        write_dxf_inserts(&mut text, document, cell.id, &cell_names, &mut report);
        write_dxf_pair(&mut text, 0, "ENDBLK");
        report.block_count += 1;
    }
    write_dxf_pair(&mut text, 0, "ENDSEC");

    write_dxf_pair(&mut text, 0, "SECTION");
    write_dxf_pair(&mut text, 2, "ENTITIES");

    for shape in document.shapes.values() {
        write_dxf_shape(&mut text, &shape, &layer_names, &mut report, document.grid);
    }
    if let Some(top) = document.cell(document.top_cell) {
        for shape in top.shapes.values() {
            write_dxf_shape(&mut text, &shape, &layer_names, &mut report, document.grid);
        }
        write_dxf_inserts(&mut text, document, top.id, &cell_names, &mut report);
    }

    write_dxf_pair(&mut text, 0, "ENDSEC");
    write_dxf_pair(&mut text, 0, "EOF");
    Ok(DxfExportResult { text, report })
}

pub fn import_dxf(input: &str) -> Result<Document, DxfError> {
    Ok(import_dxf_with_report(input)?.document)
}

fn write_dxf_shape(
    text: &mut String,
    shape: &Shape,
    layer_names: &BTreeMap<LayerId, String>,
    report: &mut DxfExportReport,
    grid: Coord,
) {
    let Some(layer_name) = layer_names.get(&shape.layer) else {
        report.skipped_shapes.push(format!(
            "shape #{} references missing layer {}",
            shape.id.0, shape.layer.0
        ));
        return;
    };
    match &shape.kind {
        ShapeKind::Rectangle(rect) => {
            write_dxf_lwpolyline(text, layer_name, &rect.corners(), true, None);
            report.shape_count += 1;
            report.polyline_count += 1;
        }
        ShapeKind::Polygon(poly) => {
            let points = cleanup_dxf_points(poly.points.clone());
            if points.len() < 3 {
                report
                    .skipped_shapes
                    .push(format!("polygon #{} has fewer than 3 points", shape.id.0));
                return;
            }
            write_dxf_lwpolyline(text, layer_name, &points, true, None);
            report.shape_count += 1;
            report.polyline_count += 1;
        }
        ShapeKind::Path { points, width } => {
            let points = cleanup_dxf_points(points.clone());
            if points.len() < 2 {
                report
                    .skipped_shapes
                    .push(format!("path #{} has fewer than 2 points", shape.id.0));
                return;
            }
            write_dxf_lwpolyline(text, layer_name, &points, false, Some(*width));
            report.shape_count += 1;
            report.polyline_count += 1;
        }
        ShapeKind::Via { center, size, .. } => {
            let size = size.abs().max(1);
            let min = Point::new(center.x - size / 2, center.y - size / 2);
            let rect = Rect::from_min_size(min, size, size);
            write_dxf_lwpolyline(text, layer_name, &rect.corners(), true, None);
            report.shape_count += 1;
            report.polyline_count += 1;
            report
                .warnings
                .push(format!("via #{} exported as a DXF polyline", shape.id.0));
        }
        ShapeKind::Label {
            position,
            text: label,
        } => {
            write_dxf_text(text, layer_name, *position, label, grid);
            report.shape_count += 1;
            report.text_count += 1;
        }
        ShapeKind::Measurement { a, b, label, .. } => {
            write_dxf_lwpolyline(text, layer_name, &[*a, *b], false, Some(grid.max(1)));
            write_dxf_text(text, layer_name, *b, label, grid);
            report.shape_count += 1;
            report.polyline_count += 1;
            report.text_count += 1;
            report.warnings.push(format!(
                "measurement #{} exported as DXF polyline and text",
                shape.id.0
            ));
        }
    }
}

fn write_dxf_inserts(
    text: &mut String,
    document: &Document,
    parent: CellId,
    cell_names: &BTreeMap<CellId, String>,
    report: &mut DxfExportReport,
) {
    let Some(parent_cell) = document.cell(parent) else {
        return;
    };
    for instance in parent_cell.instances.values() {
        let Some(block_name) = cell_names.get(&instance.cell) else {
            report.skipped_shapes.push(format!(
                "instance #{} references missing block cell {}",
                instance.id.0, instance.cell.0
            ));
            continue;
        };
        let array = instance.array.normalized();
        for row in 0..array.rows {
            for column in 0..array.columns {
                let transform = Transform::from_translation(array.element_offset(column, row))
                    .compose(instance.transform);
                let Some((position, rotation, x_scale, y_scale)) = dxf_insert_placement(transform)
                else {
                    report.warnings.push(format!(
                        "instance #{} has an unsupported transform and was omitted from DXF INSERT",
                        instance.id.0
                    ));
                    continue;
                };
                write_dxf_pair(text, 0, "INSERT");
                write_dxf_pair(text, 8, "0");
                write_dxf_pair(text, 2, block_name);
                write_dxf_pair(text, 10, position.x);
                write_dxf_pair(text, 20, position.y);
                if x_scale < 0 {
                    write_dxf_pair(text, 41, -1);
                }
                if y_scale < 0 {
                    write_dxf_pair(text, 42, -1);
                }
                if rotation != "0" {
                    write_dxf_pair(text, 50, rotation);
                }
                report.insert_count += 1;
            }
        }
    }
}

fn dxf_insert_placement(transform: Transform) -> Option<(Point, &'static str, i8, i8)> {
    let (rotation, x_scale, y_scale) = dxf_rotation_and_scale(transform.matrix)?;
    Some((
        Point::new(transform.translation.dx, transform.translation.dy),
        rotation,
        x_scale,
        y_scale,
    ))
}

fn dxf_rotation_and_scale(matrix: [i8; 4]) -> Option<(&'static str, i8, i8)> {
    for (rotation, rotation_matrix) in dxf_rotation_matrices() {
        if rotation_matrix == matrix {
            return Some((rotation, 1, 1));
        }
    }
    for (rotation, rotation_matrix) in dxf_rotation_matrices() {
        for x_scale in [1, -1] {
            for y_scale in [1, -1] {
                if dxf_scaled_rotation_matrix(rotation_matrix, x_scale, y_scale) == matrix {
                    return Some((rotation, x_scale, y_scale));
                }
            }
        }
    }
    None
}

fn dxf_rotation_matrix(angle: f64, x_scale: i8, y_scale: i8) -> Option<[i8; 4]> {
    let normalized = angle.rem_euclid(360.0);
    let (_, matrix) = dxf_rotation_matrices().into_iter().find(|(rotation, _)| {
        let rotation = rotation
            .parse::<f64>()
            .expect("hard-coded DXF rotation should parse");
        (normalized - rotation).abs() < 1.0e-6
            || (normalized - 360.0).abs() < 1.0e-6 && rotation == 0.0
    })?;
    Some(dxf_scaled_rotation_matrix(matrix, x_scale, y_scale))
}

fn dxf_rotation_matrices() -> [(&'static str, [i8; 4]); 4] {
    [
        ("0", [1, 0, 0, 1]),
        ("90", [0, -1, 1, 0]),
        ("180", [-1, 0, 0, -1]),
        ("270", [0, 1, -1, 0]),
    ]
}

fn dxf_scaled_rotation_matrix(rotation: [i8; 4], x_scale: i8, y_scale: i8) -> [i8; 4] {
    [
        rotation[0] * x_scale,
        rotation[1] * y_scale,
        rotation[2] * x_scale,
        rotation[3] * y_scale,
    ]
}

fn dxf_cell_names(document: &Document) -> BTreeMap<CellId, String> {
    document
        .cells
        .values()
        .map(|cell| {
            (
                cell.id,
                sanitize_dxf_layer_name(&cell.name, &format!("cell_{}", cell.id.0)),
            )
        })
        .collect()
}

pub fn import_dxf_with_report(input: &str) -> Result<DxfImportResult, DxfError> {
    let pairs = parse_dxf_pairs(input)?;
    let mut document = Document::new("DXF import");
    let mut report = DxfImportReport::default();
    let mut layer_map = existing_dxf_layers(&document);
    let mut block_map = BTreeMap::new();

    let mut index = 0;
    while index < pairs.len() {
        if !pair_value_eq(&pairs[index], 0, "SECTION") {
            index += 1;
            continue;
        }
        index += 1;
        let section_name = read_section_name(&pairs, &mut index);
        match section_name.as_deref() {
            Some("BLOCKS") => {
                parse_blocks_section(
                    &pairs,
                    &mut index,
                    &mut document,
                    &mut layer_map,
                    &mut block_map,
                    &mut report,
                )?;
            }
            Some("ENTITIES") => {
                let top_cell = document.top_cell;
                parse_entities_section(
                    &pairs,
                    &mut index,
                    &mut document,
                    top_cell,
                    &mut layer_map,
                    &mut block_map,
                    &mut report,
                )?;
            }
            _ => skip_section(&pairs, &mut index),
        }
    }

    report.layer_count = document.layers.len();
    document.name = "DXF import".to_string();
    document.ensure_hierarchy();
    Ok(DxfImportResult { document, report })
}

fn write_dxf_pair(output: &mut String, code: i32, value: impl fmt::Display) {
    output.push_str(&format!("{code}\n{value}\n"));
}

fn write_dxf_lwpolyline(
    output: &mut String,
    layer_name: &str,
    points: &[Point],
    closed: bool,
    width: Option<Coord>,
) {
    write_dxf_pair(output, 0, "LWPOLYLINE");
    write_dxf_pair(output, 8, layer_name);
    write_dxf_pair(output, 90, points.len());
    write_dxf_pair(output, 70, if closed { 1 } else { 0 });
    if let Some(width) = width {
        write_dxf_pair(output, 43, width.abs().max(1));
    }
    for point in points {
        write_dxf_pair(output, 10, point.x);
        write_dxf_pair(output, 20, point.y);
    }
}

fn write_dxf_text(
    output: &mut String,
    layer_name: &str,
    position: Point,
    label: &str,
    grid: Coord,
) {
    write_dxf_pair(output, 0, "TEXT");
    write_dxf_pair(output, 8, layer_name);
    write_dxf_pair(output, 10, position.x);
    write_dxf_pair(output, 20, position.y);
    write_dxf_pair(output, 40, (grid.max(1) * 10).max(10));
    write_dxf_pair(output, 1, sanitize_dxf_text(label));
}

fn parse_dxf_pairs(input: &str) -> Result<Vec<DxfPair>, DxfError> {
    let mut pairs = Vec::new();
    let mut lines = input.lines().enumerate();
    while let Some((index, code_line)) = lines.next() {
        let code_line = code_line.trim();
        if code_line.is_empty() {
            continue;
        }
        let code = code_line
            .parse::<i32>()
            .map_err(|_| DxfError::InvalidGroupCode {
                line: index + 1,
                value: code_line.to_string(),
            })?;
        let Some((_, value_line)) = lines.next() else {
            return Err(DxfError::MissingGroupValue { line: index + 1 });
        };
        pairs.push(DxfPair {
            code,
            value: value_line.trim().to_string(),
            line: index + 1,
        });
    }
    Ok(pairs)
}

fn parse_entities_section(
    pairs: &[DxfPair],
    index: &mut usize,
    document: &mut Document,
    target_cell: CellId,
    layer_map: &mut BTreeMap<String, LayerId>,
    block_map: &mut BTreeMap<String, CellId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    while *index < pairs.len() {
        if pairs[*index].code != 0 {
            *index += 1;
            continue;
        }
        let entity_type = normalized_dxf_keyword(&pairs[*index].value);
        *index += 1;
        match entity_type.as_str() {
            "ENDSEC" | "EOF" => break,
            "POLYLINE" => {
                parse_old_polyline_entity(pairs, index, document, target_cell, layer_map, report)?;
            }
            "LINE" | "LWPOLYLINE" | "CIRCLE" | "ARC" | "TEXT" | "MTEXT" | "INSERT" => {
                let start = *index;
                while *index < pairs.len() && pairs[*index].code != 0 {
                    *index += 1;
                }
                report.entity_count += 1;
                match entity_type.as_str() {
                    "LINE" => parse_line_entity(
                        &pairs[start..*index],
                        document,
                        target_cell,
                        layer_map,
                        report,
                    )?,
                    "LWPOLYLINE" => parse_lwpolyline_entity(
                        &pairs[start..*index],
                        document,
                        target_cell,
                        layer_map,
                        report,
                    )?,
                    "CIRCLE" => parse_circle_entity(
                        &pairs[start..*index],
                        document,
                        target_cell,
                        layer_map,
                        report,
                    )?,
                    "ARC" => parse_arc_entity(
                        &pairs[start..*index],
                        document,
                        target_cell,
                        layer_map,
                        report,
                    )?,
                    "TEXT" | "MTEXT" => parse_text_entity(
                        &pairs[start..*index],
                        document,
                        target_cell,
                        layer_map,
                        report,
                    )?,
                    "INSERT" => parse_insert_entity(
                        &pairs[start..*index],
                        document,
                        target_cell,
                        block_map,
                        report,
                    )?,
                    _ => unreachable!(),
                }
            }
            _ => {
                let skipped = entity_type;
                while *index < pairs.len() && pairs[*index].code != 0 {
                    *index += 1;
                }
                report.entity_count += 1;
                report
                    .skipped_entities
                    .push(format!("ignored unsupported DXF entity {skipped}"));
            }
        }
    }
    Ok(())
}

fn parse_blocks_section(
    pairs: &[DxfPair],
    index: &mut usize,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    block_map: &mut BTreeMap<String, CellId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    while *index < pairs.len() {
        if pairs[*index].code != 0 {
            *index += 1;
            continue;
        }
        let marker = normalized_dxf_keyword(&pairs[*index].value);
        *index += 1;
        match marker.as_str() {
            "ENDSEC" | "EOF" => break,
            "BLOCK" => parse_block_record(pairs, index, document, layer_map, block_map, report)?,
            _ => {
                while *index < pairs.len() && pairs[*index].code != 0 {
                    *index += 1;
                }
            }
        }
    }
    Ok(())
}

fn parse_block_record(
    pairs: &[DxfPair],
    index: &mut usize,
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    block_map: &mut BTreeMap<String, CellId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    let header_start = *index;
    while *index < pairs.len() && pairs[*index].code != 0 {
        *index += 1;
    }
    let Some(block_name) = first_code_value(&pairs[header_start..*index], 2) else {
        report
            .skipped_entities
            .push("ignored DXF BLOCK without name".to_string());
        skip_dxf_block_body(pairs, index);
        return Ok(());
    };
    let target_cell = cell_for_dxf_block_name(document, block_map, block_name);
    report.block_count += 1;
    while *index < pairs.len() {
        if pairs[*index].code != 0 {
            *index += 1;
            continue;
        }
        let marker = normalized_dxf_keyword(&pairs[*index].value);
        if marker == "ENDBLK" {
            *index += 1;
            while *index < pairs.len() && pairs[*index].code != 0 {
                *index += 1;
            }
            break;
        }
        parse_dxf_entity(
            pairs,
            index,
            document,
            target_cell,
            layer_map,
            block_map,
            report,
        )?;
    }
    Ok(())
}

fn parse_dxf_entity(
    pairs: &[DxfPair],
    index: &mut usize,
    document: &mut Document,
    target_cell: CellId,
    layer_map: &mut BTreeMap<String, LayerId>,
    block_map: &mut BTreeMap<String, CellId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    if *index >= pairs.len() || pairs[*index].code != 0 {
        return Ok(());
    }
    let entity_type = normalized_dxf_keyword(&pairs[*index].value);
    *index += 1;
    match entity_type.as_str() {
        "POLYLINE" => {
            parse_old_polyline_entity(pairs, index, document, target_cell, layer_map, report)
        }
        "LINE" | "LWPOLYLINE" | "CIRCLE" | "ARC" | "TEXT" | "MTEXT" | "INSERT" => {
            let start = *index;
            while *index < pairs.len() && pairs[*index].code != 0 {
                *index += 1;
            }
            report.entity_count += 1;
            match entity_type.as_str() {
                "LINE" => parse_line_entity(
                    &pairs[start..*index],
                    document,
                    target_cell,
                    layer_map,
                    report,
                ),
                "LWPOLYLINE" => parse_lwpolyline_entity(
                    &pairs[start..*index],
                    document,
                    target_cell,
                    layer_map,
                    report,
                ),
                "CIRCLE" => parse_circle_entity(
                    &pairs[start..*index],
                    document,
                    target_cell,
                    layer_map,
                    report,
                ),
                "ARC" => parse_arc_entity(
                    &pairs[start..*index],
                    document,
                    target_cell,
                    layer_map,
                    report,
                ),
                "TEXT" | "MTEXT" => parse_text_entity(
                    &pairs[start..*index],
                    document,
                    target_cell,
                    layer_map,
                    report,
                ),
                "INSERT" => parse_insert_entity(
                    &pairs[start..*index],
                    document,
                    target_cell,
                    block_map,
                    report,
                ),
                _ => unreachable!(),
            }
        }
        "ENDBLK" | "ENDSEC" | "EOF" => Ok(()),
        _ => {
            while *index < pairs.len() && pairs[*index].code != 0 {
                *index += 1;
            }
            report.entity_count += 1;
            report
                .skipped_entities
                .push(format!("ignored unsupported DXF entity {entity_type}"));
            Ok(())
        }
    }
}

fn skip_dxf_block_body(pairs: &[DxfPair], index: &mut usize) {
    while *index < pairs.len() {
        if pair_value_eq(&pairs[*index], 0, "ENDBLK") {
            *index += 1;
            while *index < pairs.len() && pairs[*index].code != 0 {
                *index += 1;
            }
            return;
        }
        *index += 1;
    }
}

fn parse_line_entity(
    pairs: &[DxfPair],
    document: &mut Document,
    target_cell: CellId,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    let layer = layer_for_dxf_name(document, layer_map, entity_layer_name(pairs), report);
    let context = "LINE entity";
    let Some(start) = point_from_codes(pairs, 10, 20, context)? else {
        report
            .skipped_entities
            .push("ignored LINE without start point".to_string());
        return Ok(());
    };
    let Some(end) = point_from_codes(pairs, 11, 21, context)? else {
        report
            .skipped_entities
            .push("ignored LINE without end point".to_string());
        return Ok(());
    };
    if start == end {
        report
            .skipped_entities
            .push("ignored zero-length LINE".to_string());
        return Ok(());
    }
    document.insert_shape_in_cell(
        target_cell,
        layer,
        ShapeKind::Path {
            points: vec![start, end],
            width: document.grid.max(1),
        },
    );
    report.line_count += 1;
    report.shape_count += 1;
    Ok(())
}

fn parse_lwpolyline_entity(
    pairs: &[DxfPair],
    document: &mut Document,
    target_cell: CellId,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    let layer = layer_for_dxf_name(document, layer_map, entity_layer_name(pairs), report);
    let flags = optional_i32_code(pairs, 70, "LWPOLYLINE flags")?.unwrap_or(0);
    let closed = flags & 1 != 0;
    let width = optional_coord_code(pairs, 43, "LWPOLYLINE width")?
        .or(optional_coord_code(pairs, 40, "LWPOLYLINE start width")?)
        .or(optional_coord_code(pairs, 41, "LWPOLYLINE end width")?)
        .unwrap_or(document.grid.max(1))
        .abs()
        .max(1);
    let points = lwpolyline_points(pairs, "LWPOLYLINE vertex")?;
    insert_polyline_geometry(
        document,
        target_cell,
        layer,
        points,
        closed,
        width,
        "LWPOLYLINE",
        report,
    );
    Ok(())
}

fn parse_circle_entity(
    pairs: &[DxfPair],
    document: &mut Document,
    target_cell: CellId,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    let layer = layer_for_dxf_name(document, layer_map, entity_layer_name(pairs), report);
    let Some(center) = point_from_codes(pairs, 10, 20, "CIRCLE center")? else {
        report
            .skipped_entities
            .push("ignored CIRCLE without center point".to_string());
        return Ok(());
    };
    let radius = optional_coord_code(pairs, 40, "CIRCLE radius")?
        .unwrap_or(0)
        .abs();
    if radius == 0 {
        report
            .skipped_entities
            .push("ignored CIRCLE with zero radius".to_string());
        return Ok(());
    }
    let points = curve_points(center, radius, 0.0, 360.0, true);
    if insert_polyline_geometry(
        document,
        target_cell,
        layer,
        points,
        true,
        document.grid.max(1),
        "CIRCLE",
        report,
    ) {
        report.curve_count += 1;
    }
    Ok(())
}

fn parse_arc_entity(
    pairs: &[DxfPair],
    document: &mut Document,
    target_cell: CellId,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    let layer = layer_for_dxf_name(document, layer_map, entity_layer_name(pairs), report);
    let Some(center) = point_from_codes(pairs, 10, 20, "ARC center")? else {
        report
            .skipped_entities
            .push("ignored ARC without center point".to_string());
        return Ok(());
    };
    let radius = optional_coord_code(pairs, 40, "ARC radius")?
        .unwrap_or(0)
        .abs();
    let Some(start_angle) = optional_f64_code(pairs, 50, "ARC start angle")? else {
        report
            .skipped_entities
            .push("ignored ARC without start angle".to_string());
        return Ok(());
    };
    let Some(end_angle) = optional_f64_code(pairs, 51, "ARC end angle")? else {
        report
            .skipped_entities
            .push("ignored ARC without end angle".to_string());
        return Ok(());
    };
    if radius == 0 {
        report
            .skipped_entities
            .push("ignored ARC with zero radius".to_string());
        return Ok(());
    }
    if normalized_arc_sweep(start_angle, end_angle) == 0.0 {
        report
            .skipped_entities
            .push("ignored ARC with zero sweep".to_string());
        return Ok(());
    }
    let points = curve_points(center, radius, start_angle, end_angle, false);
    if insert_polyline_geometry(
        document,
        target_cell,
        layer,
        points,
        false,
        document.grid.max(1),
        "ARC",
        report,
    ) {
        report.curve_count += 1;
    }
    Ok(())
}

fn parse_old_polyline_entity(
    pairs: &[DxfPair],
    index: &mut usize,
    document: &mut Document,
    target_cell: CellId,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    let header_start = *index;
    while *index < pairs.len() && pairs[*index].code != 0 {
        *index += 1;
    }
    let header = &pairs[header_start..*index];
    let flags = optional_i32_code(header, 70, "POLYLINE flags")?.unwrap_or(0);
    let closed = flags & 1 != 0;
    let width = optional_coord_code(header, 40, "POLYLINE width")?
        .or(optional_coord_code(header, 41, "POLYLINE width")?)
        .unwrap_or(document.grid.max(1))
        .abs()
        .max(1);
    let layer_name = entity_layer_name(header).to_string();
    let mut points = Vec::new();
    while *index < pairs.len() {
        if pairs[*index].code != 0 {
            *index += 1;
            continue;
        }
        let marker = normalized_dxf_keyword(&pairs[*index].value);
        if marker == "VERTEX" {
            *index += 1;
            let start = *index;
            while *index < pairs.len() && pairs[*index].code != 0 {
                *index += 1;
            }
            if let Some(point) = point_from_codes(&pairs[start..*index], 10, 20, "VERTEX")? {
                points.push(point);
            }
        } else if marker == "SEQEND" {
            *index += 1;
            while *index < pairs.len() && pairs[*index].code != 0 {
                *index += 1;
            }
            break;
        } else {
            break;
        }
    }
    report.entity_count += 1;
    let layer = layer_for_dxf_name(document, layer_map, &layer_name, report);
    insert_polyline_geometry(
        document,
        target_cell,
        layer,
        points,
        closed,
        width,
        "POLYLINE",
        report,
    );
    Ok(())
}

fn parse_text_entity(
    pairs: &[DxfPair],
    document: &mut Document,
    target_cell: CellId,
    layer_map: &mut BTreeMap<String, LayerId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    let layer = layer_for_dxf_name(document, layer_map, entity_layer_name(pairs), report);
    let Some(position) = point_from_codes(pairs, 10, 20, "TEXT position")? else {
        report
            .skipped_entities
            .push("ignored TEXT without insertion point".to_string());
        return Ok(());
    };
    let label = first_code_value(pairs, 1)
        .or_else(|| first_code_value(pairs, 3))
        .map(sanitize_dxf_text)
        .unwrap_or_else(|| "label".to_string());
    document.insert_shape_in_cell(
        target_cell,
        layer,
        ShapeKind::Label {
            position,
            text: label,
        },
    );
    report.text_count += 1;
    report.shape_count += 1;
    Ok(())
}

fn parse_insert_entity(
    pairs: &[DxfPair],
    document: &mut Document,
    target_cell: CellId,
    block_map: &mut BTreeMap<String, CellId>,
    report: &mut DxfImportReport,
) -> Result<(), DxfError> {
    let Some(block_name) = first_code_value(pairs, 2) else {
        report
            .skipped_entities
            .push("ignored INSERT without block name".to_string());
        return Ok(());
    };
    let Some(position) = point_from_codes(pairs, 10, 20, "INSERT insertion point")? else {
        report.skipped_entities.push(format!(
            "ignored INSERT {block_name} without insertion point"
        ));
        return Ok(());
    };
    let rotation = optional_f64_code(pairs, 50, "INSERT rotation")?.unwrap_or(0.0);
    let Some(x_scale) = optional_unit_scale_code(pairs, 41, "INSERT x scale")? else {
        report.skipped_entities.push(format!(
            "ignored INSERT {block_name} with unsupported x scale"
        ));
        return Ok(());
    };
    let Some(y_scale) = optional_unit_scale_code(pairs, 42, "INSERT y scale")? else {
        report.skipped_entities.push(format!(
            "ignored INSERT {block_name} with unsupported y scale"
        ));
        return Ok(());
    };
    let Some(z_scale) = optional_unit_scale_code(pairs, 43, "INSERT z scale")? else {
        report.skipped_entities.push(format!(
            "ignored INSERT {block_name} with unsupported z scale"
        ));
        return Ok(());
    };
    if z_scale < 0 {
        report.skipped_entities.push(format!(
            "ignored INSERT {block_name} with unsupported negative z scale"
        ));
        return Ok(());
    }
    let Some(matrix) = dxf_rotation_matrix(rotation, x_scale, y_scale) else {
        report.skipped_entities.push(format!(
            "ignored INSERT {block_name} with unsupported rotation {rotation}"
        ));
        return Ok(());
    };
    let cell = cell_for_dxf_block_name(document, block_map, block_name);
    let transform = Transform {
        matrix,
        translation: Vector::new(position.x, position.y),
    };
    if document
        .insert_instance(target_cell, cell, transform)
        .is_some()
    {
        report.insert_count += 1;
    } else {
        report.skipped_entities.push(format!(
            "ignored INSERT {block_name} because instance insertion failed"
        ));
    }
    Ok(())
}

fn insert_polyline_geometry(
    document: &mut Document,
    target_cell: CellId,
    layer: LayerId,
    points: Vec<Point>,
    closed: bool,
    width: Coord,
    label: &str,
    report: &mut DxfImportReport,
) -> bool {
    let points = cleanup_dxf_points(points);
    if closed {
        if points.len() < 3 {
            report
                .skipped_entities
                .push(format!("ignored degenerate {label}"));
            return false;
        }
        if let Some(rect) = rectangle_from_closed_points(&points) {
            document.insert_shape_in_cell(target_cell, layer, ShapeKind::Rectangle(rect));
        } else {
            document.insert_shape_in_cell(
                target_cell,
                layer,
                ShapeKind::Polygon(Polygon::new(points)),
            );
        }
    } else {
        if points.len() < 2 {
            report
                .skipped_entities
                .push(format!("ignored short {label}"));
            return false;
        }
        document.insert_shape_in_cell(target_cell, layer, ShapeKind::Path { points, width });
    }
    report.polyline_count += 1;
    report.shape_count += 1;
    true
}

fn read_section_name(pairs: &[DxfPair], index: &mut usize) -> Option<String> {
    while *index < pairs.len() {
        if pairs[*index].code == 2 {
            let value = normalized_dxf_keyword(&pairs[*index].value);
            *index += 1;
            return Some(value);
        }
        if pairs[*index].code == 0 {
            return None;
        }
        *index += 1;
    }
    None
}

fn skip_section(pairs: &[DxfPair], index: &mut usize) {
    while *index < pairs.len() {
        if pair_value_eq(&pairs[*index], 0, "ENDSEC") {
            *index += 1;
            return;
        }
        *index += 1;
    }
}

fn pair_value_eq(pair: &DxfPair, code: i32, value: &str) -> bool {
    pair.code == code && normalized_dxf_keyword(&pair.value) == normalized_dxf_keyword(value)
}

fn point_from_codes(
    pairs: &[DxfPair],
    x_code: i32,
    y_code: i32,
    context: &str,
) -> Result<Option<Point>, DxfError> {
    let Some(x) = optional_coord_code(pairs, x_code, context)? else {
        return Ok(None);
    };
    let Some(y) = optional_coord_code(pairs, y_code, context)? else {
        return Ok(None);
    };
    Ok(Some(Point::new(x, y)))
}

fn lwpolyline_points(pairs: &[DxfPair], context: &str) -> Result<Vec<Point>, DxfError> {
    let mut points = Vec::new();
    let mut pending_x = None;
    for pair in pairs {
        match pair.code {
            10 => {
                pending_x = Some(parse_coord(&pair.value, context)?);
            }
            20 => {
                if let Some(x) = pending_x.take() {
                    points.push(Point::new(x, parse_coord(&pair.value, context)?));
                }
            }
            _ => {}
        }
    }
    Ok(points)
}

fn optional_coord_code(
    pairs: &[DxfPair],
    code: i32,
    context: &str,
) -> Result<Option<Coord>, DxfError> {
    pairs
        .iter()
        .find(|pair| pair.code == code)
        .map(|pair| parse_coord(&pair.value, context))
        .transpose()
}

fn optional_i32_code(pairs: &[DxfPair], code: i32, context: &str) -> Result<Option<i32>, DxfError> {
    pairs
        .iter()
        .find(|pair| pair.code == code)
        .map(|pair| {
            pair.value
                .parse::<i32>()
                .map_err(|_| DxfError::InvalidNumber {
                    token: pair.value.clone(),
                    context: context.to_string(),
                })
        })
        .transpose()
}

fn optional_f64_code(pairs: &[DxfPair], code: i32, context: &str) -> Result<Option<f64>, DxfError> {
    pairs
        .iter()
        .find(|pair| pair.code == code)
        .map(|pair| {
            let value = pair
                .value
                .parse::<f64>()
                .map_err(|_| DxfError::InvalidNumber {
                    token: pair.value.clone(),
                    context: context.to_string(),
                })?;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(DxfError::InvalidNumber {
                    token: pair.value.clone(),
                    context: context.to_string(),
                })
            }
        })
        .transpose()
}

fn optional_unit_scale_code(
    pairs: &[DxfPair],
    code: i32,
    context: &str,
) -> Result<Option<i8>, DxfError> {
    let Some(scale) = optional_f64_code(pairs, code, context)? else {
        return Ok(Some(1));
    };
    if (scale - 1.0).abs() < 1.0e-6 {
        Ok(Some(1))
    } else if (scale + 1.0).abs() < 1.0e-6 {
        Ok(Some(-1))
    } else {
        Ok(None)
    }
}

fn first_code_value(pairs: &[DxfPair], code: i32) -> Option<&str> {
    pairs
        .iter()
        .find_map(|pair| (pair.code == code).then_some(pair.value.as_str()))
}

fn entity_layer_name(pairs: &[DxfPair]) -> &str {
    first_code_value(pairs, 8).unwrap_or("0")
}

fn parse_coord(token: &str, context: &str) -> Result<Coord, DxfError> {
    let value = token.parse::<f64>().map_err(|_| DxfError::InvalidNumber {
        token: token.to_string(),
        context: context.to_string(),
    })?;
    if !value.is_finite() || value < Coord::MIN as f64 || value > Coord::MAX as f64 {
        return Err(DxfError::InvalidNumber {
            token: token.to_string(),
            context: context.to_string(),
        });
    }
    Ok(value.round() as Coord)
}

fn existing_dxf_layers(document: &Document) -> BTreeMap<String, LayerId> {
    document
        .layers
        .values()
        .map(|layer| (normalize_dxf_layer_name(&layer.name), layer.id))
        .collect()
}

fn layer_for_dxf_name(
    document: &mut Document,
    layer_map: &mut BTreeMap<String, LayerId>,
    raw_name: &str,
    report: &mut DxfImportReport,
) -> LayerId {
    let key = normalize_dxf_layer_name(raw_name);
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
    let layer_name = sanitize_dxf_layer_name(raw_name, "dxf_layer");
    let layer = document.create_layer(layer_name.clone(), process, [0.25, 0.6, 0.85, 1.0]);
    layer_map.insert(key, layer);
    report.generated_layers.push(DxfGeneratedLayer {
        name: layer_name,
        layer_id: layer,
    });
    layer
}

fn cell_for_dxf_block_name(
    document: &mut Document,
    block_map: &mut BTreeMap<String, CellId>,
    raw_name: &str,
) -> CellId {
    let key = normalize_dxf_layer_name(raw_name);
    if let Some(cell) = block_map.get(&key) {
        return *cell;
    }
    if let Some(existing) = document
        .cells
        .values()
        .find(|cell| normalize_dxf_layer_name(&cell.name) == key)
    {
        block_map.insert(key, existing.id);
        return existing.id;
    }
    let cell = document.create_cell(sanitize_dxf_layer_name(raw_name, "dxf_block"));
    block_map.insert(key, cell);
    cell
}

fn rectangle_from_closed_points(points: &[Point]) -> Option<Rect> {
    if points.len() != 4 {
        return None;
    }
    let bounds = Rect::from_points(points)?;
    if bounds.width() <= 0 || bounds.height() <= 0 {
        return None;
    }
    let expected = bounds.corners().into_iter().collect::<BTreeSet<_>>();
    let actual = points.iter().copied().collect::<BTreeSet<_>>();
    (expected == actual).then_some(bounds)
}

fn curve_points(
    center: Point,
    radius: Coord,
    start_angle: f64,
    end_angle: f64,
    closed: bool,
) -> Vec<Point> {
    let sweep = if closed {
        360.0
    } else {
        normalized_arc_sweep(start_angle, end_angle)
    };
    let segments = ((sweep / DXF_CURVE_SEGMENT_DEGREES).ceil() as usize)
        .max(if closed { 16 } else { 2 })
        .min(DXF_MAX_CURVE_SEGMENTS);
    let sample_count = if closed { segments } else { segments + 1 };
    (0..sample_count)
        .map(|index| {
            let t = index as f64 / segments as f64;
            let angle = if closed {
                start_angle + 360.0 * t
            } else {
                start_angle + sweep * t
            };
            curve_point(center, radius, angle)
        })
        .collect()
}

fn curve_point(center: Point, radius: Coord, angle_degrees: f64) -> Point {
    let radians = angle_degrees.to_radians();
    Point::new(
        center
            .x
            .saturating_add((radius as f64 * radians.cos()).round() as Coord),
        center
            .y
            .saturating_add((radius as f64 * radians.sin()).round() as Coord),
    )
}

fn normalized_arc_sweep(start_angle: f64, end_angle: f64) -> f64 {
    let mut sweep = (end_angle - start_angle).rem_euclid(360.0);
    if sweep < 1.0e-9 {
        sweep = 0.0;
    }
    sweep
}

fn cleanup_dxf_points(mut points: Vec<Point>) -> Vec<Point> {
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

fn sanitize_dxf_layer_name(value: &str, fallback: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for ch in value.chars() {
        let allowed = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.');
        if allowed {
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

fn normalize_dxf_layer_name(value: &str) -> String {
    sanitize_dxf_layer_name(value, "layer").to_ascii_lowercase()
}

fn sanitize_dxf_text(value: &str) -> String {
    let mut output = value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect::<String>();
    output = output.split_whitespace().collect::<Vec<_>>().join(" ");
    if output.is_empty() {
        "label".to_string()
    } else {
        output
    }
}

fn normalized_dxf_keyword(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dxf_export_import_round_trips_basic_flat_geometry() {
        let mut document = Document::new("dxf round trip");
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

        let exported = export_dxf_with_report(&document).expect("DXF export should succeed");
        assert!(exported.text.contains("LWPOLYLINE"));
        assert!(exported.text.contains("TEXT"));
        assert_eq!(exported.report.polyline_count, 3);
        assert_eq!(exported.report.text_count, 1);

        let imported = import_dxf_with_report(&exported.text).expect("DXF import should succeed");
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
    fn dxf_import_generates_unknown_layers_and_skips_unsupported_entities() {
        let input = "\
0
SECTION
2
ENTITIES
0
LINE
8
CUSTOM
10
0
20
0
11
100
21
50
0
ELLIPSE
8
CUSTOM
10
0
20
0
40
20
0
ENDSEC
0
EOF
";
        let imported =
            import_dxf_with_report(input).expect("DXF import should parse simple custom layers");
        assert_eq!(imported.report.generated_layers.len(), 1);
        assert_eq!(imported.report.line_count, 1);
        assert_eq!(imported.report.shape_count, 1);
        assert_eq!(imported.report.skipped_entities.len(), 1);
        assert!(imported
            .document
            .layers
            .values()
            .any(|layer| layer.name == "CUSTOM"));
    }

    #[test]
    fn dxf_import_accepts_circle_and_arc_entities() {
        let input = "\
0
SECTION
2
ENTITIES
0
CIRCLE
8
metal1
10
0
20
0
40
20
0
ARC
8
metal1
10
100
20
0
40
10
50
0
51
90
0
ENDSEC
0
EOF
";
        let imported = import_dxf_with_report(input).expect("DXF curves should import");
        assert_eq!(imported.report.generated_layers.len(), 0);
        assert_eq!(imported.report.curve_count, 2);
        assert_eq!(imported.report.polyline_count, 2);
        assert_eq!(imported.report.shape_count, 2);
        assert!(imported.document.shapes.values().any(|shape| {
            matches!(
                &shape.kind,
                ShapeKind::Polygon(poly)
                    if poly.bounds()
                        == Some(Rect::new(Point::new(-20, -20), Point::new(20, 20)))
            )
        }));
        assert!(imported.document.shapes.values().any(|shape| {
            matches!(
                &shape.kind,
                ShapeKind::Path { points, .. }
                    if points.first() == Some(&Point::new(110, 0))
                        && points.last() == Some(&Point::new(100, 10))
            )
        }));
    }

    #[test]
    fn dxf_import_accepts_old_polyline_vertices() {
        let input = "\
0
SECTION
2
ENTITIES
0
POLYLINE
8
metal1
70
1
0
VERTEX
10
0
20
0
0
VERTEX
10
100
20
0
0
VERTEX
10
100
20
50
0
VERTEX
10
0
20
50
0
SEQEND
0
ENDSEC
0
EOF
";
        let imported = import_dxf_with_report(input).expect("old POLYLINE should import");
        assert_eq!(imported.report.generated_layers.len(), 0);
        assert!(imported.document.shapes.values().any(|shape| {
            matches!(shape.kind, ShapeKind::Rectangle(rect) if rect.width() == 100 && rect.height() == 50)
        }));
    }

    #[test]
    fn dxf_import_preserves_blocks_and_inserts() {
        let input = "\
0
SECTION
2
BLOCKS
0
BLOCK
2
unit
8
0
70
0
10
0
20
0
0
LWPOLYLINE
8
metal1
90
4
70
1
10
0
20
0
10
10
20
0
10
10
20
20
10
0
20
20
0
ENDBLK
0
ENDSEC
0
SECTION
2
ENTITIES
0
INSERT
2
unit
10
100
20
200
50
90
0
ENDSEC
0
EOF
";
        let imported = import_dxf_with_report(input).expect("DXF block insert should import");
        assert_eq!(imported.report.block_count, 1);
        assert_eq!(imported.report.insert_count, 1);
        assert_eq!(imported.report.polyline_count, 1);
        let unit = imported
            .document
            .cells
            .values()
            .find(|cell| cell.name == "unit")
            .expect("block should import as a cell");
        assert_eq!(unit.shapes.values().count(), 1);
        let top = imported
            .document
            .cell(imported.document.top_cell)
            .expect("top cell should exist");
        let instance = top
            .instances
            .values()
            .find(|instance| instance.cell == unit.id)
            .expect("INSERT should import as a top instance");
        assert_eq!(instance.transform.matrix, [0, -1, 1, 0]);
        assert_eq!(instance.transform.translation, Vector::new(100, 200));
    }

    #[test]
    fn dxf_export_writes_blocks_and_inserts_without_flattening_child_geometry() {
        let mut document = Document::new("dxf hierarchy");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = document.create_cell("unit block");
        document.insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(10, 20))),
        );
        document
            .insert_instance(
                document.top_cell,
                child,
                Transform {
                    matrix: [0, -1, 1, 0],
                    translation: Vector::new(100, 200),
                },
            )
            .expect("test instance should be inserted");

        let exported = export_dxf_with_report(&document).expect("DXF export should succeed");
        assert_eq!(exported.report.block_count, 1);
        assert_eq!(exported.report.insert_count, 1);
        assert_eq!(exported.report.polyline_count, 1);
        assert!(exported.text.contains("BLOCK"));
        assert!(exported.text.contains("unit_block"));
        assert!(exported.text.contains("INSERT"));
        assert!(exported.text.contains("50\n90\n"));

        let imported = import_dxf_with_report(&exported.text).expect("DXF import should succeed");
        assert_eq!(imported.report.block_count, 1);
        assert_eq!(imported.report.insert_count, 1);
        assert_eq!(imported.report.polyline_count, 1);
        assert_eq!(imported.document.shapes.len(), 0);
        assert_eq!(imported.document.flattened_shape_count_estimate(), 1);
    }

    #[test]
    fn dxf_insert_unit_negative_scale_round_trips_mirrored_instances() {
        let input = "\
0
SECTION
2
BLOCKS
0
BLOCK
2
mirror_unit
8
0
70
0
10
0
20
0
0
LINE
8
metal1
10
0
20
0
11
20
21
0
0
ENDBLK
0
ENDSEC
0
SECTION
2
ENTITIES
0
INSERT
2
mirror_unit
10
100
20
200
41
-1
42
1
0
ENDSEC
0
EOF
";
        let imported = import_dxf_with_report(input).expect("DXF mirrored insert should import");
        assert_eq!(imported.report.insert_count, 1);
        let top = imported
            .document
            .cell(imported.document.top_cell)
            .expect("top cell should exist");
        let instance = top
            .instances
            .values()
            .next()
            .expect("mirrored INSERT should become an instance");
        assert_eq!(instance.transform.matrix, [-1, 0, 0, 1]);
        assert_eq!(instance.transform.translation, Vector::new(100, 200));

        let exported = export_dxf_with_report(&imported.document).expect("DXF export should work");
        assert_eq!(exported.report.insert_count, 1);
        assert!(exported.text.contains("41\n-1\n"));
        assert!(!exported.text.contains("50\n"));
    }
}
