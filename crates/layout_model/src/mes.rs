use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};
use tracing::warn;

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
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

string_id!(LotId);
string_id!(WaferId);
string_id!(ProcessRouteId);
string_id!(ProcessStepId);
string_id!(RecipeId);
string_id!(ToolId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ToolClass {
    LithographyTrack,
    MaskAligner,
    PlasmaEtcher,
    CdMetrology,
    BakeOven,
}

impl ToolClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::LithographyTrack => "lithography track",
            Self::MaskAligner => "mask aligner",
            Self::PlasmaEtcher => "plasma etcher",
            Self::CdMetrology => "CD metrology",
            Self::BakeOven => "bake oven",
        }
    }
}

impl fmt::Display for ToolClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStep {
    pub id: ProcessStepId,
    pub sequence: u32,
    pub name: String,
    pub area: String,
    pub required_tool_class: ToolClass,
    pub required_recipe: RecipeId,
    pub eligible_tools: Vec<ToolId>,
    pub signoff_required: bool,
    pub rework_allowed: bool,
}

impl ProcessStep {
    pub fn primary_tool(&self) -> Option<&ToolId> {
        self.eligible_tools.first()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRoute {
    pub id: ProcessRouteId,
    pub name: String,
    pub revision: String,
    pub mask_design_id: String,
    pub layout_revision: String,
    pub steps: Vec<ProcessStep>,
}

impl ProcessRoute {
    pub fn step(&self, id: &ProcessStepId) -> Option<&ProcessStep> {
        self.steps.iter().find(|step| step.id == *id)
    }

    pub fn first_step_id(&self) -> Option<&ProcessStepId> {
        self.steps
            .iter()
            .min_by_key(|step| step.sequence)
            .map(|step| &step.id)
    }

    pub fn next_step_id_after(&self, id: &ProcessStepId) -> Option<&ProcessStepId> {
        let current = self.step(id)?;
        self.steps
            .iter()
            .filter(|step| step.sequence > current.sequence)
            .min_by_key(|step| step.sequence)
            .map(|step| &step.id)
    }

    pub fn step_name(&self, id: &ProcessStepId) -> &str {
        self.step(id)
            .map_or("unknown step", |step| step.name.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaferStatus {
    Active,
    InRework {
        from_step: Option<ProcessStepId>,
        target_step: ProcessStepId,
        reason: String,
    },
    Scrapped {
        step_id: Option<ProcessStepId>,
        reason: String,
    },
}

impl WaferStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::InRework { .. } => "rework",
            Self::Scrapped { .. } => "scrapped",
        }
    }

    pub fn is_scrapped(&self) -> bool {
        matches!(self, Self::Scrapped { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wafer {
    pub id: WaferId,
    pub slot: u8,
    pub status: WaferStatus,
    pub rework_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lot {
    pub id: LotId,
    pub product: String,
    pub priority: u8,
    pub route_id: ProcessRouteId,
    pub mask_design_id: String,
    pub layout_revision: String,
    pub wafers: Vec<Wafer>,
}

impl Lot {
    pub fn new_25_wafer_lot(
        id: impl Into<LotId>,
        product: impl Into<String>,
        route: &ProcessRoute,
    ) -> Self {
        let id = id.into();
        let wafers = (1..=25)
            .map(|slot| Wafer {
                id: WaferId::new(format!("{}-W{slot:02}", id.as_str())),
                slot,
                status: WaferStatus::Active,
                rework_count: 0,
            })
            .collect();
        Self {
            id,
            product: product.into(),
            priority: 2,
            route_id: route.id.clone(),
            mask_design_id: route.mask_design_id.clone(),
            layout_revision: route.layout_revision.clone(),
            wafers,
        }
    }

    pub fn wafer(&self, id: &WaferId) -> Option<&Wafer> {
        self.wafers.iter().find(|wafer| wafer.id == *id)
    }

    pub fn wafer_mut(&mut self, id: &WaferId) -> Option<&mut Wafer> {
        self.wafers.iter_mut().find(|wafer| wafer.id == *id)
    }

    pub fn processable_wafer_count(&self) -> usize {
        self.wafers
            .iter()
            .filter(|wafer| !wafer.status.is_scrapped())
            .count()
    }

    pub fn scrapped_wafer_count(&self) -> usize {
        self.wafers
            .iter()
            .filter(|wafer| wafer.status.is_scrapped())
            .count()
    }

    pub fn rework_wafer_count(&self) -> usize {
        self.wafers
            .iter()
            .filter(|wafer| matches!(wafer.status, WaferStatus::InRework { .. }))
            .count()
    }

    pub fn first_processable_wafer_id(&self) -> Option<WaferId> {
        self.wafers
            .iter()
            .find(|wafer| !wafer.status.is_scrapped())
            .map(|wafer| wafer.id.clone())
    }

    pub fn last_processable_wafer_id(&self) -> Option<WaferId> {
        self.wafers
            .iter()
            .rev()
            .find(|wafer| !wafer.status.is_scrapped())
            .map(|wafer| wafer.id.clone())
    }

    fn complete_rework_at_step(&mut self, step_id: &ProcessStepId) -> usize {
        let mut completed = 0;
        for wafer in &mut self.wafers {
            if matches!(
                &wafer.status,
                WaferStatus::InRework { target_step, .. } if target_step == step_id
            ) {
                wafer.status = WaferStatus::Active;
                wafer.rework_count += 1;
                completed += 1;
            }
        }
        completed
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TravelerStatus {
    WaitingForStep,
    Running,
    WaitingForSignoff,
    OnHold,
    Complete,
    Scrapped,
}

impl TravelerStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::WaitingForStep => "waiting",
            Self::Running => "running",
            Self::WaitingForSignoff => "signoff",
            Self::OnHold => "hold",
            Self::Complete => "complete",
            Self::Scrapped => "scrapped",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveStepRun {
    pub step_id: ProcessStepId,
    pub tool_id: ToolId,
    pub tool_class: ToolClass,
    pub recipe_id: RecipeId,
    pub operator: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSignoff {
    pub run: ActiveStepRun,
    pub completed_by: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedStep {
    pub step_id: ProcessStepId,
    pub tool_id: ToolId,
    pub recipe_id: RecipeId,
    pub completed_by: String,
    pub signed_off_by: Option<String>,
    pub reworked_wafers: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldState {
    pub reason: String,
    pub placed_by: String,
    pub previous_status: TravelerStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorAction {
    StartStep {
        step_id: ProcessStepId,
        tool_id: ToolId,
        tool_class: ToolClass,
        recipe_id: RecipeId,
        operator: String,
    },
    CompleteStep {
        step_id: ProcessStepId,
        operator: String,
    },
    SignOff {
        step_id: ProcessStepId,
        operator: String,
    },
    PlaceHold {
        reason: String,
        operator: String,
    },
    ReleaseHold {
        operator: String,
    },
    ScrapWafer {
        wafer_id: WaferId,
        reason: String,
        operator: String,
    },
    SendToRework {
        wafer_ids: Vec<WaferId>,
        target_step: ProcessStepId,
        reason: String,
        operator: String,
    },
}

impl OperatorAction {
    pub fn summary(&self) -> String {
        match self {
            Self::StartStep {
                step_id,
                tool_id,
                recipe_id,
                ..
            } => {
                format!("start {step_id} on {tool_id} using {recipe_id}")
            }
            Self::CompleteStep { step_id, .. } => format!("complete {step_id}"),
            Self::SignOff { step_id, .. } => format!("sign off {step_id}"),
            Self::PlaceHold { reason, .. } => format!("place hold: {reason}"),
            Self::ReleaseHold { .. } => "release hold".to_string(),
            Self::ScrapWafer {
                wafer_id, reason, ..
            } => {
                format!("scrap {wafer_id}: {reason}")
            }
            Self::SendToRework {
                wafer_ids,
                target_step,
                reason,
                ..
            } => {
                format!(
                    "send {} wafer{} to rework at {target_step}: {reason}",
                    wafer_ids.len(),
                    if wafer_ids.len() == 1 { "" } else { "s" }
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOutcome {
    Accepted,
    Rejected,
}

impl AuditOutcome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "blocked",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub sequence: u64,
    pub lot_id: LotId,
    pub action: OperatorAction,
    pub outcome: AuditOutcome,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravelerState {
    pub lot_id: LotId,
    pub route_id: ProcessRouteId,
    pub current_step_id: Option<ProcessStepId>,
    pub status: TravelerStatus,
    pub active_run: Option<ActiveStepRun>,
    pub pending_signoff: Option<PendingSignoff>,
    pub completed_steps: Vec<CompletedStep>,
    pub hold: Option<HoldState>,
    pub audit_events: Vec<AuditEvent>,
    next_audit_sequence: u64,
}

impl TravelerState {
    pub fn new(
        lot_id: impl Into<LotId>,
        route_id: impl Into<ProcessRouteId>,
        current_step_id: impl Into<ProcessStepId>,
    ) -> Self {
        Self {
            lot_id: lot_id.into(),
            route_id: route_id.into(),
            current_step_id: Some(current_step_id.into()),
            status: TravelerStatus::WaitingForStep,
            active_run: None,
            pending_signoff: None,
            completed_steps: Vec::new(),
            hold: None,
            audit_events: Vec::new(),
            next_audit_sequence: 1,
        }
    }

    pub fn current_step<'a>(&self, route: &'a ProcessRoute) -> Option<&'a ProcessStep> {
        let step_id = self.current_step_id.as_ref()?;
        route.step(step_id)
    }

    pub fn apply_action(
        &mut self,
        lot: &mut Lot,
        route: &ProcessRoute,
        action: OperatorAction,
    ) -> Result<(), TravelerTransitionError> {
        let result = self.apply_action_inner(lot, route, &action);
        match &result {
            Ok(message) => self.push_audit(action, AuditOutcome::Accepted, message.clone()),
            Err(err) => self.push_audit(action, AuditOutcome::Rejected, err.to_string()),
        }
        result.map(|_| ())
    }

    fn apply_action_inner(
        &mut self,
        lot: &mut Lot,
        route: &ProcessRoute,
        action: &OperatorAction,
    ) -> Result<String, TravelerTransitionError> {
        match action {
            OperatorAction::StartStep {
                step_id,
                tool_id,
                tool_class,
                recipe_id,
                operator,
            } => self.start_step(route, step_id, tool_id, *tool_class, recipe_id, operator),
            OperatorAction::CompleteStep { step_id, operator } => {
                self.complete_step(lot, route, step_id, operator)
            }
            OperatorAction::SignOff { step_id, operator } => {
                self.sign_off_step(lot, route, step_id, operator)
            }
            OperatorAction::PlaceHold { reason, operator } => {
                self.place_hold(reason.clone(), operator.clone())
            }
            OperatorAction::ReleaseHold { operator } => self.release_hold(operator),
            OperatorAction::ScrapWafer {
                wafer_id, reason, ..
            } => self.scrap_wafer(lot, wafer_id, reason.clone()),
            OperatorAction::SendToRework {
                wafer_ids,
                target_step,
                reason,
                ..
            } => self.send_to_rework(lot, route, wafer_ids, target_step, reason.clone()),
        }
    }

    fn start_step(
        &mut self,
        route: &ProcessRoute,
        step_id: &ProcessStepId,
        tool_id: &ToolId,
        tool_class: ToolClass,
        recipe_id: &RecipeId,
        operator: &str,
    ) -> Result<String, TravelerTransitionError> {
        self.ensure_movable()?;
        if self.active_run.is_some() {
            return Err(TravelerTransitionError::AlreadyRunning);
        }
        if self.pending_signoff.is_some() {
            return Err(TravelerTransitionError::SignoffRequired);
        }
        if self.status != TravelerStatus::WaitingForStep {
            return Err(TravelerTransitionError::NotWaitingForStep {
                status: self.status.clone(),
            });
        }
        let current = self
            .current_step_id
            .clone()
            .ok_or(TravelerTransitionError::NoCurrentStep)?;
        if current != *step_id {
            return Err(TravelerTransitionError::WrongStep {
                expected: current,
                got: step_id.clone(),
            });
        }
        let step = route
            .step(step_id)
            .ok_or_else(|| TravelerTransitionError::UnknownStep(step_id.clone()))?;
        if step.required_tool_class != tool_class {
            return Err(TravelerTransitionError::ToolClassMismatch {
                expected: step.required_tool_class,
                got: tool_class,
            });
        }
        if step.required_recipe != *recipe_id {
            return Err(TravelerTransitionError::RecipeMismatch {
                expected: step.required_recipe.clone(),
                got: recipe_id.clone(),
            });
        }
        if !step.eligible_tools.is_empty() && !step.eligible_tools.contains(tool_id) {
            return Err(TravelerTransitionError::ToolNotEligible {
                step_id: step_id.clone(),
                tool_id: tool_id.clone(),
            });
        }
        self.active_run = Some(ActiveStepRun {
            step_id: step_id.clone(),
            tool_id: tool_id.clone(),
            tool_class,
            recipe_id: recipe_id.clone(),
            operator: operator.to_string(),
        });
        self.status = TravelerStatus::Running;
        Ok(format!("started {}", step.name))
    }

    fn complete_step(
        &mut self,
        lot: &mut Lot,
        route: &ProcessRoute,
        step_id: &ProcessStepId,
        operator: &str,
    ) -> Result<String, TravelerTransitionError> {
        self.ensure_movable()?;
        if self.status != TravelerStatus::Running {
            return Err(TravelerTransitionError::NotRunning {
                status: self.status.clone(),
            });
        }
        let run = self
            .active_run
            .take()
            .ok_or(TravelerTransitionError::NotRunning {
                status: self.status.clone(),
            })?;
        if run.step_id != *step_id {
            self.active_run = Some(run);
            return Err(TravelerTransitionError::WrongStep {
                expected: self
                    .active_run
                    .as_ref()
                    .map(|run| run.step_id.clone())
                    .unwrap_or_else(|| {
                        warn!(
                            lot_id = %self.lot_id,
                            "traveler active run missing after wrong-step check; using unknown expected step"
                        );
                        ProcessStepId::new("unknown")
                    }),
                got: step_id.clone(),
            });
        }
        let step = route
            .step(step_id)
            .ok_or_else(|| TravelerTransitionError::UnknownStep(step_id.clone()))?;
        if step.signoff_required {
            self.pending_signoff = Some(PendingSignoff {
                run,
                completed_by: operator.to_string(),
            });
            self.status = TravelerStatus::WaitingForSignoff;
            Ok(format!("completed {}; waiting for signoff", step.name))
        } else {
            self.finish_step(lot, route, run, operator.to_string(), None);
            Ok(format!("completed {}", step.name))
        }
    }

    fn sign_off_step(
        &mut self,
        lot: &mut Lot,
        route: &ProcessRoute,
        step_id: &ProcessStepId,
        operator: &str,
    ) -> Result<String, TravelerTransitionError> {
        self.ensure_movable()?;
        if self.status != TravelerStatus::WaitingForSignoff {
            return Err(TravelerTransitionError::SignoffNotPending);
        }
        let pending = self
            .pending_signoff
            .take()
            .ok_or(TravelerTransitionError::SignoffNotPending)?;
        if pending.run.step_id != *step_id {
            self.pending_signoff = Some(pending);
            return Err(TravelerTransitionError::WrongStep {
                expected: self
                    .pending_signoff
                    .as_ref()
                    .map(|pending| pending.run.step_id.clone())
                    .unwrap_or_else(|| {
                        warn!(
                            lot_id = %self.lot_id,
                            "traveler pending signoff missing after wrong-step check; using unknown expected step"
                        );
                        ProcessStepId::new("unknown")
                    }),
                got: step_id.clone(),
            });
        }
        let step_name = route.step_name(step_id).to_string();
        let completed_by = pending.completed_by;
        self.finish_step(
            lot,
            route,
            pending.run,
            completed_by,
            Some(operator.to_string()),
        );
        Ok(format!("signed off {step_name}"))
    }

    fn finish_step(
        &mut self,
        lot: &mut Lot,
        route: &ProcessRoute,
        run: ActiveStepRun,
        completed_by: String,
        signed_off_by: Option<String>,
    ) {
        let reworked_wafers = lot.complete_rework_at_step(&run.step_id);
        self.completed_steps.push(CompletedStep {
            step_id: run.step_id.clone(),
            tool_id: run.tool_id,
            recipe_id: run.recipe_id,
            completed_by,
            signed_off_by,
            reworked_wafers,
        });
        if lot.processable_wafer_count() == 0 {
            self.status = TravelerStatus::Scrapped;
            self.current_step_id = None;
            return;
        }
        self.current_step_id = route.next_step_id_after(&run.step_id).cloned();
        self.status = if self.current_step_id.is_some() {
            TravelerStatus::WaitingForStep
        } else {
            TravelerStatus::Complete
        };
    }

    fn place_hold(
        &mut self,
        reason: String,
        operator: String,
    ) -> Result<String, TravelerTransitionError> {
        if self.status == TravelerStatus::OnHold {
            return Err(TravelerTransitionError::AlreadyHeld);
        }
        self.ensure_not_terminal()?;
        let previous_status = self.status.clone();
        self.hold = Some(HoldState {
            reason: reason.clone(),
            placed_by: operator,
            previous_status,
        });
        self.status = TravelerStatus::OnHold;
        Ok(format!("hold placed: {reason}"))
    }

    fn release_hold(&mut self, _operator: &str) -> Result<String, TravelerTransitionError> {
        let hold = self.hold.take().ok_or(TravelerTransitionError::NotHeld)?;
        self.status = hold.previous_status;
        Ok(format!("hold released: {}", hold.reason))
    }

    fn scrap_wafer(
        &mut self,
        lot: &mut Lot,
        wafer_id: &WaferId,
        reason: String,
    ) -> Result<String, TravelerTransitionError> {
        self.ensure_movable()?;
        let step_id = self.current_step_id.clone();
        let wafer = lot
            .wafer_mut(wafer_id)
            .ok_or_else(|| TravelerTransitionError::WaferNotFound(wafer_id.clone()))?;
        if wafer.status.is_scrapped() {
            return Err(TravelerTransitionError::WaferScrapped(wafer_id.clone()));
        }
        wafer.status = WaferStatus::Scrapped { step_id, reason };
        if lot.processable_wafer_count() == 0 {
            self.active_run = None;
            self.pending_signoff = None;
            self.current_step_id = None;
            self.status = TravelerStatus::Scrapped;
        }
        Ok(format!("scrapped {wafer_id}"))
    }

    fn send_to_rework(
        &mut self,
        lot: &mut Lot,
        route: &ProcessRoute,
        wafer_ids: &[WaferId],
        target_step: &ProcessStepId,
        reason: String,
    ) -> Result<String, TravelerTransitionError> {
        self.ensure_movable()?;
        if wafer_ids.is_empty() {
            return Err(TravelerTransitionError::NoWafersSelected);
        }
        let target = route
            .step(target_step)
            .ok_or_else(|| TravelerTransitionError::UnknownStep(target_step.clone()))?;
        if !target.rework_allowed {
            return Err(TravelerTransitionError::ReworkNotAllowed(
                target_step.clone(),
            ));
        }
        if !self.can_rework_to(target_step) {
            return Err(TravelerTransitionError::InvalidReworkTarget(
                target_step.clone(),
            ));
        }
        for wafer_id in wafer_ids {
            let wafer = lot
                .wafer(wafer_id)
                .ok_or_else(|| TravelerTransitionError::WaferNotFound(wafer_id.clone()))?;
            if wafer.status.is_scrapped() {
                return Err(TravelerTransitionError::WaferScrapped(wafer_id.clone()));
            }
        }
        let from_step = self.current_step_id.clone();
        for wafer_id in wafer_ids {
            let wafer = lot
                .wafer_mut(wafer_id)
                .ok_or_else(|| TravelerTransitionError::WaferNotFound(wafer_id.clone()))?;
            wafer.status = WaferStatus::InRework {
                from_step: from_step.clone(),
                target_step: target_step.clone(),
                reason: reason.clone(),
            };
        }
        self.active_run = None;
        self.pending_signoff = None;
        self.current_step_id = Some(target_step.clone());
        self.status = TravelerStatus::WaitingForStep;
        Ok(format!(
            "sent {} wafer{} to rework at {}",
            wafer_ids.len(),
            if wafer_ids.len() == 1 { "" } else { "s" },
            target.name
        ))
    }

    fn can_rework_to(&self, target_step: &ProcessStepId) -> bool {
        self.current_step_id.as_ref() == Some(target_step)
            || self
                .completed_steps
                .iter()
                .any(|step| step.step_id == *target_step)
    }

    fn ensure_movable(&self) -> Result<(), TravelerTransitionError> {
        if self.status == TravelerStatus::OnHold {
            return Err(TravelerTransitionError::TravelerHeld);
        }
        self.ensure_not_terminal()
    }

    fn ensure_not_terminal(&self) -> Result<(), TravelerTransitionError> {
        match self.status {
            TravelerStatus::Complete => Err(TravelerTransitionError::TravelerComplete),
            TravelerStatus::Scrapped => Err(TravelerTransitionError::TravelerScrapped),
            _ => Ok(()),
        }
    }

    fn push_audit(&mut self, action: OperatorAction, outcome: AuditOutcome, message: String) {
        let event = AuditEvent {
            sequence: self.next_audit_sequence,
            lot_id: self.lot_id.clone(),
            action,
            outcome,
            message,
        };
        self.next_audit_sequence += 1;
        self.audit_events.push(event);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TravelerTransitionError {
    UnknownStep(ProcessStepId),
    WrongStep {
        expected: ProcessStepId,
        got: ProcessStepId,
    },
    TravelerHeld,
    TravelerComplete,
    TravelerScrapped,
    AlreadyRunning,
    NotRunning {
        status: TravelerStatus,
    },
    NotWaitingForStep {
        status: TravelerStatus,
    },
    SignoffRequired,
    SignoffNotPending,
    ToolClassMismatch {
        expected: ToolClass,
        got: ToolClass,
    },
    RecipeMismatch {
        expected: RecipeId,
        got: RecipeId,
    },
    ToolNotEligible {
        step_id: ProcessStepId,
        tool_id: ToolId,
    },
    AlreadyHeld,
    NotHeld,
    NoCurrentStep,
    WaferNotFound(WaferId),
    WaferScrapped(WaferId),
    NoWafersSelected,
    ReworkNotAllowed(ProcessStepId),
    InvalidReworkTarget(ProcessStepId),
}

impl fmt::Display for TravelerTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStep(step) => write!(f, "unknown process step {step}"),
            Self::WrongStep { expected, got } => {
                write!(f, "expected current step {expected}, got {got}")
            }
            Self::TravelerHeld => f.write_str("traveler is on hold"),
            Self::TravelerComplete => f.write_str("traveler is already complete"),
            Self::TravelerScrapped => f.write_str("traveler has no processable wafers"),
            Self::AlreadyRunning => f.write_str("a step is already running"),
            Self::NotRunning { status } => write!(f, "traveler is not running ({status:?})"),
            Self::NotWaitingForStep { status } => {
                write!(f, "traveler is not waiting for a step ({status:?})")
            }
            Self::SignoffRequired => f.write_str("operator signoff is required before moving on"),
            Self::SignoffNotPending => f.write_str("no signoff is pending"),
            Self::ToolClassMismatch { expected, got } => {
                write!(f, "requires tool class {expected}, got {got}")
            }
            Self::RecipeMismatch { expected, got } => {
                write!(f, "requires recipe {expected}, got {got}")
            }
            Self::ToolNotEligible { step_id, tool_id } => {
                write!(f, "tool {tool_id} is not eligible for step {step_id}")
            }
            Self::AlreadyHeld => f.write_str("traveler is already held"),
            Self::NotHeld => f.write_str("traveler is not held"),
            Self::NoCurrentStep => f.write_str("traveler has no current step"),
            Self::WaferNotFound(wafer) => write!(f, "wafer {wafer} was not found"),
            Self::WaferScrapped(wafer) => write!(f, "wafer {wafer} is already scrapped"),
            Self::NoWafersSelected => f.write_str("no wafers were selected"),
            Self::ReworkNotAllowed(step) => write!(f, "rework is not allowed at step {step}"),
            Self::InvalidReworkTarget(step) => {
                write!(f, "step {step} is not a valid rework target")
            }
        }
    }
}

impl Error for TravelerTransitionError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FabMesData {
    pub routes: BTreeMap<ProcessRouteId, ProcessRoute>,
    pub lots: BTreeMap<LotId, Lot>,
    pub travelers: BTreeMap<LotId, TravelerState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FabMesValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FabMesValidationFinding {
    pub severity: FabMesValidationSeverity,
    pub message: String,
}

impl FabMesValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: FabMesValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: FabMesValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

impl FabMesData {
    pub fn sample() -> Self {
        sample_fab_data()
    }

    pub fn validate(&self) -> Vec<FabMesValidationFinding> {
        let mut findings = Vec::new();

        for (route_id, route) in &self.routes {
            validate_route(route_id, route, &mut findings);
        }
        for (lot_id, lot) in &self.lots {
            validate_lot(lot_id, lot, self.routes.get(&lot.route_id), &mut findings);
        }
        for (traveler_id, traveler) in &self.travelers {
            validate_traveler(
                traveler_id,
                traveler,
                self.lots.get(&traveler.lot_id),
                self.routes.get(&traveler.route_id),
                &mut findings,
            );
        }

        findings
    }

    pub fn is_valid(&self) -> bool {
        self.validate()
            .iter()
            .all(|finding| finding.severity != FabMesValidationSeverity::Error)
    }
}

fn validate_route(
    route_id: &ProcessRouteId,
    route: &ProcessRoute,
    findings: &mut Vec<FabMesValidationFinding>,
) {
    if route_id.as_str().trim().is_empty() {
        findings.push(FabMesValidationFinding::error("MES route id is empty"));
    }
    if route.id != *route_id {
        findings.push(FabMesValidationFinding::error(format!(
            "MES route key {route_id} does not match route id {}",
            route.id
        )));
    }
    if route.steps.is_empty() {
        findings.push(FabMesValidationFinding::error(format!(
            "MES route {route_id} has no process steps"
        )));
    }

    let mut step_ids = BTreeSet::new();
    let mut sequences = BTreeMap::new();
    for step in &route.steps {
        if step.id.as_str().trim().is_empty() {
            findings.push(FabMesValidationFinding::error(format!(
                "MES route {route_id} contains an empty step id"
            )));
        }
        if !step_ids.insert(step.id.clone()) {
            findings.push(FabMesValidationFinding::error(format!(
                "MES route {route_id} contains duplicate step id {}",
                step.id
            )));
        }
        if let Some(previous_step) = sequences.insert(step.sequence, step.id.clone()) {
            findings.push(FabMesValidationFinding::error(format!(
                "MES route {route_id} steps {previous_step} and {} share sequence {}",
                step.id, step.sequence
            )));
        }
        if step.sequence == 0 {
            findings.push(FabMesValidationFinding::error(format!(
                "MES route {route_id} step {} has invalid sequence 0",
                step.id
            )));
        }
        if step.name.trim().is_empty() {
            findings.push(FabMesValidationFinding::warning(format!(
                "MES route {route_id} step {} has no display name",
                step.id
            )));
        }
        if step.area.trim().is_empty() {
            findings.push(FabMesValidationFinding::warning(format!(
                "MES route {route_id} step {} has no area",
                step.id
            )));
        }
        if step.required_recipe.as_str().trim().is_empty() {
            findings.push(FabMesValidationFinding::error(format!(
                "MES route {route_id} step {} has no required recipe",
                step.id
            )));
        }

        let mut eligible_tools = BTreeSet::new();
        for tool_id in &step.eligible_tools {
            if tool_id.as_str().trim().is_empty() {
                findings.push(FabMesValidationFinding::error(format!(
                    "MES route {route_id} step {} has an empty eligible tool id",
                    step.id
                )));
            }
            if !eligible_tools.insert(tool_id.clone()) {
                findings.push(FabMesValidationFinding::warning(format!(
                    "MES route {route_id} step {} repeats eligible tool {tool_id}",
                    step.id
                )));
            }
        }
    }
}

fn validate_lot(
    lot_id: &LotId,
    lot: &Lot,
    route: Option<&ProcessRoute>,
    findings: &mut Vec<FabMesValidationFinding>,
) {
    if lot_id.as_str().trim().is_empty() {
        findings.push(FabMesValidationFinding::error("MES lot id is empty"));
    }
    if lot.id != *lot_id {
        findings.push(FabMesValidationFinding::error(format!(
            "MES lot key {lot_id} does not match lot id {}",
            lot.id
        )));
    }
    let Some(route) = route else {
        findings.push(FabMesValidationFinding::error(format!(
            "MES lot {lot_id} references missing route {}",
            lot.route_id
        )));
        return;
    };

    if lot.wafers.is_empty() {
        findings.push(FabMesValidationFinding::warning(format!(
            "MES lot {lot_id} has no wafers"
        )));
    }

    let mut wafer_ids = BTreeSet::new();
    let mut slots = BTreeSet::new();
    for wafer in &lot.wafers {
        if wafer.id.as_str().trim().is_empty() {
            findings.push(FabMesValidationFinding::error(format!(
                "MES lot {lot_id} contains an empty wafer id"
            )));
        }
        if !wafer_ids.insert(wafer.id.clone()) {
            findings.push(FabMesValidationFinding::error(format!(
                "MES lot {lot_id} contains duplicate wafer id {}",
                wafer.id
            )));
        }
        if wafer.slot == 0 {
            findings.push(FabMesValidationFinding::error(format!(
                "MES lot {lot_id} wafer {} has invalid slot 0",
                wafer.id
            )));
        }
        if !slots.insert(wafer.slot) {
            findings.push(FabMesValidationFinding::error(format!(
                "MES lot {lot_id} contains duplicate wafer slot {}",
                wafer.slot
            )));
        }
        match &wafer.status {
            WaferStatus::InRework {
                from_step,
                target_step,
                reason,
            } => {
                validate_optional_step_ref(
                    route,
                    from_step.as_ref(),
                    format!("MES lot {lot_id} wafer {} rework source", wafer.id),
                    findings,
                );
                validate_step_ref(
                    route,
                    target_step,
                    format!("MES lot {lot_id} wafer {} rework target", wafer.id),
                    findings,
                );
                if reason.trim().is_empty() {
                    findings.push(FabMesValidationFinding::warning(format!(
                        "MES lot {lot_id} wafer {} is in rework without a reason",
                        wafer.id
                    )));
                }
            }
            WaferStatus::Scrapped { step_id, reason } => {
                validate_optional_step_ref(
                    route,
                    step_id.as_ref(),
                    format!("MES lot {lot_id} wafer {} scrap step", wafer.id),
                    findings,
                );
                if reason.trim().is_empty() {
                    findings.push(FabMesValidationFinding::warning(format!(
                        "MES lot {lot_id} wafer {} is scrapped without a reason",
                        wafer.id
                    )));
                }
            }
            WaferStatus::Active => {}
        }
    }
}

fn validate_traveler(
    traveler_id: &LotId,
    traveler: &TravelerState,
    lot: Option<&Lot>,
    route: Option<&ProcessRoute>,
    findings: &mut Vec<FabMesValidationFinding>,
) {
    if traveler_id.as_str().trim().is_empty() {
        findings.push(FabMesValidationFinding::error("MES traveler id is empty"));
    }
    if traveler.lot_id != *traveler_id {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler key {traveler_id} does not match traveler lot {}",
            traveler.lot_id
        )));
    }
    let Some(lot) = lot else {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} references missing lot {}",
            traveler.lot_id
        )));
        return;
    };
    if traveler.route_id != lot.route_id {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} route {} does not match lot route {}",
            traveler.route_id, lot.route_id
        )));
    }
    let Some(route) = route else {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} references missing route {}",
            traveler.route_id
        )));
        return;
    };

    validate_optional_step_ref(
        route,
        traveler.current_step_id.as_ref(),
        format!("MES traveler {traveler_id} current step"),
        findings,
    );

    if traveler.active_run.is_some() && traveler.pending_signoff.is_some() {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} has both an active run and a pending signoff"
        )));
    }
    match traveler.status {
        TravelerStatus::Running if traveler.active_run.is_none() => {
            findings.push(FabMesValidationFinding::error(format!(
                "MES traveler {traveler_id} is running without an active run"
            )));
        }
        TravelerStatus::WaitingForSignoff if traveler.pending_signoff.is_none() => {
            findings.push(FabMesValidationFinding::error(format!(
                "MES traveler {traveler_id} is waiting for signoff without pending signoff data"
            )));
        }
        TravelerStatus::OnHold if traveler.hold.is_none() => {
            findings.push(FabMesValidationFinding::error(format!(
                "MES traveler {traveler_id} is on hold without hold details"
            )));
        }
        TravelerStatus::Complete | TravelerStatus::Scrapped
            if traveler.current_step_id.is_some()
                || traveler.active_run.is_some()
                || traveler.pending_signoff.is_some() =>
        {
            findings.push(FabMesValidationFinding::error(format!(
                "MES traveler {traveler_id} has terminal status with active step state"
            )));
        }
        _ => {}
    }
    if traveler.status != TravelerStatus::OnHold && traveler.hold.is_some() {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} has hold details while status is {:?}",
            traveler.status
        )));
    }
    if let Some(hold) = &traveler.hold
        && hold.previous_status == TravelerStatus::OnHold
    {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} hold state cannot resume to OnHold"
        )));
    }

    if let Some(run) = &traveler.active_run {
        if !traveler_can_carry_state(&traveler.status, &traveler.hold, TravelerStatus::Running) {
            findings.push(FabMesValidationFinding::error(format!(
                "MES traveler {traveler_id} has active run while status is {:?}",
                traveler.status
            )));
        }
        validate_active_run(route, traveler_id, "active run", run, findings);
    }
    if let Some(signoff) = &traveler.pending_signoff {
        if !traveler_can_carry_state(
            &traveler.status,
            &traveler.hold,
            TravelerStatus::WaitingForSignoff,
        ) {
            findings.push(FabMesValidationFinding::error(format!(
                "MES traveler {traveler_id} has pending signoff while status is {:?}",
                traveler.status
            )));
        }
        validate_active_run(
            route,
            traveler_id,
            "pending signoff",
            &signoff.run,
            findings,
        );
        if let Some(step) = route.step(&signoff.run.step_id)
            && !step.signoff_required
        {
            findings.push(FabMesValidationFinding::error(format!(
                "MES traveler {traveler_id} has pending signoff for non-signoff step {}",
                signoff.run.step_id
            )));
        }
    }
    for completed in &traveler.completed_steps {
        validate_completed_step(route, traveler_id, lot, completed, findings);
    }
}

