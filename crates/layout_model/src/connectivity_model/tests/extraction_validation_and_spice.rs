#![allow(unused_imports)]
use super::*;
use crate::*;
use crate::{
    MarkerState, Operation, ProcessLayer, ShapeId, ShapeKind, Transform, default_technology,
};
use geometry_core::{Coord, Point, Rect};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct ConnectivityGoldenFixture {
    pub(crate) name: String,
    pub(crate) shapes: Vec<ConnectivityFixtureShape>,
    pub(crate) expect: ConnectivityGoldenExpectation,
    #[serde(default)]
    pub(crate) issue_states: Vec<ConnectivityIssueStateFixture>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConnectivityGoldenExpectation {
    pub(crate) health: String,
    pub(crate) component_count: usize,
    pub(crate) labeled_component_count: usize,
    pub(crate) short_count: usize,
    pub(crate) short_names: Vec<String>,
    pub(crate) short_key: String,
    pub(crate) open_count: usize,
    pub(crate) open_name: String,
    pub(crate) open_key: String,
    pub(crate) open_component_count: usize,
    pub(crate) data_component_shape_count: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConnectivityIssueStateFixture {
    pub(crate) key: String,
    pub(crate) hidden: bool,
    pub(crate) waived: bool,
    pub(crate) note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ConnectivityFixtureShape {
    Rect {
        layer: String,
        x: Coord,
        y: Coord,
        w: Coord,
        h: Coord,
    },
    Label {
        layer: String,
        x: Coord,
        y: Coord,
        text: String,
    },
}

pub(crate) fn document_from_connectivity_fixture(fixture: &ConnectivityGoldenFixture) -> Document {
    let mut document = Document::new(&fixture.name);
    for shape in &fixture.shapes {
        match shape {
            ConnectivityFixtureShape::Rect { layer, x, y, w, h } => {
                let layer_id = fixture_layer(&document, layer);
                document.insert_shape(
                    layer_id,
                    ShapeKind::Rectangle(Rect::from_min_size(Point::new(*x, *y), *w, *h)),
                );
            }
            ConnectivityFixtureShape::Label { layer, x, y, text } => {
                let layer_id = fixture_layer(&document, layer);
                document.insert_shape(
                    layer_id,
                    ShapeKind::Label {
                        position: Point::new(*x, *y),
                        text: text.clone(),
                    },
                );
            }
        }
    }
    document
}

pub(crate) fn fixture_layer(document: &Document, layer: &str) -> LayerId {
    let process = ProcessLayer::from_technology_name(layer)
        .unwrap_or_else(|| panic!("unknown fixture layer {layer}"));
    document
        .layer_by_process(process)
        .unwrap_or_else(|| panic!("fixture layer {layer} missing from document"))
}

#[test]
pub(crate) fn labels_propagate_across_via_stack() {
    let technology = default_technology();
    let mut doc = Document::new("connectivity");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let via1 = doc.layer_by_process(ProcessLayer::Via1).unwrap();
    let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
    let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();

    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 500, 500)),
    );
    doc.insert_shape(
        via1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 200), 100, 100)),
    );
    doc.insert_shape(
        metal2,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(150, 150), 500, 500)),
    );
    doc.insert_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(250, 250),
            text: "out".to_string(),
        },
    );

    let report = extract_connectivity(&doc, &technology).unwrap();

    assert_eq!(report.components.len(), 1);
    assert_eq!(report.components[0].shapes.len(), 3);
    assert_eq!(report.components[0].net_name.as_deref(), Some("OUT"));
    assert!(report.shorts.is_empty());
    assert!(report.opens.is_empty());
}

#[test]
pub(crate) fn connectivity_extracts_hidden_layers_and_flattened_instances() {
    let technology = default_technology();
    let mut doc = Document::new("hidden connectivity");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let child = doc.create_cell("hidden conductor");
    let conductor = doc
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 500, 200)),
        )
        .unwrap();
    doc.insert_shape_in_cell(
        child,
        metal1,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "data".to_string(),
        },
    )
    .unwrap();
    let instance = doc
        .insert_instance_in_top(child, Transform::translate(1_000, 2_000))
        .unwrap();
    doc.apply_operation_without_log(&Operation::SetLayerVisibility {
        layer: metal1,
        visible: false,
    });

    assert!(
        doc.visible_flattened_shapes().is_empty(),
        "fixture should prove hidden display geometry is not the extraction source"
    );
    let report = extract_connectivity(&doc, &technology).unwrap();

    assert_eq!(report.components.len(), 1);
    let component = &report.components[0];
    assert_eq!(component.net_name.as_deref(), Some("DATA"));
    assert!(
        component
            .shapes
            .contains(&ShapeOccurrenceId::from_instance_path(
                conductor,
                &[instance],
            ))
    );
    assert_eq!(report.validate(), Vec::new());
}

