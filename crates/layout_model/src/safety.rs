use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SafetySensorId(pub String);

impl SafetySensorId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SafetySensorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for SafetySensorId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SafetySensorId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SafetyIncidentId(pub String);

impl SafetyIncidentId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for SafetyIncidentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for SafetyIncidentId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyDomain {
    Chemical,
    Gas,
    Exhaust,
    Door,
    EmergencyStop,
}

impl SafetyDomain {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chemical => "Chemical",
            Self::Gas => "Gas",
            Self::Exhaust => "Exhaust",
            Self::Door => "Door",
            Self::EmergencyStop => "Emergency stop",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetySeverity {
    Normal,
    Advisory,
    Warning,
    Critical,
}

impl SafetySeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Advisory => "advisory",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    pub fn fails_interlock(self) -> bool {
        matches!(self, Self::Warning | Self::Critical)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetySensorState {
    Normal,
    Warning,
    Failed,
    Bypassed,
}

impl SafetySensorState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warning => "warning",
            Self::Failed => "failed",
            Self::Bypassed => "bypassed",
        }
    }

    pub fn fails_interlock(self) -> bool {
        !matches!(self, Self::Normal)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlarmRouteTarget {
    EquipmentHost,
    Facilities,
    SafetyOfficer,
    Maintenance,
}

impl AlarmRouteTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::EquipmentHost => "Equipment host",
            Self::Facilities => "Facilities",
            Self::SafetyOfficer => "Safety officer",
            Self::Maintenance => "Maintenance",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Open,
    Contained,
    Closed,
}

impl IncidentStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Contained => "contained",
            Self::Closed => "closed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyAuditKind {
    SensorSample,
    AlarmRouted,
    ToolLockedOut,
    IncidentOpened,
    IncidentUpdated,
}

impl SafetyAuditKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::SensorSample => "sensor sample",
            Self::AlarmRouted => "alarm routed",
            Self::ToolLockedOut => "tool locked out",
            Self::IncidentOpened => "incident opened",
            Self::IncidentUpdated => "incident updated",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SafetyLimit {
    #[serde(default)]
    pub lower: Option<f64>,
    #[serde(default)]
    pub upper: Option<f64>,
}

impl SafetyLimit {
    pub fn new(lower: Option<f64>, upper: Option<f64>) -> Self {
        Self { lower, upper }
    }

