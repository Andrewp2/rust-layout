#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub fn builtin_demo_workspace_report() -> Result<fabricad_app::BuiltinDemoWorkspaceReport, String> {
    fabricad_app::validate_builtin_demo_workspace()
}

pub fn quality_fixture_report() -> Result<fabricad_app::QualityFixtureReport, String> {
    fabricad_app::validate_quality_fixtures()
}

pub fn persistence_fixture_report() -> Result<fabricad_app::PersistenceFixtureReport, String> {
    fabricad_app::validate_persistence_fixtures()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = validateBuiltinDemoWorkspace)]
pub fn validate_builtin_demo_workspace_js() -> Result<String, JsValue> {
    let report = builtin_demo_workspace_report().map_err(|err| JsValue::from_str(&err))?;
    Ok(format!(
        "{} workspace_schema={} document_schema={} shapes={} lots={} cross_section_steps={} tools={}",
        report.source,
        report.workspace_schema_version,
        report.document_schema_version,
        report.document_shapes,
        report.mes_lots,
        report.cross_section_steps,
        report.equipment_tools
    ))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = validateQualityFixtures)]
pub fn validate_quality_fixtures_js() -> Result<String, JsValue> {
    let report = quality_fixture_report().map_err(|err| JsValue::from_str(&err))?;
    Ok(format!(
        "drc={} rules={} connectivity_components={} shorts={} opens={} issue_keys={} issue_states={}",
        report.drc_violations,
        report.drc_rule_families,
        report.connectivity_components,
        report.connectivity_shorts,
        report.connectivity_opens,
        report.connectivity_issue_keys,
        report.connectivity_issue_states
    ))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = validatePersistenceFixtures)]
pub fn validate_persistence_fixtures_js() -> Result<String, JsValue> {
    let report = persistence_fixture_report().map_err(|err| JsValue::from_str(&err))?;
    Ok(format!(
        "legacy_migrated={} future_workspace_rejected={} future_metadata_rejected={} future_document_rejected={} malformed_schema_rejected={} malformed_metadata_rejected={} unsupported_features_rejected={}",
        report.migrated_legacy_schema_zero,
        report.rejected_future_workspace_schema,
        report.rejected_future_metadata_schema,
        report.rejected_future_document_schema,
        report.rejected_malformed_schema_fields,
        report.rejected_malformed_metadata_arrays,
        report.rejected_unsupported_feature_flags
    ))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("missing document"))?;
    let canvas = document
        .get_element_by_id("fabricad_canvas")
        .ok_or_else(|| JsValue::from_str("missing #fabricad_canvas"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| {
                Ok(Box::new(fabricad_app::FabricadApp::new_with_options(
                    cc,
                    startup_options_from_url(),
                )))
            }),
        )
        .await
}

#[cfg(target_arch = "wasm32")]
fn startup_options_from_url() -> fabricad_app::StartupOptions {
    let Some(window) = web_sys::window() else {
        return fabricad_app::StartupOptions::default();
    };
    let search = window.location().search().unwrap_or_default();
    startup_options_from_query(&search)
}

pub fn startup_options_from_query(search: &str) -> fabricad_app::StartupOptions {
    let mut options = fabricad_app::StartupOptions::default();
    for (key, value) in query_pairs(search) {
        match key.as_str() {
            "workspace" if value == "demo" => {
                options.demo_workspace = true;
            }
            "demo" if value == "1" || value == "true" => {
                options.demo_workspace = true;
            }
            "scene" if value == "hierarchy" => {
                options.hierarchy_demo = true;
            }
            "scene" if value == "stress" => {
                options.stress_count = Some(options.stress_count.unwrap_or(10_000));
            }
            "view" => {
                if let Some(view) = fabricad_app::StartupView::from_slug(&value) {
                    options.view_mode = Some(view);
                } else if value == "3d" {
                    options.view_3d = true;
                }
            }
            "options" if value == "1" || value == "true" => {
                options.show_options = true;
            }
            "count" => {
                if let Ok(count) = value.parse::<usize>() {
                    options.stress_count = Some(count);
                }
            }
            "zoom" => {
                if let Ok(zoom) = value.parse::<f32>() {
                    options.zoom = Some(zoom);
                }
            }
            "select" if value == "first" => {
                options.select_first_shape = true;
            }
            "edit" if value == "vertex_moved" => {
                options.select_first_shape = true;
                options.move_first_vertex = true;
            }
            "workflow" if value == "hierarchy_make_place" => {
                options.hierarchy_workflow_demo = true;
            }
            "pan_x" => {
                let mut pan = options.pan.unwrap_or([0.0, 0.0]);
                if let Ok(value) = value.parse::<f32>() {
                    pan[0] = value;
                    options.pan = Some(pan);
                }
            }
            "pan_y" => {
                let mut pan = options.pan.unwrap_or([0.0, 0.0]);
                if let Ok(value) = value.parse::<f32>() {
                    pan[1] = value;
                    options.pan = Some(pan);
                }
            }
            _ => {}
        }
    }
    options
}