#[test]
pub(crate) fn extracts_mos_device_from_diffusion_poly_crossing() {
    let technology = default_technology();
    let mut doc = Document::new("mos extraction");
    let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        diffusion,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 300)),
    );
    doc.insert_shape(
        diffusion,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "sd".to_string(),
        },
    );
    let gate = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, -100), 100, 500)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Label {
            position: Point::new(470, -50),
            text: "g".to_string(),
        },
    );

    let report = extract_connectivity(&doc, &technology).unwrap();

    assert_eq!(report.devices.len(), 1);
    let device = &report.devices[0];
    assert_eq!(device.kind, "mos");
    assert_eq!(device.model, "nmos");
    assert_eq!(
        device.bounds,
        Rect::from_min_size(Point::new(450, 0), 100, 300)
    );
    assert_eq!(device.length, 100);
    assert_eq!(device.width, 300);
    assert!(
        device
            .occurrences
            .contains(&ShapeOccurrenceId::top_level(gate))
    );
    assert_eq!(
        device
            .terminals
            .iter()
            .find(|terminal| terminal.name == "G")
            .and_then(|terminal| terminal.net_name.as_deref()),
        Some("G")
    );
    assert_eq!(
        device
            .terminals
            .iter()
            .find(|terminal| terminal.name == "D")
            .and_then(|terminal| terminal.net_name.as_deref()),
        Some("SD")
    );
    assert_eq!(report.validate(), Vec::new());
}

#[test]
pub(crate) fn device_extraction_uses_hidden_layers() {
    let technology = default_technology();
    let mut doc = Document::new("hidden mos extraction");
    let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        diffusion,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 300)),
    );
    doc.insert_shape(
        diffusion,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "sd".to_string(),
        },
    );
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, -100), 100, 500)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Label {
            position: Point::new(470, -50),
            text: "g".to_string(),
        },
    );
    doc.apply_operation_without_log(&Operation::SetLayerVisibility {
        layer: diffusion,
        visible: false,
    });
    doc.apply_operation_without_log(&Operation::SetLayerVisibility {
        layer: poly,
        visible: false,
    });

    assert!(
        doc.visible_flattened_shapes().is_empty(),
        "fixture should prove hidden display geometry is not the device extraction source"
    );
    let report = extract_connectivity(&doc, &technology).unwrap();

    assert_eq!(report.devices.len(), 1);
    let device = &report.devices[0];
    assert_eq!(device.kind, "mos");
    assert_eq!(
        device
            .terminals
            .iter()
            .find(|terminal| terminal.name == "G")
            .and_then(|terminal| terminal.net_name.as_deref()),
        Some("G")
    );
    assert_eq!(report.validate(), Vec::new());
}

#[test]
pub(crate) fn mos_device_source_drain_use_diffusion_side_labels() {
    let technology = default_technology();
    let mut doc = Document::new("mos source drain");
    let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        diffusion,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 300)),
    );
    for (x, text) in [(100, "drain"), (900, "source")] {
        doc.insert_shape(
            diffusion,
            ShapeKind::Label {
                position: Point::new(x, 100),
                text: text.to_string(),
            },
        );
    }
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, -100), 100, 500)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Label {
            position: Point::new(470, -50),
            text: "gate".to_string(),
        },
    );

    let report = extract_connectivity(&doc, &technology).unwrap();
    let device = report.devices.first().expect("MOS device should extract");
    let terminal_net = |name: &str| {
        device
            .terminals
            .iter()
            .find(|terminal| terminal.name == name)
            .and_then(|terminal| terminal.net_name.as_deref())
    };

    assert_eq!(terminal_net("D"), Some("DRAIN"));
    assert_eq!(terminal_net("G"), Some("GATE"));
    assert_eq!(terminal_net("S"), Some("SOURCE"));

    let exported = export_connectivity_spice(&doc.name, &report);
    assert!(exported.text.contains("MDEV1 DRAIN GATE SOURCE 0 NMOS"));
}

#[test]
pub(crate) fn mos_device_body_uses_overlapping_well_label() {
    let technology = default_technology();
    let mut doc = Document::new("mos body");
    let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let pwell = doc.create_layer("pwell", ProcessLayer::Annotation, [0.2, 0.4, 0.9, 0.2]);
    doc.insert_shape(
        diffusion,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 300)),
    );
    for (x, text) in [(100, "drain"), (900, "source")] {
        doc.insert_shape(
            diffusion,
            ShapeKind::Label {
                position: Point::new(x, 100),
                text: text.to_string(),
            },
        );
    }
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, -100), 100, 500)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Label {
            position: Point::new(470, -50),
            text: "gate".to_string(),
        },
    );
    let body = doc.insert_shape(
        pwell,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(-100, -100), 1_200, 500)),
    );
    doc.insert_shape(
        pwell,
        ShapeKind::Label {
            position: Point::new(500, 150),
            text: "vss".to_string(),
        },
    );

    let report = extract_connectivity(&doc, &technology).unwrap();
    let device = report.devices.first().expect("MOS device should extract");
    let terminal_net = |name: &str| {
        device
            .terminals
            .iter()
            .find(|terminal| terminal.name == name)
            .and_then(|terminal| terminal.net_name.as_deref())
    };

    assert_eq!(terminal_net("D"), Some("DRAIN"));
    assert_eq!(terminal_net("G"), Some("GATE"));
    assert_eq!(terminal_net("S"), Some("SOURCE"));
    assert_eq!(terminal_net("B"), Some("VSS"));
    assert!(
        device
            .occurrences
            .contains(&ShapeOccurrenceId::top_level(body))
    );
    assert_eq!(report.validate(), Vec::new());

    let exported = export_connectivity_spice(&doc.name, &report);
    assert!(exported.text.contains("MDEV1 DRAIN GATE SOURCE VSS NMOS"));
}