    pub fn contains(&self, value: f64) -> bool {
        self.lower.is_none_or(|lower| value >= lower)
            && self.upper.is_none_or(|upper| value <= upper)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SafetySensor {
    pub id: SafetySensorId,
    pub name: String,
    pub domain: SafetyDomain,
    #[serde(default)]
    pub tool_id: Option<String>,
    pub state: SafetySensorState,
    pub severity: SafetySeverity,
    pub value: f64,
    pub unit: String,
    pub limit: SafetyLimit,
    pub required_for_interlock: bool,
    pub last_seen_s: u64,
    #[serde(default)]
    pub message: String,
}

impl SafetySensor {
    pub fn in_limit(&self) -> bool {
        self.limit.contains(self.value)
    }

    pub fn fails_interlock(&self) -> bool {
        self.required_for_interlock
            && (self.state.fails_interlock() || self.severity.fails_interlock() || !self.in_limit())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlarmRoute {
    pub domain: SafetyDomain,
    pub minimum_severity: SafetySeverity,
    pub target: AlarmRouteTarget,
    pub channel: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInterlock {
    pub tool_id: String,
    pub tool_name: String,
    pub required_sensors: Vec<SafetySensorId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolLockout {
    pub tool_id: String,
    pub tool_name: String,
    pub locked_out: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyIncident {
    pub id: SafetyIncidentId,
    pub opened_at: String,
    pub status: IncidentStatus,
    pub severity: SafetySeverity,
    pub domain: SafetyDomain,
    #[serde(default)]
    pub sensor_id: Option<SafetySensorId>,
    #[serde(default)]
    pub tool_id: Option<String>,
    pub summary: String,
    pub routed_to: Vec<AlarmRouteTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyAuditEvent {
    pub sequence: u64,
    pub timestamp: String,
    pub actor: String,
    pub kind: SafetyAuditKind,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SafetySummary {
    pub sensor_count: usize,
    pub active_condition_count: usize,
    pub locked_out_tool_count: usize,
    pub open_incident_count: usize,
    pub highest_severity: Option<SafetySeverity>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SafetySystem {
    pub sensors: Vec<SafetySensor>,
    #[serde(default)]
    pub alarm_routes: Vec<AlarmRoute>,
    #[serde(default)]
    pub tool_interlocks: Vec<ToolInterlock>,
    #[serde(default)]
    pub incidents: Vec<SafetyIncident>,
    #[serde(default)]
    pub audit_events: Vec<SafetyAuditEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafetyValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyValidationFinding {
    pub severity: SafetyValidationSeverity,
    pub message: String,
}

impl SafetyValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: SafetyValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: SafetyValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SafetyValidationContext {
    pub tool_ids: BTreeSet<String>,
}

impl SafetyValidationContext {
    pub fn from_equipment(equipment: &crate::equipment::EquipmentSimulator) -> Self {
        Self {
            tool_ids: equipment
                .tools()
                .map(|tool| tool.id.as_str().to_string())
                .collect(),
        }
    }

    fn contains_tool(&self, tool_id: &str) -> bool {
        self.tool_ids.contains(tool_id)
    }
}

impl SafetySystem {
    pub fn simulated_demo() -> Self {
        Self {
            sensors: vec![
                SafetySensor {
                    id: "gas-h2-ppm".into(),
                    name: "Hydrogen cabinet ppm".to_string(),
                    domain: SafetyDomain::Gas,
                    tool_id: Some("ETCH-01".to_string()),
                    state: SafetySensorState::Warning,
                    severity: SafetySeverity::Warning,
                    value: 18.0,
                    unit: "ppm".to_string(),
                    limit: SafetyLimit::new(Some(0.0), Some(10.0)),
                    required_for_interlock: true,
                    last_seen_s: 142,
                    message: "simulated high gas reading".to_string(),
                },
                SafetySensor {
                    id: "exhaust-etch-pa".into(),
                    name: "Etch exhaust static pressure".to_string(),
                    domain: SafetyDomain::Exhaust,
                    tool_id: Some("ETCH-01".to_string()),
                    state: SafetySensorState::Warning,
                    severity: SafetySeverity::Critical,
                    value: -62.0,
                    unit: "Pa".to_string(),
                    limit: SafetyLimit::new(None, Some(-80.0)),
                    required_for_interlock: true,
                    last_seen_s: 144,
                    message: "simulated exhaust flow below threshold".to_string(),
                },
                SafetySensor {
                    id: "door-litho-bay".into(),
                    name: "Lithography bay door".to_string(),
                    domain: SafetyDomain::Door,
                    tool_id: Some("ALIGN-01".to_string()),
                    state: SafetySensorState::Normal,
                    severity: SafetySeverity::Normal,
                    value: 0.0,
                    unit: "closed=0".to_string(),
                    limit: SafetyLimit::new(Some(0.0), Some(0.0)),
                    required_for_interlock: true,
                    last_seen_s: 145,
                    message: "closed".to_string(),
                },
                SafetySensor {
                    id: "chem-solvent-voc".into(),
                    name: "Solvent bench VOC".to_string(),
                    domain: SafetyDomain::Chemical,
                    tool_id: Some("COAT-01".to_string()),
                    state: SafetySensorState::Normal,
                    severity: SafetySeverity::Advisory,
                    value: 34.0,
                    unit: "ppm".to_string(),
                    limit: SafetyLimit::new(Some(0.0), Some(50.0)),
                    required_for_interlock: true,
                    last_seen_s: 145,
                    message: "within simulated process envelope".to_string(),
                },
                SafetySensor {
                    id: "estop-main".into(),
                    name: "Main emergency stop chain".to_string(),
                    domain: SafetyDomain::EmergencyStop,
                    tool_id: None,
                    state: SafetySensorState::Normal,
                    severity: SafetySeverity::Normal,
                    value: 0.0,
                    unit: "active=1".to_string(),
                    limit: SafetyLimit::new(Some(0.0), Some(0.0)),
                    required_for_interlock: true,
                    last_seen_s: 145,
                    message: "clear".to_string(),
                },
            ],
            alarm_routes: vec![
                AlarmRoute {
                    domain: SafetyDomain::Gas,
                    minimum_severity: SafetySeverity::Warning,
                    target: AlarmRouteTarget::SafetyOfficer,
                    channel: "simulated-page:safety".to_string(),
                },
                AlarmRoute {
                    domain: SafetyDomain::Exhaust,
                    minimum_severity: SafetySeverity::Warning,
                    target: AlarmRouteTarget::Facilities,
                    channel: "simulated-ticket:facilities".to_string(),
                },
                AlarmRoute {
                    domain: SafetyDomain::EmergencyStop,
                    minimum_severity: SafetySeverity::Critical,
                    target: AlarmRouteTarget::EquipmentHost,
                    channel: "simulated-host-interlock".to_string(),
                },
            ],
            tool_interlocks: vec![
                ToolInterlock {
                    tool_id: "ETCH-01".to_string(),
                    tool_name: "ICP etcher".to_string(),
                    required_sensors: vec![
                        "gas-h2-ppm".into(),
                        "exhaust-etch-pa".into(),
                        "estop-main".into(),
                    ],
                },
                ToolInterlock {
                    tool_id: "ALIGN-01".to_string(),
                    tool_name: "Mask aligner".to_string(),
                    required_sensors: vec!["door-litho-bay".into(), "estop-main".into()],
                },
                ToolInterlock {
                    tool_id: "COAT-01".to_string(),
                    tool_name: "Spin coater".to_string(),
                    required_sensors: vec!["chem-solvent-voc".into(), "estop-main".into()],
                },
            ],
            incidents: vec![
                SafetyIncident {
                    id: "INC-SIM-0007".into(),
                    opened_at: "2026-05-08T09:18:00Z".to_string(),
                    status: IncidentStatus::Open,
                    severity: SafetySeverity::Critical,
                    domain: SafetyDomain::Exhaust,
                    sensor_id: Some("exhaust-etch-pa".into()),
                    tool_id: Some("ETCH-01".to_string()),
                    summary: "Simulated etch exhaust interlock forced tool lockout".to_string(),
                    routed_to: vec![
                        AlarmRouteTarget::Facilities,
                        AlarmRouteTarget::EquipmentHost,
                    ],
                },
                SafetyIncident {
                    id: "INC-SIM-0006".into(),
                    opened_at: "2026-05-08T08:51:00Z".to_string(),
                    status: IncidentStatus::Contained,
                    severity: SafetySeverity::Warning,
                    domain: SafetyDomain::Gas,
                    sensor_id: Some("gas-h2-ppm".into()),
                    tool_id: Some("ETCH-01".to_string()),
                    summary: "Simulated gas cabinet warning routed for review".to_string(),
                    routed_to: vec![AlarmRouteTarget::SafetyOfficer],
                },
            ],
            audit_events: vec![
                SafetyAuditEvent {
                    sequence: 44,
                    timestamp: "2026-05-08T09:18:03Z".to_string(),
                    actor: "safety.simulator".to_string(),
                    kind: SafetyAuditKind::ToolLockedOut,
                    message: "ETCH-01 locked out by simulated exhaust and gas conditions"
                        .to_string(),
                },
                SafetyAuditEvent {
                    sequence: 43,
                    timestamp: "2026-05-08T09:18:00Z".to_string(),
                    actor: "safety.simulator".to_string(),
                    kind: SafetyAuditKind::IncidentOpened,
                    message: "Opened INC-SIM-0007 for exhaust interlock".to_string(),
                },
                SafetyAuditEvent {
                    sequence: 42,
                    timestamp: "2026-05-08T09:17:57Z".to_string(),
                    actor: "safety.simulator".to_string(),
                    kind: SafetyAuditKind::AlarmRouted,
                    message: "Routed exhaust alarm to facilities".to_string(),
                },
            ],
        }
    }

    pub fn active_conditions(&self) -> Vec<&SafetySensor> {
        self.sensors
            .iter()
            .filter(|sensor| {
                sensor.state.fails_interlock()
                    || sensor.severity.fails_interlock()
                    || !sensor.in_limit()
            })
            .collect()
    }

    pub fn route_targets_for(&self, sensor: &SafetySensor) -> Vec<AlarmRouteTarget> {
        self.alarm_routes
            .iter()
            .filter(|route| {
                route.domain == sensor.domain && sensor.severity >= route.minimum_severity
            })
            .map(|route| route.target)
            .collect()
    }

    pub fn evaluate_lockouts(&self) -> Vec<ToolLockout> {
        let sensors = self
            .sensors
            .iter()
            .map(|sensor| (sensor.id.clone(), sensor))
            .collect::<BTreeMap<_, _>>();
        let global_reasons = self
            .sensors
            .iter()
            .filter(|sensor| sensor.tool_id.is_none() && sensor.fails_interlock())
            .map(sensor_reason)
            .collect::<Vec<_>>();

        self.tool_interlocks
            .iter()
            .map(|interlock| {
                let mut reasons = global_reasons.clone();
                for sensor_id in &interlock.required_sensors {
                    match sensors.get(sensor_id) {
                        Some(sensor) if sensor.tool_id.is_none() => {}
                        Some(sensor) if sensor.fails_interlock() => {
                            reasons.push(sensor_reason(sensor))
                        }
                        Some(_) => {}
                        None => reasons.push(format!("missing required sensor {sensor_id}")),
                    }
                }
                reasons.sort();
                reasons.dedup();
                ToolLockout {
                    tool_id: interlock.tool_id.clone(),
                    tool_name: interlock.tool_name.clone(),
                    locked_out: !reasons.is_empty(),
                    reasons,
                }
            })
            .collect()
    }

    pub fn summary(&self) -> SafetySummary {
        let lockouts = self.evaluate_lockouts();
        SafetySummary {
            sensor_count: self.sensors.len(),
            active_condition_count: self.active_conditions().len(),
            locked_out_tool_count: lockouts.iter().filter(|lockout| lockout.locked_out).count(),
            open_incident_count: self
                .incidents
                .iter()
                .filter(|incident| incident.status == IncidentStatus::Open)
                .count(),
            highest_severity: self.sensors.iter().map(|sensor| sensor.severity).max(),
        }
    }

    pub fn validate(&self) -> Vec<SafetyValidationFinding> {
        let mut findings = Vec::new();
        let mut sensor_ids = BTreeSet::new();
        let mut sensor_domains = BTreeMap::new();
        let mut sensors_by_id = BTreeMap::new();
        for sensor in &self.sensors {
            if sensor.id.as_str().trim().is_empty() {
                findings.push(SafetyValidationFinding::error("safety sensor id is empty"));
                continue;
            }
            if !sensor_ids.insert(sensor.id.clone()) {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety sensor {} is duplicated",
                    sensor.id
                )));
            }
            sensor_domains.insert(sensor.id.clone(), sensor.domain);
            sensors_by_id.insert(sensor.id.clone(), sensor);
            if sensor.name.trim().is_empty() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety sensor {} has an empty name",
                    sensor.id
                )));
            }
            if sensor.unit.trim().is_empty() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety sensor {} has an empty unit",
                    sensor.id
                )));
            }
            if !sensor.value.is_finite() {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety sensor {} has non-finite value",
                    sensor.id
                )));
            }
            if sensor
                .tool_id
                .as_deref()
                .is_some_and(|tool_id| tool_id.trim().is_empty())
            {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety sensor {} has an empty equipment tool id",
                    sensor.id
                )));
            }
            if sensor.state == SafetySensorState::Normal && !sensor.in_limit() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety sensor {} is normal while outside configured limits",
                    sensor.id
                )));
            }
            if !sensor.in_limit() && sensor.severity == SafetySeverity::Normal {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety sensor {} is outside configured limits but severity is normal",
                    sensor.id
                )));
            }
            if sensor.severity.fails_interlock()
                && sensor.state == SafetySensorState::Normal
                && sensor.in_limit()
            {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety sensor {} has failed severity without failed state or limit excursion",
                    sensor.id
                )));
            }
            if sensor.fails_interlock() && sensor.message.trim().is_empty() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety sensor {} fails interlock without a message",
                    sensor.id
                )));
            }
            validate_limit(sensor, &mut findings);
        }

        let mut route_keys = BTreeSet::new();
        for route in &self.alarm_routes {
            if !route_keys.insert((route.domain, route.minimum_severity, route.target)) {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety alarm route for {} to {} is duplicated",
                    route.domain.label(),
                    route.target.label()
                )));
            }
            if route.channel.trim().is_empty() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety alarm route for {} to {} has an empty channel",
                    route.domain.label(),
                    route.target.label()
                )));
            }
        }

        let mut interlock_tool_ids = BTreeSet::new();
        for interlock in &self.tool_interlocks {
            if interlock.tool_id.trim().is_empty() {
                findings.push(SafetyValidationFinding::error(
                    "safety tool interlock has an empty tool id",
                ));
            } else if !interlock_tool_ids.insert(interlock.tool_id.clone()) {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety tool interlock {} is duplicated",
                    interlock.tool_id
                )));
            }
            if interlock.required_sensors.is_empty() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety tool interlock {} has no required sensors",
                    interlock.tool_id
                )));
            }
            if interlock.tool_name.trim().is_empty() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety tool interlock {} has an empty tool name",
                    interlock.tool_id
                )));
            }
            let mut required_sensor_ids = BTreeSet::new();
            for sensor_id in &interlock.required_sensors {
                if sensor_id.as_str().trim().is_empty() {
                    findings.push(SafetyValidationFinding::error(format!(
                        "safety tool interlock {} has an empty required sensor id",
                        interlock.tool_id
                    )));
                    continue;
                }
                if !required_sensor_ids.insert(sensor_id.clone()) {
                    findings.push(SafetyValidationFinding::error(format!(
                        "safety tool interlock {} has duplicate required sensor {}",
                        interlock.tool_id, sensor_id
                    )));
                }
                if !sensor_ids.contains(sensor_id) {
                    findings.push(SafetyValidationFinding::error(format!(
                        "safety tool interlock {} references missing sensor {}",
                        interlock.tool_id, sensor_id
                    )));
                }
            }
        }

        let mut incident_ids = BTreeSet::new();
        for incident in &self.incidents {
            if incident.id.0.trim().is_empty() {
                findings.push(SafetyValidationFinding::error(
                    "safety incident id is empty",
                ));
            } else if !incident_ids.insert(incident.id.clone()) {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety incident {} is duplicated",
                    incident.id
                )));
            }
            if !is_valid_safety_timestamp(&incident.opened_at) {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety incident {} has invalid opened timestamp {}",
                    incident.id, incident.opened_at
                )));
            }
            if let Some(sensor_id) = incident.sensor_id.as_ref() {
                if sensor_id.as_str().trim().is_empty() {
                    findings.push(SafetyValidationFinding::error(format!(
                        "safety incident {} has an empty sensor id",
                        incident.id
                    )));
                }
                match sensor_domains.get(sensor_id) {
                    Some(domain) if *domain != incident.domain => {
                        findings.push(SafetyValidationFinding::warning(format!(
                            "safety incident {} domain {} does not match sensor {} domain {}",
                            incident.id,
                            incident.domain.label(),
                            sensor_id,
                            domain.label()
                        )));
                    }
                    Some(_) => {}
                    None => findings.push(SafetyValidationFinding::error(format!(
                        "safety incident {} references missing sensor {}",
                        incident.id, sensor_id
                    ))),
                }
                if let Some(sensor) = sensors_by_id.get(sensor_id)
                    && sensor.severity > incident.severity
                {
                    findings.push(SafetyValidationFinding::warning(format!(
                        "safety incident {} severity {} is below linked sensor {} severity {}",
                        incident.id,
                        incident.severity.label(),
                        sensor_id,
                        sensor.severity.label()
                    )));
                }
            }
            if incident
                .tool_id
                .as_deref()
                .is_some_and(|tool_id| tool_id.trim().is_empty())
            {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety incident {} has an empty equipment tool id",
                    incident.id
                )));
            }
            if incident.summary.trim().is_empty() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety incident {} has an empty summary",
                    incident.id
                )));
            }
            if incident.status != IncidentStatus::Closed
                && incident.severity == SafetySeverity::Normal
            {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety incident {} is open or contained with normal severity",
                    incident.id
                )));
            }
            if incident.routed_to.is_empty() && incident.status != IncidentStatus::Closed {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety incident {} is not routed",
                    incident.id
                )));
            }
            let mut routed_to = BTreeSet::new();
            for target in &incident.routed_to {
                if !routed_to.insert(*target) {
                    findings.push(SafetyValidationFinding::warning(format!(
                        "safety incident {} repeats route target {}",
                        incident.id,
                        target.label()
                    )));
                }
            }
        }

        let mut audit_sequences = BTreeSet::new();
        for event in &self.audit_events {
            if event.sequence == 0 {
                findings.push(SafetyValidationFinding::error(
                    "safety audit sequence cannot be zero",
                ));
            }
            if !audit_sequences.insert(event.sequence) {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety audit sequence {} is duplicated",
                    event.sequence
                )));
            }
            if !is_valid_safety_timestamp(&event.timestamp) {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety audit sequence {} has invalid timestamp {}",
                    event.sequence, event.timestamp
                )));
            }
            if event.actor.trim().is_empty() || event.message.trim().is_empty() {
                findings.push(SafetyValidationFinding::warning(format!(
                    "safety audit sequence {} has incomplete audit metadata",
                    event.sequence
                )));
            }
        }

        findings
    }

    pub fn validate_with_context(
        &self,
        context: &SafetyValidationContext,
    ) -> Vec<SafetyValidationFinding> {
        let mut findings = self.validate();
        self.validate_context_links(context, &mut findings);
        findings
    }

    fn validate_context_links(
        &self,
        context: &SafetyValidationContext,
        findings: &mut Vec<SafetyValidationFinding>,
    ) {
        for sensor in &self.sensors {
            if let Some(tool_id) = sensor.tool_id.as_deref()
                && !tool_id.trim().is_empty()
                && !context.contains_tool(tool_id)
            {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety sensor {} references missing equipment tool {tool_id}",
                    sensor.id
                )));
            }
        }

        for interlock in &self.tool_interlocks {
            if !interlock.tool_id.trim().is_empty() && !context.contains_tool(&interlock.tool_id) {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety tool interlock {} references missing equipment tool",
                    interlock.tool_id
                )));
            }
        }

        for incident in &self.incidents {
            if let Some(tool_id) = incident.tool_id.as_deref()
                && !tool_id.trim().is_empty()
                && !context.contains_tool(tool_id)
            {
                findings.push(SafetyValidationFinding::error(format!(
                    "safety incident {} references missing equipment tool {tool_id}",
                    incident.id
                )));
            }
        }
    }
}

