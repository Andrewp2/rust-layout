#![allow(unused_imports)]
use super::*;

pub(crate) fn malformed_schema_field_workspace_json() -> Result<String, String> {
    let mut value = serde_json::to_value(WorkspaceDataset::blank())
        .map_err(|err| format!("failed to build malformed schema fixture: {err}"))?;
    value["schema_version"] = serde_json::Value::String("future".to_string());
    serde_json::to_string(&value)
        .map_err(|err| format!("failed to encode malformed schema fixture: {err}"))
}

pub(crate) fn assert_persistence_fixture_rejected(
    label: &str,
    contents: &str,
    required_messages: &[&str],
) -> Result<(), String> {
    let err = WorkspaceDataset::from_json_str(contents)
        .err()
        .ok_or_else(|| format!("{label} persistence fixture unexpectedly loaded"))?;
    for required in required_messages {
        if !err.contains(required) {
            return Err(format!(
                "{label} persistence fixture error {err:?} did not contain {required:?}"
            ));
        }
    }
    if err.contains("missing field") {
        return Err(format!(
            "{label} persistence fixture failed with generic missing-field parse error: {err}"
        ));
    }
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct QualityConnectivityFixture {
    pub(crate) name: String,
    pub(crate) shapes: Vec<QualityFixtureShape>,
    pub(crate) expect: QualityConnectivityExpectation,
    #[serde(default)]
    pub(crate) issue_states: Vec<QualityConnectivityIssueState>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct QualityConnectivityExpectation {
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

#[derive(Debug, serde::Deserialize)]
pub(crate) struct QualityConnectivityIssueState {
    pub(crate) key: String,
    pub(crate) hidden: bool,
    pub(crate) waived: bool,
    pub(crate) note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConnectivityQualityValidation {
    pub(crate) component_count: usize,
    pub(crate) short_count: usize,
    pub(crate) open_count: usize,
    pub(crate) issue_key_count: usize,
    pub(crate) issue_state_count: usize,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct QualityDrcFixture {
    pub(crate) name: String,
    pub(crate) shapes: Vec<QualityFixtureShape>,
    pub(crate) expect: QualityDrcExpectation,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct QualityDrcExpectation {
    pub(crate) total_count: usize,
    pub(crate) omitted_count: usize,
    pub(crate) rule_counts: BTreeMap<String, usize>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum QualityFixtureShape {
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

pub(crate) fn validate_drc_quality_fixture() -> Result<(usize, usize), String> {
    let fixture: QualityDrcFixture = serde_json::from_str(include_str!(
        "../../../../../fixtures/quality/drc_golden.json"
    ))
    .map_err(|err| format!("failed to parse DRC quality fixture: {err}"))?;
    let document = document_from_quality_fixture(&fixture.name, &fixture.shapes)?;
    let rules = RuleDeck::demo(&document);
    let rule_findings = rules.validate_for_document(&document);
    if !rule_findings.is_empty() {
        let detail = rule_findings
            .iter()
            .map(|finding| format!("{:?}: {}", finding.severity, finding.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "DRC quality fixture has an invalid rule deck: {detail}"
        ));
    }
    let violations = run_drc(&document, &rules);
    let store = DrcIssueStore::from_violations(violations.clone());
    let validation_findings = store.validate();
    if !validation_findings.is_empty() {
        let detail = validation_findings
            .iter()
            .map(|finding| format!("{:?}: {}", finding.severity, finding.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "DRC quality fixture generated an invalid issue store: {detail}"
        ));
    }
    let summary = store.summary(16);
    let mut rule_counts = BTreeMap::new();
    for violation in violations {
        *rule_counts.entry(violation.rule).or_insert(0usize) += 1;
    }

    if summary.total_count != fixture.expect.total_count {
        return Err(format!(
            "DRC quality fixture expected {} violations, got {}",
            fixture.expect.total_count, summary.total_count
        ));
    }
    if summary.omitted_count != fixture.expect.omitted_count {
        return Err(format!(
            "DRC quality fixture expected {} omitted rows, got {}",
            fixture.expect.omitted_count, summary.omitted_count
        ));
    }
    if rule_counts != fixture.expect.rule_counts {
        return Err(format!(
            "DRC quality fixture rule counts differ: expected {:?}, got {:?}",
            fixture.expect.rule_counts, rule_counts
        ));
    }
    Ok((summary.total_count, rule_counts.len()))
}

pub(crate) fn validate_connectivity_quality_fixture()
-> Result<ConnectivityQualityValidation, String> {
    let fixture: QualityConnectivityFixture = serde_json::from_str(include_str!(
        "../../../../../fixtures/quality/connectivity_golden.json"
    ))
    .map_err(|err| format!("failed to parse connectivity quality fixture: {err}"))?;
    let document = document_from_quality_fixture(&fixture.name, &fixture.shapes)?;
    let report = extract_connectivity(&document, &layout_model::default_technology())
        .map_err(|err| format!("connectivity quality fixture failed: {err}"))?;
    let validation_findings = report.validate();
    if !validation_findings.is_empty() {
        let detail = validation_findings
            .iter()
            .map(|finding| format!("{:?}: {}", finding.severity, finding.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "connectivity quality fixture generated an invalid report: {detail}"
        ));
    }
    let summary = report.summary(8);

    if summary.health.label() != fixture.expect.health {
        return Err(format!(
            "connectivity quality fixture expected health {:?}, got {:?}",
            fixture.expect.health,
            summary.health.label()
        ));
    }
    if summary.component_count != fixture.expect.component_count {
        return Err(format!(
            "connectivity quality fixture expected {} components, got {}",
            fixture.expect.component_count, summary.component_count
        ));
    }
    if summary.labeled_component_count != fixture.expect.labeled_component_count {
        return Err(format!(
            "connectivity quality fixture expected {} labeled components, got {}",
            fixture.expect.labeled_component_count, summary.labeled_component_count
        ));
    }
    if report.shorts.len() != fixture.expect.short_count {
        return Err(format!(
            "connectivity quality fixture expected {} shorts, got {}",
            fixture.expect.short_count,
            report.shorts.len()
        ));
    }
    if report
        .shorts
        .first()
        .is_none_or(|short| short.names != fixture.expect.short_names)
    {
        return Err(format!(
            "connectivity quality fixture short names differ: expected {:?}, got {:?}",
            fixture.expect.short_names,
            report.shorts.first().map(|short| &short.names)
        ));
    }
    if report
        .shorts
        .first()
        .is_none_or(|short| short.stable_key() != fixture.expect.short_key)
    {
        return Err(format!(
            "connectivity quality fixture short key differs: expected {:?}, got {:?}",
            fixture.expect.short_key,
            report.shorts.first().map(|short| short.stable_key())
        ));
    }
    if report.opens.len() != fixture.expect.open_count {
        return Err(format!(
            "connectivity quality fixture expected {} opens, got {}",
            fixture.expect.open_count,
            report.opens.len()
        ));
    }
    if report.opens.first().is_none_or(|open| {
        open.name != fixture.expect.open_name
            || open.components.len() != fixture.expect.open_component_count
    }) {
        return Err(format!(
            "connectivity quality fixture open differs: expected {} with {} components, got {:?}",
            fixture.expect.open_name,
            fixture.expect.open_component_count,
            report.opens.first()
        ));
    }
    if report
        .opens
        .first()
        .is_none_or(|open| open.stable_key() != fixture.expect.open_key)
    {
        return Err(format!(
            "connectivity quality fixture open key differs: expected {:?}, got {:?}",
            fixture.expect.open_key,
            report.opens.first().map(|open| open.stable_key())
        ));
    }
    if !report.components.iter().any(|component| {
        component.net_name.as_deref() == Some("DATA")
            && component.shapes.len() == fixture.expect.data_component_shape_count
    }) {
        return Err(format!(
            "connectivity quality fixture missing DATA component with {} shapes",
            fixture.expect.data_component_shape_count
        ));
    }

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
    for expected in &fixture.issue_states {
        let record = store.get(&expected.key).ok_or_else(|| {
            format!(
                "connectivity quality fixture missing expected issue key {:?}",
                expected.key
            )
        })?;
        if !matches!(
            record.kind(),
            ConnectivityIssueKind::Short | ConnectivityIssueKind::Open
        ) {
            return Err(format!(
                "connectivity quality fixture issue key {:?} resolved to unexpected kind {:?}",
                expected.key,
                record.kind()
            ));
        }
        let state = states.get(&expected.key).ok_or_else(|| {
            format!(
                "connectivity quality fixture missing expected issue state {:?}",
                expected.key
            )
        })?;
        if state.hidden != expected.hidden
            || state.waived != expected.waived
            || state.note != expected.note
        {
            return Err(format!(
                "connectivity quality fixture issue state {:?} differs: expected hidden={} waived={} note={:?}, got {:?}",
                expected.key, expected.hidden, expected.waived, expected.note, state
            ));
        }
    }

    Ok(ConnectivityQualityValidation {
        component_count: summary.component_count,
        short_count: report.shorts.len(),
        open_count: report.opens.len(),
        issue_key_count: fixture.issue_states.len(),
        issue_state_count: states.len(),
    })
}

pub(crate) fn document_from_quality_fixture(
    name: &str,
    shapes: &[QualityFixtureShape],
) -> Result<Document, String> {
    let mut document = Document::new(name);
    for shape in shapes {
        match shape {
            QualityFixtureShape::Rect { layer, x, y, w, h } => {
                let layer_id = quality_fixture_layer(&document, layer)?;
                document.insert_shape(
                    layer_id,
                    ShapeKind::Rectangle(Rect::from_min_size(Point::new(*x, *y), *w, *h)),
                );
            }
            QualityFixtureShape::Label { layer, x, y, text } => {
                let layer_id = quality_fixture_layer(&document, layer)?;
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
    Ok(document)
}

pub(crate) fn quality_fixture_layer(document: &Document, layer: &str) -> Result<LayerId, String> {
    let process = ProcessLayer::from_technology_name(layer)
        .ok_or_else(|| format!("unknown fixture layer {layer:?}"))?;
    document
        .layer_by_process(process)
        .ok_or_else(|| format!("fixture layer {layer:?} missing from document"))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn run_offscreen_render_async(
    options: OffscreenRenderOptions,
) -> Result<OffscreenRenderReport, Box<dyn std::error::Error>> {
    let width = options.width.max(1);
    let height = options.height.max(1);
    let zoom = options.zoom.clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
    let (scene_name, document) = offscreen_document(&options.scene);
    renderer::gpu::render_document_offscreen(OffscreenRenderRequest {
        scene: scene_name,
        document,
        width,
        height,
        zoom,
        pan: options.pan,
    })
    .await
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn offscreen_document(scene: &OffscreenScene) -> (String, Document) {
    match scene {
        OffscreenScene::Demo => ("demo".to_string(), Document::demo()),
        OffscreenScene::Hierarchy => ("hierarchy".to_string(), Document::hierarchy_demo()),
        OffscreenScene::Stress { count } => (format!("stress:{count}"), Document::stress(*count)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "write".into());
    let temp_path = path.with_file_name(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_native_text_file_maybe_gzip(path: &Path) -> std::io::Result<String> {
    let bytes = read_native_bytes_file_maybe_gzip(path)?;
    String::from_utf8(bytes).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("file is not valid UTF-8: {error}"),
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_native_bytes_file_maybe_gzip(path: &Path) -> std::io::Result<Vec<u8>> {
    let bytes = fs::read(path)?;
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoded = Vec::new();
        GzDecoder::new(&bytes[..]).read_to_end(&mut decoded)?;
        return Ok(decoded);
    }
    if bytes.starts_with(&[0x50, 0x4b, 0x03, 0x04]) {
        return read_native_single_file_zip(&bytes);
    }
    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_native_single_file_zip(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut index = 0usize;
    while index + 30 <= bytes.len() {
        let signature = read_le_u32(bytes, index)?;
        match signature {
            0x0403_4b50 => {}
            0x0201_4b50 | 0x0605_4b50 => break,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "ZIP archive has an invalid local file header",
                ));
            }
        }
        let flags = read_le_u16(bytes, index + 6)?;
        let method = read_le_u16(bytes, index + 8)?;
        let compressed_size = read_le_u32(bytes, index + 18)? as usize;
        let name_len = read_le_u16(bytes, index + 26)? as usize;
        let extra_len = read_le_u16(bytes, index + 28)? as usize;
        if flags & 0x0008 != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ZIP archive uses a data descriptor, which is not supported",
            ));
        }
        let data_start = index
            .checked_add(30)
            .and_then(|value| value.checked_add(name_len))
            .and_then(|value| value.checked_add(extra_len))
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "ZIP header is too large")
            })?;
        let data_end = data_start.checked_add(compressed_size).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "ZIP entry is too large")
        })?;
        if data_end > bytes.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "ZIP entry is truncated",
            ));
        }
        let name_start = index + 30;
        let name_end = name_start + name_len;
        let name = std::str::from_utf8(&bytes[name_start..name_end]).unwrap_or_default();
        index = data_end;
        if name.ends_with('/') {
            continue;
        }
        let data = &bytes[data_start..data_end];
        return match method {
            0 => Ok(data.to_vec()),
            8 => {
                let mut decoded = Vec::new();
                DeflateDecoder::new(data).read_to_end(&mut decoded)?;
                Ok(decoded)
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("ZIP compression method {method} is not supported"),
            )),
        };
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "ZIP archive does not contain a readable file entry",
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn read_le_u16(bytes: &[u8], offset: usize) -> std::io::Result<u16> {
    let slice = bytes.get(offset..offset + 2).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "ZIP header is truncated")
    })?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

#[cfg(not(target_arch = "wasm32"))]
fn read_le_u32(bytes: &[u8], offset: usize) -> std::io::Result<u32> {
    let slice = bytes.get(offset..offset + 4).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "ZIP header is truncated")
    })?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