#[test]
pub(crate) fn spice_compare_includes_extracted_device_terminal_nets() {
    let technology = default_technology();
    let mut doc = Document::new("mos compare");
    let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        diffusion,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 300)),
    );
    doc.insert_shape(
        diffusion,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "sd".to_string(),
        },
    );
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, -100), 100, 500)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Label {
            position: Point::new(470, -50),
            text: "g".to_string(),
        },
    );
    let report = extract_connectivity(&doc, &technology).unwrap();
    let schematic =
        parse_spice_schematic_netlist(".subckt mos sd g\nM1 sd g sd 0 nmos\n.ends mos\n").unwrap();

    let comparison = compare_connectivity_to_spice_schematic(&report, &schematic);

    assert_eq!(
        comparison.layout_named_nets,
        vec!["SD".to_string(), "G".to_string(), "0".to_string()]
    );
    assert_eq!(
        comparison.layout_device_signatures,
        vec!["M|NMOS|SD,G,SD,0".to_string()]
    );
    assert_eq!(
        comparison.layout_device_signatures,
        comparison.schematic_device_signatures
    );
    assert_eq!(comparison.missing_layout_nets, Vec::<String>::new());
    assert_eq!(comparison.extra_layout_nets, Vec::<String>::new());
    assert_eq!(comparison.missing_layout_devices, Vec::<String>::new());
    assert_eq!(comparison.extra_layout_devices, Vec::<String>::new());
    assert_eq!(comparison.status, SpiceConnectivityComparisonStatus::Match);
}

#[test]
pub(crate) fn spice_compare_checks_mos_dimensions_when_schematic_supplies_them() {
    let technology = default_technology();
    let mut doc = Document::new("mos dimensions");
    let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        diffusion,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 300)),
    );
    doc.insert_shape(
        diffusion,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "sd".to_string(),
        },
    );
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, -100), 100, 500)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Label {
            position: Point::new(470, -50),
            text: "g".to_string(),
        },
    );
    let report = extract_connectivity(&doc, &technology).unwrap();
    let schematic = parse_spice_schematic_netlist(
        ".subckt mos sd g\nM1 sd g sd 0 nmos L=100 W=300\n.ends mos\n",
    )
    .unwrap();

    assert_eq!(
        schematic.devices[0].parameters.get("L").map(String::as_str),
        Some("100")
    );
    assert_eq!(
        schematic.devices[0].parameters.get("W").map(String::as_str),
        Some("300")
    );

    let comparison = compare_connectivity_to_spice_schematic(&report, &schematic);

    assert_eq!(
        comparison.layout_device_signatures,
        vec!["M|NMOS|SD,G,SD,0|L=100,W=300".to_string()]
    );
    assert_eq!(
        comparison.layout_device_signatures,
        comparison.schematic_device_signatures
    );
    assert_eq!(comparison.status, SpiceConnectivityComparisonStatus::Match);

    let mismatch = parse_spice_schematic_netlist(
        ".subckt mos sd g\nM1 sd g sd 0 nmos L=100 W=301\n.ends mos\n",
    )
    .unwrap();
    let comparison = compare_connectivity_to_spice_schematic(&report, &mismatch);

    assert_eq!(
        comparison.status,
        SpiceConnectivityComparisonStatus::Mismatch
    );
    assert_eq!(
        comparison.missing_layout_devices,
        vec!["M|NMOS|SD,G,SD,0|L=100,W=301".to_string()]
    );
    assert_eq!(
        comparison.extra_layout_devices,
        vec!["M|NMOS|SD,G,SD,0|L=100,W=300".to_string()]
    );

    let l_only =
        parse_spice_schematic_netlist(".subckt mos sd g\nM1 sd g sd 0 nmos L=100\n.ends mos\n")
            .unwrap();
    let comparison = compare_connectivity_to_spice_schematic(&report, &l_only);

    assert_eq!(
        comparison.layout_device_signatures,
        vec!["M|NMOS|SD,G,SD,0|L=100".to_string()]
    );
    assert_eq!(
        comparison.layout_device_signatures,
        comparison.schematic_device_signatures
    );
    assert_eq!(comparison.status, SpiceConnectivityComparisonStatus::Match);
}

#[test]
pub(crate) fn spice_compare_reports_extracted_device_mismatches() {
    let technology = default_technology();
    let mut doc = Document::new("mos mismatch");
    let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        diffusion,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 300)),
    );
    doc.insert_shape(
        diffusion,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "sd".to_string(),
        },
    );
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, -100), 100, 500)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Label {
            position: Point::new(470, -50),
            text: "g".to_string(),
        },
    );
    let report = extract_connectivity(&doc, &technology).unwrap();
    let schematic =
        parse_spice_schematic_netlist(".subckt mos sd g\nM1 sd g sd 0 pmos\n.ends mos\n").unwrap();

    let comparison = compare_connectivity_to_spice_schematic(&report, &schematic);

    assert_eq!(
        comparison.status,
        SpiceConnectivityComparisonStatus::Mismatch
    );
    assert_eq!(
        comparison.missing_layout_devices,
        vec!["M|PMOS|SD,G,SD,0".to_string()]
    );
    assert_eq!(
        comparison.extra_layout_devices,
        vec!["M|NMOS|SD,G,SD,0".to_string()]
    );
    assert_eq!(comparison.mismatch_count(), 2);
}

