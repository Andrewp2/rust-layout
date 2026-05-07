use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use geometry_core::{Coord, DBU_PER_MICRON, Point, Polygon, Rect, Vector};

use crate::{
    Cell, CellId, CellInstance, Document, InstanceArray, LayerId, ProcessLayer, Shape, ShapeId,
    ShapeKind, TechnologyError, TechnologyFile, Transform,
};

const HEADER: u8 = 0x00;
const BGNLIB: u8 = 0x01;
const LIBNAME: u8 = 0x02;
const UNITS: u8 = 0x03;
const ENDLIB: u8 = 0x04;
const BGNSTR: u8 = 0x05;
const STRNAME: u8 = 0x06;
const ENDSTR: u8 = 0x07;
const BOUNDARY: u8 = 0x08;
const PATH: u8 = 0x09;
const SREF: u8 = 0x0a;
const AREF: u8 = 0x0b;
const TEXT: u8 = 0x0c;
const LAYER: u8 = 0x0d;
const DATATYPE: u8 = 0x0e;
const WIDTH: u8 = 0x0f;
const XY: u8 = 0x10;
const ENDEL: u8 = 0x11;
const SNAME: u8 = 0x12;
const COLROW: u8 = 0x13;
const TEXTTYPE: u8 = 0x16;
const STRING: u8 = 0x19;

const NO_DATA: u8 = 0;
const INT_2: u8 = 2;
const INT_4: u8 = 3;
const REAL_8: u8 = 5;
const ASCII: u8 = 6;

#[derive(Debug)]
pub enum GdsError {
    InvalidRecord(String),
    UnexpectedEof,
    MissingField(&'static str),
    Unsupported(String),
    CoordinateOverflow(Coord),
    Technology(TechnologyError),
}

impl fmt::Display for GdsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRecord(message) => write!(f, "invalid GDSII record: {message}"),
            Self::UnexpectedEof => write!(f, "unexpected end of GDSII stream"),
            Self::MissingField(field) => write!(f, "GDSII element is missing {field}"),
            Self::Unsupported(message) => write!(f, "unsupported GDSII feature: {message}"),
            Self::CoordinateOverflow(value) => {
                write!(f, "coordinate {value} does not fit in GDSII i32 range")
            }
            Self::Technology(err) => write!(f, "{err}"),
        }
    }
}

impl Error for GdsError {}

