use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ProcessLayer;

const SYNTHETIC_DIE_RADIUS: i32 = 5;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeRecord {
    pub id: String,
    pub version: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FabLot {
    pub id: String,
    pub product: String,
    pub route_id: String,
    pub recipe_id: String,
    pub wafers: Vec<WaferRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaferRecord {
    pub id: String,
    pub lot_id: String,
    pub slot: u8,
    pub substrate: String,
    pub process_history: Vec<ProcessHistoryEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessHistoryEntry {
    pub step_id: String,
    pub step_name: String,
    pub layer: Option<ProcessLayer>,
    pub recipe_id: String,
    pub recipe_version: u32,
    pub tool_id: String,
    pub tool_run_id: String,
    pub completed_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessMeasurement {
    pub measurement_id: String,
    pub lot_id: String,
    pub wafer_id: String,
    pub step_id: String,
    pub tool_run_id: String,
    pub recipe_id: String,
    pub process_layer: Option<ProcessLayer>,
    pub name: String,
    pub unit: String,
    pub value: f64,
    pub target: Option<f64>,
    pub lower_spec: Option<f64>,
    pub upper_spec: Option<f64>,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct DieAddress {
    pub column: i32,
    pub row: i32,
}

impl DieAddress {
    pub const fn new(column: i32, row: i32) -> Self {
        Self { column, row }
    }

    pub fn radius(self) -> f64 {
        ((self.column * self.column + self.row * self.row) as f64).sqrt()
    }

    pub fn radial_fraction(self, wafer_radius: f64) -> f64 {
        if wafer_radius <= f64::EPSILON {
            0.0
        } else {
            (self.radius() / wafer_radius).clamp(0.0, 1.0)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TestKind {
    Continuity,
    Leakage,
    RingOscillator,
    SramMarch,
    ParametricSweep,
}

impl TestKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Continuity => "Continuity",
            Self::Leakage => "Leakage",
            Self::RingOscillator => "Ring oscillator",
            Self::SramMarch => "SRAM march",
            Self::ParametricSweep => "Parametric sweep",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FailureMode {
    OpenCircuit,
    ShortCircuit,
    HighLeakage,
    LowFrequency,
    ParametricDrift,
    ContactResistance,
    EdgeDefect,
}

impl FailureMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::OpenCircuit => "Open circuit",
            Self::ShortCircuit => "Short circuit",
            Self::HighLeakage => "High leakage",
            Self::LowFrequency => "Low frequency",
            Self::ParametricDrift => "Parametric drift",
            Self::ContactResistance => "Contact resistance",
            Self::EdgeDefect => "Edge defect",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpatialPattern {
    NoFailures,
    Sparse,
    Uniform,
    EdgeHeavy,
    CenterHeavy,
    Clustered,
}

impl SpatialPattern {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoFailures => "no failures",
            Self::Sparse => "sparse",
            Self::Uniform => "uniform",
            Self::EdgeHeavy => "edge-heavy",
            Self::CenterHeavy => "center-heavy",
            Self::Clustered => "clustered",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessContext {
    pub route_id: String,
    pub step_id: String,
    pub recipe_id: String,
    pub tool_run_id: String,
    pub measurement_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub lot_id: String,
    pub wafer_id: String,
    pub die: DieAddress,
    pub kind: TestKind,
    pub measured_value: f64,
    pub lower_spec: Option<f64>,
    pub upper_spec: Option<f64>,
    pub passed: bool,
    pub failure_mode: Option<FailureMode>,
    pub process_context: ProcessContext,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DieOutcome {
    pub lot_id: String,
    pub wafer_id: String,
    pub die: DieAddress,
    pub passed: bool,
    pub test_count: u32,
    pub failed_tests: u32,
    pub failure_modes: Vec<FailureMode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YieldSummary {
    pub lot_id: String,
    pub wafer_id: Option<String>,
    pub test_kind: Option<TestKind>,
    pub total_dies: u32,
    pub passing_dies: u32,
    pub failing_dies: u32,
    pub yield_fraction: f64,
    pub dominant_failure: Option<FailureMode>,
    pub failure_counts: BTreeMap<FailureMode, u32>,
    pub spatial_pattern: SpatialPattern,
    pub root_cause_hints: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FailureModeDelta {
    pub mode: FailureMode,
    pub baseline_fraction: f64,
    pub candidate_fraction: f64,
    pub delta_fraction: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LotComparison {
    pub baseline_lot_id: String,
    pub candidate_lot_id: String,
    pub baseline_recipe_id: String,
    pub candidate_recipe_id: String,
    pub baseline_yield: f64,
    pub candidate_yield: f64,
    pub yield_delta: f64,
    pub failure_mode_deltas: Vec<FailureModeDelta>,
    pub root_cause_hint: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorrelationRecord {
    pub measurement_name: String,
    pub unit: String,
    pub process_layer: Option<ProcessLayer>,
    pub route_step_id: String,
    pub recipe_id: String,
    pub sample_count: usize,
    pub correlation_to_failure_rate: f64,
    pub mean_low_failure_value: f64,
    pub mean_high_failure_value: f64,
    pub likely_failure_mode: Option<FailureMode>,
    pub root_cause_hint: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YieldAnalysis {
    pub lots: Vec<FabLot>,
    pub recipes: Vec<RecipeRecord>,
    pub test_results: Vec<TestResult>,
    pub process_measurements: Vec<ProcessMeasurement>,
    pub lot_summaries: Vec<YieldSummary>,
    pub wafer_summaries: Vec<YieldSummary>,
    pub lot_comparisons: Vec<LotComparison>,
    pub correlations: Vec<CorrelationRecord>,
}

impl YieldAnalysis {
    pub fn synthetic() -> Self {
        synthetic_yield_analysis()
    }

    pub fn lot_summary(&self, lot_id: &str) -> Option<&YieldSummary> {
        self.lot_summaries
            .iter()
            .find(|summary| summary.lot_id == lot_id && summary.wafer_id.is_none())
    }

    pub fn wafer_summary(&self, lot_id: &str, wafer_id: &str) -> Option<&YieldSummary> {
        self.wafer_summaries.iter().find(|summary| {
            summary.lot_id == lot_id && summary.wafer_id.as_deref() == Some(wafer_id)
        })
    }

    pub fn wafer_summaries_for_lot(&self, lot_id: &str) -> Vec<&YieldSummary> {
        let mut summaries = self
            .wafer_summaries
            .iter()
            .filter(|summary| summary.lot_id == lot_id)
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| left.wafer_id.cmp(&right.wafer_id));
        summaries
    }

    pub fn wafer_ids_for_lot(&self, lot_id: &str) -> Vec<String> {
        self.lots
            .iter()
            .find(|lot| lot.id == lot_id)
            .map(|lot| lot.wafers.iter().map(|wafer| wafer.id.clone()).collect())
            .unwrap_or_default()
    }

    pub fn die_outcomes_for_wafer(&self, lot_id: &str, wafer_id: &str) -> Vec<DieOutcome> {
        let results = self
            .test_results
            .iter()
            .filter(|result| result.lot_id == lot_id && result.wafer_id == wafer_id)
            .cloned()
            .collect::<Vec<_>>();
        die_outcomes(&results)
    }

    pub fn measurements_for_wafer(&self, lot_id: &str, wafer_id: &str) -> Vec<&ProcessMeasurement> {
        let mut measurements = self
            .process_measurements
            .iter()
            .filter(|measurement| measurement.lot_id == lot_id && measurement.wafer_id == wafer_id)
            .collect::<Vec<_>>();
        measurements.sort_by(|left, right| left.name.cmp(&right.name));
        measurements
    }

    pub fn comparison_for_lot(&self, lot_id: &str) -> Option<&LotComparison> {
        self.lot_comparisons.iter().find(|comparison| {
            comparison.baseline_lot_id == lot_id || comparison.candidate_lot_id == lot_id
        })
    }
}

pub fn synthetic_yield_analysis() -> YieldAnalysis {
    let recipes = vec![
        RecipeRecord {
            id: "POLY_ETCH_003".to_string(),
            version: 3,
            name: "Baseline poly etch".to_string(),
        },
        RecipeRecord {
            id: "POLY_ETCH_004".to_string(),
            version: 4,
            name: "Edge-uniform poly etch".to_string(),
        },
    ];

    let mut lots = Vec::new();
    let mut test_results = Vec::new();
    let mut process_measurements = Vec::new();

    for (lot_index, (lot_id, recipe_id, recipe_version)) in [
        ("L-00042", "POLY_ETCH_003", 3),
        ("L-00043", "POLY_ETCH_004", 4),
    ]
    .into_iter()
    .enumerate()
    {
        let route_id = "INV_ROUTE_A".to_string();
        let mut wafers = Vec::new();
        for slot in 1..=4_u8 {
            let wafer_id = format!("{lot_id}-W{slot:02}");
            let run_id = format!("RUN-{lot_id}-{slot:02}");
            let measurement_prefix = format!("M-{lot_id}-{slot:02}");
            let edge_oxide = if lot_index == 0 {
                92.4 - f64::from(slot) * 0.7
            } else {
                98.8 - f64::from(slot) * 0.2
            };
            let center_oxide = if lot_index == 0 {
                100.8 - f64::from(slot) * 0.15
            } else {
                101.2 - f64::from(slot) * 0.1
            };
            let contact_resistance = if lot_index == 1 && slot == 3 {
                14.8
            } else {
                8.4 + f64::from(slot) * 0.25 + lot_index as f64 * 0.15
            };

            let process_history = vec![
                ProcessHistoryEntry {
                    step_id: "LITHO_POLY".to_string(),
                    step_name: "poly lithography".to_string(),
                    layer: Some(ProcessLayer::Poly),
                    recipe_id: "LITHO_POLY_001".to_string(),
                    recipe_version: 1,
                    tool_id: "ALIGNER-01".to_string(),
                    tool_run_id: format!("RUN-LITHO-{lot_id}-{slot:02}"),
                    completed_at: format!("2026-05-0{}T08:{:02}:00Z", lot_index + 1, slot * 7),
                },
                ProcessHistoryEntry {
                    step_id: "ETCH_POLY".to_string(),
                    step_name: "poly etch".to_string(),
                    layer: Some(ProcessLayer::Poly),
                    recipe_id: recipe_id.to_string(),
                    recipe_version,
                    tool_id: "ETCHER-02".to_string(),
                    tool_run_id: run_id.clone(),
                    completed_at: format!("2026-05-0{}T11:{:02}:00Z", lot_index + 1, slot * 9),
                },
                ProcessHistoryEntry {
                    step_id: "MET_POLY_POST".to_string(),
                    step_name: "post-poly metrology".to_string(),
                    layer: Some(ProcessLayer::Oxide),
                    recipe_id: "METROLOGY_POLY_001".to_string(),
                    recipe_version: 1,
                    tool_id: "ELLIPS-01".to_string(),
                    tool_run_id: format!("RUN-MET-{lot_id}-{slot:02}"),
                    completed_at: format!("2026-05-0{}T13:{:02}:00Z", lot_index + 1, slot * 11),
                },
            ];

            process_measurements.extend([
                ProcessMeasurement {
                    measurement_id: format!("{measurement_prefix}-EDGE-OX"),
                    lot_id: lot_id.to_string(),
                    wafer_id: wafer_id.clone(),
                    step_id: "MET_POLY_POST".to_string(),
                    tool_run_id: format!("RUN-MET-{lot_id}-{slot:02}"),
                    recipe_id: recipe_id.to_string(),
                    process_layer: Some(ProcessLayer::Oxide),
                    name: "edge_oxide_thickness".to_string(),
                    unit: "nm".to_string(),
                    value: edge_oxide,
                    target: Some(100.0),
                    lower_spec: Some(96.0),
                    upper_spec: Some(104.0),
                },
                ProcessMeasurement {
                    measurement_id: format!("{measurement_prefix}-CENTER-OX"),
                    lot_id: lot_id.to_string(),
                    wafer_id: wafer_id.clone(),
                    step_id: "MET_POLY_POST".to_string(),
                    tool_run_id: format!("RUN-MET-{lot_id}-{slot:02}"),
                    recipe_id: recipe_id.to_string(),
                    process_layer: Some(ProcessLayer::Oxide),
                    name: "center_oxide_thickness".to_string(),
                    unit: "nm".to_string(),
                    value: center_oxide,
                    target: Some(100.0),
                    lower_spec: Some(96.0),
                    upper_spec: Some(104.0),
                },
                ProcessMeasurement {
                    measurement_id: format!("{measurement_prefix}-CONTACT-R"),
                    lot_id: lot_id.to_string(),
                    wafer_id: wafer_id.clone(),
                    step_id: "MET_POLY_POST".to_string(),
                    tool_run_id: format!("RUN-MET-{lot_id}-{slot:02}"),
                    recipe_id: recipe_id.to_string(),
                    process_layer: Some(ProcessLayer::Contact),
                    name: "contact_resistance".to_string(),
                    unit: "ohm".to_string(),
                    value: contact_resistance,
                    target: Some(9.0),
                    lower_spec: Some(5.0),
                    upper_spec: Some(12.0),
                },
            ]);

            for die in synthetic_die_grid() {
                append_synthetic_die_tests(
                    &mut test_results,
                    lot_index,
                    lot_id,
                    &wafer_id,
                    slot,
                    die,
                    edge_oxide,
                    contact_resistance,
                    ProcessContext {
                        route_id: route_id.clone(),
                        step_id: "ETCH_POLY".to_string(),
                        recipe_id: recipe_id.to_string(),
                        tool_run_id: run_id.clone(),
                        measurement_ids: vec![
                            format!("{measurement_prefix}-EDGE-OX"),
                            format!("{measurement_prefix}-CENTER-OX"),
                            format!("{measurement_prefix}-CONTACT-R"),
                        ],
                    },
                );
            }

            wafers.push(WaferRecord {
                id: wafer_id,
                lot_id: lot_id.to_string(),
                slot,
                substrate: "200 mm silicon".to_string(),
                process_history,
            });
        }

        lots.push(FabLot {
            id: lot_id.to_string(),
            product: "FABOS_INV_RING".to_string(),
            route_id,
            recipe_id: recipe_id.to_string(),
            wafers,
        });
    }

    build_yield_analysis(lots, recipes, test_results, process_measurements)
}

pub fn build_yield_analysis(
    lots: Vec<FabLot>,
    recipes: Vec<RecipeRecord>,
    test_results: Vec<TestResult>,
    process_measurements: Vec<ProcessMeasurement>,
) -> YieldAnalysis {
    let lot_summaries = lots
        .iter()
        .map(|lot| summarize_yield(&test_results, Some(&lot.id), None, None))
        .collect::<Vec<_>>();
    let wafer_summaries = lots
        .iter()
        .flat_map(|lot| {
            lot.wafers
                .iter()
                .map(|wafer| summarize_yield(&test_results, Some(&lot.id), Some(&wafer.id), None))
        })
        .collect::<Vec<_>>();
    let lot_comparisons = if lots.len() >= 2 {
        vec![compare_lots(
            &lot_summaries[0],
            &lot_summaries[1],
            &lots[0].recipe_id,
            &lots[1].recipe_id,
        )]
    } else {
        Vec::new()
    };
    let correlations = correlate_yield_to_measurements(&test_results, &process_measurements);

    YieldAnalysis {
        lots,
        recipes,
        test_results,
        process_measurements,
        lot_summaries,
        wafer_summaries,
        lot_comparisons,
        correlations,
    }
}

pub fn summarize_yield(
    results: &[TestResult],
    lot_filter: Option<&str>,
    wafer_filter: Option<&str>,
    test_kind: Option<TestKind>,
) -> YieldSummary {
    let filtered = results
        .iter()
        .filter(|result| lot_filter.is_none_or(|lot| result.lot_id == lot))
        .filter(|result| wafer_filter.is_none_or(|wafer| result.wafer_id == wafer))
        .filter(|result| test_kind.is_none_or(|kind| result.kind == kind))
        .cloned()
        .collect::<Vec<_>>();
    let outcomes = die_outcomes(&filtered);
    let total_dies = outcomes.len() as u32;
    let passing_dies = outcomes.iter().filter(|outcome| outcome.passed).count() as u32;
    let failing_dies = total_dies.saturating_sub(passing_dies);
    let yield_fraction = if total_dies == 0 {
        0.0
    } else {
        passing_dies as f64 / total_dies as f64
    };
    let failure_counts = failure_mode_counts_from_outcomes(&outcomes);
    let dominant_failure = dominant_failure(&failure_counts);
    let spatial_pattern = classify_spatial_pattern(&outcomes);

    YieldSummary {
        lot_id: lot_filter.unwrap_or("all").to_string(),
        wafer_id: wafer_filter.map(str::to_string),
        test_kind,
        total_dies,
        passing_dies,
        failing_dies,
        yield_fraction,
        dominant_failure,
        failure_counts,
        spatial_pattern,
        root_cause_hints: root_cause_hints(spatial_pattern, dominant_failure),
    }
}

pub fn die_outcomes(results: &[TestResult]) -> Vec<DieOutcome> {
    #[derive(Default)]
    struct OutcomeBuilder {
        test_count: u32,
        failed_tests: u32,
        failure_modes: BTreeSet<FailureMode>,
    }

    let mut grouped: BTreeMap<(String, String, DieAddress), OutcomeBuilder> = BTreeMap::new();
    for result in results {
        let entry = grouped
            .entry((result.lot_id.clone(), result.wafer_id.clone(), result.die))
            .or_default();
        entry.test_count += 1;
        if !result.passed {
            entry.failed_tests += 1;
            if let Some(mode) = result.failure_mode {
                entry.failure_modes.insert(mode);
            }
        }
    }

    grouped
        .into_iter()
        .map(|((lot_id, wafer_id, die), builder)| DieOutcome {
            lot_id,
            wafer_id,
            die,
            passed: builder.failed_tests == 0,
            test_count: builder.test_count,
            failed_tests: builder.failed_tests,
            failure_modes: builder.failure_modes.into_iter().collect(),
        })
        .collect()
}

pub fn classify_spatial_pattern(outcomes: &[DieOutcome]) -> SpatialPattern {
    let failures = outcomes
        .iter()
        .filter(|outcome| !outcome.passed)
        .collect::<Vec<_>>();
    if failures.is_empty() {
        return SpatialPattern::NoFailures;
    }
    if failures.len() <= 2 {
        return SpatialPattern::Sparse;
    }

    let wafer_radius = outcomes
        .iter()
        .map(|outcome| outcome.die.radius())
        .fold(0.0, f64::max)
        .max(1.0);
    let total_edge = outcomes
        .iter()
        .filter(|outcome| outcome.die.radial_fraction(wafer_radius) >= 0.72)
        .count()
        .max(1);
    let total_center = outcomes
        .iter()
        .filter(|outcome| outcome.die.radial_fraction(wafer_radius) <= 0.35)
        .count()
        .max(1);
    let edge_failures = failures
        .iter()
        .filter(|outcome| outcome.die.radial_fraction(wafer_radius) >= 0.72)
        .count();
    let center_failures = failures
        .iter()
        .filter(|outcome| outcome.die.radial_fraction(wafer_radius) <= 0.35)
        .count();
    let edge_failure_share = edge_failures as f64 / failures.len() as f64;
    let edge_population_share = total_edge as f64 / outcomes.len().max(1) as f64;
    let center_failure_share = center_failures as f64 / failures.len() as f64;
    let center_population_share = total_center as f64 / outcomes.len().max(1) as f64;

    if edge_failures >= 4 && edge_failure_share > edge_population_share + 0.22 {
        return SpatialPattern::EdgeHeavy;
    }
    if center_failures >= 3 && center_failure_share > center_population_share + 0.25 {
        return SpatialPattern::CenterHeavy;
    }

    let cluster_size = largest_failure_cluster_size(outcomes);
    let cluster_threshold = ((failures.len() as f64) * 0.45).ceil().max(4.0) as usize;
    if cluster_size >= cluster_threshold {
        SpatialPattern::Clustered
    } else {
        SpatialPattern::Uniform
    }
}

pub fn largest_failure_cluster_size(outcomes: &[DieOutcome]) -> usize {
    let failures = outcomes
        .iter()
        .filter(|outcome| !outcome.passed)
        .map(|outcome| outcome.die)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut largest = 0;

    for start in &failures {
        if !seen.insert(*start) {
            continue;
        }
        let mut stack = vec![*start];
        let mut size = 0;
        while let Some(die) = stack.pop() {
            size += 1;
            for column_delta in -1..=1 {
                for row_delta in -1..=1 {
                    if column_delta == 0 && row_delta == 0 {
                        continue;
                    }
                    let neighbor = DieAddress::new(die.column + column_delta, die.row + row_delta);
                    if failures.contains(&neighbor) && seen.insert(neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }
        largest = largest.max(size);
    }

    largest
}

pub fn compare_lots(
    baseline: &YieldSummary,
    candidate: &YieldSummary,
    baseline_recipe_id: &str,
    candidate_recipe_id: &str,
) -> LotComparison {
    let modes = baseline
        .failure_counts
        .keys()
        .chain(candidate.failure_counts.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let baseline_total = baseline.total_dies.max(1) as f64;
    let candidate_total = candidate.total_dies.max(1) as f64;
    let mut failure_mode_deltas = modes
        .iter()
        .map(|mode| {
            let baseline_fraction =
                f64::from(*baseline.failure_counts.get(mode).unwrap_or(&0)) / baseline_total;
            let candidate_fraction =
                f64::from(*candidate.failure_counts.get(mode).unwrap_or(&0)) / candidate_total;
            FailureModeDelta {
                mode: *mode,
                baseline_fraction,
                candidate_fraction,
                delta_fraction: candidate_fraction - baseline_fraction,
            }
        })
        .collect::<Vec<_>>();
    failure_mode_deltas.sort_by(|left, right| {
        right
            .delta_fraction
            .abs()
            .total_cmp(&left.delta_fraction.abs())
    });

    let yield_delta = candidate.yield_fraction - baseline.yield_fraction;
    let root_cause_hint = if yield_delta > 0.05
        && failure_mode_deltas
            .iter()
            .any(|delta| delta.mode == FailureMode::LowFrequency && delta.delta_fraction < -0.02)
    {
        "Recipe change reduced low-frequency edge failures; keep watching contact resistance on individual wafers."
            .to_string()
    } else if yield_delta < -0.02 {
        "Candidate lot regressed; compare recipe and tool-run history before release.".to_string()
    } else {
        "Lots are similar; no dominant recipe signal in this MVP comparison.".to_string()
    };

    LotComparison {
        baseline_lot_id: baseline.lot_id.clone(),
        candidate_lot_id: candidate.lot_id.clone(),
        baseline_recipe_id: baseline_recipe_id.to_string(),
        candidate_recipe_id: candidate_recipe_id.to_string(),
        baseline_yield: baseline.yield_fraction,
        candidate_yield: candidate.yield_fraction,
        yield_delta,
        failure_mode_deltas,
        root_cause_hint,
    }
}

pub fn correlate_yield_to_measurements(
    results: &[TestResult],
    measurements: &[ProcessMeasurement],
) -> Vec<CorrelationRecord> {
    let outcomes = die_outcomes(results);
    let mut wafer_failure_rates: BTreeMap<(String, String), (f64, Option<FailureMode>)> =
        BTreeMap::new();
    for ((lot_id, wafer_id), wafer_outcomes) in group_outcomes_by_wafer(&outcomes) {
        let total = wafer_outcomes.len().max(1) as f64;
        let failing = wafer_outcomes
            .iter()
            .filter(|outcome| !outcome.passed)
            .count() as f64;
        let counts = failure_mode_counts_from_outcomes(&wafer_outcomes);
        wafer_failure_rates.insert(
            (lot_id, wafer_id),
            (failing / total, dominant_failure(&counts)),
        );
    }

    let mut grouped: BTreeMap<String, Vec<&ProcessMeasurement>> = BTreeMap::new();
    for measurement in measurements {
        grouped
            .entry(measurement.name.clone())
            .or_default()
            .push(measurement);
    }

    let mut records = Vec::new();
    for (name, rows) in grouped {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let mut high_values = Vec::new();
        let mut low_values = Vec::new();
        let mut mode_counts = BTreeMap::new();
        let mut joined = Vec::new();
        for measurement in rows {
            if let Some((failure_rate, mode)) =
                wafer_failure_rates.get(&(measurement.lot_id.clone(), measurement.wafer_id.clone()))
            {
                xs.push(measurement.value);
                ys.push(*failure_rate);
                joined.push((measurement, *failure_rate, *mode));
            }
        }
        if xs.len() < 3 {
            continue;
        }

        let median_failure = median(ys.clone());
        for (measurement, failure_rate, mode) in &joined {
            if *failure_rate >= median_failure {
                high_values.push(measurement.value);
                if let Some(mode) = mode {
                    *mode_counts.entry(*mode).or_insert(0_u32) += 1;
                }
            } else {
                low_values.push(measurement.value);
            }
        }

        let correlation = pearson_correlation(&xs, &ys);
        let first = joined[0].0;
        records.push(CorrelationRecord {
            measurement_name: name.clone(),
            unit: first.unit.clone(),
            process_layer: first.process_layer,
            route_step_id: first.step_id.clone(),
            recipe_id: first.recipe_id.clone(),
            sample_count: xs.len(),
            correlation_to_failure_rate: correlation,
            mean_low_failure_value: mean(&low_values),
            mean_high_failure_value: mean(&high_values),
            likely_failure_mode: dominant_failure(&mode_counts),
            root_cause_hint: correlation_hint(&name, correlation, first),
        });
    }

    records.sort_by(|left, right| {
        right
            .correlation_to_failure_rate
            .abs()
            .total_cmp(&left.correlation_to_failure_rate.abs())
    });
    records
}

fn append_synthetic_die_tests(
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

fn synthetic_die_grid() -> Vec<DieAddress> {
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

fn synthetic_hash(lot_index: usize, slot: u8, die: DieAddress) -> u32 {
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

fn group_outcomes_by_wafer(outcomes: &[DieOutcome]) -> BTreeMap<(String, String), Vec<DieOutcome>> {
    let mut grouped = BTreeMap::new();
    for outcome in outcomes {
        grouped
            .entry((outcome.lot_id.clone(), outcome.wafer_id.clone()))
            .or_insert_with(Vec::new)
            .push(outcome.clone());
    }
    grouped
}

fn failure_mode_counts_from_outcomes(outcomes: &[DieOutcome]) -> BTreeMap<FailureMode, u32> {
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

fn dominant_failure(counts: &BTreeMap<FailureMode, u32>) -> Option<FailureMode> {
    counts
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(mode, _)| *mode)
}

fn root_cause_hints(
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

fn correlation_hint(name: &str, correlation: f64, measurement: &ProcessMeasurement) -> String {
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

fn pearson_correlation(xs: &[f64], ys: &[f64]) -> f64 {
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

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn median(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_yield_summaries_rank_recipe_lots() {
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
    fn spatial_classifier_detects_edge_and_local_clusters() {
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
    fn lot_comparison_reports_yield_and_failure_mode_delta() {
        let analysis = synthetic_yield_analysis();
        let comparison = analysis.comparison_for_lot("L-00042").unwrap();

        assert_eq!(comparison.baseline_recipe_id, "POLY_ETCH_003");
        assert_eq!(comparison.candidate_recipe_id, "POLY_ETCH_004");
        assert!(comparison.yield_delta > 0.05);
        assert!(comparison.failure_mode_deltas.iter().any(|delta| {
            delta.mode == FailureMode::LowFrequency && delta.delta_fraction < -0.05
        }));
    }

    #[test]
    fn correlation_summary_links_measurements_to_failure_rate() {
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
}
