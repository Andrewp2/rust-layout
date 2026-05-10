use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MaterialId(pub String);

impl MaterialId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MaterialId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessMaterial {
    pub id: MaterialId,
    pub name: String,
    pub color_rgb: [u8; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaskOpening {
    pub start_um: f32,
    pub end_um: f32,
}

impl MaskOpening {
    pub fn new(start_um: f32, end_um: f32) -> Self {
        Self {
            start_um: start_um.min(end_um),
            end_um: start_um.max(end_um),
        }
    }

    pub fn contains(&self, x_um: f32) -> bool {
        x_um >= self.start_um && x_um <= self.end_um
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ProcessStepKind {
    Deposit {
        material: MaterialId,
        thickness_um: f32,
    },
    Etch {
        material: MaterialId,
        depth_um: f32,
    },
    Pattern {
        openings: Vec<MaskOpening>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessStep {
    pub name: String,
    pub detail: String,
    pub kind: ProcessStepKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionSegment {
    pub material: MaterialId,
    pub x0_um: f32,
    pub x1_um: f32,
    pub y0_um: f32,
    pub y1_um: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionSnapshot {
    pub step_index: usize,
    pub title: String,
    pub detail: String,
    pub active_mask: Vec<MaskOpening>,
    pub segments: Vec<CrossSectionSegment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionProcess {
    pub width_um: f32,
    pub columns: usize,
    pub substrate_material: MaterialId,
    pub substrate_thickness_um: f32,
    pub materials: Vec<ProcessMaterial>,
    pub steps: Vec<ProcessStep>,
}

impl Default for CrossSectionProcess {
    fn default() -> Self {
        Self::blank()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossSectionValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossSectionValidationFinding {
    pub severity: CrossSectionValidationSeverity,
    pub message: String,
}

impl CrossSectionValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: CrossSectionValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: CrossSectionValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

impl CrossSectionProcess {
    pub fn blank() -> Self {
        let substrate = MaterialId::from("substrate");
        Self {
            width_um: 10.0,
            columns: 80,
            substrate_material: substrate.clone(),
            substrate_thickness_um: 1.0,
            materials: vec![ProcessMaterial {
                id: substrate,
                name: "Substrate".to_string(),
                color_rgb: [96, 112, 128],
            }],
            steps: Vec::new(),
        }
    }

    pub fn validate(&self) -> Vec<CrossSectionValidationFinding> {
        let mut findings = Vec::new();
        if !self.width_um.is_finite() || self.width_um <= 0.0 {
            findings.push(CrossSectionValidationFinding::error(format!(
                "cross-section width must be positive and finite, got {}",
                self.width_um
            )));
        }
        if self.columns == 0 {
            findings.push(CrossSectionValidationFinding::error(
                "cross-section column count must be greater than 0",
            ));
        }
        if !self.substrate_thickness_um.is_finite() || self.substrate_thickness_um < 0.0 {
            findings.push(CrossSectionValidationFinding::error(format!(
                "substrate thickness must be non-negative and finite, got {}",
                self.substrate_thickness_um
            )));
        } else if self.substrate_thickness_um == 0.0 {
            findings.push(CrossSectionValidationFinding::warning(
                "substrate thickness is zero",
            ));
        }

        let material_ids = validate_materials(self, &mut findings);
        if !material_ids.contains(&self.substrate_material) {
            findings.push(CrossSectionValidationFinding::error(format!(
                "substrate material {:?} is not in the material catalog",
                self.substrate_material
            )));
        }
        if self.steps.is_empty() {
            findings.push(CrossSectionValidationFinding::warning(
                "cross-section process has no process steps",
            ));
        }
        for (index, step) in self.steps.iter().enumerate() {
            validate_step(step, index, self.width_um, &material_ids, &mut findings);
        }

        findings
    }

    pub fn simulate(&self) -> Vec<CrossSectionSnapshot> {
        let columns = self.columns.max(1);
        let width_um = self.width_um.max(0.1);
        let column_width = width_um / columns as f32;
        let mut stacks = vec![
            vec![ColumnLayer {
                material: self.substrate_material.clone(),
                thickness_um: self.substrate_thickness_um.max(0.0),
            }];
            columns
        ];
        let mut active_mask = vec![MaskOpening::new(0.0, width_um)];
        let mut snapshots = vec![CrossSectionSnapshot {
            step_index: 0,
            title: "Starting substrate".to_string(),
            detail: "Base wafer cross-section before module processing".to_string(),
            active_mask: active_mask.clone(),
            segments: segments_from_columns(&stacks, column_width),
        }];

        for (step_offset, step) in self.steps.iter().enumerate() {
            match &step.kind {
                ProcessStepKind::Deposit {
                    material,
                    thickness_um,
                } => {
                    for column in &mut stacks {
                        push_layer(column, material.clone(), thickness_um.max(0.0));
                    }
                }
                ProcessStepKind::Etch { material, depth_um } => {
                    for (index, column) in stacks.iter_mut().enumerate() {
                        let center = (index as f32 + 0.5) * column_width;
                        if active_mask.iter().any(|opening| opening.contains(center)) {
                            etch_material(column, material, depth_um.max(0.0));
                        }
                    }
                }
                ProcessStepKind::Pattern { openings } => {
                    active_mask = openings.clone();
                }
            }

            snapshots.push(CrossSectionSnapshot {
                step_index: step_offset + 1,
                title: step.name.clone(),
                detail: step.detail.clone(),
                active_mask: active_mask.clone(),
                segments: segments_from_columns(&stacks, column_width),
            });
        }

        snapshots
    }

    pub fn material(&self, id: &MaterialId) -> Option<&ProcessMaterial> {
        self.materials.iter().find(|material| material.id == *id)
    }

    pub fn sample_sequence() -> Self {
        let silicon = MaterialId::from("si");
        let oxide = MaterialId::from("sio2");
        let poly = MaterialId::from("poly");
        let metal = MaterialId::from("al");
        Self {
            width_um: 10.0,
            columns: 80,
            substrate_material: silicon.clone(),
            substrate_thickness_um: 1.4,
            materials: vec![
                ProcessMaterial {
                    id: silicon,
                    name: "Silicon".to_string(),
                    color_rgb: [96, 112, 128],
                },
                ProcessMaterial {
                    id: oxide.clone(),
                    name: "Thermal oxide".to_string(),
                    color_rgb: [94, 178, 238],
                },
                ProcessMaterial {
                    id: poly.clone(),
                    name: "Polysilicon".to_string(),
                    color_rgb: [214, 174, 82],
                },
                ProcessMaterial {
                    id: metal.clone(),
                    name: "Aluminum metal".to_string(),
                    color_rgb: [216, 224, 232],
                },
            ],
            steps: vec![
                ProcessStep {
                    name: "Grow oxide".to_string(),
                    detail: "Thermal oxide grown uniformly across the exposed silicon.".to_string(),
                    kind: ProcessStepKind::Deposit {
                        material: oxide,
                        thickness_um: 0.25,
                    },
                },
                ProcessStep {
                    name: "Deposit poly".to_string(),
                    detail: "Conformal blanket polysilicon deposition.".to_string(),
                    kind: ProcessStepKind::Deposit {
                        material: poly.clone(),
                        thickness_um: 0.35,
                    },
                },
                ProcessStep {
                    name: "Pattern gate mask".to_string(),
                    detail: "Photo mask opens field regions and protects the gate stripes."
                        .to_string(),
                    kind: ProcessStepKind::Pattern {
                        openings: vec![MaskOpening::new(0.0, 3.2), MaskOpening::new(6.8, 10.0)],
                    },
                },
                ProcessStep {
                    name: "Etch exposed poly".to_string(),
                    detail: "Directional poly etch clears unprotected field regions.".to_string(),
                    kind: ProcessStepKind::Etch {
                        material: poly,
                        depth_um: 0.45,
                    },
                },
                ProcessStep {
                    name: "Deposit metal".to_string(),
                    detail: "Blanket metal fill lands on oxide and remaining gate material."
                        .to_string(),
                    kind: ProcessStepKind::Deposit {
                        material: metal.clone(),
                        thickness_um: 0.45,
                    },
                },
                ProcessStep {
                    name: "Pattern metal mask".to_string(),
                    detail: "Mask opens the center gap between metal interconnect bars."
                        .to_string(),
                    kind: ProcessStepKind::Pattern {
                        openings: vec![MaskOpening::new(4.2, 5.8)],
                    },
                },
                ProcessStep {
                    name: "Etch metal gap".to_string(),
                    detail: "Metal etch removes the exposed center gap.".to_string(),
                    kind: ProcessStepKind::Etch {
                        material: metal,
                        depth_um: 0.55,
                    },
                },
            ],
        }
    }
}

fn validate_materials(
    process: &CrossSectionProcess,
    findings: &mut Vec<CrossSectionValidationFinding>,
) -> BTreeSet<MaterialId> {
    let mut material_ids = BTreeSet::new();
    for material in &process.materials {
        if material.id.as_str().trim().is_empty() {
            findings.push(CrossSectionValidationFinding::error(
                "material id must not be empty",
            ));
        } else if !material_ids.insert(material.id.clone()) {
            findings.push(CrossSectionValidationFinding::error(format!(
                "material id {:?} is duplicated",
                material.id
            )));
        }
        if material.name.trim().is_empty() {
            findings.push(CrossSectionValidationFinding::warning(format!(
                "material {:?} has an empty display name",
                material.id
            )));
        }
    }
    material_ids
}

fn validate_step(
    step: &ProcessStep,
    index: usize,
    process_width_um: f32,
    material_ids: &BTreeSet<MaterialId>,
    findings: &mut Vec<CrossSectionValidationFinding>,
) {
    if step.name.trim().is_empty() {
        findings.push(CrossSectionValidationFinding::warning(format!(
            "process step {index} has an empty name"
        )));
    }
    match &step.kind {
        ProcessStepKind::Deposit {
            material,
            thickness_um,
        } => {
            validate_step_material("deposit", index, material, material_ids, findings);
            validate_positive_step_value("deposit thickness", index, *thickness_um, findings);
        }
        ProcessStepKind::Etch { material, depth_um } => {
            validate_step_material("etch", index, material, material_ids, findings);
            validate_positive_step_value("etch depth", index, *depth_um, findings);
        }
        ProcessStepKind::Pattern { openings } => {
            if openings.is_empty() {
                findings.push(CrossSectionValidationFinding::warning(format!(
                    "pattern step {index} has no mask openings"
                )));
            }
            validate_mask_openings(index, process_width_um, openings, findings);
        }
    }
}

fn validate_step_material(
    kind: &str,
    index: usize,
    material: &MaterialId,
    material_ids: &BTreeSet<MaterialId>,
    findings: &mut Vec<CrossSectionValidationFinding>,
) {
    if !material_ids.contains(material) {
        findings.push(CrossSectionValidationFinding::error(format!(
            "{kind} step {index} references unknown material {:?}",
            material
        )));
    }
}

fn validate_positive_step_value(
    label: &str,
    index: usize,
    value: f32,
    findings: &mut Vec<CrossSectionValidationFinding>,
) {
    if !value.is_finite() || value < 0.0 {
        findings.push(CrossSectionValidationFinding::error(format!(
            "{label} for step {index} must be non-negative and finite, got {value}"
        )));
    } else if value == 0.0 {
        findings.push(CrossSectionValidationFinding::warning(format!(
            "{label} for step {index} is zero"
        )));
    }
}

fn validate_mask_openings(
    step_index: usize,
    process_width_um: f32,
    openings: &[MaskOpening],
    findings: &mut Vec<CrossSectionValidationFinding>,
) {
    let mut sorted = openings.to_vec();
    sorted.sort_by(|a, b| a.start_um.total_cmp(&b.start_um));
    for (index, opening) in sorted.iter().enumerate() {
        if !opening.start_um.is_finite() || !opening.end_um.is_finite() {
            findings.push(CrossSectionValidationFinding::error(format!(
                "pattern step {step_index} opening {index} has non-finite bounds"
            )));
            continue;
        }
        if opening.start_um >= opening.end_um {
            findings.push(CrossSectionValidationFinding::error(format!(
                "pattern step {step_index} opening {index} has empty or inverted bounds {}..{}",
                opening.start_um, opening.end_um
            )));
        }
        if process_width_um.is_finite()
            && process_width_um > 0.0
            && (opening.start_um < 0.0 || opening.end_um > process_width_um)
        {
            findings.push(CrossSectionValidationFinding::error(format!(
                "pattern step {step_index} opening {index} bounds {}..{} exceed process width {}",
                opening.start_um, opening.end_um, process_width_um
            )));
        }
        if let Some(previous) = index.checked_sub(1).and_then(|prev| sorted.get(prev))
            && previous.end_um > opening.start_um
        {
            findings.push(CrossSectionValidationFinding::warning(format!(
                "pattern step {step_index} opening {index} overlaps a previous opening"
            )));
        }
    }
}

#[derive(Clone, Debug)]
struct ColumnLayer {
    material: MaterialId,
    thickness_um: f32,
}

fn push_layer(column: &mut Vec<ColumnLayer>, material: MaterialId, thickness_um: f32) {
    if thickness_um <= f32::EPSILON {
        return;
    }
    if let Some(top) = column.last_mut()
        && top.material == material
    {
        top.thickness_um += thickness_um;
        return;
    }
    column.push(ColumnLayer {
        material,
        thickness_um,
    });
}

fn etch_material(column: &mut Vec<ColumnLayer>, material: &MaterialId, depth_um: f32) {
    let mut remaining = depth_um;
    while remaining > f32::EPSILON {
        let Some(index) = column.iter().rposition(|layer| layer.material == *material) else {
            return;
        };
        let layer = &mut column[index];
        let removed = layer.thickness_um.min(remaining);
        layer.thickness_um -= removed;
        remaining -= removed;
        if layer.thickness_um <= f32::EPSILON {
            column.remove(index);
        }
    }
}

fn segments_from_columns(
    stacks: &[Vec<ColumnLayer>],
    column_width: f32,
) -> Vec<CrossSectionSegment> {
    let mut raw_segments = Vec::new();
    for (column_index, column) in stacks.iter().enumerate() {
        let mut y0 = 0.0;
        for layer in column {
            let y1 = y0 + layer.thickness_um;
            if y1 > y0 {
                raw_segments.push(CrossSectionSegment {
                    material: layer.material.clone(),
                    x0_um: column_index as f32 * column_width,
                    x1_um: (column_index + 1) as f32 * column_width,
                    y0_um: y0,
                    y1_um: y1,
                });
            }
            y0 = y1;
        }
    }
    merge_adjacent_segments(raw_segments)
}

fn merge_adjacent_segments(segments: Vec<CrossSectionSegment>) -> Vec<CrossSectionSegment> {
    let mut segments = segments;
    segments.sort_by(|a, b| {
        a.y0_um
            .total_cmp(&b.y0_um)
            .then_with(|| a.y1_um.total_cmp(&b.y1_um))
            .then_with(|| a.material.cmp(&b.material))
            .then_with(|| a.x0_um.total_cmp(&b.x0_um))
    });

    let mut merged: Vec<CrossSectionSegment> = Vec::new();
    for segment in segments {
        if let Some(last) = merged.last_mut()
            && last.material == segment.material
            && nearly_equal(last.x1_um, segment.x0_um)
            && nearly_equal(last.y0_um, segment.y0_um)
            && nearly_equal(last.y1_um, segment.y1_um)
        {
            last.x1_um = segment.x1_um;
            continue;
        }
        merged.push(segment);
    }
    merged
}

fn nearly_equal(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.0001
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_cross_section_process_validates() {
        let process = CrossSectionProcess::sample_sequence();

        assert_eq!(process.validate(), Vec::new());
    }

    #[test]
    fn validation_rejects_invalid_dimensions_materials_and_masks() {
        let process = CrossSectionProcess {
            width_um: f32::NAN,
            columns: 0,
            substrate_material: MaterialId::from("missing"),
            substrate_thickness_um: -1.0,
            materials: vec![
                ProcessMaterial {
                    id: MaterialId::from(""),
                    name: "blank".to_string(),
                    color_rgb: [0, 0, 0],
                },
                ProcessMaterial {
                    id: MaterialId::from("oxide"),
                    name: String::new(),
                    color_rgb: [1, 2, 3],
                },
                ProcessMaterial {
                    id: MaterialId::from("oxide"),
                    name: "duplicate".to_string(),
                    color_rgb: [4, 5, 6],
                },
            ],
            steps: vec![
                ProcessStep {
                    name: String::new(),
                    detail: String::new(),
                    kind: ProcessStepKind::Deposit {
                        material: MaterialId::from("oxide"),
                        thickness_um: 0.0,
                    },
                },
                ProcessStep {
                    name: "Etch".to_string(),
                    detail: String::new(),
                    kind: ProcessStepKind::Etch {
                        material: MaterialId::from("poly"),
                        depth_um: f32::INFINITY,
                    },
                },
                ProcessStep {
                    name: "Pattern".to_string(),
                    detail: String::new(),
                    kind: ProcessStepKind::Pattern {
                        openings: vec![
                            MaskOpening {
                                start_um: 2.0,
                                end_um: 1.0,
                            },
                            MaskOpening {
                                start_um: 0.5,
                                end_um: 1.5,
                            },
                            MaskOpening {
                                start_um: 1.0,
                                end_um: 1.25,
                            },
                        ],
                    },
                },
            ],
        };

        let findings = process.validate();
        let messages = findings
            .iter()
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            findings
                .iter()
                .any(|finding| finding.severity == CrossSectionValidationSeverity::Error)
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.severity == CrossSectionValidationSeverity::Warning)
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("width must be positive"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("column count"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("substrate thickness"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("material id must not be empty"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("duplicated"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("substrate material"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("empty display name"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("empty name"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("deposit thickness") && message.contains("zero"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("unknown material"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("etch depth"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("empty or inverted"))
        );
        assert!(messages.iter().any(|message| message.contains("overlaps")));
    }

    #[test]
    fn blanket_deposition_adds_material_across_full_width() {
        let process = CrossSectionProcess {
            width_um: 4.0,
            columns: 4,
            substrate_material: MaterialId::from("si"),
            substrate_thickness_um: 1.0,
            materials: Vec::new(),
            steps: vec![ProcessStep {
                name: "Deposit oxide".to_string(),
                detail: String::new(),
                kind: ProcessStepKind::Deposit {
                    material: MaterialId::from("oxide"),
                    thickness_um: 0.5,
                },
            }],
        };

        let snapshots = process.simulate();
        let final_snapshot = snapshots.last().expect("final snapshot");
        assert!(final_snapshot.segments.iter().any(|segment| {
            segment.material == MaterialId::from("oxide")
                && nearly_equal(segment.x0_um, 0.0)
                && nearly_equal(segment.x1_um, 4.0)
                && nearly_equal(segment.y0_um, 1.0)
                && nearly_equal(segment.y1_um, 1.5)
        }));
    }

    #[test]
    fn patterned_etch_removes_only_open_material() {
        let process = CrossSectionProcess {
            width_um: 4.0,
            columns: 4,
            substrate_material: MaterialId::from("si"),
            substrate_thickness_um: 1.0,
            materials: Vec::new(),
            steps: vec![
                ProcessStep {
                    name: "Deposit poly".to_string(),
                    detail: String::new(),
                    kind: ProcessStepKind::Deposit {
                        material: MaterialId::from("poly"),
                        thickness_um: 0.5,
                    },
                },
                ProcessStep {
                    name: "Pattern".to_string(),
                    detail: String::new(),
                    kind: ProcessStepKind::Pattern {
                        openings: vec![MaskOpening::new(0.0, 2.0)],
                    },
                },
                ProcessStep {
                    name: "Etch poly".to_string(),
                    detail: String::new(),
                    kind: ProcessStepKind::Etch {
                        material: MaterialId::from("poly"),
                        depth_um: 0.5,
                    },
                },
            ],
        };

        let snapshots = process.simulate();
        let final_snapshot = snapshots.last().expect("final snapshot");
        let poly_segments = final_snapshot
            .segments
            .iter()
            .filter(|segment| segment.material == MaterialId::from("poly"))
            .collect::<Vec<_>>();
        assert_eq!(poly_segments.len(), 1);
        assert!(nearly_equal(poly_segments[0].x0_um, 2.0));
        assert!(nearly_equal(poly_segments[0].x1_um, 4.0));
        assert!(nearly_equal(poly_segments[0].y0_um, 1.0));
        assert!(nearly_equal(poly_segments[0].y1_um, 1.5));
    }
}
