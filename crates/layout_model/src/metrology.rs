use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct DieCoord {
    pub column: i32,
    pub row: i32,
}

impl DieCoord {
    pub const fn new(column: i32, row: i32) -> Self {
        Self { column, row }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MeasurementKind {
    ThicknessNm,
    SheetResistanceOhmsPerSq,
    CriticalDimensionNm,
    DefectCount,
    PassFail,
}

impl MeasurementKind {
    pub const ALL: [Self; 5] = [
        Self::ThicknessNm,
        Self::SheetResistanceOhmsPerSq,
        Self::CriticalDimensionNm,
        Self::DefectCount,
        Self::PassFail,
    ];

    pub const NUMERIC: [Self; 4] = [
        Self::ThicknessNm,
        Self::SheetResistanceOhmsPerSq,
        Self::CriticalDimensionNm,
        Self::DefectCount,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ThicknessNm => "Thickness",
            Self::SheetResistanceOhmsPerSq => "Sheet R",
            Self::CriticalDimensionNm => "CD",
            Self::DefectCount => "Defects",
            Self::PassFail => "Pass/Fail",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Self::ThicknessNm | Self::CriticalDimensionNm => "nm",
            Self::SheetResistanceOhmsPerSq => "ohm/sq",
            Self::DefectCount => "count",
            Self::PassFail => "",
        }
    }

    pub fn spec(self) -> MeasurementSpec {
        match self {
            Self::ThicknessNm => MeasurementSpec {
                target: Some(1000.0),
                lower: Some(960.0),
                upper: Some(1045.0),
                outlier_sigma: 2.75,
            },
            Self::SheetResistanceOhmsPerSq => MeasurementSpec {
                target: Some(52.0),
                lower: Some(47.0),
                upper: Some(58.0),
                outlier_sigma: 2.75,
            },
            Self::CriticalDimensionNm => MeasurementSpec {
                target: Some(72.0),
                lower: Some(68.0),
                upper: Some(76.0),
                outlier_sigma: 2.75,
            },
            Self::DefectCount => MeasurementSpec {
                target: Some(0.0),
                lower: Some(0.0),
                upper: Some(4.0),
                outlier_sigma: 3.0,
            },
            Self::PassFail => MeasurementSpec {
                target: Some(1.0),
                lower: Some(0.5),
                upper: None,
                outlier_sigma: f64::INFINITY,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeasurementSpec {
    pub target: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub outlier_sigma: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementStatus {
    Pass,
    Fail,
    Outlier,
}

impl MeasurementStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Outlier => "outlier",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FabObjectLinks {
    pub lot_id: String,
    pub wafer_id: String,
    pub process_step_id: String,
    pub recipe_id: String,
    pub tool_run_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub id: String,
    pub die: DieCoord,
    pub kind: MeasurementKind,
    pub value: f64,
    pub status: MeasurementStatus,
    pub links: FabObjectLinks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefectClass {
    Particle,
    Scratch,
    PatternBridge,
    MissingFeature,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Defect {
    pub id: String,
    pub die: DieCoord,
    pub class: DefectClass,
    pub position_mm: [f64; 2],
    pub size_um: f64,
    pub severity: u8,
    pub linked_measurement_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationKind {
    Review,
    ToolMark,
    EdgeExclusion,
    ProcessExcursion,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InspectionAnnotation {
    pub id: String,
    pub die: Option<DieCoord>,
    pub kind: AnnotationKind,
    pub position_mm: [f64; 2],
    pub note: String,
    pub author: String,
    pub linked_measurement_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaferGeometry {
    pub diameter_mm: f64,
    pub edge_exclusion_mm: f64,
    pub die_pitch_mm: [f64; 2],
    pub die_size_mm: [f64; 2],
}

impl WaferGeometry {
    pub fn active_radius_mm(self) -> f64 {
        let raw_radius = self.diameter_mm * 0.5 - self.edge_exclusion_mm;
        let radius = raw_radius.max(0.0);
        if radius != raw_radius {
            warn!(
                diameter_mm = self.diameter_mm,
                edge_exclusion_mm = self.edge_exclusion_mm,
                raw_radius,
                "wafer active radius was negative; clamping to zero"
            );
        }
        radius
    }

    pub fn die_center_mm(self, die: DieCoord) -> [f64; 2] {
        [
            die.column as f64 * self.die_pitch_mm[0],
            die.row as f64 * self.die_pitch_mm[1],
        ]
    }

    pub fn contains_die_center(self, die: DieCoord) -> bool {
        let [x, y] = self.die_center_mm(die);
        x.hypot(y) <= self.active_radius_mm() + f64::EPSILON
    }

    pub fn die_coords(self) -> Vec<DieCoord> {
        let radius = self.active_radius_mm();
        if radius <= 0.0 || self.die_pitch_mm[0] <= 0.0 || self.die_pitch_mm[1] <= 0.0 {
            return Vec::new();
        }
        let max_column = (radius / self.die_pitch_mm[0]).floor() as i32;
        let max_row = (radius / self.die_pitch_mm[1]).floor() as i32;
        let mut dies = Vec::new();
        for row in -max_row..=max_row {
            for column in -max_column..=max_column {
                let die = DieCoord::new(column, row);
                if self.contains_die_center(die) {
                    dies.push(die);
                }
            }
        }
        dies
    }
}

impl Default for WaferGeometry {
    fn default() -> Self {
        Self {
            diameter_mm: 200.0,
            edge_exclusion_mm: 3.0,
            die_pitch_mm: [8.4, 7.6],
            die_size_mm: [7.6, 6.8],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaferMap {
    pub id: String,
    pub name: String,
    pub geometry: WaferGeometry,
    pub links: FabObjectLinks,
    pub dies: Vec<DieCoord>,
    pub measurements: Vec<Measurement>,
    pub defects: Vec<Defect>,
    pub annotations: Vec<InspectionAnnotation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeasurementSummary {
    pub kind: MeasurementKind,
    pub sample_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub outlier_count: usize,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub stddev: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistogramBin {
    pub lower: f64,
    pub upper: f64,
    pub count: usize,
}

impl WaferMap {
    pub fn synthetic_demo() -> Self {
        Self::synthetic(
            "wm-demo-poly-metrology",
            "L-00042 W07 poly etch metrology",
            WaferGeometry::default(),
            FabObjectLinks {
                lot_id: "L-00042".to_string(),
                wafer_id: "W07".to_string(),
                process_step_id: "STEP-POLY-ETCH-030".to_string(),
                recipe_id: "POLY_ETCH_003:v3".to_string(),
                tool_run_id: "ETCH-02-RUN-2026-05-06T09:20".to_string(),
            },
        )
    }

    pub fn synthetic(
        id: impl Into<String>,
        name: impl Into<String>,
        geometry: WaferGeometry,
        links: FabObjectLinks,
    ) -> Self {
        let dies = geometry.die_coords();
        let mut measurements = Vec::with_capacity(dies.len() * MeasurementKind::ALL.len());
        let mut defects = Vec::new();

        for &die in &dies {
            let [x, y] = geometry.die_center_mm(die);
            let active_radius = geometry.active_radius_mm();
            let radial_denominator = active_radius.max(1.0);
            if radial_denominator != active_radius {
                warn!(
                    active_radius,
                    "metrology active radius below one millimeter; using denominator 1"
                );
            }
            let radial = x.hypot(y) / radial_denominator;
            let thickness = 998.0 + 18.0 * radial * radial + 5.5 * (x * 0.11).sin()
                - 3.0 * (y * 0.09).cos()
                + coordinate_noise(die, 11) * 6.5
                + if die == DieCoord::new(-3, 4) {
                    58.0
                } else if die == DieCoord::new(2, 3) {
                    35.0
                } else {
                    0.0
                };
            let sheet_r = 51.5
                + 3.4 * radial
                + 1.4 * (x * 0.08).cos()
                + coordinate_noise(die, 17) * 0.85
                + if die == DieCoord::new(5, -2) {
                    -6.6
                } else {
                    0.0
                };
            let cd = 72.4 - 2.0 * radial
                + 1.1 * (y * 0.13).sin()
                + coordinate_noise(die, 23) * 1.0
                + if die == DieCoord::new(-7, -1) {
                    4.4
                } else {
                    0.0
                };
            let defect_count = synthetic_defect_count(die, radial);

            for (kind, value) in [
                (MeasurementKind::ThicknessNm, thickness),
                (MeasurementKind::SheetResistanceOhmsPerSq, sheet_r),
                (MeasurementKind::CriticalDimensionNm, cd),
                (MeasurementKind::DefectCount, defect_count as f64),
            ] {
                let status = classify_value(kind, value, None);
                let measurement_id = measurement_id(&links, die, kind);
                measurements.push(Measurement {
                    id: measurement_id.clone(),
                    die,
                    kind,
                    value,
                    status,
                    links: links.clone(),
                });

                if kind == MeasurementKind::DefectCount && defect_count > 0 {
                    let capped = defect_count.min(4);
                    if capped != defect_count {
                        warn!(
                            die_column = die.column,
                            die_row = die.row,
                            defect_count,
                            capped_defect_count = capped,
                            "synthetic defect count exceeded per-die defect budget; truncating defects"
                        );
                    }
                    for index in 0..capped {
                        let offset_x = (coordinate_unit(die, 101 + index as u64) - 0.5)
                            * geometry.die_size_mm[0];
                        let offset_y = (coordinate_unit(die, 151 + index as u64) - 0.5)
                            * geometry.die_size_mm[1];
                        defects.push(Defect {
                            id: format!("{measurement_id}-D{index}"),
                            die,
                            class: defect_class_for(die, index),
                            position_mm: [x + offset_x, y + offset_y],
                            size_um: 0.15 + 3.0 * coordinate_unit(die, 191 + index as u64),
                            severity: if defect_count > 4 { 3 } else { 1 },
                            linked_measurement_id: Some(measurement_id.clone()),
                        });
                    }
                }
            }
        }

        let mut map = Self {
            id: id.into(),
            name: name.into(),
            geometry,
            links,
            dies,
            measurements,
            defects,
            annotations: Vec::new(),
        };
        map.reclassify_outliers();
        map.recompute_pass_fail_measurements();
        map.annotations = synthetic_annotations(&map);
        map
    }

    pub fn measurement_for(&self, die: DieCoord, kind: MeasurementKind) -> Option<&Measurement> {
        self.measurements
            .iter()
            .find(|measurement| measurement.die == die && measurement.kind == kind)
    }

    pub fn measurements_for_die(&self, die: DieCoord) -> impl Iterator<Item = &Measurement> {
        self.measurements
            .iter()
            .filter(move |measurement| measurement.die == die)
    }

    pub fn defects_for_die(&self, die: DieCoord) -> impl Iterator<Item = &Defect> {
        self.defects.iter().filter(move |defect| defect.die == die)
    }

    pub fn annotations_for_die(
        &self,
        die: DieCoord,
    ) -> impl Iterator<Item = &InspectionAnnotation> {
        self.annotations
            .iter()
            .filter(move |annotation| annotation.die == Some(die))
    }

    pub fn summary(&self, kind: MeasurementKind) -> MeasurementSummary {
        let values: Vec<_> = self
            .measurements
            .iter()
            .filter(|measurement| measurement.kind == kind)
            .map(|measurement| measurement.value)
            .collect();
        let pass_count = self
            .measurements
            .iter()
            .filter(|measurement| {
                measurement.kind == kind && measurement.status == MeasurementStatus::Pass
            })
            .count();
        let fail_count = self
            .measurements
            .iter()
            .filter(|measurement| {
                measurement.kind == kind && measurement.status == MeasurementStatus::Fail
            })
            .count();
        let outlier_count = self
            .measurements
            .iter()
            .filter(|measurement| {
                measurement.kind == kind && measurement.status == MeasurementStatus::Outlier
            })
            .count();

        if values.is_empty() {
            return MeasurementSummary {
                kind,
                sample_count: 0,
                pass_count,
                fail_count,
                outlier_count,
                min: None,
                max: None,
                mean: None,
                stddev: None,
            };
        }

        let sample_count = values.len();
        let min = values
            .iter()
            .copied()
            .min_by(|a, b| a.total_cmp(b))
            .unwrap();
        let max = values
            .iter()
            .copied()
            .max_by(|a, b| a.total_cmp(b))
            .unwrap();
        let mean = values.iter().sum::<f64>() / sample_count as f64;
        let variance = values
            .iter()
            .map(|value| {
                let delta = value - mean;
                delta * delta
            })
            .sum::<f64>()
            / sample_count as f64;

        MeasurementSummary {
            kind,
            sample_count,
            pass_count,
            fail_count,
            outlier_count,
            min: Some(min),
            max: Some(max),
            mean: Some(mean),
            stddev: Some(variance.sqrt()),
        }
    }

    pub fn histogram(&self, kind: MeasurementKind, bin_count: usize) -> Vec<HistogramBin> {
        let summary = self.summary(kind);
        let (Some(min), Some(max)) = (summary.min, summary.max) else {
            return Vec::new();
        };
        if bin_count == 0 {
            return Vec::new();
        }
        if (max - min).abs() <= f64::EPSILON {
            return vec![HistogramBin {
                lower: min,
                upper: max,
                count: summary.sample_count,
            }];
        }

        let width = (max - min) / bin_count as f64;
        let mut bins = (0..bin_count)
            .map(|index| HistogramBin {
                lower: min + width * index as f64,
                upper: min + width * (index + 1) as f64,
                count: 0,
            })
            .collect::<Vec<_>>();

        for value in self
            .measurements
            .iter()
            .filter(|measurement| measurement.kind == kind)
            .map(|measurement| measurement.value)
        {
            let mut index = ((value - min) / width).floor() as usize;
            let clamped_index = index.min(bin_count - 1);
            if clamped_index != index {
                warn!(
                    value,
                    raw_bin_index = index,
                    clamped_bin_index = clamped_index,
                    bin_count,
                    "metrology histogram bin index exceeded bin range; clamping"
                );
            }
            index = clamped_index;
            bins[index].count += 1;
        }
        bins
    }

    fn reclassify_outliers(&mut self) {
        for kind in MeasurementKind::NUMERIC {
            let summary = self.summary(kind);
            let reference = summary.mean.zip(summary.stddev);
            for measurement in self
                .measurements
                .iter_mut()
                .filter(|measurement| measurement.kind == kind)
            {
                if measurement.status != MeasurementStatus::Fail {
                    measurement.status = classify_value(kind, measurement.value, reference);
                }
            }
        }
    }

    fn recompute_pass_fail_measurements(&mut self) {
        self.measurements
            .retain(|measurement| measurement.kind != MeasurementKind::PassFail);

        for &die in &self.dies {
            let pass = self
                .measurements
                .iter()
                .filter(|measurement| measurement.die == die)
                .all(|measurement| measurement.status == MeasurementStatus::Pass);
            self.measurements.push(Measurement {
                id: measurement_id(&self.links, die, MeasurementKind::PassFail),
                die,
                kind: MeasurementKind::PassFail,
                value: if pass { 1.0 } else { 0.0 },
                status: if pass {
                    MeasurementStatus::Pass
                } else {
                    MeasurementStatus::Fail
                },
                links: self.links.clone(),
            });
        }
    }
}

pub fn classify_value(
    kind: MeasurementKind,
    value: f64,
    reference: Option<(f64, f64)>,
) -> MeasurementStatus {
    if kind == MeasurementKind::PassFail {
        return if value >= 0.5 {
            MeasurementStatus::Pass
        } else {
            MeasurementStatus::Fail
        };
    }

    let spec = kind.spec();
    if spec.lower.is_some_and(|lower| value < lower)
        || spec.upper.is_some_and(|upper| value > upper)
    {
        return MeasurementStatus::Fail;
    }

    if let Some((mean, stddev)) = reference
        && stddev > f64::EPSILON
        && (value - mean).abs() > spec.outlier_sigma * stddev
    {
        return MeasurementStatus::Outlier;
    }

    MeasurementStatus::Pass
}

fn measurement_id(links: &FabObjectLinks, die: DieCoord, kind: MeasurementKind) -> String {
    format!(
        "{}:{}:C{}R{}:{kind:?}",
        links.lot_id, links.wafer_id, die.column, die.row
    )
}

fn synthetic_defect_count(die: DieCoord, radial: f64) -> u32 {
    let mut count = 0;
    let edge_probability = if radial > 0.82 { 24 } else { 6 };
    if (stable_hash(die, 31) % 100) < edge_probability {
        count += 1;
    }
    if (stable_hash(die, 37) % 100) < (3 + (radial * 5.0) as u64) {
        count += 2;
    }
    if (die.column * 31 + die.row * 17).rem_euclid(127) == 0 {
        count += 6;
    }
    count
}

fn synthetic_annotations(map: &WaferMap) -> Vec<InspectionAnnotation> {
    let mut annotations = Vec::new();
    if let Some(measurement) = map
        .measurements
        .iter()
        .find(|measurement| measurement.status == MeasurementStatus::Outlier)
    {
        let [x, y] = map.geometry.die_center_mm(measurement.die);
        annotations.push(InspectionAnnotation {
            id: "ANN-OUTLIER-REVIEW".to_string(),
            die: Some(measurement.die),
            kind: AnnotationKind::Review,
            position_mm: [x, y],
            note: format!("Review {} outlier", measurement.kind.label()),
            author: "process-eng".to_string(),
            linked_measurement_id: Some(measurement.id.clone()),
        });
    }
    if let Some(defect) = map.defects.iter().find(|defect| defect.severity >= 3) {
        annotations.push(InspectionAnnotation {
            id: "ANN-DEFECT-CLUSTER".to_string(),
            die: Some(defect.die),
            kind: AnnotationKind::ProcessExcursion,
            position_mm: defect.position_mm,
            note: "Clustered defects; compare against tool run particles".to_string(),
            author: "yield-eng".to_string(),
            linked_measurement_id: defect.linked_measurement_id.clone(),
        });
    }
    annotations.push(InspectionAnnotation {
        id: "ANN-EDGE-EXCLUSION".to_string(),
        die: None,
        kind: AnnotationKind::EdgeExclusion,
        position_mm: [0.0, map.geometry.active_radius_mm()],
        note: format!("{} mm edge exclusion", map.geometry.edge_exclusion_mm),
        author: "metrology".to_string(),
        linked_measurement_id: None,
    });
    annotations
}

fn defect_class_for(die: DieCoord, index: u32) -> DefectClass {
    match stable_hash(die, index as u64) % 5 {
        0 => DefectClass::Particle,
        1 => DefectClass::Scratch,
        2 => DefectClass::PatternBridge,
        3 => DefectClass::MissingFeature,
        _ => DefectClass::Unknown,
    }
}

fn coordinate_noise(die: DieCoord, salt: u64) -> f64 {
    coordinate_unit(die, salt) * 2.0 - 1.0
}

fn coordinate_unit(die: DieCoord, salt: u64) -> f64 {
    let value = stable_hash(die, salt) >> 11;
    (value as f64) / ((1_u64 << 53) as f64)
}

fn stable_hash(die: DieCoord, salt: u64) -> u64 {
    let mut value = salt ^ 0x9e37_79b9_7f4a_7c15;
    value ^= (die.column as i64 as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = value.rotate_left(27);
    value ^= (die.row as i64 as u64).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wafer_grid_contains_only_active_radius_die_centers() {
        let geometry = WaferGeometry {
            diameter_mm: 100.0,
            edge_exclusion_mm: 5.0,
            die_pitch_mm: [10.0, 10.0],
            die_size_mm: [9.0, 9.0],
        };

        let dies = geometry.die_coords();

        assert!(dies.contains(&DieCoord::new(0, 0)));
        assert!(dies.contains(&DieCoord::new(4, 0)));
        assert!(!dies.contains(&DieCoord::new(5, 0)));
        assert!(dies.iter().all(|die| geometry.contains_die_center(*die)));
    }

    #[test]
    fn synthetic_summary_stats_are_deterministic_and_reasonable() {
        let map = WaferMap::synthetic_demo();
        let thickness = map.summary(MeasurementKind::ThicknessNm);
        let pass_fail = map.summary(MeasurementKind::PassFail);

        assert_eq!(thickness.sample_count, map.dies.len());
        assert_eq!(pass_fail.sample_count, map.dies.len());
        assert_eq!(thickness.fail_count, 1);
        assert!(thickness.outlier_count >= 1);
        assert!((1003.0..=1008.0).contains(&thickness.mean.unwrap()));
        assert!((7.0..=13.0).contains(&thickness.stddev.unwrap()));
        assert!(pass_fail.fail_count > 0);
    }

    #[test]
    fn classification_separates_pass_outlier_and_fail() {
        assert_eq!(
            classify_value(MeasurementKind::ThicknessNm, 1002.0, Some((1000.0, 5.0))),
            MeasurementStatus::Pass
        );
        assert_eq!(
            classify_value(MeasurementKind::ThicknessNm, 1016.0, Some((1000.0, 5.0))),
            MeasurementStatus::Outlier
        );
        assert_eq!(
            classify_value(MeasurementKind::ThicknessNm, 950.0, Some((1000.0, 5.0))),
            MeasurementStatus::Fail
        );
        assert_eq!(
            classify_value(MeasurementKind::PassFail, 0.0, None),
            MeasurementStatus::Fail
        );
    }

    #[test]
    fn histogram_counts_every_measurement_once() {
        let map = WaferMap::synthetic_demo();
        let bins = map.histogram(MeasurementKind::SheetResistanceOhmsPerSq, 12);
        let total: usize = bins.iter().map(|bin| bin.count).sum();

        assert_eq!(bins.len(), 12);
        assert_eq!(total, map.dies.len());
    }

    #[test]
    fn wafer_map_serializes_with_links_defects_and_annotations() {
        let map = WaferMap::synthetic_demo();

        let restored: WaferMap =
            serde_json::from_str(&serde_json::to_string(&map).unwrap()).unwrap();

        assert_eq!(restored.links.lot_id, "L-00042");
        assert_eq!(restored.measurements.len(), map.measurements.len());
        assert_eq!(restored.defects.len(), map.defects.len());
        assert_eq!(restored.annotations.len(), map.annotations.len());
        assert_eq!(
            restored
                .measurement_for(DieCoord::new(0, 0), MeasurementKind::PassFail)
                .unwrap()
                .links
                .tool_run_id,
            "ETCH-02-RUN-2026-05-06T09:20"
        );
    }
}