impl From<TechnologyError> for GdsError {
    fn from(value: TechnologyError) -> Self {
        Self::Technology(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedLibrary {
    name: String,
    dbu_per_micron: Coord,
    structures: Vec<ParsedStructure>,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedStructure {
    name: String,
    elements: Vec<ParsedElement>,
}

#[derive(Clone, Debug, PartialEq)]
enum ParsedElement {
    Boundary {
        layer: u16,
        datatype: u16,
        points: Vec<Point>,
    },
    Path {
        layer: u16,
        datatype: u16,
        width: Coord,
        points: Vec<Point>,
    },
    Text {
        layer: u16,
        texttype: u16,
        position: Point,
        text: String,
    },
    Sref {
        name: String,
        origin: Point,
    },
    Aref {
        name: String,
        columns: u16,
        rows: u16,
        origin: Point,
        column_step: Vector,
        row_step: Vector,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ElementKind {
    Boundary,
    Path,
    Text,
    Sref,
    Aref,
}

#[derive(Clone, Debug)]
struct ElementBuilder {
    kind: ElementKind,
    layer: Option<u16>,
    datatype: u16,
    texttype: u16,
    width: Coord,
    points: Vec<Point>,
    name: Option<String>,
    text: Option<String>,
    columns: u16,
    rows: u16,
}

impl ElementBuilder {
    fn new(kind: ElementKind) -> Self {
        Self {
            kind,
            layer: None,
            datatype: 0,
            texttype: 0,
            width: 0,
            points: Vec::new(),
            name: None,
            text: None,
            columns: 1,
            rows: 1,
        }
    }

    fn finish(self) -> Result<ParsedElement, GdsError> {
        match self.kind {
            ElementKind::Boundary => Ok(ParsedElement::Boundary {
                layer: self.layer.ok_or(GdsError::MissingField("LAYER"))?,
                datatype: self.datatype,
                points: self.points,
            }),
            ElementKind::Path => Ok(ParsedElement::Path {
                layer: self.layer.ok_or(GdsError::MissingField("LAYER"))?,
                datatype: self.datatype,
                width: self.width.abs().max(1),
                points: self.points,
            }),
            ElementKind::Text => {
                let position = self
                    .points
                    .first()
                    .copied()
                    .ok_or(GdsError::MissingField("XY"))?;
                Ok(ParsedElement::Text {
                    layer: self.layer.ok_or(GdsError::MissingField("LAYER"))?,
                    texttype: self.texttype,
                    position,
                    text: self.text.unwrap_or_default(),
                })
            }
            ElementKind::Sref => {
                let origin = self
                    .points
                    .first()
                    .copied()
                    .ok_or(GdsError::MissingField("XY"))?;
                Ok(ParsedElement::Sref {
                    name: self.name.ok_or(GdsError::MissingField("SNAME"))?,
                    origin,
                })
            }
            ElementKind::Aref => {
                if self.points.len() < 3 {
                    return Err(GdsError::MissingField("AREF XY"));
                }
                let origin = self.points[0];
                let column_extent =
                    Vector::new(self.points[1].x - origin.x, self.points[1].y - origin.y);
                let row_extent =
                    Vector::new(self.points[2].x - origin.x, self.points[2].y - origin.y);
                let column_step = divide_vector(column_extent, Coord::from(self.columns.max(1)));
                let row_step = divide_vector(row_extent, Coord::from(self.rows.max(1)));
                Ok(ParsedElement::Aref {
                    name: self.name.ok_or(GdsError::MissingField("SNAME"))?,
                    columns: self.columns.max(1),
                    rows: self.rows.max(1),
                    origin,
                    column_step,
                    row_step,
                })
            }
        }
    }
}

pub fn export_gdsii(document: &Document, technology: &TechnologyFile) -> Result<Vec<u8>, GdsError> {
    technology.validate()?;
    let names = structure_names(document);
    let mut writer = GdsWriter::new();
    writer.write_i16(HEADER, &[600])?;
    writer.write_i16(BGNLIB, &timestamp_fields())?;
    writer.write_ascii(LIBNAME, &sanitize_gds_name(&document.name, "FABRICAD"))?;
    let dbu_per_micron = technology.dbu_per_micron.max(1) as f64;
    writer.write_real8(UNITS, &[1.0 / dbu_per_micron, 1.0e-6 / dbu_per_micron])?;

    let mut cells: Vec<_> = document.cells.values().collect();
    cells.sort_by_key(|cell| {
        if cell.id == document.top_cell {
            0
        } else {
            cell.id.0
        }
    });
    for cell in cells {
        write_structure(&mut writer, document, technology, &names, cell)?;
    }
    writer.write_no_data(ENDLIB)?;
    Ok(writer.into_bytes())
}

pub fn import_gdsii(bytes: &[u8], technology: &TechnologyFile) -> Result<Document, GdsError> {
    technology.validate()?;
    let library = parse_gdsii(bytes)?;
    let top_name = infer_top_structure_name(&library)
        .ok_or_else(|| GdsError::InvalidRecord("library has no structures".to_string()))?;
    let mut document = Document::from_technology(&library.name, technology)?;
    let source_dbu_per_micron = library.dbu_per_micron.max(1);
    let target_dbu_per_micron = technology.dbu_per_micron.max(1);
    document.name = library.name.clone();
    document.grid = technology.grid;
    document.shapes.clear();
    document.cells.clear();
    document.cells.insert(
        document.top_cell,
        Cell::new(document.top_cell, top_name.clone()),
    );
    document.next_shape_id = 1;
    document.next_cell_id = document.top_cell.0 + 1;
    document.next_instance_id = 1;
    document.operation_log.clear();
    document.crdt_seen.clear();
    document.crdt_actor_clocks.clear();
    document.crdt_operation_log.clear();

    let mut cell_ids = BTreeMap::new();
    cell_ids.insert(top_name.clone(), document.top_cell);
    for structure in &library.structures {
        if structure.name == top_name {
            continue;
        }
        let cell_id = document.create_cell(&structure.name);
        cell_ids.insert(structure.name.clone(), cell_id);
    }

    let mut generated_layers = BTreeMap::new();
    for structure in &library.structures {
        let Some(parent) = cell_ids.get(&structure.name).copied() else {
            continue;
        };
        for element in &structure.elements {
            import_element(
                &mut document,
                technology,
                &cell_ids,
                &mut generated_layers,
                source_dbu_per_micron,
                target_dbu_per_micron,
                parent,
                element,
            )?;
        }
    }
    document.ensure_hierarchy();
    Ok(document)
}

fn write_structure(
    writer: &mut GdsWriter,
    document: &Document,
    technology: &TechnologyFile,
    names: &BTreeMap<CellId, String>,
    cell: &Cell,
) -> Result<(), GdsError> {
    writer.write_i16(BGNSTR, &timestamp_fields())?;
    let structure_name = names
        .get(&cell.id)
        .cloned()
        .unwrap_or_else(|| sanitize_gds_name(&cell.name, "CELL"));
    writer.write_ascii(STRNAME, &structure_name)?;

    if cell.id == document.top_cell {
        for shape in document.shapes.values() {
            write_shape(writer, document, technology, &shape)?;
        }
    }
    for shape in cell.shapes.values() {
        write_shape(writer, document, technology, &shape)?;
    }
    for instance in cell.instances.values() {
        let Some(child_name) = names.get(&instance.cell) else {
            continue;
        };
        let origin = Point::new(
            instance.transform.translation.dx,
            instance.transform.translation.dy,
        );
        let array = instance.array.normalized();
        if array.is_single() {
            writer.write_no_data(SREF)?;
            writer.write_ascii(SNAME, child_name)?;
            writer.write_xy(&[origin])?;
        } else {
            writer.write_no_data(AREF)?;
            writer.write_ascii(SNAME, child_name)?;
            writer.write_i16(COLROW, &[array.columns as i16, array.rows as i16])?;
            writer.write_xy(&[
                origin,
                Point::new(
                    origin.x + array.column_pitch.dx * Coord::from(array.columns),
                    origin.y + array.column_pitch.dy * Coord::from(array.columns),
                ),
                Point::new(
                    origin.x + array.row_pitch.dx * Coord::from(array.rows),
                    origin.y + array.row_pitch.dy * Coord::from(array.rows),
                ),
            ])?;
        }
        writer.write_no_data(ENDEL)?;
    }

    writer.write_no_data(ENDSTR)?;
    Ok(())
}

fn write_shape(
    writer: &mut GdsWriter,
    document: &Document,
    technology: &TechnologyFile,
    shape: &Shape,
) -> Result<(), GdsError> {
    match &shape.kind {
        ShapeKind::Rectangle(rect) => {
            let (layer, datatype, _) = gds_mapping(document, technology, shape.layer)?;
            writer.write_no_data(BOUNDARY)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            let mut points = rect.corners().to_vec();
            points.push(points[0]);
            writer.write_xy(&points)?;
            writer.write_no_data(ENDEL)?;
        }
        ShapeKind::Polygon(poly) => {
            if poly.points.len() < 3 {
                return Ok(());
            }
            let (layer, datatype, _) = gds_mapping(document, technology, shape.layer)?;
            writer.write_no_data(BOUNDARY)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            let mut points = poly.points.clone();
            if points.first() != points.last() {
                points.push(points[0]);
            }
            writer.write_xy(&points)?;
            writer.write_no_data(ENDEL)?;
        }
        ShapeKind::Path { points, width } => {
            if points.len() < 2 {
                return Ok(());
            }
            let (layer, datatype, _) = gds_mapping(document, technology, shape.layer)?;
            writer.write_no_data(PATH)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            writer.write_i32(WIDTH, &[coord_to_i32(*width)?])?;
            writer.write_xy(points)?;
            writer.write_no_data(ENDEL)?;
        }
        ShapeKind::Via { center, size, .. } => {
            let half = *size / 2;
            let rect = Rect::new(
                Point::new(center.x - half, center.y - half),
                Point::new(center.x + half, center.y + half),
            );
            let (layer, datatype, _) = gds_mapping(document, technology, shape.layer)?;
            writer.write_no_data(BOUNDARY)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            let mut points = rect.corners().to_vec();
            points.push(points[0]);
            writer.write_xy(&points)?;
            writer.write_no_data(ENDEL)?;
        }
        ShapeKind::Label { position, text } => {
            let (layer, _, texttype) = gds_mapping(document, technology, shape.layer)?;
            writer.write_no_data(TEXT)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(TEXTTYPE, &[u16_to_i16(texttype)?])?;
            writer.write_xy(&[*position])?;
            writer.write_ascii(STRING, text)?;
            writer.write_no_data(ENDEL)?;
        }
        ShapeKind::Measurement { a, b, label } => {
            let (layer, datatype, texttype) = gds_mapping(document, technology, shape.layer)?;
            writer.write_no_data(PATH)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            writer.write_i32(WIDTH, &[10])?;
            writer.write_xy(&[*a, *b])?;
            writer.write_no_data(ENDEL)?;

            writer.write_no_data(TEXT)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(TEXTTYPE, &[u16_to_i16(texttype)?])?;
            writer.write_xy(&[*a])?;
            writer.write_ascii(STRING, label)?;
            writer.write_no_data(ENDEL)?;
        }
    }
    Ok(())
}

fn import_element(
    document: &mut Document,
    technology: &TechnologyFile,
    cell_ids: &BTreeMap<String, CellId>,
    generated_layers: &mut BTreeMap<(u16, u16, bool), LayerId>,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
    parent: CellId,
    element: &ParsedElement,
) -> Result<(), GdsError> {
    match element {
        ParsedElement::Boundary {
            layer,
            datatype,
            points,
        } => {
            let layer_id = resolve_import_layer(
                document,
                technology,
                generated_layers,
                *layer,
                *datatype,
                false,
            );
            let points = scale_points(points, source_dbu_per_micron, target_dbu_per_micron);
            let kind = import_boundary_kind(&points);
            insert_shape_in_import_cell(document, parent, layer_id, kind);
        }
        ParsedElement::Path {
            layer,
            datatype,
            width,
            points,
        } => {
            if points.len() >= 2 {
                let layer_id = resolve_import_layer(
                    document,
                    technology,
                    generated_layers,
                    *layer,
                    *datatype,
                    false,
                );
                insert_shape_in_import_cell(
                    document,
                    parent,
                    layer_id,
                    ShapeKind::Path {
                        points: scale_points(points, source_dbu_per_micron, target_dbu_per_micron),
                        width: scale_coord(*width, source_dbu_per_micron, target_dbu_per_micron)
                            .abs()
                            .max(1),
                    },
                );
            }
        }
        ParsedElement::Text {
            layer,
            texttype,
            position,
            text,
        } => {
            let layer_id = resolve_import_layer(
                document,
                technology,
                generated_layers,
                *layer,
                *texttype,
                true,
            );
            insert_shape_in_import_cell(
                document,
                parent,
                layer_id,
                ShapeKind::Label {
                    position: scale_point(*position, source_dbu_per_micron, target_dbu_per_micron),
                    text: text.clone(),
                },
            );
        }
        ParsedElement::Sref { name, origin } => {
            let child = *cell_ids
                .get(name)
                .ok_or_else(|| GdsError::InvalidRecord(format!("unknown SREF target {name:?}")))?;
            insert_import_instance(
                document,
                parent,
                child,
                name,
                scale_point(*origin, source_dbu_per_micron, target_dbu_per_micron),
                InstanceArray::single(),
            );
        }
        ParsedElement::Aref {
            name,
            columns,
            rows,
            origin,
            column_step,
            row_step,
        } => {
            let child = *cell_ids
                .get(name)
                .ok_or_else(|| GdsError::InvalidRecord(format!("unknown AREF target {name:?}")))?;
            insert_import_instance(
                document,
                parent,
                child,
                name,
                scale_point(*origin, source_dbu_per_micron, target_dbu_per_micron),
                InstanceArray {
                    columns: u32::from(*columns),
                    rows: u32::from(*rows),
                    column_pitch: scale_vector(
                        *column_step,
                        source_dbu_per_micron,
                        target_dbu_per_micron,
                    ),
                    row_pitch: scale_vector(
                        *row_step,
                        source_dbu_per_micron,
                        target_dbu_per_micron,
                    ),
                },
            );
        }
    }
    Ok(())
}

fn parse_gdsii(bytes: &[u8]) -> Result<ParsedLibrary, GdsError> {
    let mut reader = GdsReader::new(bytes);
    let mut library = ParsedLibrary {
        name: "gds_import".to_string(),
        dbu_per_micron: DBU_PER_MICRON,
        structures: Vec::new(),
    };
    let mut structure: Option<ParsedStructure> = None;
    let mut element: Option<ElementBuilder> = None;

    while let Some(record) = reader.next_record()? {
        match record.record_type {
            LIBNAME => library.name = record.as_ascii()?,
            UNITS => {
                let units = record.as_real8()?;
                if let Some(user_units_per_dbu) = units.first().copied()
                    && user_units_per_dbu.is_finite()
                    && user_units_per_dbu > 0.0
                {
                    library.dbu_per_micron = (1.0 / user_units_per_dbu).round().max(1.0) as Coord;
                }
            }
            BGNSTR => {
                structure = Some(ParsedStructure {
                    name: String::new(),
                    elements: Vec::new(),
                });
            }
            STRNAME => {
                if let Some(structure) = &mut structure {
                    structure.name = record.as_ascii()?;
                }
            }
            ENDSTR => {
                let mut finished = structure
                    .take()
                    .ok_or_else(|| GdsError::InvalidRecord("ENDSTR without BGNSTR".to_string()))?;
                if finished.name.trim().is_empty() {
                    finished.name = format!("STRUCT_{}", library.structures.len() + 1);
                }
                library.structures.push(finished);
            }
            BOUNDARY => element = Some(ElementBuilder::new(ElementKind::Boundary)),
            PATH => element = Some(ElementBuilder::new(ElementKind::Path)),
            TEXT => element = Some(ElementBuilder::new(ElementKind::Text)),
            SREF => element = Some(ElementBuilder::new(ElementKind::Sref)),
            AREF => element = Some(ElementBuilder::new(ElementKind::Aref)),
            LAYER => {
                if let Some(element) = &mut element {
                    element.layer = Some(record.first_u16()?);
                }
            }
            DATATYPE => {
                if let Some(element) = &mut element {
                    element.datatype = record.first_u16()?;
                }
            }
            TEXTTYPE => {
                if let Some(element) = &mut element {
                    element.texttype = record.first_u16()?;
                }
            }
            WIDTH => {
                if let Some(element) = &mut element {
                    element.width = Coord::from(record.first_i32()?);
                }
            }
            XY => {
                if let Some(element) = &mut element {
                    element.points = record.as_xy()?;
                }
            }
            SNAME => {
                if let Some(element) = &mut element {
                    element.name = Some(record.as_ascii()?);
                }
            }
            STRING => {
                if let Some(element) = &mut element {
                    element.text = Some(record.as_ascii()?);
                }
            }
            COLROW => {
                if let Some(element) = &mut element {
                    let values = record.as_u16s()?;
                    if values.len() >= 2 {
                        element.columns = values[0].max(1);
                        element.rows = values[1].max(1);
                    }
                }
            }
            ENDEL => {
                let finished = element
                    .take()
                    .ok_or_else(|| GdsError::InvalidRecord("ENDEL without element".to_string()))?
                    .finish()?;
                let structure = structure.as_mut().ok_or_else(|| {
                    GdsError::InvalidRecord("element outside structure".to_string())
                })?;
                structure.elements.push(finished);
            }
            ENDLIB => break,
            _ => {}
        }
    }
    Ok(library)
}

fn structure_names(document: &Document) -> BTreeMap<CellId, String> {
    let mut used = BTreeSet::new();
    let mut names = BTreeMap::new();
    for cell in document.cells.values() {
        let fallback = if cell.id == document.top_cell {
            "TOP"
        } else {
            "CELL"
        };
        let base = sanitize_gds_name(&cell.name, fallback);
        let mut candidate = base.clone();
        let mut suffix = 1;
        while !used.insert(candidate.clone()) {
            candidate = truncate_gds_name(&format!("{base}_{suffix}"));
            suffix += 1;
        }
        names.insert(cell.id, candidate);
    }
    names
}

fn infer_top_structure_name(library: &ParsedLibrary) -> Option<String> {
    let mut referenced = BTreeSet::new();
    for structure in &library.structures {
        for element in &structure.elements {
            match element {
                ParsedElement::Sref { name, .. } | ParsedElement::Aref { name, .. } => {
                    referenced.insert(name.clone());
                }
                ParsedElement::Boundary { .. }
                | ParsedElement::Path { .. }
                | ParsedElement::Text { .. } => {}
            }
        }
    }
    library
        .structures
        .iter()
        .rev()
        .find(|structure| !referenced.contains(&structure.name))
        .or_else(|| library.structures.last())
        .map(|structure| structure.name.clone())
}

fn insert_shape_in_import_cell(
    document: &mut Document,
    cell: CellId,
    layer: LayerId,
    kind: ShapeKind,
) -> ShapeId {
    let id = document.allocate_shape_id();
    let shape = Shape {
        id,
        layer,
        net: None,
        kind,
        name: None,
    };
    if cell == document.top_cell {
        document.shapes.insert(id, shape);
    } else if let Some(cell) = document.cells.get_mut(&cell) {
        cell.shapes.insert(id, shape);
    }
    id
}

fn insert_import_instance(
    document: &mut Document,
    parent: CellId,
    child: CellId,
    name: &str,
    origin: Point,
    array: InstanceArray,
) {
    let id = document.allocate_instance_id();
    let instance = CellInstance {
        id,
        name: Some(sanitize_gds_name(name, "inst").to_ascii_lowercase()),
        cell: child,
        transform: Transform::translate(origin.x, origin.y),
        array,
    };
    if let Some(parent) = document.cells.get_mut(&parent) {
        parent.instances.insert(id, instance);
    }
}

fn import_boundary_kind(points: &[Point]) -> ShapeKind {
    let mut points = points.to_vec();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    if let Some(rect) = axis_aligned_rect(&points) {
        ShapeKind::Rectangle(rect)
    } else {
        ShapeKind::Polygon(Polygon::new(points))
    }
}

fn axis_aligned_rect(points: &[Point]) -> Option<Rect> {
    if points.len() != 4 {
        return None;
    }
    let xs: BTreeSet<_> = points.iter().map(|point| point.x).collect();
    let ys: BTreeSet<_> = points.iter().map(|point| point.y).collect();
    if xs.len() != 2 || ys.len() != 2 {
        return None;
    }
    let min = Point::new(*xs.iter().next()?, *ys.iter().next()?);
    let max = Point::new(*xs.iter().next_back()?, *ys.iter().next_back()?);
    let expected: BTreeSet<_> = Rect::new(min, max).corners().into_iter().collect();
    let actual: BTreeSet<_> = points.iter().copied().collect();
    (expected == actual).then_some(Rect::new(min, max))
}

fn resolve_import_layer(
    document: &mut Document,
    technology: &TechnologyFile,
    generated_layers: &mut BTreeMap<(u16, u16, bool), LayerId>,
    gds_layer: u16,
    gds_type: u16,
    is_text: bool,
) -> LayerId {
    let technology_layer = if is_text {
        technology.layer_for_gds_text(gds_layer, gds_type)
    } else {
        technology.layer_for_gds_geometry(gds_layer, gds_type)
    };
    if let Some(layer) = technology_layer.and_then(|layer| layer.id)
        && document.layers.contains_key(&layer)
    {
        return layer;
    }
    if let Some((id, _)) = document.layers.iter().find(|(_, layer)| {
        layer.gds_layer == Some(gds_layer)
            && if is_text {
                layer.gds_texttype == gds_type
            } else {
                layer.gds_datatype == gds_type
            }
    }) {
        return *id;
    }
    let key = (gds_layer, gds_type, is_text);
    if let Some(id) = generated_layers.get(&key) {
        return *id;
    }
    let name = if is_text {
        format!("gds_{gds_layer}_text_{gds_type}")
    } else {
        format!("gds_{gds_layer}_{gds_type}")
    };
    let id = document.create_layer(
        name,
        ProcessLayer::Annotation,
        generated_layer_color(gds_layer),
    );
    if let Some(layer) = document.layers.get_mut(&id) {
        layer.purpose = if is_text {
            format!("imported GDSII texttype {gds_type}")
        } else {
            format!("imported GDSII datatype {gds_type}")
        };
        layer.gds_layer = Some(gds_layer);
        if is_text {
            layer.gds_texttype = gds_type;
        } else {
            layer.gds_datatype = gds_type;
        }
    }
    generated_layers.insert(key, id);
    id
}

fn gds_mapping(
    document: &Document,
    technology: &TechnologyFile,
    layer: LayerId,
) -> Result<(u16, u16, u16), GdsError> {
    if let Some(mapping) = technology.gds_mapping_for_layer(layer) {
        return Ok(mapping);
    }
    if let Some(layer) = document.layer(layer) {
        if let Some(gds_layer) = layer.gds_layer {
            return Ok((gds_layer, layer.gds_datatype, layer.gds_texttype));
        }
        let gds_layer = u16::try_from(layer.id.0)
            .map_err(|_| GdsError::Unsupported(format!("layer id {} exceeds GDSII", layer.id.0)))?;
        return Ok((gds_layer, 0, 0));
    }
    let gds_layer = u16::try_from(layer.0)
        .map_err(|_| GdsError::Unsupported(format!("layer id {} exceeds GDSII", layer.0)))?;
    Ok((gds_layer, 0, 0))
}

fn generated_layer_color(layer: u16) -> [f32; 4] {
    let hue = (u32::from(layer) * 47) % 360;
    let phase = hue as f32 / 360.0;
    [
        0.35 + 0.45 * phase,
        0.65 - 0.25 * phase,
        0.85 - 0.35 * (phase - 0.5).abs(),
        0.44,
    ]
}

fn sanitize_gds_name(name: &str, fallback: &str) -> String {
    let mut sanitized = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric()
            || character == '_'
            || character == '$'
            || character == '?'
        {
            sanitized.push(character);
        } else if character.is_whitespace() || character == '-' {
            sanitized.push('_');
        }
    }
    if sanitized.trim_matches('_').is_empty() {
        sanitized = fallback.to_string();
    }
    if sanitized
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        sanitized.insert(0, '_');
    }
    truncate_gds_name(&sanitized)
}

fn truncate_gds_name(name: &str) -> String {
    name.chars().take(32).collect()
}

fn timestamp_fields() -> [i16; 12] {
    [2026, 1, 1, 0, 0, 0, 2026, 1, 1, 0, 0, 0]
}

fn divide_vector(vector: Vector, divisor: Coord) -> Vector {
    if divisor <= 0 {
        return vector;
    }
    Vector::new(vector.dx / divisor, vector.dy / divisor)
}

fn scale_points(
    points: &[Point],
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
) -> Vec<Point> {
    points
        .iter()
        .copied()
        .map(|point| scale_point(point, source_dbu_per_micron, target_dbu_per_micron))
        .collect()
}

fn scale_point(point: Point, source_dbu_per_micron: Coord, target_dbu_per_micron: Coord) -> Point {
    Point::new(
        scale_coord(point.x, source_dbu_per_micron, target_dbu_per_micron),
        scale_coord(point.y, source_dbu_per_micron, target_dbu_per_micron),
    )
}

fn scale_vector(
    vector: Vector,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
) -> Vector {
    Vector::new(
        scale_coord(vector.dx, source_dbu_per_micron, target_dbu_per_micron),
        scale_coord(vector.dy, source_dbu_per_micron, target_dbu_per_micron),
    )
}

fn scale_coord(value: Coord, source_dbu_per_micron: Coord, target_dbu_per_micron: Coord) -> Coord {
    let source = i128::from(source_dbu_per_micron.max(1));
    let target = i128::from(target_dbu_per_micron.max(1));
    let numerator = i128::from(value) * target;
    let rounded = if numerator >= 0 {
        (numerator + source / 2) / source
    } else {
        (numerator - source / 2) / source
    };
    rounded.clamp(i128::from(Coord::MIN), i128::from(Coord::MAX)) as Coord
}

fn coord_to_i32(value: Coord) -> Result<i32, GdsError> {
    i32::try_from(value).map_err(|_| GdsError::CoordinateOverflow(value))
}

fn u16_to_i16(value: u16) -> Result<i16, GdsError> {
    i16::try_from(value)
        .map_err(|_| GdsError::Unsupported(format!("GDSII layer/type {value} exceeds i16")))
}

#[derive(Clone, Debug)]
struct Record {
    record_type: u8,
    data_type: u8,
    data: Vec<u8>,
}

impl Record {
    fn as_ascii(&self) -> Result<String, GdsError> {
        if self.data_type != ASCII {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not ASCII",
                self.record_type
            )));
        }
        let text = self
            .data
            .iter()
            .copied()
            .take_while(|byte| *byte != 0)
            .collect::<Vec<_>>();
        Ok(String::from_utf8_lossy(&text).trim_end().to_string())
    }

    fn as_u16s(&self) -> Result<Vec<u16>, GdsError> {
        if self.data_type != INT_2 || self.data.len() % 2 != 0 {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not INT_2",
                self.record_type
            )));
        }
        Ok(self
            .data
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect())
    }

    fn first_u16(&self) -> Result<u16, GdsError> {
        self.as_u16s()?
            .first()
            .copied()
            .ok_or(GdsError::MissingField("INT_2 value"))
    }

    fn first_i32(&self) -> Result<i32, GdsError> {
        if self.data_type != INT_4 || self.data.len() < 4 {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not INT_4",
                self.record_type
            )));
        }
        Ok(i32::from_be_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
        ]))
    }

    fn as_xy(&self) -> Result<Vec<Point>, GdsError> {
        if self.data_type != INT_4 || self.data.len() % 8 != 0 {
            return Err(GdsError::InvalidRecord(
                "XY record has bad size".to_string(),
            ));
        }
        let mut points = Vec::new();
        for chunk in self.data.chunks_exact(8) {
            let x = i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let y = i32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
            points.push(Point::new(Coord::from(x), Coord::from(y)));
        }
        Ok(points)
    }

    fn as_real8(&self) -> Result<Vec<f64>, GdsError> {
        if self.data_type != REAL_8 || self.data.len() % 8 != 0 {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not REAL_8",
                self.record_type
            )));
        }
        Ok(self
            .data
            .chunks_exact(8)
            .map(|chunk| {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(chunk);
                decode_gds_real8(bytes)
            })
            .collect())
    }
}

