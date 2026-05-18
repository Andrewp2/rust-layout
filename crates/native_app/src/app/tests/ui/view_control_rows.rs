#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn representative_view_control_rows_use_specific_labels() {
    for view in StartupView::ALL {
        let document = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        })
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("view control document should build");
        let labels = view_control_row_labels(&document);
        assert!(
            !labels.contains(&"Controls"),
            "view {view:?} should not expose generic row labels: {labels:?}"
        );
    }

    for (view, required_labels) in [
        (
            StartupView::Workflow,
            &[
                "Design",
                "Operations",
                "Materials",
                "Facilities",
                "Quality",
                "Trace",
                "Data",
                "Lot focus",
            ][..],
        ),
        (
            StartupView::Traceability,
            &[
                "Lot", "Wafer", "Scope", "Impact", "Process", "Material", "Event",
            ][..],
        ),
        (
            StartupView::LayoutDiff,
            &[
                "Baseline",
                "Candidate",
                "Compare",
                "Filter",
                "Page size",
                "Changes",
                "Review",
                "Load",
                "Reset",
            ][..],
        ),
        (
            StartupView::FabControl,
            &["Tools", "More tools", "Recipe", "Run", "Service"][..],
        ),
        (StartupView::Inventory, &["Filter", "Lot"][..]),
        (StartupView::Environment, &["Sensor", "Zone"][..]),
        (StartupView::Metrology, &["Mode", "Measure", "Review"][..]),
        (StartupView::Yield, &["Filter", "Focus", "Lot", "Wafer"][..]),
        (
            StartupView::ProcessFlow,
            &["Filter", "Review", "Export", "Node"][..],
        ),
        (
            StartupView::Safety,
            &["Tool", "Interlock", "Lockout", "Incident"][..],
        ),
        (
            StartupView::CrossSection,
            &["Step", "Display", "Material"][..],
        ),
        (
            StartupView::Notebook,
            &["Mode", "Add", "Tag", "Link", "Focus", "Entry"][..],
        ),
        (
            StartupView::Experiment,
            &["Queue", "Capture", "Response", "Filter", "Lot", "Run"][..],
        ),
    ] {
        let document = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        })
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("view control document should build");
        let labels = view_control_row_labels(&document);
        for required_label in required_labels {
            assert!(
                labels.contains(required_label),
                "view {view:?} should include {required_label:?} among {labels:?}"
            );
        }
    }
}

pub(crate) fn view_control_row_labels(document: &UiDocument) -> Vec<&str> {
    document
        .nodes()
        .iter()
        .filter(|node| {
            node.name().contains(".viewctl.")
                && node.name().contains(".row.")
                && node.name().ends_with(".label")
        })
        .filter_map(|node| match node.content() {
            UiContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
}

#[test]
pub(crate) fn all_views_are_warning_free_at_audit_sizes() {
    for viewport in [
        UiSize::new(480.0, 900.0),
        UiSize::new(768.0, 1024.0),
        UiSize::new(1280.0, 720.0),
        UiSize::new(1440.0, 920.0),
        UiSize::new(1920.0, 1080.0),
    ] {
        for view in StartupView::ALL {
            let app = GlassworksApp::new_with_options(StartupOptions {
                view_mode: Some(view),
                ..Default::default()
            });
            let document = app
                .build_operad_document(viewport)
                .expect("audit-sized view document should build");
            let warnings = document.audit_layout();
            assert!(
                warnings.is_empty(),
                "view {view:?} viewport {viewport:?} warnings: {warnings:?}"
            );
        }
    }
}

#[test]
pub(crate) fn all_views_are_warning_free_at_hidpi_scale() {
    let viewport = UiSize::new(2048.0, 1440.0);
    let ui_scale = UiScale::new(2.0);
    for view in StartupView::ALL {
        let app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        });
        let document = app
            .build_operad_document_scaled(viewport, ui_scale)
            .expect("HiDPI audit-sized view document should build");
        let warnings = document.audit_layout();
        assert!(
            warnings.is_empty(),
            "view {view:?} viewport {viewport:?} scale {ui_scale:?} warnings: {warnings:?}"
        );
    }
}

pub(crate) fn assert_clicked_node(document: &mut UiDocument, name: &str) {
    let (node_id, point) = document
        .nodes()
        .iter()
        .enumerate()
        .find(|(_, node)| node.name() == name)
        .map(|(index, node)| {
            (
                operad::UiNodeId::from_index(index),
                UiPoint::new(
                    node.layout().rect.x + node.layout().rect.width * 0.5,
                    node.layout().rect.y + node.layout().rect.height * 0.5,
                ),
            )
        })
        .unwrap_or_else(|| panic!("expected node {name} to exist"));

    let down = document.handle_input(operad::UiInputEvent::PointerDown(point));
    assert_eq!(down.pressed, Some(node_id));
    let up = document.handle_input(operad::UiInputEvent::PointerUp(point));
    assert_eq!(up.clicked, Some(node_id));
}

pub(crate) fn node_rect(document: &UiDocument, name: &str) -> UiRect {
    document
        .nodes()
        .iter()
        .find(|node| node.name() == name)
        .map(|node| node.layout().rect)
        .unwrap_or_else(|| panic!("expected node {name} to exist"))
}

pub(crate) fn node_id(document: &UiDocument, name: &str) -> operad::UiNodeId {
    document
        .nodes()
        .iter()
        .enumerate()
        .find(|(_, node)| node.name() == name)
        .map(|(index, _)| operad::UiNodeId::from_index(index))
        .unwrap_or_else(|| panic!("expected node {name} to exist"))
}
