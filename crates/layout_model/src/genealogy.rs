use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::{
    ProcessLayer,
    mes::{
        FabMesData, Lot, LotId, ProcessRouteId, ProcessStep, ProcessStepId, RecipeId, ToolId,
        WaferId, sample_fab_data,
    },
};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

string_id!(MaterialLotId);
string_id!(ToolRunId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WaferRef {
    pub lot_id: LotId,
    pub wafer_id: WaferId,
}

impl WaferRef {
    pub fn new(lot_id: impl Into<LotId>, wafer_id: impl Into<WaferId>) -> Self {
        Self {
            lot_id: lot_id.into(),
            wafer_id: wafer_id.into(),
        }
    }
}

impl fmt::Display for WaferRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.lot_id, self.wafer_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaferGenealogyState {
    Active,
    SplitTo { lot_id: LotId },
    MergedTo { lot_id: LotId },
    Scrapped { reason: String },
}

impl WaferGenealogyState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::SplitTo { .. } => "split",
            Self::MergedTo { .. } => "merged",
            Self::Scrapped { .. } => "scrapped",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenealogyWafer {
    pub id: WaferId,
    pub slot: u8,
    pub parent: Option<WaferRef>,
    pub substrate: String,
    pub state: WaferGenealogyState,
}

impl GenealogyWafer {
    pub fn new(id: impl Into<WaferId>, slot: u8) -> Self {
        Self {
            id: id.into(),
            slot,
            parent: None,
            substrate: "Si 200 mm p-type".to_string(),
            state: WaferGenealogyState::Active,
        }
    }

    pub fn with_parent(id: impl Into<WaferId>, slot: u8, parent: WaferRef) -> Self {
        Self {
            parent: Some(parent),
            ..Self::new(id, slot)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LotDisposition {
    Active,
    Closed { reason: String },
}

impl LotDisposition {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Closed { .. } => "closed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenealogyLot {
    pub id: LotId,
    pub product: String,
    pub route_id: ProcessRouteId,
    pub created_from: Vec<LotId>,
    pub wafers: BTreeMap<WaferId, GenealogyWafer>,
    pub disposition: LotDisposition,
}

impl GenealogyLot {
    pub fn from_mes_lot(lot: &Lot) -> Self {
        let wafers = lot
            .wafers
            .iter()
            .map(|wafer| {
                let mut trace_wafer = GenealogyWafer::new(wafer.id.clone(), wafer.slot);
                if wafer.status.is_scrapped() {
                    trace_wafer.state = WaferGenealogyState::Scrapped {
                        reason: "scrapped in MES traveler".to_string(),
                    };
                }
                (trace_wafer.id.clone(), trace_wafer)
            })
            .collect();
        Self {
            id: lot.id.clone(),
            product: lot.product.clone(),
            route_id: lot.route_id.clone(),
            created_from: Vec::new(),
            wafers,
            disposition: LotDisposition::Active,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaferTransfer {
    pub source_wafer_id: WaferId,
    pub target_wafer_id: WaferId,
    pub target_slot: u8,
}

impl WaferTransfer {
    pub fn new(
        source_wafer_id: impl Into<WaferId>,
        target_wafer_id: impl Into<WaferId>,
        target_slot: u8,
    ) -> Self {
        Self {
            source_wafer_id: source_wafer_id.into(),
            target_wafer_id: target_wafer_id.into(),
            target_slot,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeWaferTransfer {
    pub source: WaferRef,
    pub target_wafer_id: WaferId,
    pub target_slot: u8,
}

impl MergeWaferTransfer {
    pub fn new(source: WaferRef, target_wafer_id: impl Into<WaferId>, target_slot: u8) -> Self {
        Self {
            source,
            target_wafer_id: target_wafer_id.into(),
            target_slot,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialLot {
    pub id: MaterialLotId,
    pub name: String,
    pub supplier: String,
    pub certificate_id: String,
    pub received_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaferProcessRecord {
    pub sequence: u64,
    pub wafer: WaferRef,
    pub step_id: ProcessStepId,
    pub step_name: String,
    pub process_layer: Option<ProcessLayer>,
    pub recipe_id: RecipeId,
    pub tool_id: ToolId,
    pub tool_run_id: ToolRunId,
    pub completed_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialUse {
    pub sequence: u64,
    pub wafer: WaferRef,
    pub material_lot_id: MaterialLotId,
    pub step_id: ProcessStepId,
    pub tool_run_id: ToolRunId,
    pub quantity: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenealogyEventKind {
    LotStarted {
        lot_id: LotId,
    },
    LotSplit {
        source_lot_id: LotId,
        target_lot_id: LotId,
        wafer_count: usize,
        reason: String,
    },
    LotMerge {
        source_lot_ids: Vec<LotId>,
        target_lot_id: LotId,
        wafer_count: usize,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenealogyEvent {
    pub sequence: u64,
    pub kind: GenealogyEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExcursionQuery {
    ToolRun { tool_run_id: ToolRunId },
    ProcessStep { step_id: ProcessStepId },
    MaterialLot { material_lot_id: MaterialLotId },
}

impl ExcursionQuery {
    pub fn label(&self) -> String {
        match self {
            Self::ToolRun { tool_run_id } => format!("tool run {tool_run_id}"),
            Self::ProcessStep { step_id } => format!("process step {step_id}"),
            Self::MaterialLot { material_lot_id } => format!("material lot {material_lot_id}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactRelationship {
    Direct,
    Descendant {
        ancestor: WaferRef,
        generations: usize,
    },
}

impl ImpactRelationship {
    pub fn label(&self) -> String {
        match self {
            Self::Direct => "direct".to_string(),
            Self::Descendant {
                ancestor,
                generations,
            } => format!("{generations} gen from {ancestor}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactedWafer {
    pub wafer: WaferRef,
    pub relationship: ImpactRelationship,
    pub latest_step_id: Option<ProcessStepId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub query: ExcursionQuery,
    pub direct_wafers: Vec<WaferRef>,
    pub impacted_wafers: Vec<ImpactedWafer>,
    pub matching_process_records: Vec<WaferProcessRecord>,
    pub matching_material_uses: Vec<MaterialUse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenealogySummary {
    pub lot_count: usize,
    pub wafer_count: usize,
    pub split_count: usize,
    pub merge_count: usize,
    pub material_lot_count: usize,
    pub process_record_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LotGenealogy {
    pub lots: BTreeMap<LotId, GenealogyLot>,
    pub material_lots: BTreeMap<MaterialLotId, MaterialLot>,
    pub process_history: Vec<WaferProcessRecord>,
    pub material_uses: Vec<MaterialUse>,
    pub events: Vec<GenealogyEvent>,
    #[serde(default = "default_next_sequence")]
    next_sequence: u64,
}

impl LotGenealogy {
    pub fn new() -> Self {
        Self {
            next_sequence: 1,
            ..Self::default()
        }
    }

    pub fn sample() -> Self {
        sample_lot_genealogy()
    }

    pub fn from_mes(data: &FabMesData) -> Self {
        let mut genealogy = Self::new();
        for lot in data.lots.values() {
            genealogy
                .register_lot(GenealogyLot::from_mes_lot(lot))
                .expect("MES sample has unique lots");
        }

        for traveler in data.travelers.values() {
            let Some(lot) = data.lots.get(&traveler.lot_id) else {
                continue;
            };
            let Some(route) = data.routes.get(&traveler.route_id) else {
                continue;
            };
            let active_wafers = lot
                .wafers
                .iter()
                .filter(|wafer| !wafer.status.is_scrapped())
                .map(|wafer| wafer.id.clone())
                .collect::<Vec<_>>();
            for (index, completed) in traveler.completed_steps.iter().enumerate() {
                let Some(step) = route.step(&completed.step_id) else {
                    continue;
                };
                let tool_run_id = ToolRunId::new(format!(
                    "{}-{}-RUN-{:03}",
                    completed.tool_id.as_str(),
                    completed.step_id.as_str(),
                    index + 1
                ));
                let completed_at = format!("traveler-seq-{:03}", index + 1);
                genealogy
                    .record_lot_process_step(
                        &traveler.lot_id,
                        &active_wafers,
                        ProcessStepTrace {
                            step_id: completed.step_id.clone(),
                            step_name: step.name.clone(),
                            process_layer: process_layer_for_step(step),
                            recipe_id: completed.recipe_id.clone(),
                            tool_id: completed.tool_id.clone(),
                            tool_run_id,
                            completed_at,
                        },
                    )
                    .expect("registered MES lots can accept completed traveler history");
            }
        }
        genealogy
    }

    pub fn summary(&self) -> GenealogySummary {
        GenealogySummary {
            lot_count: self.lots.len(),
            wafer_count: self
                .lots
                .values()
                .map(|lot| lot.wafers.len())
                .sum::<usize>(),
            split_count: self
                .events
                .iter()
                .filter(|event| matches!(event.kind, GenealogyEventKind::LotSplit { .. }))
                .count(),
            merge_count: self
                .events
                .iter()
                .filter(|event| matches!(event.kind, GenealogyEventKind::LotMerge { .. }))
                .count(),
            material_lot_count: self.material_lots.len(),
            process_record_count: self.process_history.len(),
        }
    }

    pub fn register_lot(&mut self, lot: GenealogyLot) -> Result<(), GenealogyError> {
        if self.lots.contains_key(&lot.id) {
            return Err(GenealogyError::DuplicateLot(lot.id));
        }
        let lot_id = lot.id.clone();
        self.lots.insert(lot_id.clone(), lot);
        self.push_event(GenealogyEventKind::LotStarted { lot_id });
        Ok(())
    }

    pub fn register_material_lot(&mut self, material: MaterialLot) -> Result<(), GenealogyError> {
        if self.material_lots.contains_key(&material.id) {
            return Err(GenealogyError::DuplicateMaterialLot(material.id));
        }
        self.material_lots.insert(material.id.clone(), material);
        Ok(())
    }

    pub fn split_lot(
        &mut self,
        source_lot_id: &LotId,
        target_lot_id: LotId,
        product: String,
        route_id: ProcessRouteId,
        transfers: Vec<WaferTransfer>,
        reason: String,
    ) -> Result<(), GenealogyError> {
        if transfers.is_empty() {
            return Err(GenealogyError::EmptyOperation("split"));
        }
        if self.lots.contains_key(&target_lot_id) {
            return Err(GenealogyError::DuplicateLot(target_lot_id));
        }
        self.require_lot(source_lot_id)?;

        let mut target_wafers = BTreeMap::new();
        let mut target_ids = BTreeSet::new();
        for transfer in &transfers {
            let source = WaferRef {
                lot_id: source_lot_id.clone(),
                wafer_id: transfer.source_wafer_id.clone(),
            };
            self.require_wafer(&source)?;
            if !target_ids.insert(transfer.target_wafer_id.clone()) {
                return Err(GenealogyError::DuplicateWaferInOperation(
                    transfer.target_wafer_id.clone(),
                ));
            }
            let wafer = GenealogyWafer::with_parent(
                transfer.target_wafer_id.clone(),
                transfer.target_slot,
                source,
            );
            target_wafers.insert(wafer.id.clone(), wafer);
        }

        {
            let source_lot = self
                .lots
                .get_mut(source_lot_id)
                .ok_or_else(|| GenealogyError::LotNotFound(source_lot_id.clone()))?;
            for transfer in &transfers {
                let source_wafer = source_lot
                    .wafers
                    .get_mut(&transfer.source_wafer_id)
                    .ok_or_else(|| {
                        GenealogyError::WaferNotFound(WaferRef {
                            lot_id: source_lot_id.clone(),
                            wafer_id: transfer.source_wafer_id.clone(),
                        })
                    })?;
                source_wafer.state = WaferGenealogyState::SplitTo {
                    lot_id: target_lot_id.clone(),
                };
            }
        }

        let wafer_count = target_wafers.len();
        self.lots.insert(
            target_lot_id.clone(),
            GenealogyLot {
                id: target_lot_id.clone(),
                product,
                route_id,
                created_from: vec![source_lot_id.clone()],
                wafers: target_wafers,
                disposition: LotDisposition::Active,
            },
        );
        self.push_event(GenealogyEventKind::LotSplit {
            source_lot_id: source_lot_id.clone(),
            target_lot_id,
            wafer_count,
            reason,
        });
        Ok(())
    }

    pub fn merge_lots(
        &mut self,
        source_lot_ids: Vec<LotId>,
        target_lot_id: LotId,
        product: String,
        route_id: ProcessRouteId,
        transfers: Vec<MergeWaferTransfer>,
        reason: String,
    ) -> Result<(), GenealogyError> {
        if source_lot_ids.is_empty() || transfers.is_empty() {
            return Err(GenealogyError::EmptyOperation("merge"));
        }
        if self.lots.contains_key(&target_lot_id) {
            return Err(GenealogyError::DuplicateLot(target_lot_id));
        }
        for source_lot_id in &source_lot_ids {
            self.require_lot(source_lot_id)?;
        }

        let mut target_wafers = BTreeMap::new();
        let mut target_ids = BTreeSet::new();
        for transfer in &transfers {
            self.require_wafer(&transfer.source)?;
            if !source_lot_ids.contains(&transfer.source.lot_id) {
                return Err(GenealogyError::LotNotInMergeSources {
                    source_lot_id: transfer.source.lot_id.clone(),
                    target_lot_id: target_lot_id.clone(),
                });
            }
            if !target_ids.insert(transfer.target_wafer_id.clone()) {
                return Err(GenealogyError::DuplicateWaferInOperation(
                    transfer.target_wafer_id.clone(),
                ));
            }
            let wafer = GenealogyWafer::with_parent(
                transfer.target_wafer_id.clone(),
                transfer.target_slot,
                transfer.source.clone(),
            );
            target_wafers.insert(wafer.id.clone(), wafer);
        }

        for transfer in &transfers {
            let source_lot = self
                .lots
                .get_mut(&transfer.source.lot_id)
                .ok_or_else(|| GenealogyError::LotNotFound(transfer.source.lot_id.clone()))?;
            let source_wafer = source_lot
                .wafers
                .get_mut(&transfer.source.wafer_id)
                .ok_or_else(|| GenealogyError::WaferNotFound(transfer.source.clone()))?;
            source_wafer.state = WaferGenealogyState::MergedTo {
                lot_id: target_lot_id.clone(),
            };
        }

        let wafer_count = target_wafers.len();
        self.lots.insert(
            target_lot_id.clone(),
            GenealogyLot {
                id: target_lot_id.clone(),
                product,
                route_id,
                created_from: source_lot_ids.clone(),
                wafers: target_wafers,
                disposition: LotDisposition::Active,
            },
        );
        self.push_event(GenealogyEventKind::LotMerge {
            source_lot_ids,
            target_lot_id,
            wafer_count,
            reason,
        });
        Ok(())
    }

    pub fn record_process_step(
        &mut self,
        mut record: WaferProcessRecord,
    ) -> Result<(), GenealogyError> {
        self.require_wafer(&record.wafer)?;
        record.sequence = self.next_sequence();
        self.process_history.push(record);
        Ok(())
    }

    pub fn record_lot_process_step(
        &mut self,
        lot_id: &LotId,
        wafer_ids: &[WaferId],
        trace: ProcessStepTrace,
    ) -> Result<usize, GenealogyError> {
        if wafer_ids.is_empty() {
            return Err(GenealogyError::EmptyOperation("record process step"));
        }
        for wafer_id in wafer_ids {
            self.record_process_step(WaferProcessRecord {
                sequence: 0,
                wafer: WaferRef {
                    lot_id: lot_id.clone(),
                    wafer_id: wafer_id.clone(),
                },
                step_id: trace.step_id.clone(),
                step_name: trace.step_name.clone(),
                process_layer: trace.process_layer,
                recipe_id: trace.recipe_id.clone(),
                tool_id: trace.tool_id.clone(),
                tool_run_id: trace.tool_run_id.clone(),
                completed_at: trace.completed_at.clone(),
            })?;
        }
        Ok(wafer_ids.len())
    }

    pub fn record_material_use(
        &mut self,
        mut use_record: MaterialUse,
    ) -> Result<(), GenealogyError> {
        self.require_wafer(&use_record.wafer)?;
        if !self.material_lots.contains_key(&use_record.material_lot_id) {
            return Err(GenealogyError::MaterialLotNotFound(
                use_record.material_lot_id,
            ));
        }
        use_record.sequence = self.next_sequence();
        self.material_uses.push(use_record);
        Ok(())
    }

    pub fn record_lot_material_use(
        &mut self,
        lot_id: &LotId,
        wafer_ids: &[WaferId],
        trace: MaterialUseTrace,
    ) -> Result<usize, GenealogyError> {
        if wafer_ids.is_empty() {
            return Err(GenealogyError::EmptyOperation("record material use"));
        }
        for wafer_id in wafer_ids {
            self.record_material_use(MaterialUse {
                sequence: 0,
                wafer: WaferRef {
                    lot_id: lot_id.clone(),
                    wafer_id: wafer_id.clone(),
                },
                material_lot_id: trace.material_lot_id.clone(),
                step_id: trace.step_id.clone(),
                tool_run_id: trace.tool_run_id.clone(),
                quantity: trace.quantity,
                unit: trace.unit.clone(),
            })?;
        }
        Ok(wafer_ids.len())
    }

    pub fn lot_ids(&self) -> Vec<LotId> {
        self.lots.keys().cloned().collect()
    }

    pub fn wafer_refs_for_lot(&self, lot_id: &LotId) -> Vec<WaferRef> {
        self.lots
            .get(lot_id)
            .map(|lot| {
                lot.wafers
                    .keys()
                    .cloned()
                    .map(|wafer_id| WaferRef {
                        lot_id: lot_id.clone(),
                        wafer_id,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn wafer_lineage(&self, wafer: &WaferRef) -> Vec<WaferRef> {
        let mut lineage = self.wafer_ancestors(wafer);
        lineage.reverse();
        if self.wafer(wafer).is_some() {
            lineage.push(wafer.clone());
        }
        lineage
    }

    pub fn wafer_ancestors(&self, wafer: &WaferRef) -> Vec<WaferRef> {
        let mut ancestors = Vec::new();
        let mut seen = BTreeSet::new();
        let mut cursor = wafer.clone();
        while seen.insert(cursor.clone()) {
            let Some(parent) = self.wafer(&cursor).and_then(|wafer| wafer.parent.clone()) else {
                break;
            };
            ancestors.push(parent.clone());
            cursor = parent;
        }
        ancestors
    }

    pub fn wafer_descendants(&self, wafer: &WaferRef) -> Vec<WaferRef> {
        let mut descendants = Vec::new();
        let mut queue = VecDeque::from([(wafer.clone(), 0usize)]);
        let mut seen = BTreeSet::new();
        while let Some((current, _depth)) = queue.pop_front() {
            if !seen.insert(current.clone()) {
                continue;
            }
            for child in self.direct_children(&current) {
                descendants.push(child.clone());
                queue.push_back((child, _depth + 1));
            }
        }
        descendants
    }

    pub fn inherited_process_history_for_wafer(
        &self,
        wafer: &WaferRef,
    ) -> Vec<&WaferProcessRecord> {
        let lineage = self
            .wafer_lineage(wafer)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut records = self
            .process_history
            .iter()
            .filter(|record| lineage.contains(&record.wafer))
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.sequence);
        records
    }

    pub fn material_ancestry_for_wafer(&self, wafer: &WaferRef) -> Vec<&MaterialUse> {
        let lineage = self
            .wafer_lineage(wafer)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut records = self
            .material_uses
            .iter()
            .filter(|record| lineage.contains(&record.wafer))
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.sequence);
        records
    }

    pub fn impact_for(&self, query: ExcursionQuery) -> ImpactAnalysis {
        let matching_process_records = self
            .process_history
            .iter()
            .filter(|record| match &query {
                ExcursionQuery::ToolRun { tool_run_id } => record.tool_run_id == *tool_run_id,
                ExcursionQuery::ProcessStep { step_id } => record.step_id == *step_id,
                ExcursionQuery::MaterialLot { .. } => false,
            })
            .cloned()
            .collect::<Vec<_>>();
        let matching_material_uses = self
            .material_uses
            .iter()
            .filter(|record| match &query {
                ExcursionQuery::MaterialLot { material_lot_id } => {
                    record.material_lot_id == *material_lot_id
                }
                ExcursionQuery::ToolRun { .. } | ExcursionQuery::ProcessStep { .. } => false,
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut direct = matching_process_records
            .iter()
            .map(|record| record.wafer.clone())
            .chain(
                matching_material_uses
                    .iter()
                    .map(|record| record.wafer.clone()),
            )
            .collect::<BTreeSet<_>>();
        let direct_wafers = direct.iter().cloned().collect::<Vec<_>>();

        let mut impacted = BTreeMap::new();
        for wafer in direct_wafers.iter().cloned() {
            impacted.insert(wafer.clone(), ImpactRelationship::Direct);
            self.add_descendant_impacts(&wafer, &mut impacted);
        }
        for wafer in std::mem::take(&mut direct) {
            impacted.insert(wafer, ImpactRelationship::Direct);
        }

        let impacted_wafers = impacted
            .into_iter()
            .map(|(wafer, relationship)| ImpactedWafer {
                latest_step_id: self
                    .inherited_process_history_for_wafer(&wafer)
                    .last()
                    .map(|record| record.step_id.clone()),
                wafer,
                relationship,
            })
            .collect();

        ImpactAnalysis {
            query,
            direct_wafers,
            impacted_wafers,
            matching_process_records,
            matching_material_uses,
        }
    }

    pub fn wafer(&self, wafer: &WaferRef) -> Option<&GenealogyWafer> {
        self.lots
            .get(&wafer.lot_id)
            .and_then(|lot| lot.wafers.get(&wafer.wafer_id))
    }

    fn require_lot(&self, lot_id: &LotId) -> Result<(), GenealogyError> {
        if self.lots.contains_key(lot_id) {
            Ok(())
        } else {
            Err(GenealogyError::LotNotFound(lot_id.clone()))
        }
    }

    fn require_wafer(&self, wafer: &WaferRef) -> Result<(), GenealogyError> {
        if self.wafer(wafer).is_some() {
            Ok(())
        } else {
            Err(GenealogyError::WaferNotFound(wafer.clone()))
        }
    }

    fn direct_children(&self, parent: &WaferRef) -> Vec<WaferRef> {
        self.lots
            .iter()
            .flat_map(|(lot_id, lot)| {
                lot.wafers.values().filter_map(|wafer| {
                    if wafer.parent.as_ref() == Some(parent) {
                        Some(WaferRef {
                            lot_id: lot_id.clone(),
                            wafer_id: wafer.id.clone(),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn add_descendant_impacts(
        &self,
        ancestor: &WaferRef,
        impacted: &mut BTreeMap<WaferRef, ImpactRelationship>,
    ) {
        let mut queue = VecDeque::from([(ancestor.clone(), 0usize)]);
        let mut seen = BTreeSet::new();
        while let Some((current, depth)) = queue.pop_front() {
            if !seen.insert(current.clone()) {
                continue;
            }
            for child in self.direct_children(&current) {
                let generation = depth + 1;
                impacted
                    .entry(child.clone())
                    .or_insert_with(|| ImpactRelationship::Descendant {
                        ancestor: ancestor.clone(),
                        generations: generation,
                    });
                queue.push_back((child, generation));
            }
        }
    }

    fn push_event(&mut self, kind: GenealogyEventKind) {
        let sequence = self.next_sequence();
        self.events.push(GenealogyEvent { sequence, kind });
    }

    fn next_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        sequence
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessStepTrace {
    pub step_id: ProcessStepId,
    pub step_name: String,
    pub process_layer: Option<ProcessLayer>,
    pub recipe_id: RecipeId,
    pub tool_id: ToolId,
    pub tool_run_id: ToolRunId,
    pub completed_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialUseTrace {
    pub material_lot_id: MaterialLotId,
    pub step_id: ProcessStepId,
    pub tool_run_id: ToolRunId,
    pub quantity: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenealogyError {
    DuplicateLot(LotId),
    LotNotFound(LotId),
    DuplicateMaterialLot(MaterialLotId),
    MaterialLotNotFound(MaterialLotId),
    WaferNotFound(WaferRef),
    DuplicateWaferInOperation(WaferId),
    EmptyOperation(&'static str),
    LotNotInMergeSources {
        source_lot_id: LotId,
        target_lot_id: LotId,
    },
}

impl fmt::Display for GenealogyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateLot(lot_id) => write!(f, "lot {lot_id} already exists"),
            Self::LotNotFound(lot_id) => write!(f, "lot {lot_id} was not found"),
            Self::DuplicateMaterialLot(material_lot_id) => {
                write!(f, "material lot {material_lot_id} already exists")
            }
            Self::MaterialLotNotFound(material_lot_id) => {
                write!(f, "material lot {material_lot_id} was not found")
            }
            Self::WaferNotFound(wafer) => write!(f, "wafer {wafer} was not found"),
            Self::DuplicateWaferInOperation(wafer_id) => {
                write!(
                    f,
                    "wafer {wafer_id} appears more than once in the operation"
                )
            }
            Self::EmptyOperation(operation) => write!(f, "{operation} has no wafers"),
            Self::LotNotInMergeSources {
                source_lot_id,
                target_lot_id,
            } => write!(
                f,
                "lot {source_lot_id} is not listed as a merge source for {target_lot_id}"
            ),
        }
    }
}

impl Error for GenealogyError {}

pub fn sample_lot_genealogy() -> LotGenealogy {
    let mes = sample_fab_data();
    let mut genealogy = LotGenealogy::from_mes(&mes);
    let route_id = mes
        .routes
        .keys()
        .next()
        .cloned()
        .expect("sample route exists");
    let route = mes.routes.get(&route_id).expect("sample route exists");

    let photoresist = MaterialLot {
        id: MaterialLotId::new("MAT-PR-2026-041"),
        name: "Positive photoresist SPR-955".to_string(),
        supplier: "DemoChem".to_string(),
        certificate_id: "COA-PR-041".to_string(),
        received_at: "2026-05-04T08:00:00Z".to_string(),
    };
    let developer = MaterialLot {
        id: MaterialLotId::new("MAT-DEV-2026-018"),
        name: "TMAH developer 2.38%".to_string(),
        supplier: "DemoChem".to_string(),
        certificate_id: "COA-DEV-018".to_string(),
        received_at: "2026-05-04T08:15:00Z".to_string(),
    };
    let etch_gas = MaterialLot {
        id: MaterialLotId::new("MAT-CF4-2026-009"),
        name: "CF4 etch gas cylinder".to_string(),
        supplier: "ProcessGas".to_string(),
        certificate_id: "COA-CF4-009".to_string(),
        received_at: "2026-05-05T06:30:00Z".to_string(),
    };
    genealogy.register_material_lot(photoresist).unwrap();
    genealogy.register_material_lot(developer).unwrap();
    genealogy.register_material_lot(etch_gas).unwrap();

    let base_lot = LotId::new("L-00042");
    let base_wafers = genealogy
        .wafer_refs_for_lot(&base_lot)
        .into_iter()
        .map(|wafer| wafer.wafer_id)
        .collect::<Vec<_>>();
    genealogy
        .record_lot_material_use(
            &base_lot,
            &base_wafers,
            MaterialUseTrace {
                material_lot_id: MaterialLotId::new("MAT-PR-2026-041"),
                step_id: ProcessStepId::new("S010-COAT"),
                tool_run_id: ToolRunId::new("TRACK-01-S010-COAT-RUN-001"),
                quantity: 1.8,
                unit: "mL".to_string(),
            },
        )
        .unwrap();

    let focus_split = LotId::new("L-00042A");
    genealogy
        .split_lot(
            &base_lot,
            focus_split.clone(),
            "demo inverter poly focus split".to_string(),
            route_id.clone(),
            (1..=6)
                .map(|slot| {
                    WaferTransfer::new(
                        format!("L-00042-W{slot:02}"),
                        format!("L-00042A-W{slot:02}"),
                        slot,
                    )
                })
                .collect(),
            "focus-dose split for expose window".to_string(),
        )
        .unwrap();
    let dose_split = LotId::new("L-00042B");
    genealogy
        .split_lot(
            &base_lot,
            dose_split.clone(),
            "demo inverter poly dose split".to_string(),
            route_id.clone(),
            (7..=12)
                .map(|slot| {
                    WaferTransfer::new(
                        format!("L-00042-W{slot:02}"),
                        format!("L-00042B-W{slot:02}"),
                        slot,
                    )
                })
                .collect(),
            "alternate expose dose split".to_string(),
        )
        .unwrap();

    let expose_step = route.step(&ProcessStepId::new("S020-EXPOSE")).unwrap();
    let develop_step = route.step(&ProcessStepId::new("S030-DEVELOP")).unwrap();
    let focus_wafers = genealogy
        .wafer_refs_for_lot(&focus_split)
        .into_iter()
        .map(|wafer| wafer.wafer_id)
        .collect::<Vec<_>>();
    let dose_wafers = genealogy
        .wafer_refs_for_lot(&dose_split)
        .into_iter()
        .map(|wafer| wafer.wafer_id)
        .collect::<Vec<_>>();

    genealogy
        .record_lot_process_step(
            &focus_split,
            &focus_wafers,
            ProcessStepTrace::from_step(
                expose_step,
                ToolId::new("ALIGNER-01"),
                ToolRunId::new("ALIGNER-01-RUN-20260506-1100"),
                "2026-05-06T11:00:00Z",
            ),
        )
        .unwrap();
    genealogy
        .record_lot_process_step(
            &dose_split,
            &dose_wafers,
            ProcessStepTrace::from_step(
                expose_step,
                ToolId::new("ALIGNER-01"),
                ToolRunId::new("ALIGNER-01-RUN-20260506-1115"),
                "2026-05-06T11:15:00Z",
            ),
        )
        .unwrap();
    genealogy
        .record_lot_material_use(
            &focus_split,
            &focus_wafers,
            MaterialUseTrace {
                material_lot_id: MaterialLotId::new("MAT-DEV-2026-018"),
                step_id: develop_step.id.clone(),
                tool_run_id: ToolRunId::new("TRACK-02-RUN-20260506-1145"),
                quantity: 22.0,
                unit: "mL".to_string(),
            },
        )
        .unwrap();
    genealogy
        .record_lot_process_step(
            &focus_split,
            &focus_wafers,
            ProcessStepTrace::from_step(
                develop_step,
                ToolId::new("TRACK-02"),
                ToolRunId::new("TRACK-02-RUN-20260506-1145"),
                "2026-05-06T11:45:00Z",
            ),
        )
        .unwrap();
    genealogy
        .record_lot_process_step(
            &dose_split,
            &dose_wafers,
            ProcessStepTrace::from_step(
                develop_step,
                ToolId::new("TRACK-01"),
                ToolRunId::new("TRACK-01-RUN-20260506-1150"),
                "2026-05-06T11:50:00Z",
            ),
        )
        .unwrap();

    let merged_lot = LotId::new("L-00042-ETCH");
    let merge_transfers = focus_wafers
        .iter()
        .chain(dose_wafers.iter())
        .enumerate()
        .map(|(index, wafer_id)| {
            let source_lot = if wafer_id.as_str().contains("A-") {
                focus_split.clone()
            } else {
                dose_split.clone()
            };
            MergeWaferTransfer::new(
                WaferRef {
                    lot_id: source_lot,
                    wafer_id: wafer_id.clone(),
                },
                format!("L-00042-ETCH-W{:02}", index + 1),
                (index + 1) as u8,
            )
        })
        .collect::<Vec<_>>();
    genealogy
        .merge_lots(
            vec![focus_split.clone(), dose_split.clone()],
            merged_lot.clone(),
            "demo inverter poly merged etch lot".to_string(),
            route_id,
            merge_transfers,
            "recombine expose splits for shared etch".to_string(),
        )
        .unwrap();

    let etch_step = route.step(&ProcessStepId::new("S040-ETCH")).unwrap();
    let merged_wafers = genealogy
        .wafer_refs_for_lot(&merged_lot)
        .into_iter()
        .map(|wafer| wafer.wafer_id)
        .collect::<Vec<_>>();
    genealogy
        .record_lot_material_use(
            &merged_lot,
            &merged_wafers,
            MaterialUseTrace {
                material_lot_id: MaterialLotId::new("MAT-CF4-2026-009"),
                step_id: etch_step.id.clone(),
                tool_run_id: ToolRunId::new("ETCH-01-RUN-20260506-1320"),
                quantity: 0.18,
                unit: "slm-min".to_string(),
            },
        )
        .unwrap();
    genealogy
        .record_lot_process_step(
            &merged_lot,
            &merged_wafers,
            ProcessStepTrace::from_step(
                etch_step,
                ToolId::new("ETCH-01"),
                ToolRunId::new("ETCH-01-RUN-20260506-1320"),
                "2026-05-06T13:20:00Z",
            ),
        )
        .unwrap();

    genealogy
}

impl ProcessStepTrace {
    fn from_step(
        step: &ProcessStep,
        tool_id: ToolId,
        tool_run_id: ToolRunId,
        completed_at: impl Into<String>,
    ) -> Self {
        Self {
            step_id: step.id.clone(),
            step_name: step.name.clone(),
            process_layer: process_layer_for_step(step),
            recipe_id: step.required_recipe.clone(),
            tool_id,
            tool_run_id,
            completed_at: completed_at.into(),
        }
    }
}

fn process_layer_for_step(step: &ProcessStep) -> Option<ProcessLayer> {
    let normalized = format!(
        "{} {} {}",
        step.area.to_lowercase(),
        step.name.to_lowercase(),
        step.id.as_str().to_lowercase()
    );
    if normalized.contains("poly") {
        Some(ProcessLayer::Poly)
    } else if normalized.contains("contact") {
        Some(ProcessLayer::Contact)
    } else if normalized.contains("metal") {
        Some(ProcessLayer::Metal1)
    } else if normalized.contains("oxide") {
        Some(ProcessLayer::Oxide)
    } else {
        None
    }
}

fn default_next_sequence() -> u64 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_records_parentage_and_descendants() {
        let genealogy = sample_lot_genealogy();
        let parent = WaferRef::new("L-00042", "L-00042-W01");
        let child = WaferRef::new("L-00042A", "L-00042A-W01");
        let merged = WaferRef::new("L-00042-ETCH", "L-00042-ETCH-W01");

        assert_eq!(
            genealogy
                .wafer(&child)
                .and_then(|wafer| wafer.parent.clone()),
            Some(parent.clone())
        );
        assert_eq!(
            genealogy.wafer_lineage(&merged),
            vec![parent.clone(), child.clone(), merged.clone()]
        );
        assert!(genealogy.wafer_descendants(&parent).contains(&merged));
    }

    #[test]
    fn tool_run_impact_expands_through_later_merges() {
        let genealogy = sample_lot_genealogy();
        let impact = genealogy.impact_for(ExcursionQuery::ToolRun {
            tool_run_id: ToolRunId::new("ALIGNER-01-RUN-20260506-1100"),
        });

        assert_eq!(impact.direct_wafers.len(), 6);
        assert!(impact.impacted_wafers.iter().any(|impact| impact.wafer
            == WaferRef::new("L-00042-ETCH", "L-00042-ETCH-W01")
            && matches!(
                impact.relationship,
                ImpactRelationship::Descendant { generations: 1, .. }
            )));
        assert!(
            impact
                .impacted_wafers
                .iter()
                .all(|impact| impact.latest_step_id.is_some())
        );
    }

    #[test]
    fn material_ancestry_inherits_from_parent_wafers() {
        let genealogy = sample_lot_genealogy();
        let merged = WaferRef::new("L-00042-ETCH", "L-00042-ETCH-W01");
        let materials = genealogy.material_ancestry_for_wafer(&merged);

        assert!(
            materials
                .iter()
                .any(|record| record.material_lot_id == MaterialLotId::new("MAT-PR-2026-041"))
        );
        assert!(
            materials
                .iter()
                .any(|record| record.material_lot_id == MaterialLotId::new("MAT-DEV-2026-018"))
        );
        assert!(
            materials
                .iter()
                .any(|record| record.material_lot_id == MaterialLotId::new("MAT-CF4-2026-009"))
        );
    }

    #[test]
    fn material_excursion_marks_descendant_wafers() {
        let genealogy = sample_lot_genealogy();
        let impact = genealogy.impact_for(ExcursionQuery::MaterialLot {
            material_lot_id: MaterialLotId::new("MAT-PR-2026-041"),
        });

        assert!(impact.direct_wafers.len() >= 25);
        assert!(
            impact
                .impacted_wafers
                .iter()
                .any(|impact| impact.wafer == WaferRef::new("L-00042-ETCH", "L-00042-ETCH-W12"))
        );
    }
}