#[test]
pub(crate) fn extracts_poly_resistor_from_labeled_isolated_poly() {
    let technology = default_technology();
    let mut doc = Document::new("poly resistor");
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let resistor = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 120)),
    );
    for (x, text) in [(80, "a"), (920, "b")] {
        doc.insert_shape(
            poly,
            ShapeKind::Label {
                position: Point::new(x, 60),
                text: text.to_string(),
            },
        );
    }

    let report = extract_connectivity(&doc, &technology).unwrap();

    assert_eq!(report.components.len(), 0);
    assert_eq!(report.devices.len(), 1);
    let device = &report.devices[0];
    assert_eq!(device.kind, "resistor");
    assert_eq!(device.model, "polyres");
    assert_eq!(device.width, 120);
    assert_eq!(device.length, 1_000);
    assert_eq!(
        device.occurrences,
        vec![ShapeOccurrenceId::top_level(resistor)]
    );
    assert_eq!(
        device
            .terminals
            .iter()
            .map(|terminal| (terminal.name.as_str(), terminal.net_name.as_deref()))
            .collect::<Vec<_>>(),
        vec![("A", Some("A")), ("B", Some("B"))]
    );
    assert_eq!(report.validate(), Vec::new());
}

#[test]
pub(crate) fn spice_export_and_compare_include_poly_resistors() {
    let technology = default_technology();
    let mut doc = Document::new("poly resistor spice");
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 120)),
    );
    for (x, text) in [(80, "a"), (920, "b")] {
        doc.insert_shape(
            poly,
            ShapeKind::Label {
                position: Point::new(x, 60),
                text: text.to_string(),
            },
        );
    }

    let report = extract_connectivity(&doc, &technology).unwrap();
    let exported = export_connectivity_spice(&doc.name, &report);

    assert_eq!(exported.report.device_count, 1);
    assert!(exported.text.contains(".subckt POLY_RESISTOR_SPICE A B"));
    assert!(exported.text.contains("RDEV1 A B 1"));

    let schematic = parse_spice_schematic_netlist(".subckt r a b\nR1 b a 10k\n.ends r\n").unwrap();
    let comparison = compare_connectivity_to_spice_schematic(&report, &schematic);

    assert_eq!(
        comparison.layout_device_signatures,
        vec!["R|RES|A,B".to_string()]
    );
    assert_eq!(
        comparison.layout_device_signatures,
        comparison.schematic_device_signatures
    );
    assert_eq!(comparison.status, SpiceConnectivityComparisonStatus::Match);
}

#[test]
pub(crate) fn extracts_mim_capacitor_from_labeled_metal_overlap_without_via() {
    let technology = default_technology();
    let mut doc = Document::new("mim capacitor");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
    let lower = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 1_000)),
    );
    let upper = doc.insert_shape(
        metal2,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 200), 600, 600)),
    );
    doc.insert_shape(
        metal1,
        ShapeKind::Label {
            position: Point::new(300, 300),
            text: "bottom".to_string(),
        },
    );
    doc.insert_shape(
        metal2,
        ShapeKind::Label {
            position: Point::new(700, 700),
            text: "top".to_string(),
        },
    );

    let report = extract_connectivity(&doc, &technology).unwrap();
    let capacitors = report
        .devices
        .iter()
        .filter(|device| device.kind == "capacitor")
        .collect::<Vec<_>>();

    assert_eq!(capacitors.len(), 1);
    let device = capacitors[0];
    assert_eq!(device.model, "mimcap");
    assert_eq!(
        device.bounds,
        Rect::from_min_size(Point::new(200, 200), 600, 600)
    );
    assert_eq!(device.width, 600);
    assert_eq!(device.length, 600);
    assert_eq!(
        device.occurrences,
        vec![
            ShapeOccurrenceId::top_level(lower),
            ShapeOccurrenceId::top_level(upper)
        ]
    );
    assert_eq!(
        device
            .terminals
            .iter()
            .map(|terminal| (terminal.name.as_str(), terminal.net_name.as_deref()))
            .collect::<Vec<_>>(),
        vec![("A", Some("BOTTOM")), ("B", Some("TOP"))]
    );
    assert_eq!(report.validate(), Vec::new());
}

#[test]
pub(crate) fn mim_capacitor_skips_via_connected_overlap() {
    let technology = default_technology();
    let mut doc = Document::new("mim capacitor with via");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
    let via1 = doc.layer_by_process(ProcessLayer::Via1).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 1_000)),
    );
    doc.insert_shape(
        metal2,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 200), 600, 600)),
    );
    doc.insert_shape(
        via1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, 450), 100, 100)),
    );

    let report = extract_connectivity(&doc, &technology).unwrap();

    assert!(
        report
            .devices
            .iter()
            .all(|device| device.kind != "capacitor")
    );
}

