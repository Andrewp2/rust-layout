#![allow(unused_imports)]
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_synthetic_die_tests(
    test_results: &mut Vec<TestResult>,
    lot_index: usize,
    lot_id: &str,
    wafer_id: &str,
    slot: u8,
    die: DieAddress,
    edge_oxide: f64,
    contact_resistance: f64,
    process_context: ProcessContext,
) {
    let radial = die.radial_fraction(f64::from(SYNTHETIC_DIE_RADIUS));
    let hash = synthetic_hash(lot_index, slot, die);
    let edge_die = radial >= 0.78;
    let baseline_edge_fail = lot_index == 0 && edge_die && hash % 4 != 0;
    let candidate_edge_fail = lot_index == 1 && edge_die && hash % 19 == 0;
    let random_low_frequency = hash % 97 == 0;
    let low_frequency_fail = baseline_edge_fail || candidate_edge_fail || random_low_frequency;
    let ring_value = if low_frequency_fail {
        940.0 + f64::from(hash % 17) - radial * 18.0
    } else {
        1045.0 - radial * (11.0 + (100.0 - edge_oxide).max(0.0)) + f64::from(hash % 13)
    };

    test_results.push(TestResult {
        test_id: format!("T-{lot_id}-{wafer_id}-{}-{}-RO", die.column, die.row),
        lot_id: lot_id.to_string(),
        wafer_id: wafer_id.to_string(),
        die,
        kind: TestKind::RingOscillator,
        measured_value: ring_value,
        lower_spec: Some(980.0),
        upper_spec: None,
        passed: !low_frequency_fail,
        failure_mode: low_frequency_fail.then_some(if edge_die {
            FailureMode::LowFrequency
        } else {
            FailureMode::ParametricDrift
        }),
        process_context: process_context.clone(),
    });

    let contact_cluster =
        lot_index == 1 && slot == 3 && (2..=3).contains(&die.column) && (-1..=1).contains(&die.row);
    let rare_open = hash % 131 == 0;
    let contact_fail = contact_cluster || rare_open;
    let continuity_value = if contact_fail {
        contact_resistance + 5.5 + f64::from(hash % 5)
    } else {
        contact_resistance + f64::from(hash % 7) * 0.08
    };
    test_results.push(TestResult {
        test_id: format!("T-{lot_id}-{wafer_id}-{}-{}-CONT", die.column, die.row),
        lot_id: lot_id.to_string(),
        wafer_id: wafer_id.to_string(),
        die,
        kind: TestKind::Continuity,
        measured_value: continuity_value,
        lower_spec: None,
        upper_spec: Some(12.0),
        passed: !contact_fail,
        failure_mode: contact_fail.then_some(if contact_cluster {
            FailureMode::ContactResistance
        } else {
            FailureMode::OpenCircuit
        }),
        process_context: process_context.clone(),
    });

    let leakage_fail = lot_index == 0 && edge_die && hash % 11 == 0;
    let leakage_value = if leakage_fail {
        7.0 + f64::from(hash % 9) * 0.4
    } else {
        1.6 + radial * 0.9 + f64::from(hash % 5) * 0.08
    };
    test_results.push(TestResult {
        test_id: format!("T-{lot_id}-{wafer_id}-{}-{}-LEAK", die.column, die.row),
        lot_id: lot_id.to_string(),
        wafer_id: wafer_id.to_string(),
        die,
        kind: TestKind::Leakage,
        measured_value: leakage_value,
        lower_spec: None,
        upper_spec: Some(5.0),
        passed: !leakage_fail,
        failure_mode: leakage_fail.then_some(FailureMode::HighLeakage),
        process_context,
    });
}

pub(crate) fn synthetic_die_grid() -> Vec<DieAddress> {
    let mut dies = Vec::new();
    for row in -SYNTHETIC_DIE_RADIUS..=SYNTHETIC_DIE_RADIUS {
        for column in -SYNTHETIC_DIE_RADIUS..=SYNTHETIC_DIE_RADIUS {
            let die = DieAddress::new(column, row);
            if die.radius() <= f64::from(SYNTHETIC_DIE_RADIUS) {
                dies.push(die);
            }
        }
    }
    dies
}