fn query_pairs(search: &str) -> Vec<(String, String)> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = decode_query_component(parts.next()?);
            let value = decode_query_component(parts.next().unwrap_or_default());
            Some((key, value))
        })
        .collect()
}

fn decode_query_component(value: &str) -> String {
    let mut decoded = String::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hi = hex_value(bytes[index + 1]);
                let lo = hex_value(bytes[index + 2]);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    decoded.push((hi * 16 + lo) as char);
                    index += 3;
                } else {
                    decoded.push('%');
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte as char);
                index += 1;
            }
        }
    }
    decoded
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn wasm_target_only() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_demo_workspace_validates_without_filesystem_access() {
        let report = builtin_demo_workspace_report().unwrap();

        assert_eq!(report.source, "builtin");
        assert!(report.document_shapes > 0);
        assert!(report.mes_lots > 0);
        assert!(report.yield_lots > 0);
        assert!(report.wafer_dies > 0);
        assert!(report.recipes > 0);
        assert!(report.process_flow_nodes > 0);
        assert!(report.cross_section_steps > 0);
        assert!(report.notebook_entries > 0);
        assert!(report.equipment_tools > 0);
    }

    #[test]
    fn quality_fixtures_validate_without_filesystem_access() {
        let report = quality_fixture_report().unwrap();

        assert_eq!(report.drc_violations, 5);
        assert_eq!(report.drc_rule_families, 5);
        assert_eq!(report.connectivity_components, 4);
        assert_eq!(report.connectivity_shorts, 1);
        assert_eq!(report.connectivity_opens, 1);
        assert_eq!(report.connectivity_issue_keys, 2);
        assert_eq!(report.connectivity_issue_states, 2);
    }

    #[test]
    fn persistence_fixtures_validate_without_filesystem_access() {
        let report = persistence_fixture_report().unwrap();

        assert_eq!(report.migrated_legacy_schema_zero, 1);
        assert_eq!(report.rejected_future_workspace_schema, 1);
        assert_eq!(report.rejected_future_metadata_schema, 1);
        assert_eq!(report.rejected_future_document_schema, 1);
        assert_eq!(report.rejected_malformed_schema_fields, 1);
        assert_eq!(report.rejected_malformed_metadata_arrays, 1);
        assert_eq!(report.rejected_unsupported_feature_flags, 1);
    }

    #[test]
    fn startup_query_parses_demo_workspace_and_view() {
        let options = startup_options_from_query(
            "?workspace=demo&view=metrology&zoom=0.25&pan_x=12.5&pan_y=-4&select=first",
        );

        assert!(options.demo_workspace);
        assert_eq!(
            options.view_mode,
            Some(fabricad_app::StartupView::Metrology)
        );
        assert_eq!(options.zoom, Some(0.25));
        assert_eq!(options.pan, Some([12.5, -4.0]));
        assert!(options.select_first_shape);
    }

    #[test]
    fn startup_query_parses_encoded_values_and_demo_alias() {
        let options = startup_options_from_query(
            "?demo=true&view=process%2Dflow&options=1&scene=stress&count=250",
        );

        assert!(options.demo_workspace);
        assert_eq!(
            options.view_mode,
            Some(fabricad_app::StartupView::ProcessFlow)
        );
        assert!(options.show_options);
        assert_eq!(options.stress_count, Some(250));
    }
}
