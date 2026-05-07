use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

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
                    .unwrap_or_else(|| ProcessStepId::new("unknown")),
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
                    .unwrap_or_else(|| ProcessStepId::new("unknown")),
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

impl FabMesData {
    pub fn sample() -> Self {
        sample_fab_data()
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
                recipe_id: RecipeId::new("LITHO_COAT_PR_001"),
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
                required_recipe: RecipeId::new("LITHO_COAT_PR_001"),
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
                required_recipe: RecipeId::new("ETCH_POLY_CF4_001"),
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
                    recipe_id: RecipeId::new("ETCH_POLY_CF4_001"),
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
                    recipe_id: RecipeId::new("ETCH_POLY_CF4_001"),
                    operator: "op.test".to_string(),
                },
            )
            .unwrap_err();
        assert_eq!(
            wrong_recipe,
            TravelerTransitionError::RecipeMismatch {
                expected: RecipeId::new("LITHO_POLY_EXPOSE_001"),
                got: RecipeId::new("ETCH_POLY_CF4_001"),
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
