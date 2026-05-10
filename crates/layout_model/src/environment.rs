use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EnvironmentSensorKind {
    Temperature,
    Humidity,
    ParticleCount,
    Vibration,
    AirPressure,
    ChemicalCabinet,
    GasCabinet,
    DiWaterResistivity,
    ExhaustFlow,
}

impl EnvironmentSensorKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Temperature => "Temperature",
            Self::Humidity => "Humidity",
            Self::ParticleCount => "Particle count",
            Self::Vibration => "Vibration",
            Self::AirPressure => "Air pressure",
            Self::ChemicalCabinet => "Chemical cabinet",
            Self::GasCabinet => "Gas cabinet",
            Self::DiWaterResistivity => "DI water resistivity",
            Self::ExhaustFlow => "Exhaust flow",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EnvironmentAlarmSeverity {
    Advisory,
    Warning,
    Critical,
}

impl EnvironmentAlarmSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Advisory => "Advisory",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentThresholds {
    #[serde(default)]
    pub warning_low: Option<f64>,
    #[serde(default)]
    pub warning_high: Option<f64>,
    #[serde(default)]
    pub critical_low: Option<f64>,
    #[serde(default)]
    pub critical_high: Option<f64>,
}

impl EnvironmentThresholds {
    pub fn new(
        warning_low: Option<f64>,
        warning_high: Option<f64>,
        critical_low: Option<f64>,
        critical_high: Option<f64>,
    ) -> Self {
        Self {
            warning_low,
            warning_high,
            critical_low,
            critical_high,
        }
    }

    pub fn evaluate(&self, value: f64) -> Option<EnvironmentAlarmSeverity> {
        if self.critical_low.is_some_and(|limit| value < limit)
            || self.critical_high.is_some_and(|limit| value > limit)
        {
            return Some(EnvironmentAlarmSeverity::Critical);
        }
        if self.warning_low.is_some_and(|limit| value < limit)
            || self.warning_high.is_some_and(|limit| value > limit)
        {
            return Some(EnvironmentAlarmSeverity::Warning);
        }
        None
    }