#[test]
pub(crate) fn spice_export_and_compare_include_mim_capacitors() {
    let technology = default_technology();
    let mut doc = Document::new("mim cap spice");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 1_000)),
    );
    doc.insert_shape(
        metal2,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 200), 600, 600)),
    );
    doc.insert_shape(
        metal1,
        ShapeKind::Label {
            position: Point::new(300, 300),
            text: "bottom".to_string(),
        },
    );
    doc.insert_shape(
        metal2,
        ShapeKind::Label {
            position: Point::new(700, 700),
            text: "top".to_string(),
        },
    );

    let report = extract_connectivity(&doc, &technology).unwrap();
    let exported = export_connectivity_spice(&doc.name, &report);

    assert_eq!(exported.report.device_count, 1);
    assert!(exported.text.contains(".subckt MIM_CAP_SPICE BOTTOM TOP"));
    assert!(exported.text.contains("CDEV1 BOTTOM TOP 1"));

    let schematic =
        parse_spice_schematic_netlist(".subckt cap bottom top\nC1 top bottom 1f\n.ends cap\n")
            .unwrap();
    let comparison = compare_connectivity_to_spice_schematic(&report, &schematic);

    assert_eq!(
        comparison.layout_device_signatures,
        vec!["C|CAP|BOTTOM,TOP".to_string()]
    );
    assert_eq!(
        comparison.layout_device_signatures,
        comparison.schematic_device_signatures
    );
    assert_eq!(comparison.status, SpiceConnectivityComparisonStatus::Match);
}

#[test]
pub(crate) fn spice_export_writes_extracted_mos_devices() {
    let technology = default_technology();
    let mut doc = Document::new("mos spice");
    let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        diffusion,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 300)),
    );
    doc.insert_shape(
        diffusion,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "sd".to_string(),
        },
    );
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(450, -100), 100, 500)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Label {
            position: Point::new(470, -50),
            text: "g".to_string(),
        },
    );

    let report = extract_connectivity(&doc, &technology).unwrap();
    let exported = export_connectivity_spice(&doc.name, &report);

    assert_eq!(exported.report.device_count, 1);
    assert!(exported.text.contains(".subckt MOS_SPICE SD G"));
    assert!(exported.text.contains("MDEV1 SD G SD 0 NMOS L=100 W=300"));
}

#[test]
pub(crate) fn conflicting_labels_on_one_component_report_short() {
    let technology = default_technology();
    let mut doc = Document::new("short");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 800, 300)),
    );
    for (x, text) in [(100, "A"), (700, "B")] {
        doc.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(x, 100),
                text: text.to_string(),
            },
        );
    }

    let report = extract_connectivity(&doc, &technology).unwrap();

    assert_eq!(report.shorts.len(), 1);
    assert_eq!(
        report.shorts[0].names,
        vec!["A".to_string(), "B".to_string()]
    );
}

#[test]
pub(crate) fn spice_export_reports_named_generated_short_and_open_nets() {
    let technology = default_technology();
    let mut doc = Document::new("spice export");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 800, 300)),
    );
    doc.insert_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "clk".to_string(),
        },
    );
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(1_000, 0), 800, 300)),
    );
    doc.insert_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(1_100, 100),
            text: "clk".to_string(),
        },
    );
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(2_000, 0), 500, 300)),
    );
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(3_000, 0), 800, 300)),
    );
    doc.insert_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(3_100, 100),
            text: "vdd".to_string(),
        },
    );
    doc.insert_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(3_600, 100),
            text: "vss".to_string(),
        },
    );

    let report = extract_connectivity(&doc, &technology).unwrap();
    let exported = export_connectivity_spice(&doc.name, &report);

    assert_eq!(exported.report.component_count, 4);
    assert_eq!(exported.report.named_net_count, 2);
    assert_eq!(exported.report.generated_net_count, 2);
    assert_eq!(exported.report.short_count, 1);
    assert_eq!(exported.report.open_count, 1);
    assert!(exported.text.contains(".subckt SPICE_EXPORT"));
    assert!(exported.text.contains("CLK"));
    assert!(exported.text.contains("N_3"));
    assert!(exported.text.contains("* short component="));
    assert!(exported.text.contains("nets=VDD,VSS"));
    assert!(exported.text.contains("* open net=CLK"));
    assert!(exported.text.ends_with(".end\n"));
}

