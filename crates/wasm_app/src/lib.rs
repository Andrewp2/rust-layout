#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub fn builtin_demo_workspace_report() -> Result<glassworks_studio::BuiltinDemoWorkspaceReport, String> {
    glassworks_studio::validate_builtin_demo_workspace()
}

pub fn quality_fixture_report() -> Result<glassworks_studio::QualityFixtureReport, String> {
    glassworks_studio::validate_quality_fixtures()
}

pub fn persistence_fixture_report() -> Result<glassworks_studio::PersistenceFixtureReport, String> {
    glassworks_studio::validate_persistence_fixtures()
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
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("missing document"))?;
    let canvas = document
        .get_element_by_id("glassworks_canvas")
        .ok_or_else(|| JsValue::from_str("missing #glassworks_canvas"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let options = startup_options_from_url();
    let report = glassworks_studio::run_operad_audit(options).map_err(|err| JsValue::from_str(&err))?;
    let summary = report.summary();
    canvas.set_attribute("aria-label", &summary)?;
    canvas.set_text_content(Some(&summary));
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn startup_options_from_url() -> glassworks_studio::StartupOptions {
    let Some(window) = web_sys::window() else {
        return glassworks_studio::StartupOptions::default();
    };
    let search = window.location().search().unwrap_or_default();
    startup_options_from_query(&search)
}

pub fn startup_options_from_query(search: &str) -> glassworks_studio::StartupOptions {
    let mut options = glassworks_studio::StartupOptions::default();
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
                if let Some(view) = glassworks_studio::StartupView::from_slug(&value) {
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
            "edit" if value == "move-first-vertex" => {
                options.move_first_vertex = true;
            }
            "pan" => {
                if let Some(pan) = parse_pan(&value) {
                    options.pan = Some(pan);
                }
            }
            "workflow" if value == "hierarchy" => {
                options.hierarchy_workflow_demo = true;
            }
            _ => {}
        }
    }
    options
}

fn query_pairs(search: &str) -> impl Iterator<Item = (String, String)> + '_ {
    search
        .trim_start_matches('?')
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (decode_query_component(key), decode_query_component(value))
        })
}

fn decode_query_component(value: &str) -> String {
    let mut output = String::new();
    let mut bytes = value.as_bytes().iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        match byte {
            b'+' => output.push(' '),
            b'%' => {
                let hi = bytes.next();
                let lo = bytes.next();
                if let (Some(hi), Some(lo)) = (hi, lo)
                    && let Some(decoded) = decode_hex_pair(hi, lo)
                {
                    output.push(decoded as char);
                }
            }
            _ => output.push(byte as char),
        }
    }
    output
}

fn decode_hex_pair(hi: u8, lo: u8) -> Option<u8> {
    Some(hex_value(hi)? * 16 + hex_value(lo)?)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_pan(value: &str) -> Option<[f32; 2]> {
    let (x, y) = value.split_once(',')?;
    Some([x.parse().ok()?, y.parse().ok()?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_parses_view_slug() {
        let options = startup_options_from_query("?view=metrology");
        assert_eq!(
            options.view_mode,
            Some(glassworks_studio::StartupView::Metrology)
        );
    }

    #[test]
    fn query_decodes_process_flow_route() {
        let options = startup_options_from_query("?view=process-flow");
        assert_eq!(
            options.view_mode,
            Some(glassworks_studio::StartupView::ProcessFlow)
        );
    }
}
