use std::{collections::BTreeMap, fmt};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

fn sensor_reason(sensor: &SafetySensor) -> String {
    format!(
        "{}: {} ({})",
        sensor.domain.label(),
        sensor.name,
        sensor.severity.label()
    )
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
}
