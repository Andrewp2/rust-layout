#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn gds_box_elements_import_as_geometry_without_aborting() {
    let mut writer = GdsWriter::new();
    writer.write_i16(HEADER, &[600]).unwrap();
    writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
    writer.write_ascii(LIBNAME, "BOX_ELEMENT").unwrap();
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
    assert_eq!(result.report.skipped_elements, Vec::new());
    assert_eq!(result.report.generated_layers.len(), 1);
    assert_eq!(result.report.generated_layers[0].gds_layer, 31);
    assert_eq!(result.report.generated_layers[0].gds_type, 2);
    assert!(!result.report.generated_layers[0].is_text);
    assert_eq!(result.document.visible_flattened_shapes().len(), 2);
    let box_layer = result.report.generated_layers[0].layer_id;
    assert!(
        result
            .document
            .visible_flattened_shapes()
            .iter()
            .any(|shape| {
                shape.shape.layer == box_layer
                    && shape.shape.kind
                        == ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(10, 10)))
            }),
        "{:?}",
        result.document.visible_flattened_shapes()
    );
}

#[test]
pub(crate) fn unsupported_gds_elements_fixture_reports_skipped_elements_without_aborting() {
    let fixture: GdsUnsupportedImportFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/import_export/gds_unsupported_elements_import.json"
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

pub(crate) fn assert_expected_import_skips(
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
pub(crate) fn import_report_skips_missing_instance_targets_without_aborting() {
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

    let imported = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();

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
pub(crate) fn missing_gds_reference_fixture_reports_skipped_instances_without_aborting() {
    let fixture: GdsMissingRefsImportFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/import_export/gds_missing_references_import.json"
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
pub(crate) fn import_preserves_aref_arrays_as_instance_arrays() {
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
pub(crate) fn import_preserves_sref_rotation_and_reflection() {
    let mut writer = GdsWriter::new();
    writer.write_i16(HEADER, &[600]).unwrap();
    writer.write_i16(BGNLIB, &timestamp_fields()).unwrap();
    writer.write_ascii(LIBNAME, "SREF_TRANSFORM").unwrap();
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
    writer.write_no_data(SREF).unwrap();
    writer.write_ascii(SNAME, "UNIT").unwrap();
    writer
        .write_bit_array(STRANS, GDS_STRANS_REFLECT_X)
        .unwrap();
    writer.write_real8(ANGLE, &[90.0]).unwrap();
    writer.write_xy(&[Point::new(500, 600)]).unwrap();
    writer.write_no_data(ENDEL).unwrap();
    writer.write_no_data(ENDSTR).unwrap();
    writer.write_no_data(ENDLIB).unwrap();

    let imported = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();
    let top = imported.document.cell(imported.document.top_cell).unwrap();
    let instance = top.instances.values().next().unwrap();

    assert_eq!(imported.report.warnings, Vec::new());
    assert_eq!(top.instances.len(), 1);
    assert_eq!(
        instance.transform,
        Transform {
            matrix: [0, 1, 1, 0],
            translation: Vector::new(500, 600),
        }
    );
}

#[test]
pub(crate) fn export_import_round_trip_preserves_instance_arrays() {
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
pub(crate) fn export_import_round_trip_preserves_oriented_instance_transforms() {
    let mut doc = Document::new("oriented instance round trip");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let child = doc.create_cell("UNIT");
    doc.insert_shape_in_cell(
        child,
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
    )
    .unwrap();
    doc.insert_instance_in_top(
        child,
        Transform::rotate_cw90().with_translation(Vector::new(500, 600)),
    )
    .unwrap();

    let exported = export_gdsii_with_report(&doc, &default_technology()).unwrap();
    assert!(
        !exported
            .report
            .warnings
            .iter()
            .any(|warning| warning.kind == GdsExportWarningKind::UnsupportedInstanceTransform),
        "{:?}",
        exported.report.warnings
    );

    let imported = import_gdsii(&exported.bytes, &default_technology()).unwrap();
    let imported_top = imported.cell(imported.top_cell).unwrap();
    let imported_instance = imported_top.instances.values().next().unwrap();

    assert_eq!(imported_top.instances.len(), 1);
    assert_eq!(
        imported_instance.transform,
        Transform::rotate_cw90().with_translation(Vector::new(500, 600))
    );
}

#[test]
pub(crate) fn gds_round_trip_fixture_reports_and_preserves_arrays() {
    let fixture: GdsRoundTripFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/import_export/gds_round_trip.json"
    )))
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
        unit.shapes
            .values()
            .any(|shape| { matches!(&shape.kind, ShapeKind::Label { text, .. } if text == "IN") })
    );
    assert!(
        unit.shapes
            .values()
            .any(|shape| matches!(shape.kind, ShapeKind::Path { width: 80, .. }))
    );
}

#[test]
pub(crate) fn unknown_gds_layers_are_imported_as_mapped_document_layers() {
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

    let imported = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();
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
pub(crate) fn import_report_warns_when_one_gds_layer_splits_across_document_layers() {
    let fixture: GdsImportLayerFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/import_export/gds_split_layer_import.json"
    )))
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
pub(crate) fn unknown_gds_text_layers_are_imported_as_text_mapped_layers() {
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

    let imported = import_gdsii_with_report(&writer.into_bytes(), &default_technology()).unwrap();
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
pub(crate) fn import_scales_coordinates_to_active_technology_dbu() {
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
