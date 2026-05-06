#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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
    let mut options = fabricad_app::StartupOptions::default();
    for (key, value) in query_pairs(&search) {
        match key.as_str() {
            "scene" if value == "hierarchy" => {
                options.hierarchy_demo = true;
            }
            "scene" if value == "stress" => {
                options.stress_count = Some(options.stress_count.unwrap_or(10_000));
            }
            "view" if value == "3d" => {
                options.view_3d = true;
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

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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