#[test]
pub(crate) fn spice_schematic_parse_and_compare_reports_named_net_mismatches() {
    let technology = default_technology();
    let mut doc = Document::new("spice compare");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
    for (x, label) in [(0, "clk"), (1_000, "vss"), (2_000, "unused")] {
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(x, 0), 600, 300)),
        );
        doc.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(x + 100, 100),
                text: label.to_string(),
            },
        );
    }
    let report = extract_connectivity(&doc, &technology).unwrap();
    let schematic = parse_spice_schematic_netlist(
        "
* schematic comment
.subckt compare clk
+ vss data
R1 clk vss 1k
M1 data clk vss vss nmos L=1u
.ends compare
.end
",
    )
    .unwrap();

    assert_eq!(schematic.circuit_name, "COMPARE");
    assert_eq!(
        schematic.pins,
        vec!["CLK".to_string(), "VSS".to_string(), "DATA".to_string()]
    );
    assert_eq!(schematic.devices.len(), 2);
    assert_eq!(schematic.devices[1].kind, "M");
    assert_eq!(
        schematic.referenced_nets,
        vec!["CLK".to_string(), "VSS".to_string(), "DATA".to_string()]
    );

    let comparison = compare_connectivity_to_spice_schematic(&report, &schematic);
    assert_eq!(
        comparison.status,
        SpiceConnectivityComparisonStatus::Mismatch
    );
    assert_eq!(comparison.missing_layout_nets, vec!["DATA".to_string()]);
    assert_eq!(comparison.extra_layout_nets, vec!["UNUSED".to_string()]);
    assert_eq!(comparison.schematic_device_count, 2);
    assert_eq!(
        comparison.missing_layout_devices,
        vec![
            "M|NMOS|DATA,CLK,VSS,VSS|L=1U".to_string(),
            "R|RES|CLK,VSS".to_string()
        ]
    );
    assert_eq!(comparison.problem_count(), 4);
    assert!(!comparison.is_match());
}

#[test]
pub(crate) fn spice_schematic_compare_accepts_matching_connectivity() {
    let report = ConnectivityReport {
        components: vec![
            NetComponent {
                id: 1,
                shapes: vec![ShapeOccurrenceId::top_level(ShapeId(1))],
                bounds: Rect::from_min_size(Point::new(0, 0), 100, 100),
                labels: Vec::new(),
                explicit_nets: Vec::new(),
                net_name: Some("clk".to_string()),
                net_id: None,
            },
            NetComponent {
                id: 2,
                shapes: vec![ShapeOccurrenceId::top_level(ShapeId(2))],
                bounds: Rect::from_min_size(Point::new(200, 0), 100, 100),
                labels: Vec::new(),
                explicit_nets: Vec::new(),
                net_name: Some("vss".to_string()),
                net_id: None,
            },
        ],
        ..Default::default()
    };
    let schematic =
        parse_spice_schematic_netlist(".subckt ok CLK VSS\nRLOAD CLK VSS 1k\n.ends ok\n").unwrap();

    let comparison = compare_connectivity_to_spice_schematic(&report, &schematic);

    assert_eq!(comparison.status, SpiceConnectivityComparisonStatus::Match);
    assert!(comparison.is_match());
    assert_eq!(comparison.mismatch_count(), 0);
    assert_eq!(comparison.layout_issue_count(), 0);
}

#[test]
pub(crate) fn repeated_label_on_disconnected_components_reports_open() {
    let technology = default_technology();
    let mut doc = Document::new("open");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
    for (x, label_x) in [(0, 100), (1_000, 1_100)] {
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(x, 0), 300, 300)),
        );
        doc.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(label_x, 100),
                text: "clk".to_string(),
            },
        );
    }

    let report = extract_connectivity(&doc, &technology).unwrap();

    assert_eq!(report.opens.len(), 1);
    assert_eq!(report.opens[0].name, "CLK");
    assert_eq!(report.opens[0].components.len(), 2);
}

#[test]
pub(crate) fn connectivity_golden_fixture_reports_expected_shorts_and_opens() {
    let fixture: ConnectivityGoldenFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/quality/connectivity_golden.json"
    )))
    .unwrap();
    let technology = default_technology();
    let doc = document_from_connectivity_fixture(&fixture);

    let report = extract_connectivity(&doc, &technology).unwrap();
    let summary = report.summary(8);

    assert_eq!(report.validate(), Vec::new());
    assert_eq!(report.components.len(), fixture.expect.component_count);
    assert_eq!(summary.component_count, fixture.expect.component_count);
    assert_eq!(summary.health.label(), fixture.expect.health);
    assert_eq!(
        summary.labeled_component_count,
        fixture.expect.labeled_component_count
    );
    assert_eq!(report.shorts.len(), fixture.expect.short_count);
    assert_eq!(report.shorts[0].names, fixture.expect.short_names);
    assert_eq!(report.shorts[0].stable_key(), fixture.expect.short_key);
    assert_eq!(report.opens.len(), fixture.expect.open_count);
    assert_eq!(report.opens[0].name, fixture.expect.open_name);
    assert_eq!(report.opens[0].stable_key(), fixture.expect.open_key);
    assert_eq!(
        report.opens[0].components.len(),
        fixture.expect.open_component_count
    );
    assert!(
        report
            .components
            .iter()
            .any(|component| component.net_name.as_deref() == Some("DATA")
                && component.shapes.len() == fixture.expect.data_component_shape_count)
    );
}