    pub fn nominal_label(&self, unit: &str) -> String {
        match (self.warning_low, self.warning_high) {
            (Some(low), Some(high)) => format!("{low:.2}..{high:.2} {unit}"),
            (Some(low), None) => format!(">= {low:.2} {unit}"),
            (None, Some(high)) => format!("<= {high:.2} {unit}"),
            (None, None) => "unbounded".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentSensor {
    pub id: String,
    pub name: String,
    pub zone: String,
    pub kind: EnvironmentSensorKind,
    pub unit: String,
    pub thresholds: EnvironmentThresholds,
    #[serde(default)]
    pub process_tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentReading {
    pub sensor_id: String,
    pub timestamp_min: u32,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentAlarm {
    pub id: String,
    pub sensor_id: String,
    pub severity: EnvironmentAlarmSeverity,
    pub timestamp_min: u32,
    pub value: f64,
    pub message: String,
    pub active: bool,
    #[serde(default)]
    pub correlation_labels: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Falling,
    Stable,
    Rising,
}

impl TrendDirection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Falling => "falling",
            Self::Stable => "stable",
            Self::Rising => "rising",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentTrendSummary {
    pub sensor_id: String,
    pub latest_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub average_value: f64,
    pub direction: TrendDirection,
    pub excursion_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentCorrelationLabel {
    pub id: String,
    pub label: String,
    pub process_area: String,
    pub description: String,
    pub sensor_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FacilityEventKind {
    Maintenance,
    Excursion,
    Recovery,
    ProcessHold,
}

impl FacilityEventKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Maintenance => "Maintenance",
            Self::Excursion => "Excursion",
            Self::Recovery => "Recovery",
            Self::ProcessHold => "Process hold",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FacilityEvent {
    pub timestamp_min: u32,
    pub zone: String,
    pub kind: FacilityEventKind,
    pub title: String,
    pub detail: String,
    #[serde(default)]
    pub linked_alarm_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CleanroomEnvironment {
    pub sensors: Vec<EnvironmentSensor>,
    pub readings: Vec<EnvironmentReading>,
    pub alarms: Vec<EnvironmentAlarm>,
    pub correlations: Vec<EnvironmentCorrelationLabel>,
    pub events: Vec<FacilityEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvironmentValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentValidationFinding {
    pub severity: EnvironmentValidationSeverity,
    pub message: String,
}

impl EnvironmentValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: EnvironmentValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: EnvironmentValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

impl CleanroomEnvironment {
    pub fn new(
        sensors: Vec<EnvironmentSensor>,
        readings: Vec<EnvironmentReading>,
        correlations: Vec<EnvironmentCorrelationLabel>,
        events: Vec<FacilityEvent>,
    ) -> Self {
        let mut model = Self {
            sensors,
            readings,
            alarms: Vec::new(),
            correlations,
            events,
        };
        model.refresh_alarms();
        model
    }

    pub fn refresh_alarms(&mut self) {
        self.alarms = self.evaluate_alarms();
    }

    pub fn evaluate_alarms(&self) -> Vec<EnvironmentAlarm> {
        let sensors_by_id = self
            .sensors
            .iter()
            .map(|sensor| (sensor.id.as_str(), sensor))
            .collect::<BTreeMap<_, _>>();
        let mut alarms = Vec::new();
        for reading in &self.readings {
            let Some(sensor) = sensors_by_id.get(reading.sensor_id.as_str()) else {
                continue;
            };
            let Some(severity) = sensor.thresholds.evaluate(reading.value) else {
                continue;
            };
            let labels = self
                .correlations
                .iter()
                .filter(|label| label.sensor_ids.iter().any(|id| id == &sensor.id))
                .map(|label| label.label.clone())
                .collect::<Vec<_>>();
            alarms.push(EnvironmentAlarm {
                id: format!("env-{}-{}", sensor.id, reading.timestamp_min),
                sensor_id: sensor.id.clone(),
                severity,
                timestamp_min: reading.timestamp_min,
                value: reading.value,
                message: format!(
                    "{} in {} measured {:.2} {}, outside {}",
                    sensor.name,
                    sensor.zone,
                    reading.value,
                    sensor.unit,
                    sensor.thresholds.nominal_label(&sensor.unit)
                ),
                active: self.latest_reading(&sensor.id) == Some(reading),
                correlation_labels: labels,
            });
        }
        alarms.sort_by(|left, right| {
            right
                .severity
                .cmp(&left.severity)
                .then_with(|| right.timestamp_min.cmp(&left.timestamp_min))
        });
        alarms
    }

    pub fn active_alarms(&self) -> Vec<&EnvironmentAlarm> {
        self.alarms.iter().filter(|alarm| alarm.active).collect()
    }

    pub fn latest_reading(&self, sensor_id: &str) -> Option<&EnvironmentReading> {
        self.readings
            .iter()
            .filter(|reading| reading.sensor_id == sensor_id)
            .max_by_key(|reading| reading.timestamp_min)
    }

    pub fn readings_for_sensor(&self, sensor_id: &str) -> Vec<&EnvironmentReading> {
        let mut readings = self
            .readings
            .iter()
            .filter(|reading| reading.sensor_id == sensor_id)
            .collect::<Vec<_>>();
        readings.sort_by_key(|reading| reading.timestamp_min);
        readings
    }

    pub fn trend_summary(&self, sensor_id: &str) -> Option<EnvironmentTrendSummary> {
        let readings = self.readings_for_sensor(sensor_id);
        let latest = readings.last()?;
        let min_value = readings
            .iter()
            .map(|reading| reading.value)
            .fold(f64::INFINITY, f64::min);
        let max_value = readings
            .iter()
            .map(|reading| reading.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let average_value =
            readings.iter().map(|reading| reading.value).sum::<f64>() / readings.len() as f64;
        let direction = readings
            .first()
            .map(|first| latest.value - first.value)
            .map(|delta| {
                if delta.abs() < 0.05 {
                    TrendDirection::Stable
                } else if delta > 0.0 {
                    TrendDirection::Rising
                } else {
                    TrendDirection::Falling
                }
            })
            .unwrap_or(TrendDirection::Stable);
        let sensor = self.sensors.iter().find(|sensor| sensor.id == sensor_id)?;
        let excursion_count = readings
            .iter()
            .filter(|reading| sensor.thresholds.evaluate(reading.value).is_some())
            .count();
        Some(EnvironmentTrendSummary {
            sensor_id: sensor_id.to_string(),
            latest_value: latest.value,
            min_value,
            max_value,
            average_value,
            direction,
            excursion_count,
        })
    }

    pub fn validate(&self) -> Vec<EnvironmentValidationFinding> {
        let mut findings = Vec::new();
        let mut sensor_ids = BTreeSet::new();
        for sensor in &self.sensors {
            if sensor.id.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::error(
                    "environment sensor id is empty",
                ));
                continue;
            }
            if !sensor_ids.insert(sensor.id.clone()) {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment sensor {} is duplicated",
                    sensor.id
                )));
            }
            if sensor.name.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment sensor {} has an empty name",
                    sensor.id
                )));
            }
            if sensor.zone.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment sensor {} has an empty zone",
                    sensor.id
                )));
            }
            if sensor.unit.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment sensor {} has an empty unit",
                    sensor.id
                )));
            }
            validate_thresholds(sensor, &mut findings);
        }

        let mut reading_keys = BTreeSet::new();
        let mut readings_by_key = BTreeMap::new();
        for reading in &self.readings {
            let reading_key = (reading.sensor_id.clone(), reading.timestamp_min);
            if !reading_keys.insert(reading_key.clone()) {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment has duplicate reading for sensor {} at {}",
                    reading.sensor_id, reading.timestamp_min
                )));
            } else {
                readings_by_key.insert(reading_key, reading);
            }
            if !sensor_ids.contains(&reading.sensor_id) {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment reading at {} references missing sensor {}",
                    reading.timestamp_min, reading.sensor_id
                )));
            }
            if !reading.value.is_finite() {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment reading for sensor {} has non-finite value",
                    reading.sensor_id
                )));
            }
        }

        let mut alarm_ids = BTreeSet::new();
        let latest_readings = self
            .readings
            .iter()
            .map(|reading| (reading.sensor_id.as_str(), reading))
            .fold(BTreeMap::new(), |mut latest, (sensor_id, reading)| {
                latest
                    .entry(sensor_id)
                    .and_modify(|current: &mut &EnvironmentReading| {
                        if reading.timestamp_min > current.timestamp_min {
                            *current = reading;
                        }
                    })
                    .or_insert(reading);
                latest
            });
        for alarm in &self.alarms {
            if alarm.id.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::error(
                    "environment alarm id is empty",
                ));
            } else if !alarm_ids.insert(alarm.id.clone()) {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment alarm {} is duplicated",
                    alarm.id
                )));
            }
            if !sensor_ids.contains(&alarm.sensor_id) {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment alarm {} references missing sensor {}",
                    alarm.id, alarm.sensor_id
                )));
            }
            if !alarm.value.is_finite() {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment alarm {} has non-finite value",
                    alarm.id
                )));
            }
            if alarm.message.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment alarm {} has an empty message",
                    alarm.id
                )));
            }
            let expected_active = latest_readings
                .get(alarm.sensor_id.as_str())
                .is_some_and(|reading| reading.timestamp_min == alarm.timestamp_min);
            if alarm.active != expected_active {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment alarm {} active state does not match latest reading for sensor {}",
                    alarm.id, alarm.sensor_id
                )));
            }
            if let Some(reading) =
                readings_by_key.get(&(alarm.sensor_id.clone(), alarm.timestamp_min))
            {
                if (alarm.value - reading.value).abs() > f64::EPSILON {
                    findings.push(EnvironmentValidationFinding::error(format!(
                        "environment alarm {} value does not match source reading",
                        alarm.id
                    )));
                }
                if let Some(sensor) = self
                    .sensors
                    .iter()
                    .find(|sensor| sensor.id == alarm.sensor_id)
                {
                    match sensor.thresholds.evaluate(reading.value) {
                        Some(expected) if expected != alarm.severity => {
                            findings.push(EnvironmentValidationFinding::error(format!(
                                "environment alarm {} severity {} does not match source reading severity {}",
                                alarm.id,
                                alarm.severity.label(),
                                expected.label()
                            )));
                        }
                        None => findings.push(EnvironmentValidationFinding::error(format!(
                            "environment alarm {} does not correspond to a threshold excursion",
                            alarm.id
                        ))),
                        Some(_) => {}
                    }
                }
            } else if sensor_ids.contains(&alarm.sensor_id) {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment alarm {} references missing source reading {} at {}",
                    alarm.id, alarm.sensor_id, alarm.timestamp_min
                )));
            }
        }

        let mut correlation_ids = BTreeSet::new();
        for correlation in &self.correlations {
            if correlation.id.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::error(
                    "environment correlation id is empty",
                ));
            } else if !correlation_ids.insert(correlation.id.clone()) {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment correlation {} is duplicated",
                    correlation.id
                )));
            }
            if correlation.sensor_ids.is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment correlation {} has no linked sensors",
                    correlation.id
                )));
            }
            if correlation.label.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment correlation {} has an empty label",
                    correlation.id
                )));
            }
            if correlation.process_area.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment correlation {} has an empty process area",
                    correlation.id
                )));
            }
            if correlation.description.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment correlation {} has an empty description",
                    correlation.id
                )));
            }
            let mut correlation_sensor_ids = BTreeSet::new();
            for sensor_id in &correlation.sensor_ids {
                if sensor_id.trim().is_empty() {
                    findings.push(EnvironmentValidationFinding::error(format!(
                        "environment correlation {} has an empty linked sensor id",
                        correlation.id
                    )));
                    continue;
                }
                if !correlation_sensor_ids.insert(sensor_id.clone()) {
                    findings.push(EnvironmentValidationFinding::warning(format!(
                        "environment correlation {} repeats linked sensor {}",
                        correlation.id, sensor_id
                    )));
                }
                if !sensor_ids.contains(sensor_id) {
                    findings.push(EnvironmentValidationFinding::error(format!(
                        "environment correlation {} references missing sensor {}",
                        correlation.id, sensor_id
                    )));
                }
            }
        }

        let mut event_keys = BTreeSet::new();
        for event in &self.events {
            let event_key = (
                event.timestamp_min,
                event.zone.trim().to_string(),
                event.title.trim().to_string(),
            );
            if !event_keys.insert(event_key) {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment event {} at {} in {} is duplicated",
                    event.title, event.timestamp_min, event.zone
                )));
            }
            if event.zone.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment event at {} has an empty zone",
                    event.timestamp_min
                )));
            }
            if event.title.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment event at {} has an empty title",
                    event.timestamp_min
                )));
            }
            if event.detail.trim().is_empty() {
                findings.push(EnvironmentValidationFinding::warning(format!(
                    "environment event {} has an empty detail",
                    event.title
                )));
            }
            if let Some(alarm_id) = event.linked_alarm_id.as_ref()
                && !alarm_ids.contains(alarm_id)
            {
                findings.push(EnvironmentValidationFinding::error(format!(
                    "environment event {} references missing alarm {}",
                    event.title, alarm_id
                )));
            }
        }

        findings
    }

    pub fn sample() -> Self {
        let sensors = vec![
            EnvironmentSensor {
                id: "temp-litho".to_string(),
                name: "Litho bay temperature".to_string(),
                zone: "Lithography".to_string(),
                kind: EnvironmentSensorKind::Temperature,
                unit: "C".to_string(),
                thresholds: EnvironmentThresholds::new(
                    Some(20.8),
                    Some(21.8),
                    Some(20.4),
                    Some(22.2),
                ),
                process_tags: vec!["overlay".to_string(), "resist coat".to_string()],
            },
            EnvironmentSensor {
                id: "rh-litho".to_string(),
                name: "Litho bay humidity".to_string(),
                zone: "Lithography".to_string(),
                kind: EnvironmentSensorKind::Humidity,
                unit: "%RH".to_string(),
                thresholds: EnvironmentThresholds::new(
                    Some(43.0),
                    Some(47.0),
                    Some(40.0),
                    Some(50.0),
                ),
                process_tags: vec!["photoresist".to_string()],
            },
            EnvironmentSensor {
                id: "particle-cmp".to_string(),
                name: "CMP particle counter".to_string(),
                zone: "CMP".to_string(),
                kind: EnvironmentSensorKind::ParticleCount,
                unit: "ct/m3".to_string(),
                thresholds: EnvironmentThresholds::new(None, Some(120.0), None, Some(180.0)),
                process_tags: vec!["scratch risk".to_string(), "post CMP clean".to_string()],
            },
            EnvironmentSensor {
                id: "vibe-stepper".to_string(),
                name: "Stepper slab vibration".to_string(),
                zone: "Lithography".to_string(),
                kind: EnvironmentSensorKind::Vibration,
                unit: "um/s".to_string(),
                thresholds: EnvironmentThresholds::new(None, Some(1.8), None, Some(2.5)),
                process_tags: vec!["overlay".to_string(), "CD variation".to_string()],
            },
            EnvironmentSensor {
                id: "di-water".to_string(),
                name: "DI loop resistivity".to_string(),
                zone: "Wet bench".to_string(),
                kind: EnvironmentSensorKind::DiWaterResistivity,
                unit: "Mohm-cm".to_string(),
                thresholds: EnvironmentThresholds::new(Some(17.8), None, Some(17.2), None),
                process_tags: vec!["wet clean".to_string()],
            },
            EnvironmentSensor {
                id: "exhaust-acid".to_string(),
                name: "Acid exhaust flow".to_string(),
                zone: "Wet bench".to_string(),
                kind: EnvironmentSensorKind::ExhaustFlow,
                unit: "m3/min".to_string(),
                thresholds: EnvironmentThresholds::new(Some(480.0), None, Some(430.0), None),
                process_tags: vec!["chemical safety".to_string()],
            },
        ];

        let mut readings = Vec::new();
        let series = [
            (
                "temp-litho",
                [21.05, 21.10, 21.18, 21.34, 21.52, 21.74, 21.91, 22.31],
            ),
            ("rh-litho", [45.0, 45.2, 45.5, 46.0, 46.5, 47.4, 48.1, 48.4]),
            (
                "particle-cmp",
                [82.0, 90.0, 98.0, 121.0, 156.0, 192.0, 174.0, 138.0],
            ),
            (
                "vibe-stepper",
                [1.10, 1.18, 1.25, 1.36, 1.58, 1.92, 2.14, 2.62],
            ),
            ("di-water", [18.2, 18.1, 18.0, 17.9, 17.7, 17.5, 17.4, 17.6]),
            (
                "exhaust-acid",
                [512.0, 505.0, 498.0, 486.0, 474.0, 452.0, 428.0, 441.0],
            ),
        ];
        for (sensor_id, values) in series {
            for (index, value) in values.into_iter().enumerate() {
                readings.push(EnvironmentReading {
                    sensor_id: sensor_id.to_string(),
                    timestamp_min: 8 * 60 + index as u32 * 15,
                    value,
                });
            }
        }

        let correlations = vec![
            EnvironmentCorrelationLabel {
                id: "overlay-correlation".to_string(),
                label: "Overlay excursion candidate".to_string(),
                process_area: "Lithography".to_string(),
                description: "Temperature or vibration drift can correlate with overlay residuals."
                    .to_string(),
                sensor_ids: vec!["temp-litho".to_string(), "vibe-stepper".to_string()],
            },
            EnvironmentCorrelationLabel {
                id: "post-cmp-defectivity".to_string(),
                label: "Post-CMP defectivity candidate".to_string(),
                process_area: "CMP".to_string(),
                description:
                    "Particle excursions are tagged for defectivity and scratch Pareto review."
                        .to_string(),
                sensor_ids: vec!["particle-cmp".to_string()],
            },
            EnvironmentCorrelationLabel {
                id: "wet-clean-risk".to_string(),
                label: "Wet clean process risk".to_string(),
                process_area: "Wet bench".to_string(),
                description: "DI water and exhaust degradation can put wet-process lots on hold."
                    .to_string(),
                sensor_ids: vec!["di-water".to_string(), "exhaust-acid".to_string()],
            },
        ];

        let events = vec![
            FacilityEvent {
                timestamp_min: 8 * 60 + 15,
                zone: "Lithography".to_string(),
                kind: FacilityEventKind::Maintenance,
                title: "Make-up air handler filter swap".to_string(),
                detail: "Facilities completed filter swap on AHU-2; litho temperature began drifting upward.".to_string(),
                linked_alarm_id: None,
            },
            FacilityEvent {
                timestamp_min: 9 * 60 + 15,
                zone: "CMP".to_string(),
                kind: FacilityEventKind::Excursion,
                title: "Particle burst near CMP chase".to_string(),
                detail: "Particle counter exceeded warning threshold during pad conditioner service.".to_string(),
                linked_alarm_id: Some("env-particle-cmp-555".to_string()),
            },
            FacilityEvent {
                timestamp_min: 9 * 60 + 45,
                zone: "Wet bench".to_string(),
                kind: FacilityEventKind::ProcessHold,
                title: "Wet bench lots held for facility review".to_string(),
                detail: "DI water resistivity and acid exhaust remained near limits.".to_string(),
                linked_alarm_id: Some("env-exhaust-acid-585".to_string()),
            },
            FacilityEvent {
                timestamp_min: 10 * 60 + 5,
                zone: "CMP".to_string(),
                kind: FacilityEventKind::Recovery,
                title: "CMP particle counter returning to baseline".to_string(),
                detail: "Follow-up readings dropped below critical after chase cleanup.".to_string(),
                linked_alarm_id: Some("env-particle-cmp-585".to_string()),
            },
        ];

        Self::new(sensors, readings, correlations, events)
    }
}