pub(crate) fn synthetic_hash(lot_index: usize, slot: u8, die: DieAddress) -> u32 {
    let value = (lot_index as i64 + 1) * 101
        + i64::from(slot) * 37
        + i64::from(die.column + 17) * 13
        + i64::from(die.row + 19) * 29
        + i64::from(die.column * die.row) * 7;
    (value.unsigned_abs() as u32)
        .wrapping_mul(1_103_515_245)
        .wrapping_add(12_345)
        % 997
}

pub(crate) fn group_outcomes_by_wafer(
    outcomes: &[DieOutcome],
) -> BTreeMap<(String, String), Vec<DieOutcome>> {
    let mut grouped = BTreeMap::new();
    for outcome in outcomes {
        grouped
            .entry((outcome.lot_id.clone(), outcome.wafer_id.clone()))
            .or_insert_with(Vec::new)
            .push(outcome.clone());
    }
    grouped
}

pub(crate) fn failure_mode_counts_from_outcomes(
    outcomes: &[DieOutcome],
) -> BTreeMap<FailureMode, u32> {
    let mut counts = BTreeMap::new();
    for outcome in outcomes {
        if outcome.passed {
            continue;
        }
        for mode in &outcome.failure_modes {
            *counts.entry(*mode).or_insert(0) += 1;
        }
    }
    counts
}

pub(crate) fn dominant_failure(counts: &BTreeMap<FailureMode, u32>) -> Option<FailureMode> {
    counts
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(mode, _)| *mode)
}

pub(crate) fn root_cause_hints(
    spatial_pattern: SpatialPattern,
    dominant_failure: Option<FailureMode>,
) -> Vec<String> {
    match (spatial_pattern, dominant_failure) {
        (SpatialPattern::EdgeHeavy, Some(FailureMode::LowFrequency)) => vec![
            "Edge-heavy low-frequency failures point to oxide or etch nonuniformity near the wafer edge."
                .to_string(),
        ],
        (SpatialPattern::EdgeHeavy, _) => {
            vec!["Edge-heavy failures should be checked against wafer-edge metrology.".to_string()]
        }
        (SpatialPattern::Clustered, Some(FailureMode::ContactResistance)) => vec![
            "Localized contact-resistance cluster points to a tool-run or contact module excursion."
                .to_string(),
        ],
        (SpatialPattern::Clustered, _) => {
            vec!["Localized cluster suggests a wafer handling or tool-run event.".to_string()]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn correlation_hint(
    name: &str,
    correlation: f64,
    measurement: &ProcessMeasurement,
) -> String {
    let normalized = name.replace('_', " ");
    if name.contains("edge_oxide") && correlation < -0.45 {
        format!(
            "Lower {normalized} tracks higher failure rate; inspect {} / {} history.",
            measurement.step_id, measurement.tool_run_id
        )
    } else if name.contains("contact") && correlation > 0.45 {
        format!(
            "Higher {normalized} tracks failures; review contact module and recipe {}.",
            measurement.recipe_id
        )
    } else if correlation.abs() > 0.45 {
        format!(
            "{normalized} has a strong wafer-level relationship to failure rate at step {}.",
            measurement.step_id
        )
    } else {
        format!("{normalized} has weak correlation in the synthetic MVP data.")
    }
}

pub(crate) fn pearson_correlation(xs: &[f64], ys: &[f64]) -> f64 {
    if xs.len() != ys.len() || xs.len() < 2 {
        return 0.0;
    }
    let mean_x = mean(xs);
    let mean_y = mean(ys);
    let mut numerator = 0.0;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for (x, y) in xs.iter().zip(ys) {
        let dx = x - mean_x;
        let dy = y - mean_y;
        numerator += dx * dy;
        sum_x += dx * dx;
        sum_y += dy * dy;
    }
    let denominator = (sum_x * sum_y).sqrt();
    if denominator <= f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

pub(crate) fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

pub(crate) fn median(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