#[test]
pub(crate) fn connectivity_report_validation_rejects_stale_component_and_issue_metadata() {
    let first = ShapeOccurrenceId::top_level(ShapeId(1));
    let second = ShapeOccurrenceId::top_level(ShapeId(2));
    let stale = ShapeOccurrenceId::top_level(ShapeId(99));
    let bounds = Rect::from_min_size(Point::new(0, 0), 100, 100);
    let mut shape_to_component = BTreeMap::new();
    shape_to_component.insert(first.clone(), 1);
    shape_to_component.insert(second.clone(), 1);
    shape_to_component.insert(stale, 9);
    let report = ConnectivityReport {
        components: vec![
            NetComponent {
                id: 1,
                shapes: vec![first.clone(), second.clone()],
                bounds,
                labels: Vec::new(),
                explicit_nets: Vec::new(),
                net_name: Some("DATA".to_string()),
                net_id: Some(NetId(7)),
            },
            NetComponent {
                id: 3,
                shapes: vec![second],
                bounds,
                labels: Vec::new(),
                explicit_nets: Vec::new(),
                net_name: None,
                net_id: None,
            },
        ],
        shape_to_component,
        devices: Vec::new(),
        shorts: vec![
            NetShort {
                component: 42,
                names: vec!["VDD".to_string(), "VSS".to_string()],
                bounds,
            },
            NetShort {
                component: 1,
                names: vec!["VSS".to_string(), "VDD".to_string()],
                bounds,
            },
            NetShort {
                component: 1,
                names: vec!["CLK".to_string(), "clk".to_string()],
                bounds: Rect::from_min_size(Point::new(200, 0), 100, 100),
            },
        ],
        opens: vec![
            NetOpen {
                name: "CLK".to_string(),
                components: vec![1, 9, 1],
                bounds,
            },
            NetOpen {
                name: " ".to_string(),
                components: vec![1, 3],
                bounds: Rect::from_min_size(Point::new(400, 0), 100, 100),
            },
        ],
        skipped: Some(" ".to_string()),
    };

    let findings = report.validate();
    let messages = findings
        .iter()
        .map(|finding| finding.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        findings
            .iter()
            .any(|finding| finding.severity == ConnectivityValidationSeverity::Error)
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("empty skipped reason"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("expected 2"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("appears in components 1 and 3"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("maps to missing component 9"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("net_id NetId(7) is not present"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("net_name \"DATA\" has no label"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("references missing component 42"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("stable issue key") && message.contains("duplicated"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("fewer than two distinct net names"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("repeats component 1"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("open has an empty net name"))
    );
}

#[test]
pub(crate) fn connectivity_golden_fixture_carries_issue_state_keys() {
    let fixture: ConnectivityGoldenFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/quality/connectivity_golden.json"
    )))
    .unwrap();
    let technology = default_technology();
    let doc = document_from_connectivity_fixture(&fixture);
    let report = extract_connectivity(&doc, &technology).unwrap();
    let store = report.issue_store();
    let states = fixture
        .issue_states
        .iter()
        .map(|state| {
            (
                state.key.clone(),
                MarkerState {
                    hidden: state.hidden,
                    waived: state.waived,
                    note: state.note.clone(),
                    ..MarkerState::default()
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    assert_eq!(states.len(), fixture.issue_states.len());
    for expected in &fixture.issue_states {
        let record = store
            .get(&expected.key)
            .unwrap_or_else(|| panic!("missing fixture issue key {}", expected.key));
        let state = states.get(&expected.key).unwrap();
        assert_eq!(state.hidden, expected.hidden);
        assert_eq!(state.waived, expected.waived);
        assert_eq!(state.note, expected.note);
        assert!(matches!(
            record.kind(),
            ConnectivityIssueKind::Short | ConnectivityIssueKind::Open
        ));
    }
}

#[test]
pub(crate) fn connectivity_summary_captures_health_and_caps_issue_counts() {
    let technology = default_technology();
    let mut doc = Document::new("summary");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
    for (x, text) in [(0, "A"), (500, "B"), (1_000, "CLK"), (1_500, "CLK")] {
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(x, 0), 300, 300)),
        );
        doc.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(x + 100, 100),
                text: text.to_string(),
            },
        );
    }
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(2_000, 0), 500, 300)),
    );
    for (x, text) in [(2_050, "VDD"), (2_350, "VSS")] {
        doc.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(x, 100),
                text: text.to_string(),
            },
        );
    }

    let report = extract_connectivity(&doc, &technology).unwrap();
    let summary = report.summary(1);

    assert_eq!(summary.health, ConnectivityHealth::ShortsAndOpens);
    assert_eq!(summary.short_count, 1);
    assert_eq!(summary.open_count, 1);
    assert_eq!(summary.displayed_short_count, 1);
    assert_eq!(summary.displayed_open_count, 1);
    assert_eq!(summary.omitted_short_count, 0);
    assert_eq!(summary.omitted_open_count, 0);
    assert_eq!(summary.largest_component_shape_count, 1);
    assert!(summary.labeled_component_count >= 4);
}

#[test]
pub(crate) fn connectivity_summary_reports_skipped_and_omitted_rows() {
    let mut report = ConnectivityReport::skipped("too many shapes");
    report.shorts = vec![
        NetShort {
            component: 1,
            names: vec!["A".to_string(), "B".to_string()],
            bounds: Rect::from_min_size(Point::new(0, 0), 1, 1),
        },
        NetShort {
            component: 2,
            names: vec!["C".to_string(), "D".to_string()],
            bounds: Rect::from_min_size(Point::new(2, 0), 1, 1),
        },
    ];
    report.opens = vec![NetOpen {
        name: "CLK".to_string(),
        components: vec![3, 4],
        bounds: Rect::from_min_size(Point::new(4, 0), 1, 1),
    }];

    let summary = report.summary(1);

    assert_eq!(summary.health, ConnectivityHealth::Skipped);
    assert_eq!(summary.displayed_short_count, 1);
    assert_eq!(summary.omitted_short_count, 1);
    assert_eq!(summary.displayed_open_count, 1);
    assert_eq!(summary.omitted_open_count, 0);
    assert_eq!(summary.skipped.as_deref(), Some("too many shapes"));
}

