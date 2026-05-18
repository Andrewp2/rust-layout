#![allow(unused_imports)]
use super::*;

pub(crate) const HEADER: u8 = 0x00;
pub(crate) const BGNLIB: u8 = 0x01;
pub(crate) const LIBNAME: u8 = 0x02;
pub(crate) const UNITS: u8 = 0x03;
pub(crate) const ENDLIB: u8 = 0x04;
pub(crate) const BGNSTR: u8 = 0x05;
pub(crate) const STRNAME: u8 = 0x06;
pub(crate) const ENDSTR: u8 = 0x07;
pub(crate) const BOUNDARY: u8 = 0x08;
pub(crate) const PATH: u8 = 0x09;
pub(crate) const SREF: u8 = 0x0a;
pub(crate) const AREF: u8 = 0x0b;
pub(crate) const TEXT: u8 = 0x0c;
pub(crate) const LAYER: u8 = 0x0d;
pub(crate) const DATATYPE: u8 = 0x0e;
pub(crate) const WIDTH: u8 = 0x0f;
pub(crate) const XY: u8 = 0x10;
pub(crate) const ENDEL: u8 = 0x11;
pub(crate) const SNAME: u8 = 0x12;
pub(crate) const COLROW: u8 = 0x13;
pub(crate) const TEXTNODE: u8 = 0x14;
pub(crate) const NODE: u8 = 0x15;
pub(crate) const TEXTTYPE: u8 = 0x16;
pub(crate) const STRING: u8 = 0x19;
pub(crate) const STRANS: u8 = 0x1a;
pub(crate) const MAG: u8 = 0x1b;
pub(crate) const ANGLE: u8 = 0x1c;
pub(crate) const PATHTYPE: u8 = 0x21;
pub(crate) const NODETYPE: u8 = 0x2a;
pub(crate) const PROPATTR: u8 = 0x2b;
pub(crate) const PROPVALUE: u8 = 0x2c;
pub(crate) const BOX: u8 = 0x2d;
pub(crate) const BOXTYPE: u8 = 0x2e;
pub(crate) const BGNEXTN: u8 = 0x30;
pub(crate) const ENDEXTN: u8 = 0x31;