fn sensor_reason(sensor: &SafetySensor) -> String {
    format!(
        "{}: {} ({})",
        sensor.domain.label(),
        sensor.name,
        sensor.severity.label()
    )
}

fn validate_limit(sensor: &SafetySensor, findings: &mut Vec<SafetyValidationFinding>) {
    if sensor.limit.lower.is_some_and(|value| !value.is_finite()) {
        findings.push(SafetyValidationFinding::error(format!(
            "safety sensor {} has non-finite lower limit",
            sensor.id
        )));
    }
    if sensor.limit.upper.is_some_and(|value| !value.is_finite()) {
        findings.push(SafetyValidationFinding::error(format!(
            "safety sensor {} has non-finite upper limit",
            sensor.id
        )));
    }
    if let (Some(lower), Some(upper)) = (sensor.limit.lower, sensor.limit.upper)
        && lower > upper
    {
        findings.push(SafetyValidationFinding::error(format!(
            "safety sensor {} lower limit is above upper limit",
            sensor.id
        )));
    }
}

fn is_valid_safety_timestamp(timestamp: &str) -> bool {
    if timestamp.len() != 20 || !timestamp.ends_with('Z') {
        return false;
    }
    let bytes = timestamp.as_bytes();
    if bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return false;
    }
    for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }
    let year = parse_timestamp_field(timestamp, 0, 4);
    let month = parse_timestamp_field(timestamp, 5, 7);
    let day = parse_timestamp_field(timestamp, 8, 10);
    let hour = parse_timestamp_field(timestamp, 11, 13);
    let minute = parse_timestamp_field(timestamp, 14, 16);
    let second = parse_timestamp_field(timestamp, 17, 19);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };

    (1900..=9999).contains(&year)
        && (1..=max_day).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