fn validate_thresholds(
    sensor: &EnvironmentSensor,
    findings: &mut Vec<EnvironmentValidationFinding>,
) {
    for (label, value) in [
        ("warning low", sensor.thresholds.warning_low),
        ("warning high", sensor.thresholds.warning_high),
        ("critical low", sensor.thresholds.critical_low),
        ("critical high", sensor.thresholds.critical_high),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            findings.push(EnvironmentValidationFinding::error(format!(
                "environment sensor {} has non-finite {label} threshold",
                sensor.id
            )));
        }
    }
    if let (Some(low), Some(high)) = (
        sensor.thresholds.warning_low,
        sensor.thresholds.warning_high,
    ) && low > high
    {
        findings.push(EnvironmentValidationFinding::error(format!(
            "environment sensor {} warning low threshold is above warning high threshold",
            sensor.id
        )));
    }
    if let (Some(low), Some(high)) = (
        sensor.thresholds.critical_low,
        sensor.thresholds.critical_high,
    ) && low > high
    {
        findings.push(EnvironmentValidationFinding::error(format!(
            "environment sensor {} critical low threshold is above critical high threshold",
            sensor.id
        )));
    }
    if let (Some(critical_low), Some(warning_low)) = (
        sensor.thresholds.critical_low,
        sensor.thresholds.warning_low,
    ) && critical_low > warning_low
    {
        findings.push(EnvironmentValidationFinding::error(format!(
            "environment sensor {} critical low threshold is above warning low threshold",
            sensor.id
        )));
    }
    if let (Some(critical_high), Some(warning_high)) = (
        sensor.thresholds.critical_high,
        sensor.thresholds.warning_high,
    ) && critical_high < warning_high
    {
        findings.push(EnvironmentValidationFinding::error(format!(
            "environment sensor {} critical high threshold is below warning high threshold",
            sensor.id
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_treat_boundary_as_in_control() {
        let thresholds = EnvironmentThresholds::new(Some(10.0), Some(20.0), Some(5.0), Some(25.0));

        assert_eq!(thresholds.evaluate(10.0), None);
        assert_eq!(thresholds.evaluate(20.0), None);
    }

    #[test]
    fn critical_threshold_takes_precedence_over_warning() {
        let thresholds = EnvironmentThresholds::new(Some(10.0), Some(20.0), Some(5.0), Some(25.0));

        assert_eq!(
            thresholds.evaluate(22.0),
            Some(EnvironmentAlarmSeverity::Warning)
        );
        assert_eq!(
            thresholds.evaluate(26.0),
            Some(EnvironmentAlarmSeverity::Critical)
        );
    }

    #[test]
    fn alarm_generation_marks_only_latest_excursion_active() {
        let sensor = EnvironmentSensor {
            id: "pressure".to_string(),
            name: "Cleanroom pressure".to_string(),
            zone: "Bay 1".to_string(),
            kind: EnvironmentSensorKind::AirPressure,
            unit: "Pa".to_string(),
            thresholds: EnvironmentThresholds::new(Some(8.0), None, Some(5.0), None),
            process_tags: Vec::new(),
        };
        let model = CleanroomEnvironment::new(
            vec![sensor],
            vec![
                EnvironmentReading {
                    sensor_id: "pressure".to_string(),
                    timestamp_min: 1,
                    value: 7.0,
                },
                EnvironmentReading {
                    sensor_id: "pressure".to_string(),
                    timestamp_min: 2,
                    value: 6.0,
                },
            ],
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(model.alarms.len(), 2);
        assert_eq!(model.active_alarms().len(), 1);
        assert_eq!(model.active_alarms()[0].timestamp_min, 2);
    }

    #[test]
    fn trend_summary_counts_excursions() {
        let model = CleanroomEnvironment::sample();
        let summary = model.trend_summary("particle-cmp").unwrap();

        assert_eq!(summary.direction, TrendDirection::Rising);
        assert_eq!(summary.excursion_count, 5);
    }

    #[test]
    fn sample_environment_validates() {
        let findings = CleanroomEnvironment::sample().validate();

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn validation_rejects_stale_alarm_and_correlation_metadata() {
        let mut model = CleanroomEnvironment::sample();
        let active_alarm = model
            .alarms
            .iter_mut()
            .find(|alarm| alarm.active)
            .expect("sample has active environment alarms");
        active_alarm.active = false;
        active_alarm.value += 1.0;
        active_alarm.severity = EnvironmentAlarmSeverity::Advisory;
        active_alarm.message.clear();

        let in_control_reading = model
            .readings
            .iter()
            .find(|reading| reading.sensor_id == "temp-litho" && reading.timestamp_min == 480)
            .expect("sample has in-control litho reading")
            .clone();
        model.alarms.push(EnvironmentAlarm {
            id: "manual-in-control".to_string(),
            sensor_id: in_control_reading.sensor_id,
            severity: EnvironmentAlarmSeverity::Warning,
            timestamp_min: in_control_reading.timestamp_min,
            value: in_control_reading.value,
            message: "Manual alarm on in-control reading".to_string(),
            active: false,
            correlation_labels: Vec::new(),
        });
        model.alarms.push(EnvironmentAlarm {
            id: "missing-reading".to_string(),
            sensor_id: "temp-litho".to_string(),
            severity: EnvironmentAlarmSeverity::Warning,
            timestamp_min: 9999,
            value: 23.0,
            message: String::new(),
            active: false,
            correlation_labels: Vec::new(),
        });
        model.correlations.push(EnvironmentCorrelationLabel {
            id: "bad-correlation".to_string(),
            label: String::new(),
            process_area: String::new(),
            description: String::new(),
            sensor_ids: vec![
                String::new(),
                "temp-litho".to_string(),
                "temp-litho".to_string(),
            ],
        });

        let findings = model.validate();

        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("active state does not match latest reading")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty message")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("value does not match source reading")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("severity Advisory does not match")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("does not correspond to a threshold excursion")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("references missing source reading")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty linked sensor id")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("repeats linked sensor temp-litho")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty label")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty process area")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty description")),
            "{findings:?}"
        );
    }

    #[test]
    fn validation_rejects_broken_sensor_references_and_thresholds() {
        let mut model = CleanroomEnvironment::sample();
        model.sensors[0].thresholds.critical_high = Some(21.0);
        let duplicate_reading = model.readings[0].clone();
        model.readings.push(duplicate_reading);
        model.readings.push(EnvironmentReading {
            sensor_id: "missing-sensor".to_string(),
            timestamp_min: 42,
            value: f64::NAN,
        });
        model.correlations.push(EnvironmentCorrelationLabel {
            id: "bad-correlation".to_string(),
            label: "Broken".to_string(),
            process_area: "Lithography".to_string(),
            description: String::new(),
            sensor_ids: vec!["missing-sensor".to_string()],
        });
        model.events.push(FacilityEvent {
            timestamp_min: 43,
            zone: String::new(),
            kind: FacilityEventKind::Excursion,
            title: "Broken event".to_string(),
            detail: String::new(),
            linked_alarm_id: Some("missing-alarm".to_string()),
        });
        model.events.push(FacilityEvent {
            timestamp_min: model.events[0].timestamp_min,
            zone: model.events[0].zone.clone(),
            kind: model.events[0].kind,
            title: model.events[0].title.clone(),
            detail: "duplicated event row".to_string(),
            linked_alarm_id: None,
        });

        let findings = model.validate();

        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("critical high threshold is below warning high threshold")),
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
                .any(|finding| finding.message.contains("duplicate reading")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("missing alarm")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty zone")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("empty detail")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("event")
                    && finding.message.contains("duplicated")),
            "{findings:?}"
        );
    }
}