pub(crate) const NO_DATA: u8 = 0;
pub(crate) const BIT_ARRAY: u8 = 1;
pub(crate) const INT_2: u8 = 2;
pub(crate) const INT_4: u8 = 3;
pub(crate) const REAL_8: u8 = 5;
pub(crate) const ASCII: u8 = 6;
pub(crate) const GDS_MAX_COLROW_DIMENSION: u32 = i16::MAX as u32;
pub(crate) const GDS_STRANS_REFLECT_X: u16 = 0x8000;
pub(crate) const GDS_TRANSFORM_EPSILON: f64 = 1.0e-9;
pub(crate) const GLASSWORKS_GDS_SHAPE_NAME_PROP_ATTR: u16 = 127;
pub(crate) const GLASSWORKS_GDS_SHAPE_NET_PROP_ATTR: u16 = 126;
pub(crate) const GLASSWORKS_GDS_SHAPE_PROPERTY_PROP_ATTR: u16 = 125;

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
    UnsupportedInstanceTransform,
    UnsupportedPathStyle,
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
pub(crate) struct GdsImportMappingKey {
    pub(crate) gds_layer: u16,
    pub(crate) gds_type: u16,
    pub(crate) is_text: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GdsScaleWarningContext {
    pub(crate) element_kind: &'static str,
    pub(crate) gds_layer: Option<u16>,
    pub(crate) gds_type: Option<u16>,
    pub(crate) is_text: Option<bool>,
    pub(crate) layer_id: Option<LayerId>,
}

impl GdsScaleWarningContext {
    pub(crate) const fn new(
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
pub(crate) struct ParsedLibrary {
    pub(crate) name: String,
    pub(crate) dbu_per_micron: Coord,
    pub(crate) warnings: Vec<GdsImportWarning>,
    pub(crate) structures: Vec<ParsedStructure>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedStructure {
    pub(crate) name: String,
    pub(crate) elements: Vec<ParsedElement>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ParsedElement {
    Boundary {
        layer: u16,
        datatype: u16,
        points: Vec<Point>,
        properties: Vec<GdsProperty>,
    },
    Box {
        layer: u16,
        boxtype: u16,
        points: Vec<Point>,
        properties: Vec<GdsProperty>,
    },
    Path {
        layer: u16,
        datatype: u16,
        pathtype: u16,
        begin_extension: Option<Coord>,
        end_extension: Option<Coord>,
        raw_width: Coord,
        width: Coord,
        points: Vec<Point>,
        properties: Vec<GdsProperty>,
    },
    Text {
        layer: u16,
        texttype: u16,
        position: Point,
        text: String,
        properties: Vec<GdsProperty>,
    },
    Sref {
        name: String,
        origin: Point,
        transform: GdsInstanceTransformSpec,
        properties: Vec<GdsProperty>,
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
        transform: GdsInstanceTransformSpec,
        properties: Vec<GdsProperty>,
    },
    Unsupported {
        kind: String,
        layer: Option<u16>,
        gds_type: Option<u16>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GdsProperty {
    pub(crate) attr: u16,
    pub(crate) value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ElementKind {
    Boundary,
    Box,
    Path,
    Text,
    Sref,
    Aref,
    Unsupported(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GdsInstanceTransformSpec {
    pub(crate) reflect_x: bool,
    pub(crate) magnification: Option<f64>,
    pub(crate) angle_degrees: Option<f64>,
}

impl GdsInstanceTransformSpec {
    pub(crate) const IDENTITY: Self = Self {
        reflect_x: false,
        magnification: None,
        angle_degrees: None,
    };

    pub(crate) fn strans_bits(self) -> u16 {
        if self.reflect_x {
            GDS_STRANS_REFLECT_X
        } else {
            0
        }
    }

    pub(crate) fn needs_transform_record(self) -> bool {
        self.reflect_x
            || self
                .magnification
                .is_some_and(|magnification| !approximately_one(magnification))
            || self
                .angle_degrees
                .is_some_and(|angle| !approximately_zero(normalize_gds_angle(angle)))
    }
}

impl ElementKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Boundary => "BOUNDARY",
            Self::Box => "BOX",
            Self::Path => "PATH",
            Self::Text => "TEXT",
            Self::Sref => "SREF",
            Self::Aref => "AREF",
            Self::Unsupported(kind) => kind,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ElementBuilder {
    pub(crate) kind: ElementKind,
    pub(crate) layer: Option<u16>,
    pub(crate) datatype: u16,
    pub(crate) texttype: u16,
    pub(crate) width: Coord,
    pub(crate) points: Vec<Point>,
    pub(crate) name: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) columns: u16,
    pub(crate) rows: u16,
    pub(crate) transform: GdsInstanceTransformSpec,
    pub(crate) pathtype: u16,
    pub(crate) begin_extension: Option<Coord>,
    pub(crate) end_extension: Option<Coord>,
    pub(crate) properties: Vec<GdsProperty>,
    pub(crate) pending_property_attr: Option<u16>,
}

impl ElementBuilder {
    pub(crate) fn new(kind: ElementKind) -> Self {
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
            transform: GdsInstanceTransformSpec::IDENTITY,
            pathtype: 0,
            begin_extension: None,
            end_extension: None,
            properties: Vec::new(),
            pending_property_attr: None,
        }
    }

    pub(crate) fn finish(self) -> Result<ParsedElement, GdsError> {
        match self.kind {
            ElementKind::Boundary => Ok(ParsedElement::Boundary {
                layer: self.layer.ok_or(GdsError::MissingField("LAYER"))?,
                datatype: self.datatype,
                points: self.points,
                properties: self.properties,
            }),
            ElementKind::Box => Ok(ParsedElement::Box {
                layer: self.layer.ok_or(GdsError::MissingField("LAYER"))?,
                boxtype: self.datatype,
                points: self.points,
                properties: self.properties,
            }),
            ElementKind::Path => Ok(ParsedElement::Path {
                layer: self.layer.ok_or(GdsError::MissingField("LAYER"))?,
                datatype: self.datatype,
                pathtype: self.pathtype,
                begin_extension: self.begin_extension,
                end_extension: self.end_extension,
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
                properties: self.properties,
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
                    properties: self.properties,
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
                    transform: self.transform,
                    properties: self.properties,
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
                    transform: self.transform,
                    properties: self.properties,
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
pub(crate) struct GdsExportMappingKey {
    pub(crate) gds_layer: u16,
    pub(crate) gds_type: u16,
    pub(crate) is_text: bool,
}

pub(crate) fn ambiguous_gds_export_mappings(
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
    let library_name = sanitize_gds_name(&document.name, "GLASSWORKS");
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

pub(crate) fn write_structure(
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
        let gds_transform = gds_transform_spec_for_matrix(instance.transform.matrix);
        if instance.transform.matrix != Transform::IDENTITY.matrix && gds_transform.is_none() {
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
            if let Some(transform) = gds_transform {
                write_gds_instance_transform_records(writer, transform)?;
            }
            writer.write_xy(&[origin])?;
            write_gds_key_value_properties(writer, &instance.properties)?;
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
            if let Some(transform) = gds_transform {
                write_gds_instance_transform_records(writer, transform)?;
            }
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
            write_gds_key_value_properties(writer, &instance.properties)?;
            report.aref_count += 1;
        }
        writer.write_no_data(ENDEL)?;
    }

    writer.write_no_data(ENDSTR)?;
    Ok(())
}

pub(crate) fn write_gds_instance_transform_records(
    writer: &mut GdsWriter,
    transform: GdsInstanceTransformSpec,
) -> Result<(), GdsError> {
    if !transform.needs_transform_record() {
        return Ok(());
    }
    writer.write_bit_array(STRANS, transform.strans_bits())?;
    if let Some(magnification) = transform.magnification
        && !approximately_one(magnification)
    {
        writer.write_real8(MAG, &[magnification])?;
    }
    if let Some(angle) = transform.angle_degrees {
        let angle = normalize_gds_angle(angle);
        if !approximately_zero(angle) {
            writer.write_real8(ANGLE, &[angle])?;
        }
    }
    Ok(())
}

pub(crate) fn gds_transform_spec_for_matrix(matrix: [i8; 4]) -> Option<GdsInstanceTransformSpec> {
    let (reflect_x, angle_degrees) = match matrix {
        [1, 0, 0, 1] => (false, None),
        [0, -1, 1, 0] => (false, Some(90.0)),
        [-1, 0, 0, -1] => (false, Some(180.0)),
        [0, 1, -1, 0] => (false, Some(270.0)),
        [1, 0, 0, -1] => (true, None),
        [0, 1, 1, 0] => (true, Some(90.0)),
        [-1, 0, 0, 1] => (true, Some(180.0)),
        [0, -1, -1, 0] => (true, Some(270.0)),
        _ => return None,
    };
    Some(GdsInstanceTransformSpec {
        reflect_x,
        magnification: None,
        angle_degrees,
    })
}

pub(crate) fn write_shape(
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
            write_shape_properties(writer, shape)?;
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
            write_shape_properties(writer, shape)?;
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
            write_shape_properties(writer, shape)?;
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
            write_shape_properties(writer, shape)?;
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
            write_shape_properties(writer, shape)?;
            writer.write_no_data(ENDEL)?;
            report.text_count += 1;
        }
        ShapeKind::Measurement { a, b, label, .. } => {
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
            write_shape_properties(writer, shape)?;
            writer.write_no_data(ENDEL)?;
            report.path_count += 1;

            writer.write_no_data(TEXT)?;
            writer.write_i16(LAYER, &[u16_to_i16(layer)?])?;
            writer.write_i16(TEXTTYPE, &[u16_to_i16(texttype)?])?;
            writer.write_xy(&[*a])?;
            writer.write_ascii(STRING, label)?;
            write_shape_properties(writer, shape)?;
            writer.write_no_data(ENDEL)?;
            report.text_count += 1;
        }
    }
    Ok(())
}

pub(crate) fn write_shape_properties(
    writer: &mut GdsWriter,
    shape: &Shape,
) -> Result<(), GdsError> {
    if let Some(name) = shape
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        writer.write_i16(PROPATTR, &[u16_to_i16(GLASSWORKS_GDS_SHAPE_NAME_PROP_ATTR)?])?;
        writer.write_ascii(PROPVALUE, name)?;
    }
    if let Some(net) = shape.net {
        writer.write_i16(PROPATTR, &[u16_to_i16(GLASSWORKS_GDS_SHAPE_NET_PROP_ATTR)?])?;
        writer.write_ascii(PROPVALUE, &net.0.to_string())?;
    }
    write_gds_key_value_properties(writer, &shape.properties)?;
    Ok(())
}

pub(crate) fn write_gds_key_value_properties(
    writer: &mut GdsWriter,
    properties: &BTreeMap<String, String>,
) -> Result<(), GdsError> {
    for (key, value) in properties {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        if let Some(attr) = shape_property_gds_attr_key(key) {
            writer.write_i16(PROPATTR, &[u16_to_i16(attr)?])?;
            writer.write_ascii(PROPVALUE, value)?;
            continue;
        }
        writer.write_i16(
            PROPATTR,
            &[u16_to_i16(GLASSWORKS_GDS_SHAPE_PROPERTY_PROP_ATTR)?],
        )?;
        writer.write_ascii(PROPVALUE, &encode_glassworks_shape_property(key, value))?;
    }
    Ok(())
}

pub(crate) fn shape_property_gds_attr_key(key: &str) -> Option<u16> {
    let attr = key
        .strip_prefix("gds.attr.")?
        .parse::<u16>()
        .ok()
        .filter(|attr| {
            !matches!(
                *attr,
                GLASSWORKS_GDS_SHAPE_PROPERTY_PROP_ATTR
                    | GLASSWORKS_GDS_SHAPE_NET_PROP_ATTR
                    | GLASSWORKS_GDS_SHAPE_NAME_PROP_ATTR
            )
        })?;
    u16_to_i16(attr).ok()?;
    Some(attr)
}

pub(crate) fn encode_glassworks_shape_property(key: &str, value: &str) -> String {
    let mut encoded = String::with_capacity(key.len() + value.len() + 1);
    push_escaped_shape_property_text(&mut encoded, key);
    encoded.push('=');
    push_escaped_shape_property_text(&mut encoded, value);
    encoded
}

pub(crate) fn push_escaped_shape_property_text(encoded: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '\\' => encoded.push_str("\\\\"),
            '=' => encoded.push_str("\\="),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            _ => encoded.push(ch),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn import_element(
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
            properties,
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
            insert_shape_in_import_cell(
                document,
                parent,
                layer_id,
                kind,
                gds_shape_name_property(properties),
                gds_shape_net_property(properties),
                gds_shape_properties(properties),
            );
        }
        ParsedElement::Box {
            layer,
            boxtype,
            points,
            properties,
        } => {
            let scale_context =
                GdsScaleWarningContext::new("BOX", Some(*layer), Some(*boxtype), Some(false), None);
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
                    element_kind: "BOX".to_string(),
                    gds_layer: Some(*layer),
                    gds_type: Some(*boxtype),
                    reason: format!(
                        "GDS BOX has {} point(s); import needs at least 3 distinct points",
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
                *boxtype,
                false,
            );
            record_import_layer_mapping(resolved_layer_mappings, *layer, *boxtype, false, layer_id);
            let kind = import_boundary_kind(&points);
            insert_shape_in_import_cell(
                document,
                parent,
                layer_id,
                kind,
                gds_shape_name_property(properties),
                gds_shape_net_property(properties),
                gds_shape_properties(properties),
            );
        }
        ParsedElement::Path {
            layer,
            datatype,
            pathtype,
            begin_extension,
            end_extension,
            raw_width,
            width,
            points,
            properties,
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
            let points = scale_points(
                points,
                source_dbu_per_micron,
                target_dbu_per_micron,
                report,
                scale_context,
            );
            let begin_extension = begin_extension.map(|extension| {
                scale_coord_with_report(
                    extension,
                    source_dbu_per_micron,
                    target_dbu_per_micron,
                    report,
                    scale_context,
                    "begin extension",
                )
            });
            let end_extension = end_extension.map(|extension| {
                scale_coord_with_report(
                    extension,
                    source_dbu_per_micron,
                    target_dbu_per_micron,
                    report,
                    scale_context,
                    "end extension",
                )
            });
            let points = apply_gds_path_style(
                &points,
                imported_width,
                *pathtype,
                begin_extension,
                end_extension,
                report,
                *layer,
                *datatype,
                layer_id,
            );
            insert_shape_in_import_cell(
                document,
                parent,
                layer_id,
                ShapeKind::Path {
                    points,
                    width: imported_width,
                },
                gds_shape_name_property(properties),
                gds_shape_net_property(properties),
                gds_shape_properties(properties),
            );
        }
        ParsedElement::Text {
            layer,
            texttype,
            position,
            text,
            properties,
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
                gds_shape_name_property(properties),
                gds_shape_net_property(properties),
                gds_shape_properties(properties),
            );
        }
        ParsedElement::Sref {
            name,
            origin,
            transform,
            properties,
        } => {
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
                import_gds_instance_transform(
                    scale_point(
                        *origin,
                        source_dbu_per_micron,
                        target_dbu_per_micron,
                        report,
                        GdsScaleWarningContext::new("SREF", None, None, None, None),
                    ),
                    *transform,
                    report,
                    "SREF",
                    name,
                ),
                InstanceArray::single(),
                gds_instance_properties(properties),
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
            transform,
            properties,
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
                import_gds_instance_transform(
                    scale_point(
                        *origin,
                        source_dbu_per_micron,
                        target_dbu_per_micron,
                        report,
                        GdsScaleWarningContext::new("AREF", None, None, None, None),
                    ),
                    *transform,
                    report,
                    "AREF",
                    name,
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
                gds_instance_properties(properties),
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

pub(crate) fn import_gds_instance_transform(
    origin: Point,
    transform: GdsInstanceTransformSpec,
    report: &mut GdsImportReport,
    element_kind: &str,
    target_name: &str,
) -> Transform {
    if let Some(magnification) = transform.magnification
        && !approximately_one(magnification)
    {
        report.warnings.push(GdsImportWarning {
            kind: GdsImportWarningKind::UnsupportedInstanceTransform,
            gds_layer: None,
            gds_type: None,
            is_text: None,
            layer_id: None,
            message: format!(
                "GDS {element_kind} target {target_name:?} has MAG {magnification}; importing rotation/reflection and translation only"
            ),
        });
    }

    let angle = transform.angle_degrees.unwrap_or(0.0);
    let matrix = gds_instance_matrix(transform.reflect_x, angle).unwrap_or_else(|| {
        report.warnings.push(GdsImportWarning {
            kind: GdsImportWarningKind::UnsupportedInstanceTransform,
            gds_layer: None,
            gds_type: None,
            is_text: None,
            layer_id: None,
            message: format!(
                "GDS {element_kind} target {target_name:?} has unsupported ANGLE {angle}; importing reflection and translation only"
            ),
        });
        gds_instance_matrix(transform.reflect_x, 0.0).unwrap_or(Transform::IDENTITY.matrix)
    });

    Transform {
        matrix,
        translation: Vector::new(origin.x, origin.y),
    }
}

pub(crate) fn gds_instance_matrix(reflect_x: bool, angle_degrees: f64) -> Option<[i8; 4]> {
    let angle = gds_quadrant_angle(angle_degrees)?;
    Some(match (reflect_x, angle) {
        (false, 0) => [1, 0, 0, 1],
        (false, 90) => [0, -1, 1, 0],
        (false, 180) => [-1, 0, 0, -1],
        (false, 270) => [0, 1, -1, 0],
        (true, 0) => [1, 0, 0, -1],
        (true, 90) => [0, 1, 1, 0],
        (true, 180) => [-1, 0, 0, 1],
        (true, 270) => [0, -1, -1, 0],
        _ => return None,
    })
}

pub(crate) fn gds_quadrant_angle(angle_degrees: f64) -> Option<u16> {
    let angle = normalize_gds_angle(angle_degrees);
    [0_u16, 90, 180, 270]
        .into_iter()
        .find(|candidate| approximately_zero(angle - f64::from(*candidate)))
}

pub(crate) fn normalize_gds_angle(angle: f64) -> f64 {
    let angle = angle.rem_euclid(360.0);
    if approximately_zero(angle - 360.0) {
        0.0
    } else {
        angle
    }
}

pub(crate) fn approximately_zero(value: f64) -> bool {
    value.abs() <= GDS_TRANSFORM_EPSILON
}

pub(crate) fn approximately_one(value: f64) -> bool {
    approximately_zero(value - 1.0)
}