fn validate_completed_step(
    route: &ProcessRoute,
    traveler_id: &LotId,
    lot: &Lot,
    completed: &CompletedStep,
    findings: &mut Vec<FabMesValidationFinding>,
) {
    let Some(step) = route.step(&completed.step_id) else {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} completed step {} is missing from route {}",
            completed.step_id, route.id
        )));
        return;
    };
    if completed.recipe_id != step.required_recipe {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} completed step {} used recipe {}, but step requires {}",
            completed.step_id, completed.recipe_id, step.required_recipe
        )));
    }
    if !step.eligible_tools.is_empty() && !step.eligible_tools.contains(&completed.tool_id) {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} completed step {} used ineligible tool {}",
            completed.step_id, completed.tool_id
        )));
    }
    if step.signoff_required && completed.signed_off_by.is_none() {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} completed signoff step {} has no signoff operator",
            completed.step_id
        )));
    }
    if !step.signoff_required && completed.signed_off_by.is_some() {
        findings.push(FabMesValidationFinding::warning(format!(
            "MES traveler {traveler_id} completed non-signoff step {} carries signoff metadata",
            completed.step_id
        )));
    }
    if completed.reworked_wafers > lot.wafers.len() {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} completed step {} reports {} reworked wafers for {} wafer lot",
            completed.step_id,
            completed.reworked_wafers,
            lot.wafers.len()
        )));
    }
    if completed.completed_by.trim().is_empty() {
        findings.push(FabMesValidationFinding::warning(format!(
            "MES traveler {traveler_id} completed step {} has no completing operator",
            completed.step_id
        )));
    }
    if completed
        .signed_off_by
        .as_deref()
        .is_some_and(|actor| actor.trim().is_empty())
    {
        findings.push(FabMesValidationFinding::warning(format!(
            "MES traveler {traveler_id} completed step {} has empty signoff operator",
            completed.step_id
        )));
    }
}