#[test]
pub(crate) fn connectivity_issue_keys_ignore_generated_component_ids_and_ordering() {
    let first_short = NetShort {
        component: 7,
        names: vec!["VSS".to_string(), "VDD".to_string()],
        bounds: Rect::from_min_size(Point::new(0, 0), 100, 100),
    };
    let second_short = NetShort {
        component: 99,
        names: vec!["VDD".to_string(), "VSS".to_string()],
        bounds: first_short.bounds,
    };
    assert_eq!(first_short.stable_key(), second_short.stable_key());

    let first_open = NetOpen {
        name: "CLK".to_string(),
        components: vec![9, 3, 5],
        bounds: Rect::from_min_size(Point::new(200, 0), 100, 100),
    };
    let second_open = NetOpen {
        name: "CLK".to_string(),
        components: vec![50, 90, 30],
        bounds: first_open.bounds,
    };
    assert_eq!(first_open.stable_key(), second_open.stable_key());
}

#[test]
pub(crate) fn connectivity_issue_keys_survive_recomputed_component_numbering() {
    fn fixture(with_unrelated_component_first: bool) -> ConnectivityReport {
        let technology = default_technology();
        let mut doc = Document::new("connectivity key fixture");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();

        if with_unrelated_component_first {
            doc.insert_shape(
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(-2_000, 0), 100, 100)),
            );
        }

        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 500, 300)),
        );
        for (x, text) in [(50, "VDD"), (350, "VSS")] {
            doc.insert_shape(
                annotation,
                ShapeKind::Label {
                    position: Point::new(x, 100),
                    text: text.to_string(),
                },
            );
        }

        for (x, label_x) in [(1_000, 1_100), (2_000, 2_100)] {
            doc.insert_shape(
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(x, 0), 300, 300)),
            );
            doc.insert_shape(
                annotation,
                ShapeKind::Label {
                    position: Point::new(label_x, 100),
                    text: "CLK".to_string(),
                },
            );
        }

        extract_connectivity(&doc, &technology).unwrap()
    }

    let baseline = fixture(false);
    let shifted = fixture(true);
    assert_eq!(baseline.shorts.len(), 1);
    assert_eq!(baseline.opens.len(), 1);
    assert_eq!(shifted.shorts.len(), 1);
    assert_eq!(shifted.opens.len(), 1);
    assert_ne!(baseline.shorts[0].component, shifted.shorts[0].component);
    assert_ne!(baseline.opens[0].components, shifted.opens[0].components);

    assert_eq!(
        baseline.shorts[0].stable_key(),
        shifted.shorts[0].stable_key()
    );
    assert_eq!(
        baseline.opens[0].stable_key(),
        shifted.opens[0].stable_key()
    );
    assert!(
        shifted
            .issue_store()
            .get(&baseline.shorts[0].stable_key())
            .is_some()
    );
    assert!(
        shifted
            .issue_store()
            .get(&baseline.opens[0].stable_key())
            .is_some()
    );
}

#[test]
pub(crate) fn connectivity_issue_store_queries_and_caps_mixed_issues() {
    let short = NetShort {
        component: 1,
        names: vec!["A".to_string(), "B".to_string()],
        bounds: Rect::from_min_size(Point::new(0, 0), 1, 1),
    };
    let open = NetOpen {
        name: "CLK".to_string(),
        components: vec![3, 4],
        bounds: Rect::from_min_size(Point::new(4, 0), 1, 1),
    };
    let open_key = open.stable_key();
    let report = ConnectivityReport {
        shorts: vec![short],
        opens: vec![open],
        ..Default::default()
    };

    let store = report.issue_store();
    let summary = store.summary(1);

    assert_eq!(summary.total_count, 2);
    assert_eq!(summary.displayed_count, 1);
    assert_eq!(summary.omitted_count, 1);
    assert_eq!(summary.short_count, 1);
    assert_eq!(summary.open_count, 1);
    assert_eq!(store.displayed_records(1).len(), 1);
    assert_eq!(
        store.get(&open_key).map(ConnectivityIssueRecord::kind),
        Some(ConnectivityIssueKind::Open)
    );
    assert_eq!(
        store.get(&open_key).map(|record| record.kind().label()),
        Some("open")
    );
}

#[test]
pub(crate) fn explicit_net_ids_propagate_to_components() {
    let technology = default_technology();
    let mut doc = Document::new("explicit nets");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let id = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 300, 300)),
    );
    doc.shapes.get_mut(&id).unwrap().net = Some(NetId(42));

    let report = extract_connectivity(&doc, &technology).unwrap();

    assert_eq!(report.components[0].net_id, Some(NetId(42)));
    assert_eq!(report.components[0].net_name.as_deref(), Some("NET42"));
}