struct GdsReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> GdsReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn next_record(&mut self) -> Result<Option<Record>, GdsError> {
        if self.offset >= self.bytes.len() {
            return Ok(None);
        }
        if self.bytes.len() - self.offset < 4 {
            return Err(GdsError::UnexpectedEof);
        }
        let length = u16::from_be_bytes([self.bytes[self.offset], self.bytes[self.offset + 1]]);
        let length = usize::from(length);
        if length < 4 || self.offset + length > self.bytes.len() {
            return Err(GdsError::InvalidRecord(format!(
                "bad record length {length} at byte {}",
                self.offset
            )));
        }
        let record_type = self.bytes[self.offset + 2];
        let data_type = self.bytes[self.offset + 3];
        let data = self.bytes[self.offset + 4..self.offset + length].to_vec();
        self.offset += length;
        Ok(Some(Record {
            record_type,
            data_type,
            data,
        }))
    }
}

struct GdsWriter {
    bytes: Vec<u8>,
}

impl GdsWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn write_no_data(&mut self, record_type: u8) -> Result<(), GdsError> {
        self.write_record(record_type, NO_DATA, &[])
    }

    fn write_i16(&mut self, record_type: u8, values: &[i16]) -> Result<(), GdsError> {
        let mut data = Vec::with_capacity(values.len() * 2);
        for value in values {
            data.extend_from_slice(&value.to_be_bytes());
        }
        self.write_record(record_type, INT_2, &data)
    }

    fn write_i32(&mut self, record_type: u8, values: &[i32]) -> Result<(), GdsError> {
        let mut data = Vec::with_capacity(values.len() * 4);
        for value in values {
            data.extend_from_slice(&value.to_be_bytes());
        }
        self.write_record(record_type, INT_4, &data)
    }

    fn write_real8(&mut self, record_type: u8, values: &[f64]) -> Result<(), GdsError> {
        let mut data = Vec::with_capacity(values.len() * 8);
        for value in values {
            data.extend_from_slice(&encode_gds_real8(*value));
        }
        self.write_record(record_type, REAL_8, &data)
    }

    fn write_ascii(&mut self, record_type: u8, value: &str) -> Result<(), GdsError> {
        let mut data = value.as_bytes().to_vec();
        if data.len() % 2 != 0 {
            data.push(0);
        }
        self.write_record(record_type, ASCII, &data)
    }

    fn write_xy(&mut self, points: &[Point]) -> Result<(), GdsError> {
        let mut data = Vec::with_capacity(points.len() * 8);
        for point in points {
            data.extend_from_slice(&coord_to_i32(point.x)?.to_be_bytes());
            data.extend_from_slice(&coord_to_i32(point.y)?.to_be_bytes());
        }
        self.write_record(XY, INT_4, &data)
    }

    fn write_record(
        &mut self,
        record_type: u8,
        data_type: u8,
        data: &[u8],
    ) -> Result<(), GdsError> {
        let length = data.len() + 4;
        let length = u16::try_from(length).map_err(|_| {
            GdsError::Unsupported(format!("record {:02x} exceeds 65535 bytes", record_type))
        })?;
        self.bytes.extend_from_slice(&length.to_be_bytes());
        self.bytes.push(record_type);
        self.bytes.push(data_type);
        self.bytes.extend_from_slice(data);
        Ok(())
    }
}