fn validate_active_run(
    route: &ProcessRoute,
    traveler_id: &LotId,
    label: &str,
    run: &ActiveStepRun,
    findings: &mut Vec<FabMesValidationFinding>,
) {
    let Some(step) = route.step(&run.step_id) else {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} {label} step {} is missing from route {}",
            run.step_id, route.id
        )));
        return;
    };
    if step.required_tool_class != run.tool_class {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} {label} uses tool class {}, but step {} requires {}",
            run.tool_class, run.step_id, step.required_tool_class
        )));
    }
    if step.required_recipe != run.recipe_id {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} {label} uses recipe {}, but step {} requires {}",
            run.recipe_id, run.step_id, step.required_recipe
        )));
    }
    if !step.eligible_tools.is_empty() && !step.eligible_tools.contains(&run.tool_id) {
        findings.push(FabMesValidationFinding::error(format!(
            "MES traveler {traveler_id} {label} uses ineligible tool {} for step {}",
            run.tool_id, run.step_id
        )));
    }
    if run.operator.trim().is_empty() {
        findings.push(FabMesValidationFinding::warning(format!(
            "MES traveler {traveler_id} {label} has no operator"
        )));
    }
}

fn traveler_can_carry_state(
    status: &TravelerStatus,
    hold: &Option<HoldState>,
    active_status: TravelerStatus,
) -> bool {
    status == &active_status
        || matches!(
            (status, hold),
            (TravelerStatus::OnHold, Some(hold)) if hold.previous_status == active_status
        )
}

