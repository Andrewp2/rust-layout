#![allow(unused_imports)]
use super::*;
use crate::*;
use crate::{LayerId, Operation, default_technology};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct GdsRoundTripFixture {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) top_shapes: Vec<GdsFixtureShape>,
    #[serde(default)]
    pub(crate) cells: Vec<GdsFixtureCell>,
    #[serde(default)]
    pub(crate) instances: Vec<GdsFixtureInstance>,
    pub(crate) expect: GdsRoundTripExpectation,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsFixtureCell {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) shapes: Vec<GdsFixtureShape>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsFixtureInstance {
    pub(crate) cell: String,
    pub(crate) dx: Coord,
    pub(crate) dy: Coord,
    #[serde(default)]
    pub(crate) array: Option<GdsFixtureArray>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub(crate) struct GdsFixtureArray {
    pub(crate) columns: u32,
    pub(crate) rows: u32,
    pub(crate) column_pitch: [Coord; 2],
    pub(crate) row_pitch: [Coord; 2],
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum GdsFixtureShape {
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
pub(crate) struct GdsRoundTripExpectation {
    pub(crate) export: GdsExportExpectation,
    pub(crate) import: GdsImportExpectation,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsExportExpectation {
    pub(crate) structure_count: usize,
    pub(crate) boundary_count: usize,
    pub(crate) path_count: usize,
    pub(crate) text_count: usize,
    pub(crate) sref_count: usize,
    pub(crate) aref_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) warning_count: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsImportExpectation {
    pub(crate) structure_count: usize,
    pub(crate) element_count: usize,
    pub(crate) top_shape_count: usize,
    pub(crate) top_instance_count: usize,
    pub(crate) flattened_shape_count: usize,
    pub(crate) array_columns: u32,
    pub(crate) array_rows: u32,
    pub(crate) array_column_pitch: [Coord; 2],
    pub(crate) array_row_pitch: [Coord; 2],
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsImportLayerFixture {
    pub(crate) name: String,
    pub(crate) elements: Vec<GdsImportFixtureElement>,
    pub(crate) expect: GdsImportLayerExpectation,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsUnsupportedImportFixture {
    pub(crate) name: String,
    pub(crate) elements: Vec<GdsImportFixtureElement>,
    pub(crate) expect: GdsUnsupportedImportExpectation,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsDegenerateImportFixture {
    pub(crate) name: String,
    pub(crate) elements: Vec<GdsImportFixtureElement>,
    pub(crate) expect: GdsDegenerateImportExpectation,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsMissingRefsImportFixture {
    pub(crate) name: String,
    pub(crate) elements: Vec<GdsImportFixtureElement>,
    pub(crate) expect: GdsMissingRefsImportExpectation,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum GdsImportFixtureElement {
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
pub(crate) struct GdsImportLayerExpectation {
    pub(crate) generated_layer_count: usize,
    pub(crate) warning_count: usize,
    pub(crate) split_warning_gds_layer: u16,
    pub(crate) message_contains: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsUnsupportedImportExpectation {
    pub(crate) element_count: usize,
    pub(crate) generated_layer_count: usize,
    pub(crate) visible_shape_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) skipped: Vec<GdsExpectedSkippedImportElement>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsDegenerateImportExpectation {
    pub(crate) element_count: usize,
    pub(crate) generated_layer_count: usize,
    pub(crate) visible_shape_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) skipped: Vec<GdsExpectedSkippedImportElement>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsMissingRefsImportExpectation {
    pub(crate) element_count: usize,
    pub(crate) generated_layer_count: usize,
    pub(crate) visible_shape_count: usize,
    pub(crate) top_instance_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) skipped: Vec<GdsExpectedSkippedImportElement>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GdsExpectedSkippedImportElement {
    pub(crate) element_kind: String,
    #[serde(default)]
    pub(crate) gds_layer: Option<u16>,
    #[serde(default)]
    pub(crate) gds_type: Option<u16>,
    pub(crate) reason_contains: String,
}

pub(crate) fn document_from_gds_fixture(fixture: &GdsRoundTripFixture) -> Document {
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

pub(crate) fn gds_bytes_from_import_layer_fixture(fixture: &GdsImportLayerFixture) -> Vec<u8> {
    gds_bytes_from_import_fixture(&fixture.name, &fixture.elements)
}

pub(crate) fn gds_bytes_from_import_fixture(
    name: &str,
    elements: &[GdsImportFixtureElement],
) -> Vec<u8> {
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

pub(crate) fn gds_bytes_from_single_path_with_style(
    name: &str,
    pathtype: Option<u16>,
    begin_extension: Option<Coord>,
    end_extension: Option<Coord>,
) -> Vec<u8> {
    let mut writer = GdsWriter::new();
    writer.write_i16(HEADER, &[600]).unwrap();
    writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
    writer.write_ascii(LIBNAME, name).unwrap();
    writer.write_real8(UNITS, &[0.001, 1.0e-9]).unwrap();
    writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
    writer.write_ascii(STRNAME, "TOP").unwrap();
    writer.write_no_data(PATH).unwrap();
    writer.write_i16(LAYER, &[4]).unwrap();
    writer.write_i16(DATATYPE, &[0]).unwrap();
    writer.write_i32(WIDTH, &[20]).unwrap();
    if let Some(pathtype) = pathtype {
        writer
            .write_i16(PATHTYPE, &[u16_to_i16(pathtype).unwrap()])
            .unwrap();
    }
    if let Some(extension) = begin_extension {
        writer
            .write_i32(BGNEXTN, &[i32::try_from(extension).unwrap()])
            .unwrap();
    }
    if let Some(extension) = end_extension {
        writer
            .write_i32(ENDEXTN, &[i32::try_from(extension).unwrap()])
            .unwrap();
    }
    writer
        .write_xy(&[Point::new(100, 0), Point::new(200, 0)])
        .unwrap();
    writer.write_no_data(ENDEL).unwrap();
    writer.write_no_data(ENDSTR).unwrap();
    writer.write_no_data(ENDLIB).unwrap();
    writer.into_bytes()
}

pub(crate) fn imported_single_path(result: &GdsImportResult) -> (Vec<Point>, Coord) {
    result
        .document
        .shapes
        .values()
        .find_map(|shape| match &shape.kind {
            ShapeKind::Path { points, width } => Some((points.clone(), *width)),
            _ => None,
        })
        .expect("imported document should contain a path")
}

pub(crate) fn insert_fixture_shape(
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

pub(crate) fn fixture_layer(document: &Document, layer: &str) -> LayerId {
    let process = ProcessLayer::from_technology_name(layer)
        .unwrap_or_else(|| panic!("unknown fixture layer {layer}"));
    document
        .layer_by_process(process)
        .unwrap_or_else(|| panic!("fixture layer {layer} missing from document"))
}

#[test]
pub(crate) fn gds_real8_round_trips_common_units() {
    for value in [0.001, 1.0e-9, 0.0005, 2.5] {
        let decoded = decode_gds_real8(encode_gds_real8(value));
        assert!((decoded - value).abs() / value < 1.0e-12);
    }
}

#[test]
pub(crate) fn export_import_round_trip_preserves_hierarchy_geometry() {
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
        child
            .shapes
            .values()
            .any(|shape| { matches!(&shape.kind, ShapeKind::Label { text, .. } if text == "IN") })
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
pub(crate) fn export_report_counts_emitted_elements_and_skipped_geometry() {
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
            mode: Default::default(),
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
pub(crate) fn export_report_skips_aref_dimensions_that_exceed_gds_colrow_range() {
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
pub(crate) fn export_report_warns_about_lossy_gds_fallbacks_and_metadata() {
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
        shape.net = Some(NetId(7));
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
            mode: Default::default(),
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
            properties: BTreeMap::new(),
        },
    );

    let unsupported_instance = document
        .insert_instance_in_top(
            child,
            Transform {
                matrix: [1, 1, 0, 1],
                translation: Vector::new(500, 0),
            },
        )
        .unwrap();

    let report = export_gdsii_with_report(&document, &default_technology())
        .unwrap()
        .report;

    assert!(
        !report.warnings.iter().any(|warning| {
            warning.kind == GdsExportWarningKind::NonRoundTrippableMetadata
                && warning.shape_id == Some(lossy_metadata_shape)
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
                && warning.instance_id == Some(unsupported_instance)
                && warning.message.contains("translation only")
        }),
        "{:?}",
        report.warnings
    );
}

#[test]
pub(crate) fn export_import_round_trip_preserves_shape_metadata_as_gds_properties() {
    let mut document = Document::new("shape name properties");
    let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
    let shape_id = document.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
    );
    {
        let mut shape = document.shapes.get_mut(&shape_id).unwrap();
        shape.name = Some("critical device".to_string());
        shape.net = Some(NetId(42));
        shape.properties.insert(
            "device.role".to_string(),
            "gate=strap\ncritical".to_string(),
        );
        shape
            .properties
            .insert("gds.attr.88".to_string(), "raw-owner".to_string());
    }

    let exported = export_gdsii_with_report(&document, &default_technology()).unwrap();
    assert!(
        !exported.report.warnings.iter().any(|warning| {
            warning.kind == GdsExportWarningKind::NonRoundTrippableMetadata
                && warning.shape_id == Some(shape_id)
        }),
        "{:?}",
        exported.report.warnings
    );

    let imported = import_gdsii(&exported.bytes, &default_technology()).unwrap();
    let shape = imported
        .shapes
        .values()
        .find(|shape| shape.kind.bounds() == Rect::new(Point::new(0, 0), Point::new(100, 50)))
        .expect("round-tripped rectangle should exist");

    assert_eq!(shape.name.as_deref(), Some("critical device"));
    assert_eq!(shape.net, Some(NetId(42)));
    assert_eq!(
        shape.properties.get("device.role").map(String::as_str),
        Some("gate=strap\ncritical")
    );
    assert_eq!(
        shape.properties.get("gds.attr.88").map(String::as_str),
        Some("raw-owner")
    );
}

#[test]
pub(crate) fn export_import_round_trip_preserves_instance_metadata_as_gds_properties() {
    let mut document = Document::new("instance properties");
    let metal1 = document.layer_by_process(ProcessLayer::Metal1).unwrap();
    let child = document.create_cell("UNIT");
    document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        )
        .unwrap();
    let single = document
        .insert_instance_in_top(child, Transform::translate(100, 200))
        .unwrap();
    {
        let mut instance = document.instance_mut(document.top_cell, single).unwrap();
        instance
            .properties
            .insert("placement.role".to_string(), "guard=ring\nA".to_string());
        instance
            .properties
            .insert("gds.attr.88".to_string(), "raw-owner".to_string());
    }
    let array = document
        .insert_instance_in_top(child, Transform::translate(500, 600))
        .unwrap();
    {
        let mut instance = document.instance_mut(document.top_cell, array).unwrap();
        instance.array = InstanceArray {
            columns: 2,
            rows: 3,
            column_pitch: Vector::new(200, 0),
            row_pitch: Vector::new(0, 150),
        };
        instance
            .properties
            .insert("placement.role".to_string(), "array_anchor".to_string());
    }

    let exported = export_gdsii_with_report(&document, &default_technology()).unwrap();
    assert_eq!(exported.report.sref_count, 1);
    assert_eq!(exported.report.aref_count, 1);

    let imported = import_gdsii(&exported.bytes, &default_technology()).unwrap();
    let top = imported.cell(imported.top_cell).unwrap();
    let imported_single = top
        .instances
        .values()
        .find(|instance| instance.array.is_single())
        .expect("round-tripped SREF instance should exist");
    let imported_array = top
        .instances
        .values()
        .find(|instance| !instance.array.is_single())
        .expect("round-tripped AREF instance should exist");

    assert_eq!(imported_single.transform, Transform::translate(100, 200));
    assert_eq!(
        imported_single
            .properties
            .get("placement.role")
            .map(String::as_str),
        Some("guard=ring\nA")
    );
    assert_eq!(
        imported_single
            .properties
            .get("gds.attr.88")
            .map(String::as_str),
        Some("raw-owner")
    );
    assert_eq!(imported_array.transform, Transform::translate(500, 600));
    assert_eq!(imported_array.array.columns, 2);
    assert_eq!(imported_array.array.rows, 3);
    assert_eq!(
        imported_array
            .properties
            .get("placement.role")
            .map(String::as_str),
        Some("array_anchor")
    );
}

#[test]
pub(crate) fn import_gds_shape_properties_name_geometry_and_net() {
    let mut writer = GdsWriter::new();
    writer.write_i16(HEADER, &[600]).unwrap();
    writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
    writer.write_ascii(LIBNAME, "SHAPE_NAME_PROPERTY").unwrap();
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
            Point::new(100, 50),
            Point::new(0, 50),
            Point::new(0, 0),
        ])
        .unwrap();
    writer
        .write_i16(
            PROPATTR,
            &[u16_to_i16(GLASSWORKS_GDS_SHAPE_NAME_PROP_ATTR).unwrap()],
        )
        .unwrap();
    writer.write_ascii(PROPVALUE, "poly strap").unwrap();
    writer
        .write_i16(
            PROPATTR,
            &[u16_to_i16(GLASSWORKS_GDS_SHAPE_NET_PROP_ATTR).unwrap()],
        )
        .unwrap();
    writer.write_ascii(PROPVALUE, "17").unwrap();
    writer
        .write_i16(
            PROPATTR,
            &[u16_to_i16(GLASSWORKS_GDS_SHAPE_PROPERTY_PROP_ATTR).unwrap()],
        )
        .unwrap();
    writer
        .write_ascii(PROPVALUE, "mask.owner=OPC\\=team")
        .unwrap();
    writer.write_i16(PROPATTR, &[44]).unwrap();
    writer.write_ascii(PROPVALUE, "raw_attr").unwrap();
    writer.write_no_data(ENDEL).unwrap();
    writer.write_no_data(ENDSTR).unwrap();
    writer.write_no_data(ENDLIB).unwrap();

    let imported = import_gdsii(&writer.into_bytes(), &default_technology()).unwrap();
    let shape = imported
        .shapes
        .values()
        .next()
        .expect("imported boundary should create a shape");

    assert_eq!(shape.name.as_deref(), Some("poly strap"));
    assert_eq!(shape.net, Some(NetId(17)));
    assert_eq!(
        shape.properties.get("mask.owner").map(String::as_str),
        Some("OPC=team")
    );
    assert_eq!(
        shape.properties.get("gds.attr.44").map(String::as_str),
        Some("raw_attr")
    );
}

#[test]
pub(crate) fn export_report_warns_about_ambiguous_document_gds_mappings() {
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
pub(crate) fn export_report_warns_when_path_width_is_normalized() {
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
pub(crate) fn import_report_records_skipped_degenerate_geometry() {
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

    let imported = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();

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
pub(crate) fn import_report_warns_when_path_width_is_normalized() {
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
pub(crate) fn import_applies_gds_square_path_end_extensions() {
    let imported = import_gdsii_with_report(
        &gds_bytes_from_single_path_with_style("PATH_TYPE_TWO", Some(2), None, None),
        &default_technology(),
    )
    .unwrap();

    assert_eq!(imported.report.warnings, Vec::new());
    let (points, width) = imported_single_path(&imported);
    assert_eq!(width, 20);
    assert_eq!(points, vec![Point::new(90, 0), Point::new(210, 0)]);
}

#[test]
pub(crate) fn import_applies_gds_custom_path_end_extensions() {
    let imported = import_gdsii_with_report(
        &gds_bytes_from_single_path_with_style("PATH_TYPE_FOUR", Some(4), Some(30), Some(40)),
        &default_technology(),
    )
    .unwrap();

    assert_eq!(imported.report.warnings, Vec::new());
    let (points, width) = imported_single_path(&imported);
    assert_eq!(width, 20);
    assert_eq!(points, vec![Point::new(70, 0), Point::new(240, 0)]);
}

#[test]
pub(crate) fn import_warns_and_approximates_gds_round_path_caps() {
    let imported = import_gdsii_with_report(
        &gds_bytes_from_single_path_with_style("PATH_TYPE_ROUND", Some(1), None, None),
        &default_technology(),
    )
    .unwrap();

    assert_eq!(imported.report.warnings.len(), 1);
    let warning = &imported.report.warnings[0];
    assert_eq!(warning.kind, GdsImportWarningKind::UnsupportedPathStyle);
    assert_eq!(warning.gds_layer, Some(4));
    assert_eq!(warning.gds_type, Some(0));
    assert!(warning.message.contains("round caps"));
    let (points, width) = imported_single_path(&imported);
    assert_eq!(width, 20);
    assert_eq!(points, vec![Point::new(90, 0), Point::new(210, 0)]);
}

#[test]
pub(crate) fn import_report_warns_when_units_are_normalized() {
    let mut writer = GdsWriter::new();
    writer.write_i16(HEADER, &[600]).unwrap();
    writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
    writer.write_ascii(LIBNAME, "BAD_UNITS").unwrap();
    writer.write_real8(UNITS, &[10.0, 1.0e-5]).unwrap();
    writer.write_i16(BGNSTR, &timestamp_fields()).unwrap();
    writer.write_ascii(STRNAME, "TOP").unwrap();
    writer.write_no_data(ENDSTR).unwrap();
    writer.write_no_data(ENDLIB).unwrap();

    let imported = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();

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
pub(crate) fn import_report_warns_when_scaled_coordinates_are_clamped() {
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
pub(crate) fn import_report_warns_when_aref_dimensions_are_normalized() {
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
pub(crate) fn import_report_warns_about_duplicate_structure_names() {
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
pub(crate) fn import_rejects_unterminated_gds_structure() {
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
pub(crate) fn import_rejects_unterminated_gds_element() {
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
pub(crate) fn import_rejects_new_gds_element_before_endel() {
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
pub(crate) fn import_rejects_missing_gds_endlib() {
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
pub(crate) fn degenerate_gds_geometry_fixture_reports_skipped_elements() {
    let fixture: GdsDegenerateImportFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/import_export/gds_degenerate_geometry_import.json"
    )))
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
