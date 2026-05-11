use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use geometry_core::{Coord, DBU_PER_MICRON, Point, Polygon, Rect, Vector};
use tracing::warn;

use crate::{
    Cell, CellId, CellInstance, Document, InstanceArray, InstanceId, LayerId, ProcessLayer, Shape,
    ShapeId, ShapeKind, TechnologyError, TechnologyFile, Transform,
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
const TEXTNODE: u8 = 0x14;
const NODE: u8 = 0x15;
const TEXTTYPE: u8 = 0x16;
const STRING: u8 = 0x19;
const NODETYPE: u8 = 0x2a;
const BOX: u8 = 0x2d;
const BOXTYPE: u8 = 0x2e;

const NO_DATA: u8 = 0;
const INT_2: u8 = 2;
const INT_4: u8 = 3;
const REAL_8: u8 = 5;
const ASCII: u8 = 6;
const GDS_MAX_COLROW_DIMENSION: u32 = i16::MAX as u32;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdsGeneratedLayer {
    pub gds_layer: u16,
    pub gds_type: u16,
    pub is_text: bool,
    pub layer_id: LayerId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdsImportReport {
    pub library_name: String,
    pub source_dbu_per_micron: Coord,
    pub structure_count: usize,
    pub element_count: usize,
    pub generated_layers: Vec<GdsGeneratedLayer>,
    pub skipped_elements: Vec<GdsImportSkippedElement>,
    pub warnings: Vec<GdsImportWarning>,
}

#[derive(Clone, Debug)]
pub struct GdsImportResult {
    pub document: Document,
    pub report: GdsImportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdsImportSkippedElement {
    pub cell_id: CellId,
    pub element_kind: String,
    pub gds_layer: Option<u16>,
    pub gds_type: Option<u16>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GdsImportWarningKind {
    NormalizedUnits,
    SplitIncomingLayerMapping,
    NormalizedPathWidth,
    NormalizedArefDimensions,
    DuplicateStructureName,
    CoordinateClamped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdsImportWarning {
    pub kind: GdsImportWarningKind,
    pub gds_layer: Option<u16>,
    pub gds_type: Option<u16>,
    pub is_text: Option<bool>,
    pub layer_id: Option<LayerId>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GdsImportMappingKey {
    gds_layer: u16,
    gds_type: u16,
    is_text: bool,
}

#[derive(Clone, Copy, Debug)]
struct GdsScaleWarningContext {
    element_kind: &'static str,
    gds_layer: Option<u16>,
    gds_type: Option<u16>,
    is_text: Option<bool>,
    layer_id: Option<LayerId>,
}

impl GdsScaleWarningContext {
    const fn new(
        element_kind: &'static str,
        gds_layer: Option<u16>,
        gds_type: Option<u16>,
        is_text: Option<bool>,
        layer_id: Option<LayerId>,
    ) -> Self {
        Self {
            element_kind,
            gds_layer,
            gds_type,
            is_text,
            layer_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdsSkippedElement {
    pub cell_id: CellId,
    pub shape_id: Option<ShapeId>,
    pub instance_id: Option<InstanceId>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GdsExportWarningKind {
    FallbackLayerMapping,
    MissingLayerFallback,
    AmbiguousDocumentLayerMapping,
    UnsupportedInstanceTransform,
    NormalizedPathWidth,
    NonRoundTrippableMetadata,
    NonRoundTrippableShapeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdsExportWarning {
    pub kind: GdsExportWarningKind,
    pub cell_id: CellId,
    pub shape_id: Option<ShapeId>,
    pub instance_id: Option<InstanceId>,
    pub layer_id: Option<LayerId>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdsExportReport {
    pub library_name: String,
    pub dbu_per_micron: Coord,
    pub structure_count: usize,
    pub boundary_count: usize,
    pub path_count: usize,
    pub text_count: usize,
    pub sref_count: usize,
    pub aref_count: usize,
    pub skipped_elements: Vec<GdsSkippedElement>,
    pub warnings: Vec<GdsExportWarning>,
}

impl GdsExportReport {
    pub fn element_count(&self) -> usize {
        self.boundary_count + self.path_count + self.text_count + self.sref_count + self.aref_count
    }
}

#[derive(Clone, Debug)]
pub struct GdsExportResult {
    pub bytes: Vec<u8>,
    pub report: GdsExportReport,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedLibrary {
    name: String,
    dbu_per_micron: Coord,
    warnings: Vec<GdsImportWarning>,
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
        raw_width: Coord,
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
        raw_columns: u16,
        raw_rows: u16,
        columns: u16,
        rows: u16,
        origin: Point,
        column_step: Vector,
        row_step: Vector,
    },
    Unsupported {
        kind: String,
        layer: Option<u16>,
        gds_type: Option<u16>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ElementKind {
    Boundary,
    Path,
    Text,
    Sref,
    Aref,
    Unsupported(&'static str),
}

impl ElementKind {
    fn label(self) -> &'static str {
        match self {
            Self::Boundary => "BOUNDARY",
            Self::Path => "PATH",
            Self::Text => "TEXT",
            Self::Sref => "SREF",
            Self::Aref => "AREF",
            Self::Unsupported(kind) => kind,
        }
    }
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
                width: {
                    let width = normalize_gds_path_width(self.width);
                    if width != self.width {
                        warn!(
                            raw_width = self.width,
                            effective_width = width,
                            "GDS PATH width was outside supported range"
                        );
                    }
                    width
                },
                raw_width: self.width,
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
                    text: self.text.unwrap_or_else(|| {
                        warn!("GDS TEXT element missing STRING; using empty text");
                        String::new()
                    }),
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
                let raw_columns = self.columns;
                let raw_rows = self.rows;
                let columns = raw_columns.max(1);
                let rows = raw_rows.max(1);
                if columns != raw_columns || rows != raw_rows {
                    warn!(
                        columns = raw_columns,
                        rows = raw_rows,
                        effective_columns = columns,
                        effective_rows = rows,
                        "GDS AREF dimensions were outside supported range"
                    );
                }
                let column_step = divide_vector(column_extent, Coord::from(columns));
                let row_step = divide_vector(row_extent, Coord::from(rows));
                Ok(ParsedElement::Aref {
                    name: self.name.ok_or(GdsError::MissingField("SNAME"))?,
                    raw_columns,
                    raw_rows,
                    columns,
                    rows,
                    origin,
                    column_step,
                    row_step,
                })
            }
            ElementKind::Unsupported(kind) => Ok(ParsedElement::Unsupported {
                kind: kind.to_string(),
                layer: self.layer,
                gds_type: Some(self.datatype),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GdsExportMappingKey {
    gds_layer: u16,
    gds_type: u16,
    is_text: bool,
}

fn ambiguous_gds_export_mappings(
    document: &Document,
    technology: &TechnologyFile,
) -> BTreeMap<GdsExportMappingKey, Vec<LayerId>> {
    let mut mappings: BTreeMap<GdsExportMappingKey, Vec<LayerId>> = BTreeMap::new();
    for (layer_id, layer) in document.layers.iter() {
        let mapping = technology
            .gds_mapping_for_layer(*layer_id)
            .or_else(|| {
                layer
                    .gds_layer
                    .map(|gds_layer| (gds_layer, layer.gds_datatype, layer.gds_texttype))
            })
            .or_else(|| {
                u16::try_from(layer_id.0)
                    .ok()
                    .map(|gds_layer| (gds_layer, 0, 0))
            });
        let Some((gds_layer, gds_datatype, gds_texttype)) = mapping else {
            continue;
        };
        mappings
            .entry(GdsExportMappingKey {
                gds_layer,
                gds_type: gds_datatype,
                is_text: false,
            })
            .or_default()
            .push(*layer_id);
        mappings
            .entry(GdsExportMappingKey {
                gds_layer,
                gds_type: gds_texttype,
                is_text: true,
            })
            .or_default()
            .push(*layer_id);
    }
    mappings.retain(|_, layers| layers.len() > 1);
    mappings
}

pub fn export_gdsii(document: &Document, technology: &TechnologyFile) -> Result<Vec<u8>, GdsError> {
    Ok(export_gdsii_with_report(document, technology)?.bytes)
}

pub fn export_gdsii_with_report(
    document: &Document,
    technology: &TechnologyFile,
) -> Result<GdsExportResult, GdsError> {
    technology.validate()?;
    let names = structure_names(document);
    let mut writer = GdsWriter::new();
    writer.write_i16(HEADER, &[600])?;
    writer.write_i16(BGNLIB, &timestamp_fields())?;
    let library_name = sanitize_gds_name(&document.name, "FABRICAD");
    writer.write_ascii(LIBNAME, &library_name)?;
    let dbu_per_micron = technology.dbu_per_micron.max(1) as f64;
    if technology.dbu_per_micron < 1 {
        warn!(
            dbu_per_micron = technology.dbu_per_micron,
            "technology dbu_per_micron below supported range during GDS export; using 1"
        );
    }
    writer.write_real8(UNITS, &[1.0 / dbu_per_micron, 1.0e-6 / dbu_per_micron])?;

    let mut cells: Vec<_> = document.cells.values().collect();
    cells.sort_by_key(|cell| {
        if cell.id == document.top_cell {
            0
        } else {
            cell.id.0
        }
    });
    let mut report = GdsExportReport {
        library_name,
        dbu_per_micron: technology.dbu_per_micron.max(1),
        structure_count: cells.len(),
        boundary_count: 0,
        path_count: 0,
        text_count: 0,
        sref_count: 0,
        aref_count: 0,
        skipped_elements: Vec::new(),
        warnings: Vec::new(),
    };
    let ambiguous_mappings = ambiguous_gds_export_mappings(document, technology);
    for cell in cells {
        write_structure(
            &mut writer,
            document,
            technology,
            &names,
            &ambiguous_mappings,
            cell,
            &mut report,
        )?;
    }
    writer.write_no_data(ENDLIB)?;
    Ok(GdsExportResult {
        bytes: writer.into_bytes(),
        report,
    })
}

pub fn import_gdsii(bytes: &[u8], technology: &TechnologyFile) -> Result<Document, GdsError> {
    Ok(import_gdsii_with_report(bytes, technology)?.document)
}

pub fn import_gdsii_with_report(
    bytes: &[u8],
    technology: &TechnologyFile,
) -> Result<GdsImportResult, GdsError> {
    technology.validate()?;
    let library = parse_gdsii(bytes)?;
    let top_name = infer_top_structure_name(&library)
        .ok_or_else(|| GdsError::InvalidRecord("library has no structures".to_string()))?;
    let mut document = Document::from_technology(&library.name, technology)?;
    let source_dbu_per_micron = library.dbu_per_micron.max(1);
    let target_dbu_per_micron = technology.dbu_per_micron.max(1);
    let mut report = GdsImportReport {
        library_name: library.name.clone(),
        source_dbu_per_micron,
        structure_count: library.structures.len(),
        element_count: library
            .structures
            .iter()
            .map(|structure| structure.elements.len())
            .sum(),
        generated_layers: Vec::new(),
        skipped_elements: Vec::new(),
        warnings: library.warnings.clone(),
    };
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
    report_duplicate_structure_name_warnings(&library, &mut report);

    let mut cell_ids = BTreeMap::new();
    cell_ids.insert(top_name.clone(), document.top_cell);
    for structure in &library.structures {
        if structure.name == top_name || cell_ids.contains_key(&structure.name) {
            continue;
        }
        let cell_id = document.create_cell(&structure.name);
        cell_ids.insert(structure.name.clone(), cell_id);
    }

    let mut generated_layers = BTreeMap::new();
    let mut resolved_layer_mappings = BTreeMap::new();
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
                &mut resolved_layer_mappings,
                &mut report,
                source_dbu_per_micron,
                target_dbu_per_micron,
                parent,
                element,
            )?;
        }
    }
    report_split_import_layer_mapping_warnings(&resolved_layer_mappings, &mut report);
    document.ensure_hierarchy();
    Ok(GdsImportResult { document, report })
}

fn write_structure(
    writer: &mut GdsWriter,
    document: &Document,
    technology: &TechnologyFile,
    names: &BTreeMap<CellId, String>,
    ambiguous_mappings: &BTreeMap<GdsExportMappingKey, Vec<LayerId>>,
    cell: &Cell,
    report: &mut GdsExportReport,
) -> Result<(), GdsError> {
    writer.write_i16(BGNSTR, &timestamp_fields())?;
    let structure_name = names
        .get(&cell.id)
        .cloned()
        .unwrap_or_else(|| sanitize_gds_name(&cell.name, "CELL"));
    writer.write_ascii(STRNAME, &structure_name)?;

    if cell.id == document.top_cell {
        for shape in document.shapes.values() {
            write_shape(
                writer,
                document,
                technology,
                ambiguous_mappings,
                cell.id,
                &shape,
                report,
            )?;
        }
    }
    for shape in cell.shapes.values() {
        write_shape(
            writer,
            document,
            technology,
            ambiguous_mappings,
            cell.id,
            &shape,
            report,
        )?;
    }
    for instance in cell.instances.values() {
        let Some(child_name) = names.get(&instance.cell) else {
            report.skipped_elements.push(GdsSkippedElement {
                cell_id: cell.id,
                shape_id: None,
                instance_id: Some(instance.id),
                reason: format!("instance target cell {} was not found", instance.cell.0),
            });
            continue;
        };
        if instance.transform.matrix != Transform::IDENTITY.matrix {
            report.warnings.push(GdsExportWarning {
                kind: GdsExportWarningKind::UnsupportedInstanceTransform,
                cell_id: cell.id,
                shape_id: None,
                instance_id: Some(instance.id),
                layer_id: None,
                message: format!(
                    "instance transform {:?} cannot be represented by this GDS exporter; exported translation only",
                    instance.transform.matrix
                ),
            });
        }
        let origin = Point::new(
            instance.transform.translation.dx,
            instance.transform.translation.dy,
        );
        let array = instance.array.normalized();
        if array.is_single() {
            writer.write_no_data(SREF)?;
            writer.write_ascii(SNAME, child_name)?;
            writer.write_xy(&[origin])?;
            report.sref_count += 1;
        } else {
            if array.columns > GDS_MAX_COLROW_DIMENSION || array.rows > GDS_MAX_COLROW_DIMENSION {
                report.skipped_elements.push(GdsSkippedElement {
                    cell_id: cell.id,
                    shape_id: None,
                    instance_id: Some(instance.id),
                    reason: format!(
                        "instance array dimensions {}x{} exceed GDSII COLROW limit {}",
                        array.columns, array.rows, GDS_MAX_COLROW_DIMENSION
                    ),
                });
                continue;
            }
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
            report.aref_count += 1;
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
    ambiguous_mappings: &BTreeMap<GdsExportMappingKey, Vec<LayerId>>,
    cell_id: CellId,
    shape: &Shape,
    report: &mut GdsExportReport,
) -> Result<(), GdsError> {
    match &shape.kind {
        ShapeKind::Rectangle(rect) => {
            report_shape_metadata_warning(report, cell_id, shape);
            let (layer, datatype, _) = gds_mapping_for_export(
                document,
                technology,
                ambiguous_mappings,
                cell_id,
                shape,
                report,
            )?;
            writer.write_no_data(BOUNDARY)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            let mut points = rect.corners().to_vec();
            points.push(points[0]);
            writer.write_xy(&points)?;
            writer.write_no_data(ENDEL)?;
            report.boundary_count += 1;
        }
        ShapeKind::Polygon(poly) => {
            if poly.points.len() < 3 {
                report.skipped_elements.push(GdsSkippedElement {
                    cell_id,
                    shape_id: Some(shape.id),
                    instance_id: None,
                    reason: format!(
                        "polygon has {} points; GDS BOUNDARY needs at least 3",
                        poly.points.len()
                    ),
                });
                return Ok(());
            }
            report_shape_metadata_warning(report, cell_id, shape);
            let (layer, datatype, _) = gds_mapping_for_export(
                document,
                technology,
                ambiguous_mappings,
                cell_id,
                shape,
                report,
            )?;
            writer.write_no_data(BOUNDARY)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            let mut points = poly.points.clone();
            if points.first() != points.last() {
                points.push(points[0]);
            }
            writer.write_xy(&points)?;
            writer.write_no_data(ENDEL)?;
            report.boundary_count += 1;
        }
        ShapeKind::Path { points, width } => {
            if points.len() < 2 {
                report.skipped_elements.push(GdsSkippedElement {
                    cell_id,
                    shape_id: Some(shape.id),
                    instance_id: None,
                    reason: format!(
                        "path has {} points; GDS PATH needs at least 2",
                        points.len()
                    ),
                });
                return Ok(());
            }
            report_shape_metadata_warning(report, cell_id, shape);
            let (layer, datatype, _) = gds_mapping_for_export(
                document,
                technology,
                ambiguous_mappings,
                cell_id,
                shape,
                report,
            )?;
            let export_width = normalize_gds_path_width(*width);
            if export_width != *width {
                report.warnings.push(GdsExportWarning {
                    kind: GdsExportWarningKind::NormalizedPathWidth,
                    cell_id,
                    shape_id: Some(shape.id),
                    instance_id: None,
                    layer_id: Some(shape.layer),
                    message: format!(
                        "path width {width} exported as positive GDS PATH width {export_width}"
                    ),
                });
            }
            writer.write_no_data(PATH)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            writer.write_i32(WIDTH, &[coord_to_i32(export_width)?])?;
            writer.write_xy(points)?;
            writer.write_no_data(ENDEL)?;
            report.path_count += 1;
        }
        ShapeKind::Via { center, size, .. } => {
            let half = *size / 2;
            let rect = Rect::new(
                Point::new(center.x - half, center.y - half),
                Point::new(center.x + half, center.y + half),
            );
            report_shape_kind_warning(
                report,
                cell_id,
                shape,
                "via shapes export as GDS BOUNDARY geometry; lower/upper layer metadata will not round-trip",
            );
            report_shape_metadata_warning(report, cell_id, shape);
            let (layer, datatype, _) = gds_mapping_for_export(
                document,
                technology,
                ambiguous_mappings,
                cell_id,
                shape,
                report,
            )?;
            writer.write_no_data(BOUNDARY)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            let mut points = rect.corners().to_vec();
            points.push(points[0]);
            writer.write_xy(&points)?;
            writer.write_no_data(ENDEL)?;
            report.boundary_count += 1;
        }
        ShapeKind::Label { position, text } => {
            report_shape_metadata_warning(report, cell_id, shape);
            let (layer, _, texttype) = gds_mapping_for_export(
                document,
                technology,
                ambiguous_mappings,
                cell_id,
                shape,
                report,
            )?;
            writer.write_no_data(TEXT)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(TEXTTYPE, &[u16_to_i16(texttype)?])?;
            writer.write_xy(&[*position])?;
            writer.write_ascii(STRING, text)?;
            writer.write_no_data(ENDEL)?;
            report.text_count += 1;
        }
        ShapeKind::Measurement { a, b, label } => {
            report_shape_kind_warning(
                report,
                cell_id,
                shape,
                "measurement shapes export as separate GDS PATH and TEXT elements and will not import as one measurement shape",
            );
            report_shape_metadata_warning(report, cell_id, shape);
            let (layer, datatype, texttype) = gds_mapping_for_export(
                document,
                technology,
                ambiguous_mappings,
                cell_id,
                shape,
                report,
            )?;
            writer.write_no_data(PATH)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(DATATYPE, &[u16_to_i16(datatype)?])?;
            writer.write_i32(WIDTH, &[10])?;
            writer.write_xy(&[*a, *b])?;
            writer.write_no_data(ENDEL)?;
            report.path_count += 1;

            writer.write_no_data(TEXT)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(TEXTTYPE, &[u16_to_i16(texttype)?])?;
            writer.write_xy(&[*a])?;
            writer.write_ascii(STRING, label)?;
            writer.write_no_data(ENDEL)?;
            report.text_count += 1;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn import_element(
    document: &mut Document,
    technology: &TechnologyFile,
    cell_ids: &BTreeMap<String, CellId>,
    generated_layers: &mut BTreeMap<(u16, u16, bool), LayerId>,
    resolved_layer_mappings: &mut BTreeMap<GdsImportMappingKey, BTreeSet<LayerId>>,
    report: &mut GdsImportReport,
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
            let scale_context = GdsScaleWarningContext::new(
                "BOUNDARY",
                Some(*layer),
                Some(*datatype),
                Some(false),
                None,
            );
            let points = scale_points(
                points,
                source_dbu_per_micron,
                target_dbu_per_micron,
                report,
                scale_context,
            );
            if !import_boundary_has_minimum_points(&points) {
                report.skipped_elements.push(GdsImportSkippedElement {
                    cell_id: parent,
                    element_kind: "BOUNDARY".to_string(),
                    gds_layer: Some(*layer),
                    gds_type: Some(*datatype),
                    reason: format!(
                        "GDS BOUNDARY has {} point(s); import needs at least 3 distinct points",
                        points.len()
                    ),
                });
                return Ok(());
            }
            let layer_id = resolve_import_layer(
                document,
                technology,
                generated_layers,
                report,
                *layer,
                *datatype,
                false,
            );
            record_import_layer_mapping(
                resolved_layer_mappings,
                *layer,
                *datatype,
                false,
                layer_id,
            );
            let kind = import_boundary_kind(&points);
            insert_shape_in_import_cell(document, parent, layer_id, kind);
        }
        ParsedElement::Path {
            layer,
            datatype,
            raw_width,
            width,
            points,
        } => {
            if points.len() < 2 {
                report.skipped_elements.push(GdsImportSkippedElement {
                    cell_id: parent,
                    element_kind: "PATH".to_string(),
                    gds_layer: Some(*layer),
                    gds_type: Some(*datatype),
                    reason: format!(
                        "GDS PATH has {} point(s); import needs at least 2",
                        points.len()
                    ),
                });
                return Ok(());
            }
            let layer_id = resolve_import_layer(
                document,
                technology,
                generated_layers,
                report,
                *layer,
                *datatype,
                false,
            );
            record_import_layer_mapping(
                resolved_layer_mappings,
                *layer,
                *datatype,
                false,
                layer_id,
            );
            let scale_context = GdsScaleWarningContext::new(
                "PATH",
                Some(*layer),
                Some(*datatype),
                Some(false),
                Some(layer_id),
            );
            let raw_scaled_width = scale_coord_with_report(
                *raw_width,
                source_dbu_per_micron,
                target_dbu_per_micron,
                report,
                scale_context,
                "raw width",
            );
            let scaled_width = scale_coord_with_report(
                *width,
                source_dbu_per_micron,
                target_dbu_per_micron,
                report,
                scale_context,
                "width",
            );
            let imported_width = normalize_gds_path_width(scaled_width);
            if raw_width != width || imported_width != scaled_width {
                report.warnings.push(GdsImportWarning {
                    kind: GdsImportWarningKind::NormalizedPathWidth,
                    gds_layer: Some(*layer),
                    gds_type: Some(*datatype),
                    is_text: Some(false),
                    layer_id: Some(layer_id),
                    message: format!(
                        "GDS PATH width {raw_width} scaled to {raw_scaled_width}; imported as positive width {imported_width}"
                    ),
                });
            }
            insert_shape_in_import_cell(
                document,
                parent,
                layer_id,
                ShapeKind::Path {
                    points: scale_points(
                        points,
                        source_dbu_per_micron,
                        target_dbu_per_micron,
                        report,
                        scale_context,
                    ),
                    width: imported_width,
                },
            );
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
                report,
                *layer,
                *texttype,
                true,
            );
            record_import_layer_mapping(resolved_layer_mappings, *layer, *texttype, true, layer_id);
            let scale_context = GdsScaleWarningContext::new(
                "TEXT",
                Some(*layer),
                Some(*texttype),
                Some(true),
                Some(layer_id),
            );
            insert_shape_in_import_cell(
                document,
                parent,
                layer_id,
                ShapeKind::Label {
                    position: scale_point(
                        *position,
                        source_dbu_per_micron,
                        target_dbu_per_micron,
                        report,
                        scale_context,
                    ),
                    text: text.clone(),
                },
            );
        }
        ParsedElement::Sref { name, origin } => {
            let Some(child) = cell_ids.get(name).copied() else {
                report.skipped_elements.push(GdsImportSkippedElement {
                    cell_id: parent,
                    element_kind: "SREF".to_string(),
                    gds_layer: None,
                    gds_type: None,
                    reason: format!("unknown SREF target {name:?}"),
                });
                return Ok(());
            };
            insert_import_instance(
                document,
                parent,
                child,
                name,
                scale_point(
                    *origin,
                    source_dbu_per_micron,
                    target_dbu_per_micron,
                    report,
                    GdsScaleWarningContext::new("SREF", None, None, None, None),
                ),
                InstanceArray::single(),
            );
        }
        ParsedElement::Aref {
            name,
            raw_columns,
            raw_rows,
            columns,
            rows,
            origin,
            column_step,
            row_step,
        } => {
            if raw_columns != columns || raw_rows != rows {
                report.warnings.push(GdsImportWarning {
                    kind: GdsImportWarningKind::NormalizedArefDimensions,
                    gds_layer: None,
                    gds_type: None,
                    is_text: None,
                    layer_id: None,
                    message: format!(
                        "GDS AREF COLROW {raw_columns}x{raw_rows} imported as {columns}x{rows}"
                    ),
                });
            }
            let Some(child) = cell_ids.get(name).copied() else {
                report.skipped_elements.push(GdsImportSkippedElement {
                    cell_id: parent,
                    element_kind: "AREF".to_string(),
                    gds_layer: None,
                    gds_type: None,
                    reason: format!("unknown AREF target {name:?}"),
                });
                return Ok(());
            };
            insert_import_instance(
                document,
                parent,
                child,
                name,
                scale_point(
                    *origin,
                    source_dbu_per_micron,
                    target_dbu_per_micron,
                    report,
                    GdsScaleWarningContext::new("AREF", None, None, None, None),
                ),
                InstanceArray {
                    columns: u32::from(*columns),
                    rows: u32::from(*rows),
                    column_pitch: scale_vector(
                        *column_step,
                        source_dbu_per_micron,
                        target_dbu_per_micron,
                        report,
                        GdsScaleWarningContext::new("AREF", None, None, None, None),
                    ),
                    row_pitch: scale_vector(
                        *row_step,
                        source_dbu_per_micron,
                        target_dbu_per_micron,
                        report,
                        GdsScaleWarningContext::new("AREF", None, None, None, None),
                    ),
                },
            );
        }
        ParsedElement::Unsupported {
            kind,
            layer,
            gds_type,
        } => {
            report.skipped_elements.push(GdsImportSkippedElement {
                cell_id: parent,
                element_kind: kind.clone(),
                gds_layer: *layer,
                gds_type: *gds_type,
                reason: format!("unsupported GDSII {kind} element"),
            });
        }
    }
    Ok(())
}

fn report_duplicate_structure_name_warnings(library: &ParsedLibrary, report: &mut GdsImportReport) {
    let mut counts = BTreeMap::<&str, usize>::new();
    for structure in &library.structures {
        *counts.entry(structure.name.as_str()).or_default() += 1;
    }
    for (name, count) in counts {
        if count < 2 {
            continue;
        }
        report.warnings.push(GdsImportWarning {
            kind: GdsImportWarningKind::DuplicateStructureName,
            gds_layer: None,
            gds_type: None,
            is_text: None,
            layer_id: None,
            message: format!(
                "GDS structure name {name:?} appears {count} times; duplicate definitions import into one Fabricad cell and SREF/AREF references are ambiguous"
            ),
        });
    }
}

fn record_import_layer_mapping(
    resolved_layer_mappings: &mut BTreeMap<GdsImportMappingKey, BTreeSet<LayerId>>,
    gds_layer: u16,
    gds_type: u16,
    is_text: bool,
    layer_id: LayerId,
) {
    resolved_layer_mappings
        .entry(GdsImportMappingKey {
            gds_layer,
            gds_type,
            is_text,
        })
        .or_default()
        .insert(layer_id);
}

fn report_split_import_layer_mapping_warnings(
    resolved_layer_mappings: &BTreeMap<GdsImportMappingKey, BTreeSet<LayerId>>,
    report: &mut GdsImportReport,
) {
    let mut by_gds_layer: BTreeMap<u16, BTreeMap<LayerId, Vec<GdsImportMappingKey>>> =
        BTreeMap::new();
    for (mapping, layer_ids) in resolved_layer_mappings {
        for layer_id in layer_ids {
            by_gds_layer
                .entry(mapping.gds_layer)
                .or_default()
                .entry(*layer_id)
                .or_default()
                .push(*mapping);
        }
    }
    for (gds_layer, layer_groups) in by_gds_layer {
        if layer_groups.len() < 2 {
            continue;
        }
        let details = layer_groups
            .iter()
            .map(|(layer_id, mappings)| {
                let mapping_labels = mappings
                    .iter()
                    .map(format_import_mapping_label)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("layer {} from {mapping_labels}", layer_id.0)
            })
            .collect::<Vec<_>>()
            .join("; ");
        report.warnings.push(GdsImportWarning {
            kind: GdsImportWarningKind::SplitIncomingLayerMapping,
            gds_layer: Some(gds_layer),
            gds_type: None,
            is_text: None,
            layer_id: None,
            message: format!(
                "incoming GDS layer {gds_layer} resolved to multiple Fabricad layers: {details}"
            ),
        });
    }
}

fn format_import_mapping_label(mapping: &GdsImportMappingKey) -> String {
    if mapping.is_text {
        format!("TEXTTYPE {}", mapping.gds_type)
    } else {
        format!("DATATYPE {}", mapping.gds_type)
    }
}

fn parse_gdsii(bytes: &[u8]) -> Result<ParsedLibrary, GdsError> {
    let mut reader = GdsReader::new(bytes);
    let mut library = ParsedLibrary {
        name: "gds_import".to_string(),
        dbu_per_micron: DBU_PER_MICRON,
        warnings: Vec::new(),
        structures: Vec::new(),
    };
    let mut structure: Option<ParsedStructure> = None;
    let mut element: Option<ElementBuilder> = None;
    let mut saw_endlib = false;

    while let Some(record) = reader.next_record()? {
        match record.record_type {
            LIBNAME => library.name = record.as_ascii()?,
            UNITS => {
                let units = record.as_real8()?;
                if let Some(user_units_per_dbu) = units.first().copied()
                    && user_units_per_dbu.is_finite()
                    && user_units_per_dbu > 0.0
                {
                    let parsed = (1.0 / user_units_per_dbu).round();
                    let effective = parsed.max(1.0) as Coord;
                    if parsed < 1.0 {
                        warn!(
                            parsed_dbu_per_micron = parsed,
                            effective_dbu_per_micron = effective,
                            "GDS UNITS yielded unsupported dbu_per_micron; using minimum 1"
                        );
                        library.warnings.push(GdsImportWarning {
                            kind: GdsImportWarningKind::NormalizedUnits,
                            gds_layer: None,
                            gds_type: None,
                            is_text: None,
                            layer_id: None,
                            message: format!(
                                "GDS UNITS resolved to unsupported dbu_per_micron {parsed}; using minimum dbu_per_micron {effective}"
                            ),
                        });
                    }
                    library.dbu_per_micron = effective;
                } else {
                    library.warnings.push(GdsImportWarning {
                        kind: GdsImportWarningKind::NormalizedUnits,
                        gds_layer: None,
                        gds_type: None,
                        is_text: None,
                        layer_id: None,
                        message: format!(
                            "GDS UNITS record is invalid; using default dbu_per_micron {}",
                            library.dbu_per_micron
                        ),
                    });
                }
            }
            BGNSTR => {
                if let Some(open_element) = &element {
                    return Err(GdsError::InvalidRecord(format!(
                        "BGNSTR before ENDEL for {} element",
                        open_element.kind.label()
                    )));
                }
                if structure.is_some() {
                    return Err(GdsError::InvalidRecord(
                        "BGNSTR before ENDSTR for previous structure".to_string(),
                    ));
                }
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
                if let Some(open_element) = &element {
                    return Err(GdsError::InvalidRecord(format!(
                        "ENDSTR before ENDEL for {} element",
                        open_element.kind.label()
                    )));
                }
                let mut finished = structure
                    .take()
                    .ok_or_else(|| GdsError::InvalidRecord("ENDSTR without BGNSTR".to_string()))?;
                if finished.name.trim().is_empty() {
                    finished.name = format!("STRUCT_{}", library.structures.len() + 1);
                }
                library.structures.push(finished);
            }
            BOUNDARY => begin_gds_element(&structure, &mut element, ElementKind::Boundary)?,
            PATH => begin_gds_element(&structure, &mut element, ElementKind::Path)?,
            TEXT => begin_gds_element(&structure, &mut element, ElementKind::Text)?,
            SREF => begin_gds_element(&structure, &mut element, ElementKind::Sref)?,
            AREF => begin_gds_element(&structure, &mut element, ElementKind::Aref)?,
            TEXTNODE => begin_gds_element(
                &structure,
                &mut element,
                ElementKind::Unsupported("TEXTNODE"),
            )?,
            NODE => begin_gds_element(&structure, &mut element, ElementKind::Unsupported("NODE"))?,
            BOX => begin_gds_element(&structure, &mut element, ElementKind::Unsupported("BOX"))?,
            LAYER => {
                if let Some(element) = &mut element {
                    element.layer = Some(record.first_u16()?);
                }
            }
            DATATYPE | NODETYPE | BOXTYPE => {
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
                        element.columns = values[0];
                        element.rows = values[1];
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
            ENDLIB => {
                if let Some(open_element) = &element {
                    return Err(GdsError::InvalidRecord(format!(
                        "ENDLIB before ENDEL for {} element",
                        open_element.kind.label()
                    )));
                }
                if structure.is_some() {
                    return Err(GdsError::InvalidRecord(
                        "ENDLIB before ENDSTR for open structure".to_string(),
                    ));
                }
                saw_endlib = true;
                break;
            }
            _ => {}
        }
    }
    if let Some(open_element) = &element {
        return Err(GdsError::InvalidRecord(format!(
            "unexpected EOF before ENDEL for {} element",
            open_element.kind.label()
        )));
    }
    if structure.is_some() {
        return Err(GdsError::InvalidRecord(
            "unexpected EOF before ENDSTR for open structure".to_string(),
        ));
    }
    if !saw_endlib {
        return Err(GdsError::InvalidRecord(
            "unexpected EOF before ENDLIB".to_string(),
        ));
    }
    Ok(library)
}

fn begin_gds_element(
    structure: &Option<ParsedStructure>,
    element: &mut Option<ElementBuilder>,
    kind: ElementKind,
) -> Result<(), GdsError> {
    if structure.is_none() {
        return Err(GdsError::InvalidRecord(format!(
            "{} element outside structure",
            kind.label()
        )));
    }
    if let Some(open_element) = element {
        return Err(GdsError::InvalidRecord(format!(
            "{} before ENDEL for {} element",
            kind.label(),
            open_element.kind.label()
        )));
    }
    *element = Some(ElementBuilder::new(kind));
    Ok(())
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
                | ParsedElement::Text { .. }
                | ParsedElement::Unsupported { .. } => {}
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

fn import_boundary_has_minimum_points(points: &[Point]) -> bool {
    let mut points = points.to_vec();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    let distinct = points.iter().copied().collect::<BTreeSet<_>>();
    distinct.len() >= 3
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
    report: &mut GdsImportReport,
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
    report.generated_layers.push(GdsGeneratedLayer {
        gds_layer,
        gds_type,
        is_text,
        layer_id: id,
        name: document
            .layers
            .get(&id)
            .map(|layer| layer.name.clone())
            .unwrap_or_else(|| format!("gds_{gds_layer}_{gds_type}")),
    });
    id
}

fn gds_mapping_for_export(
    document: &Document,
    technology: &TechnologyFile,
    ambiguous_mappings: &BTreeMap<GdsExportMappingKey, Vec<LayerId>>,
    cell_id: CellId,
    shape: &Shape,
    report: &mut GdsExportReport,
) -> Result<(u16, u16, u16), GdsError> {
    let layer_id = shape.layer;
    if let Some(mapping) = technology.gds_mapping_for_layer(layer_id) {
        report_ambiguous_gds_mapping_warnings(
            ambiguous_mappings,
            report,
            cell_id,
            shape,
            layer_id,
            mapping,
        );
        return Ok(mapping);
    }
    if let Some(layer) = document.layer(layer_id) {
        if let Some(gds_layer) = layer.gds_layer {
            let mapping = (gds_layer, layer.gds_datatype, layer.gds_texttype);
            report_ambiguous_gds_mapping_warnings(
                ambiguous_mappings,
                report,
                cell_id,
                shape,
                layer_id,
                mapping,
            );
            return Ok(mapping);
        }
        let gds_layer = u16::try_from(layer.id.0)
            .map_err(|_| GdsError::Unsupported(format!("layer id {} exceeds GDSII", layer.id.0)))?;
        let mapping = (gds_layer, 0, 0);
        report.warnings.push(GdsExportWarning {
            kind: GdsExportWarningKind::FallbackLayerMapping,
            cell_id,
            shape_id: Some(shape.id),
            instance_id: None,
            layer_id: Some(layer.id),
            message: format!(
                "layer {} has no technology or document GDS mapping; exported with layer id {}",
                layer.id.0, gds_layer
            ),
        });
        report_ambiguous_gds_mapping_warnings(
            ambiguous_mappings,
            report,
            cell_id,
            shape,
            layer_id,
            mapping,
        );
        return Ok(mapping);
    }
    let gds_layer = u16::try_from(layer_id.0)
        .map_err(|_| GdsError::Unsupported(format!("layer id {} exceeds GDSII", layer_id.0)))?;
    report.warnings.push(GdsExportWarning {
        kind: GdsExportWarningKind::MissingLayerFallback,
        cell_id,
        shape_id: Some(shape.id),
        instance_id: None,
        layer_id: Some(layer_id),
        message: format!(
            "shape references missing layer {}; exported with that numeric GDS layer",
            layer_id.0
        ),
    });
    Ok((gds_layer, 0, 0))
}

fn report_ambiguous_gds_mapping_warnings(
    ambiguous_mappings: &BTreeMap<GdsExportMappingKey, Vec<LayerId>>,
    report: &mut GdsExportReport,
    cell_id: CellId,
    shape: &Shape,
    layer_id: LayerId,
    mapping: (u16, u16, u16),
) {
    let usages = match &shape.kind {
        ShapeKind::Label { .. } => vec![(true, mapping.2)],
        ShapeKind::Measurement { .. } => vec![(false, mapping.1), (true, mapping.2)],
        _ => vec![(false, mapping.1)],
    };
    for (is_text, gds_type) in usages {
        let key = GdsExportMappingKey {
            gds_layer: mapping.0,
            gds_type,
            is_text,
        };
        let Some(layer_ids) = ambiguous_mappings.get(&key) else {
            continue;
        };
        if !layer_ids.contains(&layer_id) {
            continue;
        }
        let other_layers: Vec<_> = layer_ids
            .iter()
            .copied()
            .filter(|candidate| *candidate != layer_id)
            .collect();
        if other_layers.is_empty() {
            continue;
        }
        let mapping_kind = if is_text { "text" } else { "geometry" };
        let exported_kind = if is_text { "TEXT" } else { "BOUNDARY/PATH" };
        report.warnings.push(GdsExportWarning {
            kind: GdsExportWarningKind::AmbiguousDocumentLayerMapping,
            cell_id,
            shape_id: Some(shape.id),
            instance_id: None,
            layer_id: Some(layer_id),
            message: format!(
                "layer {} shares GDSII {mapping_kind} mapping ({}, {}) with layer(s) {}; exported {exported_kind} elements will merge on that GDS pair",
                layer_id.0,
                key.gds_layer,
                key.gds_type,
                format_layer_ids(&other_layers)
            ),
        });
    }
}

fn format_layer_ids(layer_ids: &[LayerId]) -> String {
    layer_ids
        .iter()
        .map(|layer_id| layer_id.0.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn report_shape_metadata_warning(report: &mut GdsExportReport, cell_id: CellId, shape: &Shape) {
    let mut metadata = Vec::new();
    if shape.name.is_some() {
        metadata.push("name");
    }
    if shape.net.is_some() {
        metadata.push("net");
    }
    if metadata.is_empty() {
        return;
    }
    report.warnings.push(GdsExportWarning {
        kind: GdsExportWarningKind::NonRoundTrippableMetadata,
        cell_id,
        shape_id: Some(shape.id),
        instance_id: None,
        layer_id: Some(shape.layer),
        message: format!(
            "shape metadata ({}) is not represented in GDSII and will not round-trip",
            metadata.join(", ")
        ),
    });
}

fn report_shape_kind_warning(
    report: &mut GdsExportReport,
    cell_id: CellId,
    shape: &Shape,
    message: impl Into<String>,
) {
    report.warnings.push(GdsExportWarning {
        kind: GdsExportWarningKind::NonRoundTrippableShapeKind,
        cell_id,
        shape_id: Some(shape.id),
        instance_id: None,
        layer_id: Some(shape.layer),
        message: message.into(),
    });
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
    report: &mut GdsImportReport,
    context: GdsScaleWarningContext,
) -> Vec<Point> {
    points
        .iter()
        .copied()
        .map(|point| {
            scale_point(
                point,
                source_dbu_per_micron,
                target_dbu_per_micron,
                report,
                context,
            )
        })
        .collect()
}

fn scale_point(
    point: Point,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
    report: &mut GdsImportReport,
    context: GdsScaleWarningContext,
) -> Point {
    Point::new(
        scale_coord_with_report(
            point.x,
            source_dbu_per_micron,
            target_dbu_per_micron,
            report,
            context,
            "x",
        ),
        scale_coord_with_report(
            point.y,
            source_dbu_per_micron,
            target_dbu_per_micron,
            report,
            context,
            "y",
        ),
    )
}

fn scale_vector(
    vector: Vector,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
    report: &mut GdsImportReport,
    context: GdsScaleWarningContext,
) -> Vector {
    Vector::new(
        scale_coord_with_report(
            vector.dx,
            source_dbu_per_micron,
            target_dbu_per_micron,
            report,
            context,
            "dx",
        ),
        scale_coord_with_report(
            vector.dy,
            source_dbu_per_micron,
            target_dbu_per_micron,
            report,
            context,
            "dy",
        ),
    )
}

fn scale_coord_with_report(
    value: Coord,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
    report: &mut GdsImportReport,
    context: GdsScaleWarningContext,
    component: &'static str,
) -> Coord {
    let (scaled, clamped) = scale_coord_inner(value, source_dbu_per_micron, target_dbu_per_micron);
    if let Some((rounded, clamped_value)) = clamped {
        report.warnings.push(GdsImportWarning {
            kind: GdsImportWarningKind::CoordinateClamped,
            gds_layer: context.gds_layer,
            gds_type: context.gds_type,
            is_text: context.is_text,
            layer_id: context.layer_id,
            message: format!(
                "GDS {} {component} coordinate {value} scaled to {rounded}; clamped to {clamped_value}",
                context.element_kind
            ),
        });
    }
    scaled
}

fn scale_coord_inner(
    value: Coord,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
) -> (Coord, Option<(i128, Coord)>) {
    let source_effective = source_dbu_per_micron.max(1);
    let target_effective = target_dbu_per_micron.max(1);
    if source_effective != source_dbu_per_micron || target_effective != target_dbu_per_micron {
        warn!(
            source_dbu_per_micron,
            target_dbu_per_micron,
            source_effective,
            target_effective,
            "GDS coordinate scale used out-of-range dbu_per_micron; applying minimum 1"
        );
    }
    let source = i128::from(source_effective);
    let target = i128::from(target_effective);
    let numerator = i128::from(value) * target;
    let rounded = if numerator >= 0 {
        (numerator + source / 2) / source
    } else {
        (numerator - source / 2) / source
    };
    let clamped = rounded.clamp(i128::from(Coord::MIN), i128::from(Coord::MAX));
    if clamped != rounded {
        warn!(
            value,
            rounded, clamped, "scaled GDS coordinate exceeded Coord range"
        );
    }
    let scaled = clamped as Coord;
    let clamped = (clamped != rounded).then_some((rounded, scaled));
    (scaled, clamped)
}

fn normalize_gds_path_width(width: Coord) -> Coord {
    if width > 0 {
        width
    } else if width == Coord::MIN {
        Coord::MAX
    } else {
        width.abs().max(1)
    }
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
            GdsError::Unsupported(format!("record {record_type:02x} exceeds 65535 bytes"))
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
    use crate::{LayerId, Operation, default_technology};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct GdsRoundTripFixture {
        name: String,
        #[serde(default)]
        top_shapes: Vec<GdsFixtureShape>,
        #[serde(default)]
        cells: Vec<GdsFixtureCell>,
        #[serde(default)]
        instances: Vec<GdsFixtureInstance>,
        expect: GdsRoundTripExpectation,
    }

    #[derive(Debug, Deserialize)]
    struct GdsFixtureCell {
        name: String,
        #[serde(default)]
        shapes: Vec<GdsFixtureShape>,
    }

    #[derive(Debug, Deserialize)]
    struct GdsFixtureInstance {
        cell: String,
        dx: Coord,
        dy: Coord,
        #[serde(default)]
        array: Option<GdsFixtureArray>,
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    struct GdsFixtureArray {
        columns: u32,
        rows: u32,
        column_pitch: [Coord; 2],
        row_pitch: [Coord; 2],
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum GdsFixtureShape {
        Rect {
            layer: String,
            x: Coord,
            y: Coord,
            w: Coord,
            h: Coord,
        },
        Path {
            layer: String,
            width: Coord,
            points: Vec<[Coord; 2]>,
        },
        Label {
            layer: String,
            x: Coord,
            y: Coord,
            text: String,
        },
    }

    #[derive(Debug, Deserialize)]
    struct GdsRoundTripExpectation {
        export: GdsExportExpectation,
        import: GdsImportExpectation,
    }

    #[derive(Debug, Deserialize)]
    struct GdsExportExpectation {
        structure_count: usize,
        boundary_count: usize,
        path_count: usize,
        text_count: usize,
        sref_count: usize,
        aref_count: usize,
        skipped_count: usize,
        warning_count: usize,
    }

    #[derive(Debug, Deserialize)]
    struct GdsImportExpectation {
        structure_count: usize,
        element_count: usize,
        top_shape_count: usize,
        top_instance_count: usize,
        flattened_shape_count: usize,
        array_columns: u32,
        array_rows: u32,
        array_column_pitch: [Coord; 2],
        array_row_pitch: [Coord; 2],
    }

    #[derive(Debug, Deserialize)]
    struct GdsImportLayerFixture {
        name: String,
        elements: Vec<GdsImportFixtureElement>,
        expect: GdsImportLayerExpectation,
    }

    #[derive(Debug, Deserialize)]
    struct GdsUnsupportedImportFixture {
        name: String,
        elements: Vec<GdsImportFixtureElement>,
        expect: GdsUnsupportedImportExpectation,
    }

    #[derive(Debug, Deserialize)]
    struct GdsDegenerateImportFixture {
        name: String,
        elements: Vec<GdsImportFixtureElement>,
        expect: GdsDegenerateImportExpectation,
    }

    #[derive(Debug, Deserialize)]
    struct GdsMissingRefsImportFixture {
        name: String,
        elements: Vec<GdsImportFixtureElement>,
        expect: GdsMissingRefsImportExpectation,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum GdsImportFixtureElement {
        Boundary {
            gds_layer: u16,
            datatype: u16,
            points: Vec<[Coord; 2]>,
        },
        Path {
            gds_layer: u16,
            datatype: u16,
            width: Coord,
            points: Vec<[Coord; 2]>,
        },
        Box {
            gds_layer: u16,
            gds_type: u16,
            points: Vec<[Coord; 2]>,
        },
        Node {
            gds_layer: u16,
            gds_type: u16,
            points: Vec<[Coord; 2]>,
        },
        Textnode {
            gds_layer: u16,
            gds_type: u16,
            points: Vec<[Coord; 2]>,
        },
        Sref {
            name: String,
            origin: [Coord; 2],
        },
        Aref {
            name: String,
            columns: u16,
            rows: u16,
            points: Vec<[Coord; 2]>,
        },
    }

    #[derive(Debug, Deserialize)]
    struct GdsImportLayerExpectation {
        generated_layer_count: usize,
        warning_count: usize,
        split_warning_gds_layer: u16,
        message_contains: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct GdsUnsupportedImportExpectation {
        element_count: usize,
        generated_layer_count: usize,
        visible_shape_count: usize,
        skipped_count: usize,
        skipped: Vec<GdsExpectedSkippedImportElement>,
    }

    #[derive(Debug, Deserialize)]
    struct GdsDegenerateImportExpectation {
        element_count: usize,
        generated_layer_count: usize,
        visible_shape_count: usize,
        skipped_count: usize,
        skipped: Vec<GdsExpectedSkippedImportElement>,
    }

    #[derive(Debug, Deserialize)]
    struct GdsMissingRefsImportExpectation {
        element_count: usize,
        generated_layer_count: usize,
        visible_shape_count: usize,
        top_instance_count: usize,
        skipped_count: usize,
        skipped: Vec<GdsExpectedSkippedImportElement>,
    }

    #[derive(Debug, Deserialize)]
    struct GdsExpectedSkippedImportElement {
        element_kind: String,
        #[serde(default)]
        gds_layer: Option<u16>,
        #[serde(default)]
        gds_type: Option<u16>,
        reason_contains: String,
    }

    fn document_from_gds_fixture(fixture: &GdsRoundTripFixture) -> Document {
        let mut document = Document::new(&fixture.name);
        for shape in &fixture.top_shapes {
            insert_fixture_shape(&mut document, None, shape);
        }

        let mut cell_ids = BTreeMap::new();
        for cell in &fixture.cells {
            let id = document.create_cell(&cell.name);
            cell_ids.insert(cell.name.clone(), id);
            for shape in &cell.shapes {
                insert_fixture_shape(&mut document, Some(id), shape);
            }
        }

        for instance in &fixture.instances {
            let cell = *cell_ids.get(&instance.cell).unwrap_or_else(|| {
                panic!(
                    "fixture instance references missing cell {:?}",
                    instance.cell
                )
            });
            let id = document
                .insert_instance_in_top(cell, Transform::translate(instance.dx, instance.dy))
                .unwrap();
            if let Some(array) = instance.array {
                let mut cell_instance = document.instance(document.top_cell, id).unwrap().clone();
                cell_instance.array = InstanceArray {
                    columns: array.columns,
                    rows: array.rows,
                    column_pitch: Vector::new(array.column_pitch[0], array.column_pitch[1]),
                    row_pitch: Vector::new(array.row_pitch[0], array.row_pitch[1]),
                };
                document.apply_operation_without_log(&Operation::ReplaceInstance {
                    parent: document.top_cell,
                    id,
                    instance: cell_instance,
                });
            }
        }

        document
    }

    fn gds_bytes_from_import_layer_fixture(fixture: &GdsImportLayerFixture) -> Vec<u8> {
        gds_bytes_from_import_fixture(&fixture.name, &fixture.elements)
    }

    fn gds_bytes_from_import_fixture(name: &str, elements: &[GdsImportFixtureElement]) -> Vec<u8> {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, name).unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();

        for element in elements {
            match element {
                GdsImportFixtureElement::Boundary {
                    gds_layer,
                    datatype,
                    points,
                } => {
                    writer.write_no_data(BOUNDARY).unwrap();
                    writer
                        .write_i16(LAYER, &[u16_to_i16(*gds_layer).unwrap()])
                        .unwrap();
                    writer
                        .write_i16(DATATYPE, &[u16_to_i16(*datatype).unwrap()])
                        .unwrap();
                    let points = points
                        .iter()
                        .map(|[x, y]| Point::new(*x, *y))
                        .collect::<Vec<_>>();
                    writer.write_xy(&points).unwrap();
                    writer.write_no_data(ENDEL).unwrap();
                }
                GdsImportFixtureElement::Path {
                    gds_layer,
                    datatype,
                    width,
                    points,
                } => {
                    writer.write_no_data(PATH).unwrap();
                    writer
                        .write_i16(LAYER, &[u16_to_i16(*gds_layer).unwrap()])
                        .unwrap();
                    writer
                        .write_i16(DATATYPE, &[u16_to_i16(*datatype).unwrap()])
                        .unwrap();
                    writer
                        .write_i32(WIDTH, &[i32::try_from(*width).unwrap()])
                        .unwrap();
                    let points = points
                        .iter()
                        .map(|[x, y]| Point::new(*x, *y))
                        .collect::<Vec<_>>();
                    writer.write_xy(&points).unwrap();
                    writer.write_no_data(ENDEL).unwrap();
                }
                GdsImportFixtureElement::Box {
                    gds_layer,
                    gds_type,
                    points,
                } => {
                    writer.write_no_data(BOX).unwrap();
                    writer
                        .write_i16(LAYER, &[u16_to_i16(*gds_layer).unwrap()])
                        .unwrap();
                    writer
                        .write_i16(BOXTYPE, &[u16_to_i16(*gds_type).unwrap()])
                        .unwrap();
                    let points = points
                        .iter()
                        .map(|[x, y]| Point::new(*x, *y))
                        .collect::<Vec<_>>();
                    writer.write_xy(&points).unwrap();
                    writer.write_no_data(ENDEL).unwrap();
                }
                GdsImportFixtureElement::Node {
                    gds_layer,
                    gds_type,
                    points,
                } => {
                    writer.write_no_data(NODE).unwrap();
                    writer
                        .write_i16(LAYER, &[u16_to_i16(*gds_layer).unwrap()])
                        .unwrap();
                    writer
                        .write_i16(NODETYPE, &[u16_to_i16(*gds_type).unwrap()])
                        .unwrap();
                    let points = points
                        .iter()
                        .map(|[x, y]| Point::new(*x, *y))
                        .collect::<Vec<_>>();
                    writer.write_xy(&points).unwrap();
                    writer.write_no_data(ENDEL).unwrap();
                }
                GdsImportFixtureElement::Textnode {
                    gds_layer,
                    gds_type,
                    points,
                } => {
                    writer.write_no_data(TEXTNODE).unwrap();
                    writer
                        .write_i16(LAYER, &[u16_to_i16(*gds_layer).unwrap()])
                        .unwrap();
                    writer
                        .write_i16(NODETYPE, &[u16_to_i16(*gds_type).unwrap()])
                        .unwrap();
                    let points = points
                        .iter()
                        .map(|[x, y]| Point::new(*x, *y))
                        .collect::<Vec<_>>();
                    writer.write_xy(&points).unwrap();
                    writer.write_no_data(ENDEL).unwrap();
                }
                GdsImportFixtureElement::Sref { name, origin } => {
                    writer.write_no_data(SREF).unwrap();
                    writer.write_ascii(SNAME, name).unwrap();
                    writer
                        .write_xy(&[Point::new(origin[0], origin[1])])
                        .unwrap();
                    writer.write_no_data(ENDEL).unwrap();
                }
                GdsImportFixtureElement::Aref {
                    name,
                    columns,
                    rows,
                    points,
                } => {
                    writer.write_no_data(AREF).unwrap();
                    writer.write_ascii(SNAME, name).unwrap();
                    writer
                        .write_i16(
                            COLROW,
                            &[u16_to_i16(*columns).unwrap(), u16_to_i16(*rows).unwrap()],
                        )
                        .unwrap();
                    let points = points
                        .iter()
                        .map(|[x, y]| Point::new(*x, *y))
                        .collect::<Vec<_>>();
                    writer.write_xy(&points).unwrap();
                    writer.write_no_data(ENDEL).unwrap();
                }
            }
        }

        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();
        writer.into_bytes()
    }

    fn insert_fixture_shape(
        document: &mut Document,
        cell: Option<CellId>,
        shape: &GdsFixtureShape,
    ) -> ShapeId {
        let (layer, kind) = match shape {
            GdsFixtureShape::Rect { layer, x, y, w, h } => (
                fixture_layer(document, layer),
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(*x, *y), *w, *h)),
            ),
            GdsFixtureShape::Path {
                layer,
                width,
                points,
            } => (
                fixture_layer(document, layer),
                ShapeKind::Path {
                    points: points
                        .iter()
                        .map(|point| Point::new(point[0], point[1]))
                        .collect(),
                    width: *width,
                },
            ),
            GdsFixtureShape::Label { layer, x, y, text } => (
                fixture_layer(document, layer),
                ShapeKind::Label {
                    position: Point::new(*x, *y),
                    text: text.clone(),
                },
            ),
        };
        if let Some(cell) = cell {
            document.insert_shape_in_cell(cell, layer, kind).unwrap()
        } else {
            document.insert_shape(layer, kind)
        }
    }

    fn fixture_layer(document: &Document, layer: &str) -> LayerId {
        let process = ProcessLayer::from_technology_name(layer)
            .unwrap_or_else(|| panic!("unknown fixture layer {layer}"));
        document
            .layer_by_process(process)
            .unwrap_or_else(|| panic!("fixture layer {layer} missing from document"))
    }

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
    fn export_report_counts_emitted_elements_and_skipped_geometry() {
        let mut document = Document::new("export report");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let via1 = document.layer_by_process(ProcessLayer::Via1).unwrap();
        let annotation = document.layer_by_process(ProcessLayer::Annotation).unwrap();
        let child = document.create_cell("unit");

        document.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        );
        document.insert_shape(
            metal1,
            ShapeKind::Path {
                points: vec![Point::new(0, 100), Point::new(100, 100)],
                width: 20,
            },
        );
        document.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(10, 10),
                text: "A".to_string(),
            },
        );
        document.insert_shape(
            via1,
            ShapeKind::Via {
                center: Point::new(200, 200),
                size: 40,
                lower: metal1,
                upper: metal1,
            },
        );
        document.insert_shape(
            annotation,
            ShapeKind::Measurement {
                a: Point::new(0, 0),
                b: Point::new(50, 50),
                label: "M".to_string(),
            },
        );
        let degenerate_path = document.insert_shape(
            metal1,
            ShapeKind::Path {
                points: vec![Point::new(0, 0)],
                width: 10,
            },
        );
        let degenerate_polygon = document.insert_shape(
            metal1,
            ShapeKind::Polygon(Polygon::new(vec![Point::new(0, 0), Point::new(10, 0)])),
        );
        document
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 30, 30)),
            )
            .unwrap();
        document
            .insert_instance_in_top(child, Transform::translate(500, 0))
            .unwrap();
        let array_id = document
            .insert_instance_in_top(child, Transform::translate(1_000, 0))
            .unwrap();
        let mut instance = document
            .instance(document.top_cell, array_id)
            .unwrap()
            .clone();
        instance.array = InstanceArray {
            columns: 2,
            rows: 3,
            column_pitch: Vector::new(100, 0),
            row_pitch: Vector::new(0, 100),
        };
        document.apply_operation_without_log(&crate::Operation::ReplaceInstance {
            parent: document.top_cell,
            id: array_id,
            instance,
        });

        let result = export_gdsii_with_report(&document, &default_technology()).unwrap();
        let report = result.report;

        assert!(!result.bytes.is_empty());
        assert_eq!(report.library_name, "export_report");
        assert_eq!(report.dbu_per_micron, DBU_PER_MICRON);
        assert_eq!(report.structure_count, 2);
        assert_eq!(report.boundary_count, 3);
        assert_eq!(report.path_count, 2);
        assert_eq!(report.text_count, 2);
        assert_eq!(report.sref_count, 1);
        assert_eq!(report.aref_count, 1);
        assert_eq!(report.element_count(), 9);
        assert_eq!(report.skipped_elements.len(), 2);
        assert!(
            report
                .skipped_elements
                .iter()
                .any(|skipped| skipped.shape_id == Some(degenerate_path)
                    && skipped.reason.contains("GDS PATH"))
        );
        assert!(
            report
                .skipped_elements
                .iter()
                .any(|skipped| skipped.shape_id == Some(degenerate_polygon)
                    && skipped.reason.contains("GDS BOUNDARY"))
        );
    }

    #[test]
    fn export_report_skips_aref_dimensions_that_exceed_gds_colrow_range() {
        let mut document = Document::new("oversized aref");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = document.create_cell("unit");
        document
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 30, 30)),
            )
            .unwrap();
        let array_id = document
            .insert_instance_in_top(child, Transform::translate(0, 0))
            .unwrap();
        let mut instance = document
            .instance(document.top_cell, array_id)
            .unwrap()
            .clone();
        instance.array = InstanceArray {
            columns: GDS_MAX_COLROW_DIMENSION + 1,
            rows: 2,
            column_pitch: Vector::new(100, 0),
            row_pitch: Vector::new(0, 100),
        };
        document.apply_operation_without_log(&crate::Operation::ReplaceInstance {
            parent: document.top_cell,
            id: array_id,
            instance,
        });

        let result = export_gdsii_with_report(&document, &default_technology()).unwrap();

        assert_eq!(result.report.aref_count, 0);
        assert_eq!(result.report.sref_count, 0);
        assert_eq!(result.report.skipped_elements.len(), 1);
        let skipped = &result.report.skipped_elements[0];
        assert_eq!(skipped.instance_id, Some(array_id));
        assert!(skipped.reason.contains("exceed GDSII COLROW limit"));
        assert!(skipped.reason.contains("32768x2"));
    }

    #[test]
    fn export_report_warns_about_lossy_gds_fallbacks_and_metadata() {
        let mut document = Document::new("export warnings");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let via1 = document.layer_by_process(ProcessLayer::Via1).unwrap();
        let annotation = document.layer_by_process(ProcessLayer::Annotation).unwrap();
        let child = document.create_cell("unit");

        document
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 30, 30)),
            )
            .unwrap();
        let lossy_metadata_shape = document.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        );
        {
            let mut shape = document.shapes.get_mut(&lossy_metadata_shape).unwrap();
            shape.name = Some("critical_device".to_string());
        }
        let lossy_via_shape = document.insert_shape(
            via1,
            ShapeKind::Via {
                center: Point::new(50, 120),
                size: 40,
                lower: metal1,
                upper: metal1,
            },
        );
        let lossy_measurement_shape = document.insert_shape(
            annotation,
            ShapeKind::Measurement {
                a: Point::new(0, 200),
                b: Point::new(100, 200),
                label: "CD".to_string(),
            },
        );

        let fallback_layer = document.create_layer(
            "no explicit gds mapping",
            ProcessLayer::Annotation,
            [1.0; 4],
        );
        document.layers.get_mut(&fallback_layer).unwrap().gds_layer = None;
        let fallback_layer_shape = document.insert_shape(
            fallback_layer,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(150, 0), 80, 40)),
        );

        let missing_layer_shape = document.allocate_shape_id();
        document.shapes.insert(
            missing_layer_shape,
            Shape {
                id: missing_layer_shape,
                layer: LayerId(88),
                net: None,
                kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(260, 0), 60, 40)),
                name: None,
            },
        );

        let rotated_instance = document
            .insert_instance_in_top(
                child,
                Transform::rotate_cw90().with_translation(Vector::new(500, 0)),
            )
            .unwrap();

        let report = export_gdsii_with_report(&document, &default_technology())
            .unwrap()
            .report;

        assert!(
            report.warnings.iter().any(|warning| {
                warning.kind == GdsExportWarningKind::NonRoundTrippableMetadata
                    && warning.shape_id == Some(lossy_metadata_shape)
                    && warning.message.contains("name")
            }),
            "{:?}",
            report.warnings
        );
        assert!(
            report.warnings.iter().any(|warning| {
                warning.kind == GdsExportWarningKind::NonRoundTrippableShapeKind
                    && warning.shape_id == Some(lossy_via_shape)
                    && warning.message.contains("via")
            }),
            "{:?}",
            report.warnings
        );
        assert!(
            report.warnings.iter().any(|warning| {
                warning.kind == GdsExportWarningKind::NonRoundTrippableShapeKind
                    && warning.shape_id == Some(lossy_measurement_shape)
                    && warning.message.contains("measurement")
            }),
            "{:?}",
            report.warnings
        );
        assert!(
            report.warnings.iter().any(|warning| {
                warning.kind == GdsExportWarningKind::FallbackLayerMapping
                    && warning.shape_id == Some(fallback_layer_shape)
                    && warning.layer_id == Some(fallback_layer)
            }),
            "{:?}",
            report.warnings
        );
        assert!(
            report.warnings.iter().any(|warning| {
                warning.kind == GdsExportWarningKind::MissingLayerFallback
                    && warning.shape_id == Some(missing_layer_shape)
                    && warning.layer_id == Some(LayerId(88))
            }),
            "{:?}",
            report.warnings
        );
        assert!(
            report.warnings.iter().any(|warning| {
                warning.kind == GdsExportWarningKind::UnsupportedInstanceTransform
                    && warning.instance_id == Some(rotated_instance)
                    && warning.message.contains("translation only")
            }),
            "{:?}",
            report.warnings
        );
    }

    #[test]
    fn export_report_warns_about_ambiguous_document_gds_mappings() {
        let mut document = Document::new("ambiguous export mappings");
        let first = document.create_layer(
            "custom implant a",
            ProcessLayer::Annotation,
            [0.2, 0.8, 0.4, 0.5],
        );
        let second = document.create_layer(
            "custom implant b",
            ProcessLayer::Annotation,
            [0.4, 0.3, 0.9, 0.5],
        );
        for layer_id in [first, second] {
            let layer = document.layers.get_mut(&layer_id).unwrap();
            layer.gds_layer = Some(60);
            layer.gds_datatype = 4;
            layer.gds_texttype = 9;
        }

        let rectangle = document.insert_shape(
            first,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
        );
        let label = document.insert_shape(
            second,
            ShapeKind::Label {
                position: Point::new(20, 20),
                text: "probe".to_string(),
            },
        );

        let report = export_gdsii_with_report(&document, &default_technology())
            .unwrap()
            .report;

        assert!(
            report.warnings.iter().any(|warning| {
                warning.kind == GdsExportWarningKind::AmbiguousDocumentLayerMapping
                    && warning.shape_id == Some(rectangle)
                    && warning.layer_id == Some(first)
                    && warning.message.contains("geometry mapping (60, 4)")
                    && warning.message.contains(&format!("{}", second.0))
            }),
            "{:?}",
            report.warnings
        );
        assert!(
            report.warnings.iter().any(|warning| {
                warning.kind == GdsExportWarningKind::AmbiguousDocumentLayerMapping
                    && warning.shape_id == Some(label)
                    && warning.layer_id == Some(second)
                    && warning.message.contains("text mapping (60, 9)")
                    && warning.message.contains(&format!("{}", first.0))
            }),
            "{:?}",
            report.warnings
        );
    }

    #[test]
    fn export_report_warns_when_path_width_is_normalized() {
        let mut document = Document::new("export path width warning");
        let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let path_id = document.insert_shape(
            metal1,
            ShapeKind::Path {
                points: vec![Point::new(0, 0), Point::new(100, 0)],
                width: 0,
            },
        );

        let exported = export_gdsii_with_report(&document, &default_technology()).unwrap();

        assert_eq!(exported.report.path_count, 1);
        let warning = exported
            .report
            .warnings
            .iter()
            .find(|warning| warning.kind == GdsExportWarningKind::NormalizedPathWidth)
            .expect("export report should include normalized path width warning");
        assert_eq!(warning.shape_id, Some(path_id));
        assert!(warning.message.contains("width 0"), "{warning:?}");
        assert!(
            warning.message.contains("positive GDS PATH width 1"),
            "{warning:?}"
        );

        let imported = import_gdsii_with_report(&exported.bytes, &default_technology()).unwrap();
        let imported_width = imported
            .document
            .shapes
            .values()
            .find_map(|shape| match &shape.kind {
                ShapeKind::Path { width, .. } => Some(*width),
                _ => None,
            })
            .expect("round-tripped document should contain a path");
        assert_eq!(imported_width, 1);
    }

    #[test]
    fn import_report_records_skipped_degenerate_geometry() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "BAD_GEOMETRY").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();

        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[4]).unwrap();
        writer.write_i16(DATATYPE, &[0]).unwrap();
        writer
            .write_xy(&[Point::new(0, 0), Point::new(100, 0), Point::new(0, 0)])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();

        writer.write_no_data(PATH).unwrap();
        writer.write_i16(LAYER, &[5]).unwrap();
        writer.write_i16(DATATYPE, &[0]).unwrap();
        writer.write_i32(WIDTH, &[20]).unwrap();
        writer.write_xy(&[Point::new(0, 0)]).unwrap();
        writer.write_no_data(ENDEL).unwrap();

        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let imported =
            import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();

        assert_eq!(imported.report.element_count, 2);
        assert_eq!(imported.report.skipped_elements.len(), 2);
        assert!(imported.document.visible_flattened_shapes().is_empty());
        assert!(
            imported.report.skipped_elements.iter().any(|skipped| {
                skipped.element_kind == "BOUNDARY"
                    && skipped.gds_layer == Some(4)
                    && skipped.reason.contains("at least 3 distinct points")
            }),
            "{:?}",
            imported.report.skipped_elements
        );
        assert!(
            imported.report.skipped_elements.iter().any(|skipped| {
                skipped.element_kind == "PATH"
                    && skipped.gds_layer == Some(5)
                    && skipped.reason.contains("at least 2")
            }),
            "{:?}",
            imported.report.skipped_elements
        );
    }

    #[test]
    fn import_report_warns_when_path_width_is_normalized() {
        let imported = import_gdsii_with_report(
            &gds_bytes_from_import_fixture(
                "PATH_WIDTH_NORMALIZED",
                &[GdsImportFixtureElement::Path {
                    gds_layer: 7,
                    datatype: 0,
                    width: 0,
                    points: vec![[0, 0], [100, 0]],
                }],
            ),
            &default_technology(),
        )
        .unwrap();

        assert_eq!(imported.report.skipped_elements, Vec::new());
        assert_eq!(imported.report.warnings.len(), 1);
        let warning = &imported.report.warnings[0];
        assert_eq!(warning.kind, GdsImportWarningKind::NormalizedPathWidth);
        assert_eq!(warning.gds_layer, Some(7));
        assert_eq!(warning.gds_type, Some(0));
        assert!(warning.message.contains("width 0"));
        assert!(warning.message.contains("positive width 1"));

        let imported_width = imported
            .document
            .shapes
            .values()
            .find_map(|shape| match &shape.kind {
                ShapeKind::Path { width, .. } => Some(*width),
                _ => None,
            })
            .unwrap();
        assert_eq!(imported_width, 1);
    }

    #[test]
    fn import_report_warns_when_units_are_normalized() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "BAD_UNITS").unwrap();
        writer.write_real8(UNITS, &[10.0, 1.0e-5]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let imported =
            import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();

        assert_eq!(imported.report.source_dbu_per_micron, 1);
        assert_eq!(imported.report.element_count, 0);
        assert_eq!(imported.report.warnings.len(), 1);
        let warning = &imported.report.warnings[0];
        assert_eq!(warning.kind, GdsImportWarningKind::NormalizedUnits);
        assert_eq!(warning.gds_layer, None);
        assert!(warning.message.contains("GDS UNITS"));
        assert!(warning.message.contains("dbu_per_micron"));
        assert!(warning.message.contains("minimum"));
    }

    #[test]
    fn import_report_warns_when_scaled_coordinates_are_clamped() {
        let mut report = GdsImportReport {
            library_name: "CLAMP".to_string(),
            source_dbu_per_micron: 1,
            structure_count: 1,
            element_count: 1,
            generated_layers: Vec::new(),
            skipped_elements: Vec::new(),
            warnings: Vec::new(),
        };
        let context = GdsScaleWarningContext::new("BOUNDARY", Some(12), Some(0), Some(false), None);

        let point = scale_point(
            Point::new(Coord::MAX, Coord::MIN),
            1,
            1_000,
            &mut report,
            context,
        );

        assert_eq!(point, Point::new(Coord::MAX, Coord::MIN));
        assert_eq!(report.warnings.len(), 2);
        for warning in &report.warnings {
            assert_eq!(warning.kind, GdsImportWarningKind::CoordinateClamped);
            assert_eq!(warning.gds_layer, Some(12));
            assert_eq!(warning.gds_type, Some(0));
            assert_eq!(warning.is_text, Some(false));
            assert!(warning.message.contains("BOUNDARY"));
            assert!(warning.message.contains("clamped"));
        }
    }

    #[test]
    fn import_report_warns_when_aref_dimensions_are_normalized() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "AREF_DIMENSIONS").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();

        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(AREF).unwrap();
        writer.write_ascii(SNAME, "UNIT").unwrap();
        writer.write_i16(COLROW, &[0, 0]).unwrap();
        writer
            .write_xy(&[Point::new(0, 0), Point::new(500, 0), Point::new(0, 500)])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();

        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "UNIT").unwrap();
        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[4]).unwrap();
        writer.write_i16(DATATYPE, &[0]).unwrap();
        writer
            .write_xy(&[
                Point::new(0, 0),
                Point::new(100, 0),
                Point::new(100, 100),
                Point::new(0, 100),
                Point::new(0, 0),
            ])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let result = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();
        let warning = result
            .report
            .warnings
            .iter()
            .find(|warning| warning.kind == GdsImportWarningKind::NormalizedArefDimensions)
            .expect("expected normalized AREF dimension warning");

        assert_eq!(warning.gds_layer, None);
        assert_eq!(warning.gds_type, None);
        assert!(warning.message.contains("0x0"));
        assert!(warning.message.contains("1x1"));
        let top = result.document.cell(result.document.top_cell).unwrap();
        let instance = top.instances.values().next().unwrap();
        assert_eq!(instance.array.columns, 1);
        assert_eq!(instance.array.rows, 1);
        assert_eq!(result.document.visible_flattened_shapes().len(), 1);
    }

    #[test]
    fn import_report_warns_about_duplicate_structure_names() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "DUPLICATE_STRUCTURES").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();

        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(SREF).unwrap();
        writer.write_ascii(SNAME, "UNIT").unwrap();
        writer.write_xy(&[Point::new(0, 0)]).unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();

        for x_offset in [0, 300] {
            writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
            writer.write_ascii(STRNAME, "UNIT").unwrap();
            writer.write_no_data(BOUNDARY).unwrap();
            writer.write_i16(LAYER, &[4]).unwrap();
            writer.write_i16(DATATYPE, &[0]).unwrap();
            writer
                .write_xy(&[
                    Point::new(x_offset, 0),
                    Point::new(x_offset + 100, 0),
                    Point::new(x_offset + 100, 100),
                    Point::new(x_offset, 100),
                    Point::new(x_offset, 0),
                ])
                .unwrap();
            writer.write_no_data(ENDEL).unwrap();
            writer.write_no_data(ENDSTR).unwrap();
        }
        writer.write_no_data(ENDLIB).unwrap();

        let result = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();
        let warning = result
            .report
            .warnings
            .iter()
            .find(|warning| warning.kind == GdsImportWarningKind::DuplicateStructureName)
            .expect("expected duplicate structure warning");

        assert!(warning.message.contains("UNIT"));
        assert!(warning.message.contains("2 times"));
        assert_eq!(result.report.structure_count, 3);
        assert_eq!(
            result
                .document
                .cells
                .values()
                .filter(|cell| cell.name == "UNIT")
                .count(),
            1
        );
        let unit = result
            .document
            .cells
            .values()
            .find(|cell| cell.name == "UNIT")
            .expect("UNIT cell should import");
        assert_eq!(unit.shapes.len(), 2);
    }

    #[test]
    fn import_rejects_unterminated_gds_structure() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer
            .write_ascii(LIBNAME, "UNTERMINATED_STRUCTURE")
            .unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();

        let error = import_gdsii_with_report(&writer.into_bytes(), &default_technology())
            .unwrap_err()
            .to_string();

        assert!(error.contains("unexpected EOF before ENDSTR"), "{error}");
    }

    #[test]
    fn import_rejects_unterminated_gds_element() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "UNTERMINATED_ELEMENT").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[4]).unwrap();
        writer.write_i16(DATATYPE, &[0]).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let error = import_gdsii_with_report(&writer.into_bytes(), &default_technology())
            .unwrap_err()
            .to_string();

        assert!(
            error.contains("ENDLIB before ENDEL for BOUNDARY"),
            "{error}"
        );
    }

    #[test]
    fn import_rejects_new_gds_element_before_endel() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "OVERLAPPED_ELEMENTS").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[4]).unwrap();
        writer.write_no_data(PATH).unwrap();

        let error = import_gdsii_with_report(&writer.into_bytes(), &default_technology())
            .unwrap_err()
            .to_string();

        assert!(error.contains("PATH before ENDEL for BOUNDARY"), "{error}");
    }

    #[test]
    fn import_rejects_missing_gds_endlib() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "MISSING_ENDLIB").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(ENDSTR).unwrap();

        let error = import_gdsii_with_report(&writer.into_bytes(), &default_technology())
            .unwrap_err()
            .to_string();

        assert!(error.contains("unexpected EOF before ENDLIB"), "{error}");
    }

    #[test]
    fn degenerate_gds_geometry_fixture_reports_skipped_elements() {
        let fixture: GdsDegenerateImportFixture = serde_json::from_str(include_str!(
            "../../../fixtures/import_export/gds_degenerate_geometry_import.json"
        ))
        .unwrap();
        let result = import_gdsii_with_report(
            &gds_bytes_from_import_fixture(&fixture.name, &fixture.elements),
            &default_technology(),
        )
        .unwrap();

        assert_eq!(result.report.element_count, fixture.expect.element_count);
        assert_eq!(
            result.report.generated_layers.len(),
            fixture.expect.generated_layer_count
        );
        assert_eq!(
            result.document.visible_flattened_shapes().len(),
            fixture.expect.visible_shape_count
        );
        assert_eq!(
            result.report.skipped_elements.len(),
            fixture.expect.skipped_count
        );
        assert_expected_import_skips(&result.report, &fixture.expect.skipped);
    }

    #[test]
    fn import_report_skips_known_unsupported_elements_without_aborting() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "UNSUPPORTED_ELEMENT").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();

        writer.write_no_data(BOX).unwrap();
        writer.write_i16(LAYER, &[31]).unwrap();
        writer.write_i16(BOXTYPE, &[2]).unwrap();
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

        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[1]).unwrap();
        writer.write_i16(DATATYPE, &[0]).unwrap();
        writer
            .write_xy(&[
                Point::new(20, 20),
                Point::new(40, 20),
                Point::new(40, 40),
                Point::new(20, 40),
                Point::new(20, 20),
            ])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let result = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();

        assert_eq!(result.report.element_count, 2);
        assert_eq!(result.document.visible_flattened_shapes().len(), 1);
        assert!(
            result.report.skipped_elements.iter().any(|skipped| {
                skipped.element_kind == "BOX"
                    && skipped.gds_layer == Some(31)
                    && skipped.gds_type == Some(2)
                    && skipped.reason.contains("unsupported GDSII BOX")
            }),
            "{:?}",
            result.report.skipped_elements
        );
    }

    #[test]
    fn unsupported_gds_elements_fixture_reports_skipped_elements_without_aborting() {
        let fixture: GdsUnsupportedImportFixture = serde_json::from_str(include_str!(
            "../../../fixtures/import_export/gds_unsupported_elements_import.json"
        ))
        .unwrap();
        let result = import_gdsii_with_report(
            &gds_bytes_from_import_fixture(&fixture.name, &fixture.elements),
            &default_technology(),
        )
        .unwrap();

        assert_eq!(result.report.element_count, fixture.expect.element_count);
        assert_eq!(
            result.report.generated_layers.len(),
            fixture.expect.generated_layer_count
        );
        assert_eq!(
            result.document.visible_flattened_shapes().len(),
            fixture.expect.visible_shape_count
        );
        assert_eq!(
            result.report.skipped_elements.len(),
            fixture.expect.skipped_count
        );
        assert_expected_import_skips(&result.report, &fixture.expect.skipped);
    }

    fn assert_expected_import_skips(
        report: &GdsImportReport,
        expected_skips: &[GdsExpectedSkippedImportElement],
    ) {
        for expected in expected_skips {
            let skipped = report
                .skipped_elements
                .iter()
                .find(|skipped| {
                    skipped.element_kind == expected.element_kind
                        && skipped.gds_layer == expected.gds_layer
                        && skipped.gds_type == expected.gds_type
                })
                .unwrap_or_else(|| {
                    panic!(
                        "missing expected skipped GDS element {:?}; got {:?}",
                        expected.element_kind, report.skipped_elements
                    )
                });
            assert!(
                skipped.reason.contains(&expected.reason_contains),
                "{:?} did not contain {:?}",
                skipped.reason,
                expected.reason_contains
            );
        }
    }

    #[test]
    fn import_report_skips_missing_instance_targets_without_aborting() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "MISSING_REFS").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();

        writer.write_no_data(BOUNDARY).unwrap();
        writer.write_i16(LAYER, &[4]).unwrap();
        writer.write_i16(DATATYPE, &[0]).unwrap();
        writer
            .write_xy(&[
                Point::new(0, 0),
                Point::new(100, 0),
                Point::new(100, 100),
                Point::new(0, 100),
                Point::new(0, 0),
            ])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();

        writer.write_no_data(SREF).unwrap();
        writer.write_ascii(SNAME, "MISSING_UNIT").unwrap();
        writer.write_xy(&[Point::new(200, 0)]).unwrap();
        writer.write_no_data(ENDEL).unwrap();

        writer.write_no_data(AREF).unwrap();
        writer.write_ascii(SNAME, "MISSING_ARRAY_UNIT").unwrap();
        writer.write_i16(COLROW, &[2, 2]).unwrap();
        writer
            .write_xy(&[Point::new(300, 0), Point::new(500, 0), Point::new(300, 200)])
            .unwrap();
        writer.write_no_data(ENDEL).unwrap();

        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let imported =
            import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();

        assert_eq!(imported.report.element_count, 3);
        assert_eq!(imported.document.visible_flattened_shapes().len(), 1);
        assert_eq!(
            imported
                .document
                .cell(imported.document.top_cell)
                .unwrap()
                .instances
                .len(),
            0
        );
        assert_eq!(imported.report.skipped_elements.len(), 2);
        assert!(imported.report.skipped_elements.iter().any(|skipped| {
            skipped.element_kind == "SREF"
                && skipped.gds_layer.is_none()
                && skipped.gds_type.is_none()
                && skipped.reason.contains("MISSING_UNIT")
        }));
        assert!(imported.report.skipped_elements.iter().any(|skipped| {
            skipped.element_kind == "AREF"
                && skipped.gds_layer.is_none()
                && skipped.gds_type.is_none()
                && skipped.reason.contains("MISSING_ARRAY_UNIT")
        }));
    }

    #[test]
    fn missing_gds_reference_fixture_reports_skipped_instances_without_aborting() {
        let fixture: GdsMissingRefsImportFixture = serde_json::from_str(include_str!(
            "../../../fixtures/import_export/gds_missing_references_import.json"
        ))
        .unwrap();
        let result = import_gdsii_with_report(
            &gds_bytes_from_import_fixture(&fixture.name, &fixture.elements),
            &default_technology(),
        )
        .unwrap();

        assert_eq!(result.report.element_count, fixture.expect.element_count);
        assert_eq!(
            result.report.generated_layers.len(),
            fixture.expect.generated_layer_count
        );
        assert_eq!(
            result.document.visible_flattened_shapes().len(),
            fixture.expect.visible_shape_count
        );
        assert_eq!(
            result
                .document
                .cell(result.document.top_cell)
                .unwrap()
                .instances
                .len(),
            fixture.expect.top_instance_count
        );
        assert_eq!(
            result.report.skipped_elements.len(),
            fixture.expect.skipped_count
        );
        assert_expected_import_skips(&result.report, &fixture.expect.skipped);
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
    fn gds_round_trip_fixture_reports_and_preserves_arrays() {
        let fixture: GdsRoundTripFixture = serde_json::from_str(include_str!(
            "../../../fixtures/import_export/gds_round_trip.json"
        ))
        .unwrap();
        let document = document_from_gds_fixture(&fixture);
        let technology = default_technology();

        let exported = export_gdsii_with_report(&document, &technology).unwrap();
        assert_eq!(
            exported.report.structure_count,
            fixture.expect.export.structure_count
        );
        assert_eq!(
            exported.report.boundary_count,
            fixture.expect.export.boundary_count
        );
        assert_eq!(exported.report.path_count, fixture.expect.export.path_count);
        assert_eq!(exported.report.text_count, fixture.expect.export.text_count);
        assert_eq!(exported.report.sref_count, fixture.expect.export.sref_count);
        assert_eq!(exported.report.aref_count, fixture.expect.export.aref_count);
        assert_eq!(
            exported.report.skipped_elements.len(),
            fixture.expect.export.skipped_count
        );
        assert_eq!(
            exported.report.warnings.len(),
            fixture.expect.export.warning_count
        );

        let imported = import_gdsii_with_report(&exported.bytes, &technology).unwrap();
        assert_eq!(
            imported.report.structure_count,
            fixture.expect.import.structure_count
        );
        assert_eq!(
            imported.report.element_count,
            fixture.expect.import.element_count
        );
        assert_eq!(
            imported.document.shapes.len(),
            fixture.expect.import.top_shape_count
        );
        let top = imported.document.cell(imported.document.top_cell).unwrap();
        assert_eq!(
            top.instances.len(),
            fixture.expect.import.top_instance_count
        );
        assert_eq!(
            imported.document.visible_flattened_shapes().len(),
            fixture.expect.import.flattened_shape_count
        );

        let imported_instance = top.instances.values().next().unwrap();
        assert_eq!(imported_instance.transform, Transform::translate(500, 600));
        assert_eq!(
            imported_instance.array.columns,
            fixture.expect.import.array_columns
        );
        assert_eq!(
            imported_instance.array.rows,
            fixture.expect.import.array_rows
        );
        assert_eq!(
            imported_instance.array.column_pitch,
            Vector::new(
                fixture.expect.import.array_column_pitch[0],
                fixture.expect.import.array_column_pitch[1]
            )
        );
        assert_eq!(
            imported_instance.array.row_pitch,
            Vector::new(
                fixture.expect.import.array_row_pitch[0],
                fixture.expect.import.array_row_pitch[1]
            )
        );

        let unit = imported
            .document
            .cells
            .values()
            .find(|cell| cell.name == "UNIT")
            .unwrap();
        assert!(
            unit.shapes.values().any(|shape| {
                matches!(&shape.kind, ShapeKind::Label { text, .. } if text == "IN")
            })
        );
        assert!(
            unit.shapes
                .values()
                .any(|shape| matches!(shape.kind, ShapeKind::Path { width: 80, .. }))
        );
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

        let imported =
            import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();
        let report = imported.report;
        let imported = imported.document;
        let layer = imported
            .layers
            .values()
            .find(|layer| layer.gds_layer == Some(55))
            .unwrap();

        assert_eq!(report.library_name, "UNKNOWN_LAYER");
        assert_eq!(report.structure_count, 1);
        assert_eq!(report.element_count, 1);
        assert_eq!(report.generated_layers.len(), 1);
        assert_eq!(report.generated_layers[0].gds_layer, 55);
        assert_eq!(report.generated_layers[0].gds_type, 7);
        assert!(!report.generated_layers[0].is_text);
        assert_eq!(report.generated_layers[0].layer_id, layer.id);
        assert_eq!(layer.gds_datatype, 7);
        assert_ne!(layer.id, LayerId(55));
        assert_eq!(imported.shapes.values().next().unwrap().layer, layer.id);
    }

    #[test]
    fn import_report_warns_when_one_gds_layer_splits_across_document_layers() {
        let fixture: GdsImportLayerFixture = serde_json::from_str(include_str!(
            "../../../fixtures/import_export/gds_split_layer_import.json"
        ))
        .unwrap();
        let report = import_gdsii_with_report(
            &gds_bytes_from_import_layer_fixture(&fixture),
            &default_technology(),
        )
        .unwrap()
        .report;

        assert_eq!(
            report.generated_layers.len(),
            fixture.expect.generated_layer_count
        );
        assert_eq!(
            report.warnings.len(),
            fixture.expect.warning_count,
            "{:?}",
            report.warnings
        );
        let warning = &report.warnings[0];
        assert_eq!(
            warning.kind,
            GdsImportWarningKind::SplitIncomingLayerMapping
        );
        assert_eq!(
            warning.gds_layer,
            Some(fixture.expect.split_warning_gds_layer)
        );
        for needle in &fixture.expect.message_contains {
            assert!(
                warning.message.contains(needle),
                "{:?} did not contain {:?}",
                warning.message,
                needle
            );
        }
    }

    #[test]
    fn unknown_gds_text_layers_are_imported_as_text_mapped_layers() {
        let mut writer = GdsWriter::new();
        writer.write_i16(HEADER, &[600]).unwrap();
        writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
        writer.write_ascii(LIBNAME, "UNKNOWN_TEXT").unwrap();
        writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
        writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
        writer.write_ascii(STRNAME, "TOP").unwrap();
        writer.write_no_data(TEXT).unwrap();
        writer.write_i16(LAYER, &[77]).unwrap();
        writer.write_i16(TEXTTYPE, &[9]).unwrap();
        writer.write_xy(&[Point::new(120, -80)]).unwrap();
        writer.write_ascii(STRING, "probe_label").unwrap();
        writer.write_no_data(ENDEL).unwrap();
        writer.write_no_data(ENDSTR).unwrap();
        writer.write_no_data(ENDLIB).unwrap();

        let imported =
            import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();
        let report = imported.report;
        let imported = imported.document;
        let shape = imported.shapes.values().next().unwrap();
        let layer = imported.layers.get(&shape.layer).unwrap();

        assert_eq!(report.library_name, "UNKNOWN_TEXT");
        assert_eq!(report.structure_count, 1);
        assert_eq!(report.element_count, 1);
        assert_eq!(report.generated_layers.len(), 1);
        assert_eq!(report.generated_layers[0].gds_layer, 77);
        assert_eq!(report.generated_layers[0].gds_type, 9);
        assert!(report.generated_layers[0].is_text);
        assert_eq!(report.generated_layers[0].layer_id, layer.id);
        assert_eq!(layer.gds_layer, Some(77));
        assert_eq!(layer.gds_texttype, 9);
        assert_ne!(layer.id, LayerId(77));
        assert!(matches!(
            &shape.kind,
            ShapeKind::Label { position, text }
                if *position == Point::new(120, -80) && text == "probe_label"
        ));
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