fn validate_optional_step_ref(
    route: &ProcessRoute,
    step_id: Option<&ProcessStepId>,
    label: String,
    findings: &mut Vec<FabMesValidationFinding>,
) {
    if let Some(step_id) = step_id {
        validate_step_ref(route, step_id, label, findings);
    }
}

fn validate_step_ref(
    route: &ProcessRoute,
    step_id: &ProcessStepId,
    label: String,
    findings: &mut Vec<FabMesValidationFinding>,
) {
    if route.step(step_id).is_none() {
        findings.push(FabMesValidationFinding::error(format!(
            "{label} {step_id} is missing from route {}",
            route.id
        )));
    }
}

pub fn sample_fab_data() -> FabMesData {
    let route = demo_process_route();
    let route_id = route.id.clone();
    let first_step = route
        .first_step_id()
        .cloned()
        .expect("demo route has at least one step");
    let mut lot = Lot::new_25_wafer_lot("L-00042", "demo inverter poly loop", &route);
    let lot_id = lot.id.clone();
    let mut traveler = TravelerState::new(lot_id.clone(), route_id.clone(), first_step.clone());

    traveler
        .apply_action(
            &mut lot,
            &route,
            OperatorAction::StartStep {
                step_id: first_step.clone(),
                tool_id: ToolId::new("TRACK-01"),
                tool_class: ToolClass::LithographyTrack,
                recipe_id: RecipeId::new("SPIN_PR_3000"),
                operator: "op.martinez".to_string(),
            },
        )
        .expect("demo coat step starts");
    traveler
        .apply_action(
            &mut lot,
            &route,
            OperatorAction::CompleteStep {
                step_id: first_step,
                operator: "op.martinez".to_string(),
            },
        )
        .expect("demo coat step completes");

    let mut routes = BTreeMap::new();
    routes.insert(route_id, route);
    let mut lots = BTreeMap::new();
    lots.insert(lot_id.clone(), lot);
    let mut travelers = BTreeMap::new();
    travelers.insert(lot_id, traveler);
    FabMesData {
        routes,
        lots,
        travelers,
    }
}