fn encode_gds_real8(value: f64) -> [u8; 8] {
    if value == 0.0 || !value.is_finite() {
        return [0; 8];
    }
    let sign = if value.is_sign_negative() { 0x80 } else { 0x00 };
    let mut mantissa = value.abs();
    let mut exponent = 0i32;
    while mantissa >= 1.0 {
        mantissa /= 16.0;
        exponent += 1;
    }
    while mantissa < 0.0625 {
        mantissa *= 16.0;
        exponent -= 1;
    }
    let mut mantissa_int = (mantissa * ((1u64 << 56) as f64)).round() as u64;
    if mantissa_int >= (1u64 << 56) {
        mantissa_int >>= 4;
        exponent += 1;
    }
    let mut bytes = [0u8; 8];
    bytes[0] = sign | ((exponent + 64) as u8 & 0x7f);
    let mantissa_bytes = mantissa_int.to_be_bytes();
    bytes[1..].copy_from_slice(&mantissa_bytes[1..]);
    bytes
}

fn decode_gds_real8(bytes: [u8; 8]) -> f64 {
    if bytes == [0; 8] {
        return 0.0;
    }
    let sign = if bytes[0] & 0x80 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from(bytes[0] & 0x7f) - 64;
    let mut mantissa_bytes = [0u8; 8];
    mantissa_bytes[1..].copy_from_slice(&bytes[1..]);
    let mantissa = u64::from_be_bytes(mantissa_bytes) as f64 / ((1u64 << 56) as f64);
    sign * mantissa * 16f64.powi(exponent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayerId, default_technology};

    #[test]
    fn gds_real8_round_trips_common_units() {
        for value in [0.001, 1.0e-9, 0.0005, 2.5] {
            let decoded = decode_gds_real8(encode_gds_real8(value));
            assert!((decoded - value).abs() / value < 1.0e-12);
        }
    }

    #[test]
    fn export_import_round_trip_preserves_hierarchy_geometry() {
        let technology = default_technology();
        let mut document = Document::new("GDS round trip");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let poly = document.layer_by_process(ProcessLayer::Poly).unwrap();
        let annotation = document.layer_by_process(ProcessLayer::Annotation).unwrap();
        document.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-400, -200), 800, 400)),
        );
        let child = document.create_cell("unit-cell");
        document
            .insert_shape_in_cell(
                child,
                poly,
                ShapeKind::Path {
                    points: vec![Point::new(0, 0), Point::new(500, 0)],
                    width: 120,
                },
            )
            .unwrap();
        document
            .insert_shape_in_cell(
                child,
                annotation,
                ShapeKind::Label {
                    position: Point::new(0, 240),
                    text: "IN".to_string(),
                },
            )
            .unwrap();
        document
            .insert_instance_in_top(child, Transform::translate(2_000, 3_000))
            .unwrap();

        let bytes = export_gdsii(&document, &technology).unwrap();
        let imported = import_gdsii(&bytes, &technology).unwrap();

        assert_eq!(imported.shapes.len(), 1);
        assert_eq!(imported.cell(imported.top_cell).unwrap().instances.len(), 1);
        let child = imported
            .cells
            .values()
            .find(|cell| cell.name == "unit_cell")
            .unwrap();
        assert_eq!(child.shapes.len(), 2);
        assert!(
            child
                .shapes
                .values()
                .any(|shape| matches!(shape.kind, ShapeKind::Path { width: 120, .. }))
        );
        assert!(
            child.shapes.values().any(|shape| {
                matches!(&shape.kind, ShapeKind::Label { text, .. } if text == "IN")
            })
        );
        let instance = imported
            .cell(imported.top_cell)
            .unwrap()
            .instances
            .values()
            .next()
            .unwrap();
        assert_eq!(instance.transform, Transform::translate(2_000, 3_000));
        assert_eq!(imported.visible_flattened_shapes().len(), 3);
    }

    #[test]
    fn import_preserves_aref_arrays_as_instance_arrays() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "AREF_TEST").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();

        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "UNIT").unwrap();
        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[4]).unwrap();
        writer.write_i16(DATATYPE, &[0]).unwrap();
        writer
            .write_xy(&[
                Point::new(0, 0),
                Point::new(100, 0),
                Point::new(100, 50),
                Point::new(0, 50),
                Point::new(0, 0),
            ])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();

        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(AREF).unwrap();
        writer.write_ascii(SNAME, "UNIT").unwrap();
        writer.write_i16(COLROW, &[3, 2]).unwrap();
        writer
            .write_xy(&[Point::new(0, 0), Point::new(300, 0), Point::new(0, 200)])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let imported = import_gdsii(&writer.into_bytes(), &default_technology()).unwrap();
        let top = imported.cell(imported.top_cell).unwrap();

        assert_eq!(top.instances.len(), 1);
        let instance = top.instances.values().next().unwrap();
        assert_eq!(instance.transform, Transform::translate(0, 0));
        assert_eq!(instance.array.columns, 3);
        assert_eq!(instance.array.rows, 2);
        assert_eq!(instance.array.column_pitch, Vector::new(100, 0));
        assert_eq!(instance.array.row_pitch, Vector::new(0, 100));
        assert_eq!(imported.visible_flattened_shapes().len(), 6);
    }

    #[test]
    fn export_import_round_trip_preserves_instance_arrays() {
        let mut doc = Document::new("array round trip");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = doc.create_cell("UNIT");
        doc.insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        )
        .unwrap();
        let instance_id = doc
            .insert_instance_in_top(child, Transform::translate(500, 600))
            .unwrap();
        let mut instance = doc.instance(doc.top_cell, instance_id).unwrap().clone();
        instance.array = InstanceArray {
            columns: 3,
            rows: 2,
            column_pitch: Vector::new(200, 0),
            row_pitch: Vector::new(0, 160),
        };
        doc.apply_operation_without_log(&crate::Operation::ReplaceInstance {
            parent: doc.top_cell,
            id: instance_id,
            instance,
        });

        let bytes = export_gdsii(&doc, &default_technology()).unwrap();
        let imported = import_gdsii(&bytes, &default_technology()).unwrap();
        let imported_top = imported.cell(imported.top_cell).unwrap();
        let imported_instance = imported_top.instances.values().next().unwrap();

        assert_eq!(imported_top.instances.len(), 1);
        assert_eq!(imported_instance.transform, Transform::translate(500, 600));
        assert_eq!(imported_instance.array.columns, 3);
        assert_eq!(imported_instance.array.rows, 2);
        assert_eq!(imported_instance.array.column_pitch, Vector::new(200, 0));
        assert_eq!(imported_instance.array.row_pitch, Vector::new(0, 160));
        assert_eq!(imported.visible_flattened_shapes().len(), 6);
    }

    #[test]
    fn unknown_gds_layers_are_imported_as_mapped_document_layers() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "UNKNOWN_LAYER").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[55]).unwrap();
        writer.write_i16(DATATYPE, &[7]).unwrap();
        writer
            .write_xy(&[
                Point::new(0, 0),
                Point::new(10, 0),
                Point::new(10, 10),
                Point::new(0, 10),
                Point::new(0, 0),
            ])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let imported = import_gdsii(&writer.into_bytes(), &default_technology()).unwrap();
        let layer = imported
            .layers
            .values()
            .find(|layer| layer.gds_layer == Some(55))
            .unwrap();

        assert_eq!(layer.gds_datatype, 7);
        assert_ne!(layer.id, LayerId(55));
        assert_eq!(imported.shapes.values().next().unwrap().layer, layer.id);
    }

    #[test]
    fn import_scales_coordinates_to_active_technology_dbu() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "SCALED_UNITS").unwrap();
        writer.write_real8(UNITS, &[0.0005, 5.0e-10]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[4]).unwrap();
        writer.write_i16(DATATYPE, &[0]).unwrap();
        writer
            .write_xy(&[
                Point::new(0, 0),
                Point::new(2_000, 0),
                Point::new(2_000, 1_000),
                Point::new(0, 1_000),
                Point::new(0, 0),
            ])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let imported = import_gdsii(&writer.into_bytes(), &default_technology()).unwrap();
        let shape = imported.shapes.values().next().unwrap();

        assert_eq!(
            shape.kind.bounds(),
            Rect::from_min_size(Point::new(0, 0), 1_000, 500)
        );
    }
}