fn parse_timestamp_field(timestamp: &str, start: usize, end: usize) -> u32 {
    timestamp[start..end].parse().unwrap_or_default()
}

fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_demo_has_visible_safety_conditions() {
        let model = SafetySystem::simulated_demo();
        let summary = model.summary();

        assert!(summary.sensor_count >= 5);
        assert!(summary.active_condition_count > 0);
        assert!(summary.locked_out_tool_count > 0);
        assert!(summary.open_incident_count > 0);
    }

    #[test]
    fn failed_required_sensor_locks_only_linked_tool_when_not_global() {
        let mut model = SafetySystem::simulated_demo();
        for sensor in &mut model.sensors {
            sensor.state = SafetySensorState::Normal;
            sensor.severity = SafetySeverity::Normal;
            sensor.value = match (sensor.limit.lower, sensor.limit.upper) {
                (Some(lower), Some(upper)) => (lower + upper) / 2.0,
                (Some(lower), None) => lower,
                (None, Some(upper)) => upper,
                (None, None) => 0.0,
            };
        }
        let gas = model
            .sensors
            .iter_mut()
            .find(|sensor| sensor.id.as_str() == "gas-h2-ppm")
            .unwrap();
        gas.state = SafetySensorState::Warning;
        gas.severity = SafetySeverity::Warning;
        gas.value = 18.0;

        let lockouts = model.evaluate_lockouts();

        assert!(
            lockouts
                .iter()
                .any(|lockout| lockout.tool_id == "ETCH-01" && lockout.locked_out)
        );
        assert!(
            lockouts
                .iter()
                .any(|lockout| lockout.tool_id == "ALIGN-01" && !lockout.locked_out)
        );
    }

    #[test]
    fn simulated_safety_system_validates() {
        let findings = SafetySystem::simulated_demo().validate();

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn validation_rejects_stale_sensor_incident_and_audit_metadata() {
        assert!(is_valid_safety_timestamp("2026-05-08T09:18:00Z"));
        assert!(!is_valid_safety_timestamp("2026-02-30T09:18:00Z"));
        assert!(!is_valid_safety_timestamp("2026-05-08T25:18:00Z"));

        let mut model = SafetySystem::simulated_demo();
        model.sensors[0].state = SafetySensorState::Normal;
        model.sensors[0].severity = SafetySeverity::Normal;
        model.sensors[0].tool_id = Some(String::new());
        model.sensors[0].message.clear();
        model.sensors[1].state = SafetySensorState::Normal;
        model.sensors[1].value = -90.0;
        model.alarm_routes.push(model.alarm_routes[0].clone());
        model.tool_interlocks[0].tool_name.clear();
        model.tool_interlocks[0].required_sensors.push("".into());
        model.incidents[0].opened_at = "2026-02-30T09:18:00Z".to_string();
        model.incidents[0].severity = SafetySeverity::Warning;
        model.incidents[0].tool_id = Some(String::new());
        let repeated_route_target = model.incidents[0].routed_to[0];
        model.incidents[0].routed_to.push(repeated_route_target);
        model.incidents[1].severity = SafetySeverity::Normal;
        model.audit_events[0].sequence = 0;
        model.audit_events[0].timestamp = "bad-timestamp".to_string();

        let findings = model.validate();

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty equipment tool id")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("normal while outside configured limits")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("outside configured limits but severity is normal")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("fails interlock without a message")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("failed severity without failed state or limit excursion")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("alarm route for Gas")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty tool name")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty required sensor id")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("invalid opened timestamp")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("severity warning is below linked sensor")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("repeats route target Facilities")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("normal severity")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("audit sequence cannot be zero")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("invalid timestamp bad-timestamp")),
            "{findings:?}"
        );
    }

    #[test]
    fn simulated_safety_system_validates_against_equipment_context() {
        let context = SafetyValidationContext::from_equipment(
            &crate::equipment::EquipmentSimulator::demo_fab(),
        );
        let findings = SafetySystem::simulated_demo().validate_with_context(&context);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn validation_context_rejects_missing_equipment_tools() {
        let context = SafetyValidationContext::from_equipment(
            &crate::equipment::EquipmentSimulator::demo_fab(),
        );
        let mut model = SafetySystem::simulated_demo();
        model.sensors[0].tool_id = Some("MISSING-SENSOR-TOOL".to_string());
        model.tool_interlocks[0].tool_id = "MISSING-INTERLOCK-TOOL".to_string();
        model.incidents[0].tool_id = Some("MISSING-INCIDENT-TOOL".to_string());

        let findings = model.validate_with_context(&context);

        assert!(
            findings.iter().any(|finding| {
                finding
                    .message
                    .contains("safety sensor gas-h2-ppm references missing equipment tool")
            }),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| {
                finding
                    .message
                    .contains("safety tool interlock MISSING-INTERLOCK-TOOL")
            }),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| {
                finding
                    .message
                    .contains("safety incident INC-SIM-0007 references missing equipment tool")
            }),
            "{findings:?}"
        );
    }

    #[test]
    fn validation_rejects_broken_sensors_interlocks_and_incidents() {
        let mut model = SafetySystem::simulated_demo();
        model.sensors[0].value = f64::NAN;
        model.sensors[1].limit = SafetyLimit::new(Some(10.0), Some(1.0));
        model.tool_interlocks[0]
            .required_sensors
            .push("missing-sensor".into());
        let duplicated_sensor = model.tool_interlocks[0].required_sensors[0].clone();
        model.tool_interlocks[0]
            .required_sensors
            .push(duplicated_sensor);
        model.incidents.push(SafetyIncident {
            id: "INC-BAD".into(),
            opened_at: "2026-05-08T10:00:00Z".to_string(),
            status: IncidentStatus::Open,
            severity: SafetySeverity::Warning,
            domain: SafetyDomain::Gas,
            sensor_id: Some("missing-sensor".into()),
            tool_id: Some("ETCH-01".to_string()),
            summary: String::new(),
            routed_to: Vec::new(),
        });

        let findings = model.validate();

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("non-finite value")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("lower limit is above upper limit")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("missing sensor")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("duplicate required sensor")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("is not routed")),
            "{findings:?}"
        );
    }
}
