#![allow(unused_imports)]
use super::*;
use crate::*;

pub(crate) fn has_validation_error(findings: &[YieldValidationFinding], needle: &str) -> bool {
    findings.iter().any(|finding| {
        finding.severity == YieldValidationSeverity::Error && finding.message.contains(needle)
    })
}

#[test]
pub(crate) fn synthetic_yield_analysis_validates() {
    let analysis = synthetic_yield_analysis();
    let findings = analysis.validate();

    assert!(findings.is_empty(), "{findings:?}");
    assert!(analysis.is_valid());
}

#[test]
pub(crate) fn yield_validation_rejects_missing_summary_lot_and_wafer() {
    let mut analysis = synthetic_yield_analysis();
    analysis.lot_summaries[0].lot_id = "L-MISSING".to_string();
    analysis.wafer_summaries[0].wafer_id = Some("W-MISSING".to_string());

    let findings = analysis.validate();

    assert!(has_validation_error(
        &findings,
        "yield summary references missing lot L-MISSING"
    ));
    assert!(has_validation_error(
        &findings,
        "yield summary references missing wafer L-00042 / W-MISSING"
    ));
}

#[test]
pub(crate) fn yield_validation_rejects_bad_measurement_and_test_links() {
    let mut analysis = synthetic_yield_analysis();
    let duplicate_id = analysis.test_results[0].test_id.clone();
    let mut duplicate = analysis.test_results[0].clone();
    duplicate.measured_value = f64::NAN;
    duplicate.process_context.measurement_ids = vec!["M-MISSING".to_string()];
    analysis.test_results.push(duplicate);
    analysis.process_measurements[0].lower_spec = Some(10.0);
    analysis.process_measurements[0].upper_spec = Some(1.0);

    let findings = analysis.validate();

    assert!(has_validation_error(
        &findings,
        &format!("yield contains duplicate test result {duplicate_id}")
    ));
    assert!(has_validation_error(
        &findings,
        &format!("yield test result {duplicate_id} has non-finite measured value")
    ));
    assert!(has_validation_error(
        &findings,
        &format!(
            "yield test result {duplicate_id} references missing process measurement M-MISSING"
        )
    ));
    assert!(has_validation_error(
        &findings,
        "yield process measurement M-L-00042-01-EDGE-OX lower spec exceeds upper spec"
    ));
}

#[test]
pub(crate) fn yield_validation_rejects_duplicate_summaries_and_stale_comparisons() {
    let mut analysis = synthetic_yield_analysis();
    analysis
        .lot_summaries
        .push(analysis.lot_summaries[0].clone());
    let mut misplaced_lot_summary = analysis.lot_summaries[0].clone();
    misplaced_lot_summary.wafer_id = Some(analysis.lots[0].wafers[0].id.clone());
    analysis.lot_summaries.push(misplaced_lot_summary);
    let mut missing_wafer_summary = analysis.wafer_summaries[0].clone();
    missing_wafer_summary.wafer_id = None;
    analysis.wafer_summaries.push(missing_wafer_summary);

    analysis
        .lot_comparisons
        .push(analysis.lot_comparisons[0].clone());
    let mut stale_comparison = analysis.lot_comparisons[0].clone();
    stale_comparison.candidate_lot_id = stale_comparison.baseline_lot_id.clone();
    stale_comparison.yield_delta = 1.0;
    stale_comparison.failure_mode_deltas[0].baseline_fraction = -0.1;
    stale_comparison.failure_mode_deltas[0].delta_fraction = 10.0;
    stale_comparison
        .failure_mode_deltas
        .push(stale_comparison.failure_mode_deltas[0].clone());
    analysis.lot_comparisons.push(stale_comparison);

    let findings = analysis.validate();

    for needle in [
        "duplicate lot summary L-00042",
        "lot summary L-00042 unexpectedly references wafer",
        "wafer summary for lot L-00042 is missing wafer id",
        "duplicate lot comparison L-00042 vs L-00043",
        "uses lot L-00042 as both baseline and candidate",
        "delta does not match candidate-baseline yield",
        "repeats failure mode delta",
        "has invalid baseline fraction",
        "delta does not match candidate-baseline fraction",
    ] {
        assert!(
            has_validation_error(&findings, needle),
            "missing {needle}: {findings:?}"
        );
    }
}

#[test]
pub(crate) fn synthetic_yield_summaries_rank_recipe_lots() {
    let analysis = synthetic_yield_analysis();
    let baseline = analysis.lot_summary("L-00042").unwrap();
    let improved = analysis.lot_summary("L-00043").unwrap();

    assert_eq!(analysis.lots.len(), 2);
    assert!(baseline.total_dies >= 300);
    assert!(improved.yield_fraction > baseline.yield_fraction);
    assert_eq!(baseline.spatial_pattern, SpatialPattern::EdgeHeavy);
    assert_eq!(baseline.dominant_failure, Some(FailureMode::LowFrequency));
}

#[test]
pub(crate) fn spatial_classifier_detects_edge_and_local_clusters() {
    let edge_outcomes = synthetic_die_grid()
        .into_iter()
        .map(|die| {
            let edge = die.radial_fraction(f64::from(SYNTHETIC_DIE_RADIUS)) >= 0.78;
            DieOutcome {
                lot_id: "L".to_string(),
                wafer_id: "W".to_string(),
                die,
                passed: !edge,
                test_count: 1,
                failed_tests: u32::from(edge),
                failure_modes: edge
                    .then_some(FailureMode::LowFrequency)
                    .into_iter()
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    assert_eq!(
        classify_spatial_pattern(&edge_outcomes),
        SpatialPattern::EdgeHeavy
    );

    let cluster_outcomes = synthetic_die_grid()
        .into_iter()
        .map(|die| {
            let clustered = (2..=3).contains(&die.column) && (-1..=1).contains(&die.row);
            DieOutcome {
                lot_id: "L".to_string(),
                wafer_id: "W".to_string(),
                die,
                passed: !clustered,
                test_count: 1,
                failed_tests: u32::from(clustered),
                failure_modes: clustered
                    .then_some(FailureMode::ContactResistance)
                    .into_iter()
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    assert_eq!(largest_failure_cluster_size(&cluster_outcomes), 6);
    assert_eq!(
        classify_spatial_pattern(&cluster_outcomes),
        SpatialPattern::Clustered
    );
}

#[test]
pub(crate) fn lot_comparison_reports_yield_and_failure_mode_delta() {
    let analysis = synthetic_yield_analysis();
    let comparison = analysis.comparison_for_lot("L-00042").unwrap();

    assert_eq!(comparison.baseline_recipe_id, "POLY_ETCH_003");
    assert_eq!(comparison.candidate_recipe_id, "POLY_ETCH_004");
    assert!(comparison.yield_delta > 0.05);
    assert!(
        comparison.failure_mode_deltas.iter().any(|delta| {
            delta.mode == FailureMode::LowFrequency && delta.delta_fraction < -0.05
        })
    );
}

#[test]
pub(crate) fn correlation_summary_links_measurements_to_failure_rate() {
    let analysis = synthetic_yield_analysis();
    let edge_oxide = analysis
        .correlations
        .iter()
        .find(|record| record.measurement_name == "edge_oxide_thickness")
        .unwrap();

    assert!(edge_oxide.sample_count >= 8);
    assert!(edge_oxide.correlation_to_failure_rate < -0.7);
    assert_eq!(edge_oxide.process_layer, Some(ProcessLayer::Oxide));
    assert!(
        edge_oxide
            .root_cause_hint
            .contains("Lower edge oxide thickness")
    );
}
