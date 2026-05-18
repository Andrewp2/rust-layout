#![allow(unused_imports)]
use super::*;

#[cfg(not(target_arch = "wasm32"))]
pub fn export_demo_gds(
    path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.as_ref();
    let document = Document::demo();
    let technology = layout_model::default_technology();
    let bytes = export_gdsii(&document, &technology)?;
    atomic_write_bytes(path, &bytes)?;
    Ok(())
}

pub fn validate_builtin_demo_workspace() -> Result<BuiltinDemoWorkspaceReport, String> {
    WorkspaceDataset::demo().builtin_demo_report()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistenceFixtureReport {
    pub migrated_legacy_schema_zero: usize,
    pub rejected_future_workspace_schema: usize,
    pub rejected_future_metadata_schema: usize,
    pub rejected_future_document_schema: usize,
    pub rejected_malformed_schema_fields: usize,
    pub rejected_malformed_metadata_arrays: usize,
    pub rejected_unsupported_feature_flags: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualityFixtureReport {
    pub drc_violations: usize,
    pub drc_rule_families: usize,
    pub connectivity_components: usize,
    pub connectivity_shorts: usize,
    pub connectivity_opens: usize,
    pub connectivity_issue_keys: usize,
    pub connectivity_issue_states: usize,
}

pub fn validate_persistence_fixtures() -> Result<PersistenceFixtureReport, String> {
    let legacy = WorkspaceDataset::from_json_str(include_str!(
        "../../../../../fixtures/persistence/workspace_legacy_schema0_blank.json"
    ))?;
    if legacy.schema_version != WORKSPACE_DATASET_SCHEMA_VERSION {
        return Err(format!(
            "legacy persistence fixture migrated to workspace schema {}, expected {}",
            legacy.schema_version, WORKSPACE_DATASET_SCHEMA_VERSION
        ));
    }
    if !legacy.validate().is_valid() {
        return Err(format!(
            "legacy persistence fixture migrated into invalid workspace: {}",
            legacy.validate().error_summary()
        ));
    }

    assert_persistence_fixture_rejected(
        "future workspace schema",
        include_str!("../../../../../fixtures/persistence/workspace_future_schema_preflight.json"),
        &["workspace schema", "unsupported"],
    )?;
    assert_persistence_fixture_rejected(
        "future metadata schema",
        include_str!(
            "../../../../../fixtures/persistence/workspace_future_metadata_preflight.json"
        ),
        &["metadata schema", "unsupported"],
    )?;
    assert_persistence_fixture_rejected(
        "future document schema",
        include_str!(
            "../../../../../fixtures/persistence/workspace_future_document_schema_preflight.json"
        ),
        &["document schema", "unsupported"],
    )?;
    let malformed_schema_field = malformed_schema_field_workspace_json()?;
    assert_persistence_fixture_rejected(
        "malformed schema field",
        &malformed_schema_field,
        &["workspace schema version", "unsigned integer"],
    )?;
    assert_persistence_fixture_rejected(
        "malformed metadata arrays",
        include_str!(
            "../../../../../fixtures/persistence/workspace_malformed_metadata_arrays_preflight.json"
        ),
        &[
            "workspace metadata feature flags",
            "entry 1",
            "must be a string",
        ],
    )?;
    assert_persistence_fixture_rejected(
        "unsupported feature flag",
        include_str!("../../../../../fixtures/persistence/workspace_unsupported_feature_flag.json"),
        &["unsupported feature flag", "future_mask_revision_model"],
    )?;

    Ok(PersistenceFixtureReport {
        migrated_legacy_schema_zero: 1,
        rejected_future_workspace_schema: 1,
        rejected_future_metadata_schema: 1,
        rejected_future_document_schema: 1,
        rejected_malformed_schema_fields: 1,
        rejected_malformed_metadata_arrays: 1,
        rejected_unsupported_feature_flags: 1,
    })
}

pub fn validate_quality_fixtures() -> Result<QualityFixtureReport, String> {
    let drc = validate_drc_quality_fixture()?;
    let connectivity = validate_connectivity_quality_fixture()?;
    Ok(QualityFixtureReport {
        drc_violations: drc.0,
        drc_rule_families: drc.1,
        connectivity_components: connectivity.component_count,
        connectivity_shorts: connectivity.short_count,
        connectivity_opens: connectivity.open_count,
        connectivity_issue_keys: connectivity.issue_key_count,
        connectivity_issue_states: connectivity.issue_state_count,
    })
}