pub fn demo_process_route() -> ProcessRoute {
    ProcessRoute {
        id: ProcessRouteId::new("ROUTE-DEMO-INVERTER-POLY-A"),
        name: "Fabricad demo inverter poly module".to_string(),
        revision: "A.1".to_string(),
        mask_design_id: "FABRICAD-DEMO-INVERTER".to_string(),
        layout_revision: "layout_model:demo-inverter@rev-5".to_string(),
        steps: vec![
            ProcessStep {
                id: ProcessStepId::new("S010-COAT"),
                sequence: 10,
                name: "Coat PR and soft bake".to_string(),
                area: "lithography".to_string(),
                required_tool_class: ToolClass::LithographyTrack,
                required_recipe: RecipeId::new("SPIN_PR_3000"),
                eligible_tools: vec![ToolId::new("TRACK-01"), ToolId::new("TRACK-02")],
                signoff_required: false,
                rework_allowed: true,
            },
            ProcessStep {
                id: ProcessStepId::new("S020-EXPOSE"),
                sequence: 20,
                name: "Align and expose poly mask".to_string(),
                area: "lithography".to_string(),
                required_tool_class: ToolClass::MaskAligner,
                required_recipe: RecipeId::new("LITHO_POLY_EXPOSE_001"),
                eligible_tools: vec![ToolId::new("ALIGNER-01")],
                signoff_required: true,
                rework_allowed: true,
            },
            ProcessStep {
                id: ProcessStepId::new("S030-DEVELOP"),
                sequence: 30,
                name: "Develop and inspect resist".to_string(),
                area: "lithography".to_string(),
                required_tool_class: ToolClass::LithographyTrack,
                required_recipe: RecipeId::new("LITHO_DEVELOP_001"),
                eligible_tools: vec![ToolId::new("TRACK-01"), ToolId::new("TRACK-02")],
                signoff_required: false,
                rework_allowed: true,
            },
            ProcessStep {
                id: ProcessStepId::new("S040-ETCH"),
                sequence: 40,
                name: "Poly plasma etch".to_string(),
                area: "etch".to_string(),
                required_tool_class: ToolClass::PlasmaEtcher,
                required_recipe: RecipeId::new("ETCH_CF4_POLY_001"),
                eligible_tools: vec![ToolId::new("ETCH-01")],
                signoff_required: true,
                rework_allowed: false,
            },
            ProcessStep {
                id: ProcessStepId::new("S050-CD-METRO"),
                sequence: 50,
                name: "Poly CD metrology".to_string(),
                area: "metrology".to_string(),
                required_tool_class: ToolClass::CdMetrology,
                required_recipe: RecipeId::new("METRO_POLY_CD_001"),
                eligible_tools: vec![ToolId::new("CDSEM-01")],
                signoff_required: false,
                rework_allowed: false,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> (ProcessRoute, Lot, TravelerState) {
        let data = sample_fab_data();
        let lot_id = LotId::new("L-00042");
        let lot = data.lots.get(&lot_id).cloned().unwrap();
        let traveler = data.travelers.get(&lot_id).cloned().unwrap();
        let route = data.routes.get(&lot.route_id).cloned().unwrap();
        (route, lot, traveler)
    }

    fn current_start_action(route: &ProcessRoute, traveler: &TravelerState) -> OperatorAction {
        let step = traveler.current_step(route).unwrap();
        OperatorAction::StartStep {
            step_id: step.id.clone(),
            tool_id: step.primary_tool().unwrap().clone(),
            tool_class: step.required_tool_class,
            recipe_id: step.required_recipe.clone(),
            operator: "op.test".to_string(),
        }
    }

    fn has_validation_error(findings: &[FabMesValidationFinding], needle: &str) -> bool {
        findings.iter().any(|finding| {
            finding.severity == FabMesValidationSeverity::Error && finding.message.contains(needle)
        })
    }

    fn has_validation_warning(findings: &[FabMesValidationFinding], needle: &str) -> bool {
        findings.iter().any(|finding| {
            finding.severity == FabMesValidationSeverity::Warning
                && finding.message.contains(needle)
        })
    }

    #[test]
    fn fab_mes_validation_accepts_sample_data() {
        let data = sample_fab_data();
        let findings = data.validate();

        assert!(findings.is_empty(), "{findings:?}");
        assert!(data.is_valid());
    }

    #[test]
    fn fab_mes_validation_rejects_broken_lot_route() {
        let mut data = sample_fab_data();
        let lot_id = LotId::new("L-00042");
        data.lots.get_mut(&lot_id).unwrap().route_id = ProcessRouteId::new("ROUTE-MISSING");

        let findings = data.validate();

        assert!(has_validation_error(
            &findings,
            "MES lot L-00042 references missing route ROUTE-MISSING"
        ));
        assert!(!data.is_valid());
    }

    #[test]
    fn fab_mes_validation_rejects_mismatched_traveler_key_and_step() {
        let mut data = sample_fab_data();
        let lot_id = LotId::new("L-00042");
        let mut traveler = data.travelers.remove(&lot_id).unwrap();
        traveler.current_step_id = Some(ProcessStepId::new("S999-MISSING"));
        data.travelers
            .insert(LotId::new("L-TRAVELER-KEY-MISMATCH"), traveler);

        let findings = data.validate();

        assert!(has_validation_error(
            &findings,
            "MES traveler key L-TRAVELER-KEY-MISMATCH does not match traveler lot L-00042"
        ));
        assert!(has_validation_error(
            &findings,
            "MES traveler L-TRAVELER-KEY-MISMATCH current step S999-MISSING"
        ));
    }

    #[test]
    fn fab_mes_validation_rejects_empty_ids_and_invalid_completed_steps() {
        let mut data = sample_fab_data();
        let route = data
            .routes
            .get_mut(&ProcessRouteId::new("ROUTE-DEMO-INVERTER-POLY-A"))
            .unwrap();
        route.steps[0].sequence = 0;
        route.steps[0].eligible_tools.push(ToolId::new(""));
        let lot_id = LotId::new("L-00042");
        let lot = data.lots.get_mut(&lot_id).unwrap();
        lot.wafers[0].id = WaferId::new("");
        lot.wafers[0].slot = 0;
        lot.wafers[1].status = WaferStatus::Scrapped {
            step_id: Some(ProcessStepId::new("S010-COAT")),
            reason: String::new(),
        };
        lot.wafers[2].status = WaferStatus::InRework {
            from_step: Some(ProcessStepId::new("S010-COAT")),
            target_step: ProcessStepId::new("S010-COAT"),
            reason: String::new(),
        };
        let traveler = data.travelers.get_mut(&lot_id).unwrap();
        traveler.completed_steps[0].recipe_id = RecipeId::new("ETCH_CF4_POLY_001");
        traveler.completed_steps[0].tool_id = ToolId::new("ETCH-99");
        traveler.completed_steps[0].signed_off_by = Some("qa.lead".to_string());
        traveler.completed_steps[0].reworked_wafers = 26;
        traveler.completed_steps.push(CompletedStep {
            step_id: ProcessStepId::new("S020-EXPOSE"),
            tool_id: ToolId::new("ALIGNER-01"),
            recipe_id: RecipeId::new("LITHO_POLY_EXPOSE_001"),
            completed_by: "op.litho".to_string(),
            signed_off_by: None,
            reworked_wafers: 0,
        });

        let findings = data.validate();

        assert!(has_validation_error(
            &findings,
            "MES route ROUTE-DEMO-INVERTER-POLY-A step S010-COAT has invalid sequence 0"
        ));
        assert!(has_validation_error(
            &findings,
            "MES route ROUTE-DEMO-INVERTER-POLY-A step S010-COAT has an empty eligible tool id"
        ));
        assert!(has_validation_error(
            &findings,
            "MES lot L-00042 contains an empty wafer id"
        ));
        assert!(has_validation_error(
            &findings,
            "MES lot L-00042 wafer  has invalid slot 0"
        ));
        assert!(has_validation_error(
            &findings,
            "MES traveler L-00042 completed step S010-COAT used recipe ETCH_CF4_POLY_001"
        ));
        assert!(has_validation_error(
            &findings,
            "MES traveler L-00042 completed step S010-COAT used ineligible tool ETCH-99"
        ));
        assert!(has_validation_error(
            &findings,
            "MES traveler L-00042 completed step S010-COAT reports 26 reworked wafers for 25 wafer lot"
        ));
        assert!(has_validation_error(
            &findings,
            "MES traveler L-00042 completed signoff step S020-EXPOSE has no signoff operator"
        ));
        assert!(has_validation_warning(
            &findings,
            "MES lot L-00042 wafer L-00042-W02 is scrapped without a reason"
        ));
        assert!(has_validation_warning(
            &findings,
            "MES lot L-00042 wafer L-00042-W03 is in rework without a reason"
        ));
        assert!(has_validation_warning(
            &findings,
            "MES traveler L-00042 completed non-signoff step S010-COAT carries signoff metadata"
        ));
    }

    #[test]
    fn fab_mes_validation_rejects_impossible_active_state() {
        let mut data = sample_fab_data();
        let lot_id = LotId::new("L-00042");
        let traveler = data.travelers.get_mut(&lot_id).unwrap();
        traveler.status = TravelerStatus::Running;
        traveler.active_run = Some(ActiveStepRun {
            step_id: ProcessStepId::new("S010-COAT"),
            tool_id: ToolId::new("TRACK-01"),
            tool_class: ToolClass::LithographyTrack,
            recipe_id: RecipeId::new("SPIN_PR_3000"),
            operator: String::new(),
        });

        let findings = data.validate();

        assert!(has_validation_warning(
            &findings,
            "MES traveler L-00042 active run has no operator"
        ));

        let traveler = data.travelers.get_mut(&lot_id).unwrap();
        traveler.active_run = None;
        let findings = data.validate();
        assert!(has_validation_error(
            &findings,
            "MES traveler L-00042 is running without an active run"
        ));
    }

    #[test]
    fn cannot_skip_required_step() {
        let (route, mut lot, mut traveler) = sample_state();
        let etch = ProcessStepId::new("S040-ETCH");

        let err = traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::StartStep {
                    step_id: etch.clone(),
                    tool_id: ToolId::new("ETCH-01"),
                    tool_class: ToolClass::PlasmaEtcher,
                    recipe_id: RecipeId::new("ETCH_CF4_POLY_001"),
                    operator: "op.test".to_string(),
                },
            )
            .unwrap_err();

        assert_eq!(
            err,
            TravelerTransitionError::WrongStep {
                expected: ProcessStepId::new("S020-EXPOSE"),
                got: etch
            }
        );
        assert_eq!(
            traveler.current_step_id,
            Some(ProcessStepId::new("S020-EXPOSE"))
        );
        assert!(matches!(
            traveler.audit_events.last().unwrap().outcome,
            AuditOutcome::Rejected
        ));
    }

    #[test]
    fn cannot_run_wrong_tool_class_recipe_or_tool() {
        let (route, mut lot, mut traveler) = sample_state();
        let expose = ProcessStepId::new("S020-EXPOSE");

        let wrong_tool_class = traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::StartStep {
                    step_id: expose.clone(),
                    tool_id: ToolId::new("ALIGNER-01"),
                    tool_class: ToolClass::PlasmaEtcher,
                    recipe_id: RecipeId::new("LITHO_POLY_EXPOSE_001"),
                    operator: "op.test".to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(
            wrong_tool_class,
            TravelerTransitionError::ToolClassMismatch {
                expected: ToolClass::MaskAligner,
                got: ToolClass::PlasmaEtcher,
            }
        );

        let wrong_recipe = traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::StartStep {
                    step_id: expose.clone(),
                    tool_id: ToolId::new("ALIGNER-01"),
                    tool_class: ToolClass::MaskAligner,
                    recipe_id: RecipeId::new("ETCH_CF4_POLY_001"),
                    operator: "op.test".to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(
            wrong_recipe,
            TravelerTransitionError::RecipeMismatch {
                expected: RecipeId::new("LITHO_POLY_EXPOSE_001"),
                got: RecipeId::new("ETCH_CF4_POLY_001"),
            }
        );

        let wrong_tool = traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::StartStep {
                    step_id: expose.clone(),
                    tool_id: ToolId::new("ALIGNER-99"),
                    tool_class: ToolClass::MaskAligner,
                    recipe_id: RecipeId::new("LITHO_POLY_EXPOSE_001"),
                    operator: "op.test".to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(
            wrong_tool,
            TravelerTransitionError::ToolNotEligible {
                step_id: expose,
                tool_id: ToolId::new("ALIGNER-99"),
            }
        );
        assert_eq!(traveler.status, TravelerStatus::WaitingForStep);
    }

    #[test]
    fn hold_blocks_movement_until_release() {
        let (route, mut lot, mut traveler) = sample_state();

        traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::PlaceHold {
                    reason: "awaiting mask inspection".to_string(),
                    operator: "qa.lead".to_string(),
                },
            )
            .unwrap();
        assert_eq!(traveler.status, TravelerStatus::OnHold);

        let blocked = traveler
            .apply_action(&mut lot, &route, current_start_action(&route, &traveler))
            .unwrap_err();
        assert_eq!(blocked, TravelerTransitionError::TravelerHeld);

        traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::ReleaseHold {
                    operator: "qa.lead".to_string(),
                },
            )
            .unwrap();
        assert_eq!(traveler.status, TravelerStatus::WaitingForStep);

        traveler
            .apply_action(&mut lot, &route, current_start_action(&route, &traveler))
            .unwrap();
        assert_eq!(traveler.status, TravelerStatus::Running);
    }

    #[test]
    fn scrap_and_rework_paths_are_predictable() {
        let (route, mut lot, mut traveler) = sample_state();
        let scrapped_wafer = WaferId::new("L-00042-W25");
        let rework_wafer = WaferId::new("L-00042-W01");
        let expose = ProcessStepId::new("S020-EXPOSE");

        traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::ScrapWafer {
                    wafer_id: scrapped_wafer.clone(),
                    reason: "edge chip".to_string(),
                    operator: "op.test".to_string(),
                },
            )
            .unwrap();
        assert_eq!(lot.scrapped_wafer_count(), 1);
        assert_eq!(lot.processable_wafer_count(), 24);

        let scrapped_rework = traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::SendToRework {
                    wafer_ids: vec![scrapped_wafer.clone()],
                    target_step: expose.clone(),
                    reason: "should not work".to_string(),
                    operator: "op.test".to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(
            scrapped_rework,
            TravelerTransitionError::WaferScrapped(scrapped_wafer)
        );

        traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::SendToRework {
                    wafer_ids: vec![rework_wafer.clone()],
                    target_step: expose.clone(),
                    reason: "resist focus check".to_string(),
                    operator: "process.eng".to_string(),
                },
            )
            .unwrap();
        assert_eq!(traveler.current_step_id, Some(expose.clone()));
        assert_eq!(lot.rework_wafer_count(), 1);

        traveler
            .apply_action(&mut lot, &route, current_start_action(&route, &traveler))
            .unwrap();
        traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::CompleteStep {
                    step_id: expose.clone(),
                    operator: "op.test".to_string(),
                },
            )
            .unwrap();
        assert_eq!(traveler.status, TravelerStatus::WaitingForSignoff);

        traveler
            .apply_action(
                &mut lot,
                &route,
                OperatorAction::SignOff {
                    step_id: expose,
                    operator: "process.eng".to_string(),
                },
            )
            .unwrap();
        assert_eq!(
            traveler.current_step_id,
            Some(ProcessStepId::new("S030-DEVELOP"))
        );
        let wafer = lot.wafer(&rework_wafer).unwrap();
        assert_eq!(wafer.status, WaferStatus::Active);
        assert_eq!(wafer.rework_count, 1);
    }
}
