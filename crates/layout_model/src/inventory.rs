use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MaterialLotId(pub String);

impl MaterialLotId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MaterialLotId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for MaterialLotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialCategory {
    Photoresist,
    Developer,
    Gas,
    Chemical,
    Substrate,
    Consumable,
}

impl MaterialCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Photoresist => "Photoresist",
            Self::Developer => "Developer",
            Self::Gas => "Gas",
            Self::Chemical => "Chemical",
            Self::Substrate => "Substrate",
            Self::Consumable => "Consumable",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityUnit {
    Milliliter,
    Liter,
    Gram,
    Kilogram,
    SccmHour,
    Each,
    Wafer,
}

impl QuantityUnit {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Milliliter => "mL",
            Self::Liter => "L",
            Self::Gram => "g",
            Self::Kilogram => "kg",
            Self::SccmHour => "sccm-hr",
            Self::Each => "ea",
            Self::Wafer => "wafers",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StockQuantity {
    pub amount: f64,
    pub unit: QuantityUnit,
}

impl StockQuantity {
    pub const fn new(amount: f64, unit: QuantityUnit) -> Self {
        Self { amount, unit }
    }

    pub fn format(self) -> String {
        let amount = if (self.amount.fract()).abs() < f64::EPSILON {
            format!("{:.0}", self.amount)
        } else {
            format!("{:.1}", self.amount)
        };
        format!("{} {}", amount, self.unit.symbol())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageLocation {
    pub area: String,
    pub cabinet: String,
    pub bin: String,
    pub temperature: String,
}

impl StorageLocation {
    pub fn label(&self) -> String {
        format!("{} / {} / {}", self.area, self.cabinet, self.bin)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierMetadata {
    pub supplier: String,
    pub supplier_lot: String,
    pub certificate_id: Option<String>,
    pub certificate_url: Option<String>,
    pub received_by: String,
    pub received_date: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FabObjectLink {
    Lot { lot_id: String },
    Wafer { lot_id: String, wafer_id: String },
    ToolRun { run_id: String },
    LayoutShape { shape_id: u64 },
}

impl FabObjectLink {
    pub fn label(&self) -> String {
        match self {
            Self::Lot { lot_id } => format!("Lot {lot_id}"),
            Self::Wafer { lot_id, wafer_id } => format!("{lot_id} / {wafer_id}"),
            Self::ToolRun { run_id } => format!("Run {run_id}"),
            Self::LayoutShape { shape_id } => format!("Shape {shape_id}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialUsage {
    pub timestamp: u32,
    pub actor: String,
    pub quantity: StockQuantity,
    pub links: Vec<FabObjectLink>,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialLot {
    pub id: MaterialLotId,
    pub material_name: String,
    pub category: MaterialCategory,
    pub stock: StockQuantity,
    pub reorder_threshold: StockQuantity,
    pub location: StorageLocation,
    pub expires_on: Option<u32>,
    pub supplier: SupplierMetadata,
    pub usage: Vec<MaterialUsage>,
    pub notes: String,
}

impl MaterialLot {
    pub fn is_low_stock(&self) -> bool {
        self.stock.unit == self.reorder_threshold.unit
            && self.stock.amount <= self.reorder_threshold.amount
    }

    pub fn is_expired(&self, today: u32) -> bool {
        self.expires_on.is_some_and(|expires_on| expires_on < today)
    }

    pub fn expires_within_days(&self, today: u32, days: u32) -> bool {
        self.expires_on
            .and_then(|expires_on| date_distance_days(today, expires_on))
            .is_some_and(|distance| distance <= days)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryAlertKind {
    LowStock,
    Expired,
    ExpiringSoon,
}

impl InventoryAlertKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::LowStock => "Low stock",
            Self::Expired => "Expired",
            Self::ExpiringSoon => "Expiring soon",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryAlert {
    pub lot_id: MaterialLotId,
    pub material_name: String,
    pub kind: InventoryAlertKind,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub lots: BTreeMap<MaterialLotId, MaterialLot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryValidationFinding {
    pub severity: InventoryValidationSeverity,
    pub message: String,
}

impl InventoryValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: InventoryValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: InventoryValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InventoryValidationContext {
    pub lot_ids: BTreeSet<String>,
    pub wafer_ids_by_lot: BTreeMap<String, BTreeSet<String>>,
    pub layout_shape_ids: BTreeSet<u64>,
}

impl InventoryValidationContext {
    pub fn from_mes_and_shape_ids<I>(mes: &crate::mes::FabMesData, shape_ids: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        let mut context = Self {
            layout_shape_ids: shape_ids.into_iter().collect(),
            ..Self::default()
        };
        for (lot_id, lot) in &mes.lots {
            let lot_id = lot_id.as_str().to_string();
            context.lot_ids.insert(lot_id.clone());
            let wafer_ids = context.wafer_ids_by_lot.entry(lot_id).or_default();
            for wafer in &lot.wafers {
                wafer_ids.insert(wafer.id.as_str().to_string());
                wafer_ids.insert(format!("W{:02}", wafer.slot));
            }
        }
        context
    }

    fn contains_lot(&self, lot_id: &str) -> bool {
        self.lot_ids.contains(lot_id)
    }

    fn contains_wafer(&self, lot_id: &str, wafer_id: &str) -> bool {
        self.wafer_ids_by_lot
            .get(lot_id)
            .is_some_and(|wafer_ids| wafer_ids.contains(wafer_id))
    }

    fn contains_shape(&self, shape_id: u64) -> bool {
        self.layout_shape_ids.contains(&shape_id)
    }
}

impl Inventory {
    pub fn sample() -> Self {
        sample_inventory()
    }

    pub fn sorted_lots(&self) -> Vec<&MaterialLot> {
        let mut lots = self.lots.values().collect::<Vec<_>>();
        lots.sort_by(|a, b| {
            a.category
                .label()
                .cmp(b.category.label())
                .then_with(|| a.material_name.cmp(&b.material_name))
                .then_with(|| a.id.cmp(&b.id))
        });
        lots
    }

    pub fn lot(&self, id: &MaterialLotId) -> Option<&MaterialLot> {
        self.lots.get(id)
    }

    pub fn validate(&self) -> Vec<InventoryValidationFinding> {
        let mut findings = Vec::new();
        self.validate_internal(&mut findings);
        findings
    }

    pub fn validate_with_context(
        &self,
        context: &InventoryValidationContext,
    ) -> Vec<InventoryValidationFinding> {
        let mut findings = self.validate();
        self.validate_links(context, &mut findings);
        findings
    }

    pub fn is_valid(&self) -> bool {
        self.validate()
            .iter()
            .all(|finding| finding.severity != InventoryValidationSeverity::Error)
    }

    fn validate_internal(&self, findings: &mut Vec<InventoryValidationFinding>) {
        for (lot_id, lot) in &self.lots {
            if lot.id != *lot_id {
                findings.push(InventoryValidationFinding::error(format!(
                    "inventory lot key {lot_id} does not match material lot id {}",
                    lot.id
                )));
            }
            if lot.id.as_str().trim().is_empty() {
                findings.push(InventoryValidationFinding::error(
                    "inventory material lot id cannot be empty",
                ));
            }
            if lot.material_name.trim().is_empty() {
                findings.push(InventoryValidationFinding::error(format!(
                    "inventory material lot {lot_id} has no material name"
                )));
            }
            if lot.location.area.trim().is_empty()
                || lot.location.cabinet.trim().is_empty()
                || lot.location.bin.trim().is_empty()
                || lot.location.temperature.trim().is_empty()
            {
                findings.push(InventoryValidationFinding::warning(format!(
                    "inventory material lot {lot_id} has incomplete storage location"
                )));
            }
            validate_quantity(lot_id, "stock", lot.stock, false, findings);
            validate_quantity(
                lot_id,
                "reorder threshold",
                lot.reorder_threshold,
                true,
                findings,
            );
            if lot.stock.unit != lot.reorder_threshold.unit {
                findings.push(InventoryValidationFinding::error(format!(
                    "inventory material lot {lot_id} stock unit {} does not match reorder unit {}",
                    lot.stock.unit.symbol(),
                    lot.reorder_threshold.unit.symbol()
                )));
            }
            if let Some(expires_on) = lot.expires_on
                && parse_date(expires_on).is_none()
            {
                findings.push(InventoryValidationFinding::error(format!(
                    "inventory material lot {lot_id} has invalid expiration date {expires_on}"
                )));
            }
            if parse_date(lot.supplier.received_date).is_none() {
                findings.push(InventoryValidationFinding::error(format!(
                    "inventory material lot {lot_id} has invalid received date {}",
                    lot.supplier.received_date
                )));
            }
            if lot
                .expires_on
                .and_then(|expires_on| date_distance_days(lot.supplier.received_date, expires_on))
                .is_none()
                && lot.expires_on.is_some()
                && parse_date(lot.supplier.received_date).is_some()
            {
                findings.push(InventoryValidationFinding::warning(format!(
                    "inventory material lot {lot_id} expires before its received date"
                )));
            }
            if lot.supplier.supplier.trim().is_empty() {
                findings.push(InventoryValidationFinding::warning(format!(
                    "inventory material lot {lot_id} has no supplier"
                )));
            }
            if lot.supplier.supplier_lot.trim().is_empty() {
                findings.push(InventoryValidationFinding::warning(format!(
                    "inventory material lot {lot_id} has no supplier lot id"
                )));
            }
            if lot.supplier.received_by.trim().is_empty() {
                findings.push(InventoryValidationFinding::warning(format!(
                    "inventory material lot {lot_id} has no receiving actor"
                )));
            }
            validate_certificate_metadata(lot_id, &lot.supplier, findings);

            for usage in &lot.usage {
                validate_usage(
                    lot_id,
                    lot.stock.unit,
                    lot.supplier.received_date,
                    usage,
                    findings,
                );
            }
        }
    }

    fn validate_links(
        &self,
        context: &InventoryValidationContext,
        findings: &mut Vec<InventoryValidationFinding>,
    ) {
        for material in self.lots.values() {
            for usage in &material.usage {
                for link in &usage.links {
                    match link {
                        FabObjectLink::Lot { lot_id } => {
                            if lot_id.trim().is_empty() {
                                continue;
                            }
                            if !context.contains_lot(lot_id) {
                                findings.push(InventoryValidationFinding::error(format!(
                                    "inventory material {} references missing lot {lot_id}",
                                    material.id
                                )));
                            }
                        }
                        FabObjectLink::Wafer { lot_id, wafer_id } => {
                            if lot_id.trim().is_empty() || wafer_id.trim().is_empty() {
                                continue;
                            }
                            if !context.contains_lot(lot_id) {
                                findings.push(InventoryValidationFinding::error(format!(
                                    "inventory material {} references missing wafer lot {lot_id}",
                                    material.id
                                )));
                            } else if !context.contains_wafer(lot_id, wafer_id) {
                                findings.push(InventoryValidationFinding::error(format!(
                                    "inventory material {} references missing wafer {lot_id}/{wafer_id}",
                                    material.id
                                )));
                            }
                        }
                        FabObjectLink::LayoutShape { shape_id } => {
                            if *shape_id == 0 {
                                continue;
                            }
                            if !context.contains_shape(*shape_id) {
                                findings.push(InventoryValidationFinding::error(format!(
                                    "inventory material {} references missing layout shape {shape_id}",
                                    material.id
                                )));
                            }
                        }
                        FabObjectLink::ToolRun { .. } => {}
                    }
                }
            }
        }
    }

    pub fn alerts(&self, today: u32) -> Vec<InventoryAlert> {
        let mut alerts = Vec::new();
        for lot in self.sorted_lots() {
            if lot.is_expired(today) {
                alerts.push(InventoryAlert {
                    lot_id: lot.id.clone(),
                    material_name: lot.material_name.clone(),
                    kind: InventoryAlertKind::Expired,
                    message: format!(
                        "Expired on {}",
                        lot.expires_on.map(format_date).unwrap_or_default()
                    ),
                });
            } else if lot.expires_within_days(today, 30) {
                alerts.push(InventoryAlert {
                    lot_id: lot.id.clone(),
                    material_name: lot.material_name.clone(),
                    kind: InventoryAlertKind::ExpiringSoon,
                    message: format!(
                        "Expires on {}",
                        lot.expires_on.map(format_date).unwrap_or_default()
                    ),
                });
            }

            if lot.is_low_stock() {
                alerts.push(InventoryAlert {
                    lot_id: lot.id.clone(),
                    material_name: lot.material_name.clone(),
                    kind: InventoryAlertKind::LowStock,
                    message: format!(
                        "{} remaining; reorder at {}",
                        lot.stock.format(),
                        lot.reorder_threshold.format()
                    ),
                });
            }
        }
        alerts
    }
}

fn validate_certificate_metadata(
    lot_id: &MaterialLotId,
    supplier: &SupplierMetadata,
    findings: &mut Vec<InventoryValidationFinding>,
) {
    match supplier.certificate_id.as_deref() {
        Some(id) if id.trim().is_empty() => {
            findings.push(InventoryValidationFinding::error(format!(
                "inventory material lot {lot_id} certificate id is empty"
            )));
        }
        Some(_) if supplier.certificate_url.is_none() => {
            findings.push(InventoryValidationFinding::warning(format!(
                "inventory material lot {lot_id} has certificate id without certificate URL"
            )));
        }
        None if supplier.certificate_url.is_some() => {
            findings.push(InventoryValidationFinding::warning(format!(
                "inventory material lot {lot_id} has certificate URL without certificate id"
            )));
        }
        _ => {}
    }
    if supplier
        .certificate_url
        .as_deref()
        .is_some_and(|url| url.trim().is_empty())
    {
        findings.push(InventoryValidationFinding::error(format!(
            "inventory material lot {lot_id} certificate URL is empty"
        )));
    }
}

fn validate_quantity(
    lot_id: &MaterialLotId,
    label: &str,
    quantity: StockQuantity,
    allow_zero: bool,
    findings: &mut Vec<InventoryValidationFinding>,
) {
    if !quantity.amount.is_finite() {
        findings.push(InventoryValidationFinding::error(format!(
            "inventory material lot {lot_id} {label} amount must be finite"
        )));
    } else if quantity.amount < 0.0 || (!allow_zero && quantity.amount == 0.0) {
        findings.push(InventoryValidationFinding::error(format!(
            "inventory material lot {lot_id} {label} amount must be {}",
            if allow_zero {
                "non-negative"
            } else {
                "positive"
            }
        )));
    }
}

fn validate_usage(
    lot_id: &MaterialLotId,
    stock_unit: QuantityUnit,
    received_date: u32,
    usage: &MaterialUsage,
    findings: &mut Vec<InventoryValidationFinding>,
) {
    let usage_date_valid = parse_date(usage.timestamp).is_some();
    let received_date_valid = parse_date(received_date).is_some();
    if !usage_date_valid {
        findings.push(InventoryValidationFinding::error(format!(
            "inventory material lot {lot_id} has invalid usage date {}",
            usage.timestamp
        )));
    }
    if usage_date_valid
        && received_date_valid
        && date_distance_days(received_date, usage.timestamp).is_none()
    {
        findings.push(InventoryValidationFinding::error(format!(
            "inventory material lot {lot_id} has usage before received date"
        )));
    }
    if usage.actor.trim().is_empty() {
        findings.push(InventoryValidationFinding::warning(format!(
            "inventory material lot {lot_id} has usage without an actor"
        )));
    }
    validate_quantity(lot_id, "usage", usage.quantity, false, findings);
    if !units_are_compatible(usage.quantity.unit, stock_unit) {
        findings.push(InventoryValidationFinding::error(format!(
            "inventory material lot {lot_id} usage unit {} is incompatible with stock unit {}",
            usage.quantity.unit.symbol(),
            stock_unit.symbol()
        )));
    }
    if usage.links.is_empty() {
        findings.push(InventoryValidationFinding::warning(format!(
            "inventory material lot {lot_id} has usage without traceability links"
        )));
    } else {
        validate_usage_links(lot_id, &usage.links, findings);
    }
    if usage.note.trim().is_empty() {
        findings.push(InventoryValidationFinding::warning(format!(
            "inventory material lot {lot_id} has usage without a note"
        )));
    }
}

fn validate_usage_links(
    lot_id: &MaterialLotId,
    links: &[FabObjectLink],
    findings: &mut Vec<InventoryValidationFinding>,
) {
    let mut seen_links = BTreeSet::new();
    for link in links {
        match link {
            FabObjectLink::Lot { lot_id: linked_lot } => {
                if linked_lot.trim().is_empty() {
                    findings.push(InventoryValidationFinding::error(format!(
                        "inventory material lot {lot_id} has lot traceability link with empty lot id"
                    )));
                }
            }
            FabObjectLink::Wafer {
                lot_id: linked_lot,
                wafer_id,
            } => {
                if linked_lot.trim().is_empty() {
                    findings.push(InventoryValidationFinding::error(format!(
                        "inventory material lot {lot_id} has wafer traceability link with empty lot id"
                    )));
                }
                if wafer_id.trim().is_empty() {
                    findings.push(InventoryValidationFinding::error(format!(
                        "inventory material lot {lot_id} has wafer traceability link with empty wafer id"
                    )));
                }
            }
            FabObjectLink::ToolRun { run_id } => {
                if run_id.trim().is_empty() {
                    findings.push(InventoryValidationFinding::error(format!(
                        "inventory material lot {lot_id} has an empty tool run link"
                    )));
                }
            }
            FabObjectLink::LayoutShape { shape_id } => {
                if *shape_id == 0 {
                    findings.push(InventoryValidationFinding::error(format!(
                        "inventory material lot {lot_id} has layout shape traceability link with shape id 0"
                    )));
                }
            }
        }

        let key = traceability_link_key(link);
        if !seen_links.insert(key) {
            findings.push(InventoryValidationFinding::warning(format!(
                "inventory material lot {lot_id} repeats a usage traceability link"
            )));
        }
    }
}

fn traceability_link_key(link: &FabObjectLink) -> String {
    match link {
        FabObjectLink::Lot { lot_id } => format!("lot:{lot_id}"),
        FabObjectLink::Wafer { lot_id, wafer_id } => format!("wafer:{lot_id}:{wafer_id}"),
        FabObjectLink::ToolRun { run_id } => format!("tool_run:{run_id}"),
        FabObjectLink::LayoutShape { shape_id } => format!("shape:{shape_id}"),
    }
}

fn units_are_compatible(left: QuantityUnit, right: QuantityUnit) -> bool {
    use QuantityUnit::*;
    matches!(
        (left, right),
        (Milliliter | Liter, Milliliter | Liter)
            | (Gram | Kilogram, Gram | Kilogram)
            | (SccmHour, SccmHour)
            | (Each, Each)
            | (Wafer, Wafer)
    )
}

pub fn format_date(date: u32) -> String {
    let year = date / 10_000;
    let month = (date / 100) % 100;
    let day = date % 100;
    format!("{year:04}-{month:02}-{day:02}")
}

fn date_distance_days(start: u32, end: u32) -> Option<u32> {
    let (start_year, start_month, start_day) = parse_date(start)?;
    let (end_year, end_month, end_day) = parse_date(end)?;
    let start_ordinal = days_from_civil(start_year, start_month, start_day);
    let end_ordinal = days_from_civil(end_year, end_month, end_day);
    end_ordinal
        .checked_sub(start_ordinal)
        .and_then(|days| u32::try_from(days).ok())
}

fn parse_date(date: u32) -> Option<(i32, u32, u32)> {
    let year = i32::try_from(date / 10_000).ok()?;
    let month = (date / 100) % 100;
    let day = date % 100;
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return None,
    };

    if !(1900..=9999).contains(&year) || !(1..=max_day).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = i32::try_from(month).unwrap_or_default();
    let day = i32::try_from(day).unwrap_or_default();
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era * 146_097 + doe - 719_468)
}

pub fn sample_inventory() -> Inventory {
    let lots = [
        material_lot(
            "MAT-PR-0007",
            "AZ1512 positive photoresist",
            MaterialCategory::Photoresist,
            StockQuantity::new(85.0, QuantityUnit::Milliliter),
            StockQuantity::new(100.0, QuantityUnit::Milliliter),
            location("Chem bay", "Fridge A", "Shelf 2", "4 C"),
            Some(20260601),
            supplier(
                "MicroChem",
                "MC-AZ1512-4421",
                Some("COA-AZ1512-4421"),
                20260410,
            ),
            vec![
                usage(
                    20260428,
                    "op.demo",
                    30.0,
                    QuantityUnit::Milliliter,
                    "Coated demo inverter split",
                    vec![
                        FabObjectLink::Lot {
                            lot_id: "L-00042".to_string(),
                        },
                        FabObjectLink::ToolRun {
                            run_id: "RUN-SPIN-00042".to_string(),
                        },
                    ],
                ),
                usage(
                    20260503,
                    "process.engineer",
                    15.0,
                    QuantityUnit::Milliliter,
                    "DOE focus wafer coating",
                    vec![FabObjectLink::Wafer {
                        lot_id: "L-00042".to_string(),
                        wafer_id: "W07".to_string(),
                    }],
                ),
            ],
            "Opened bottle; nitrogen blanket after each dispense.",
        ),
        material_lot(
            "MAT-DEV-0012",
            "AZ 300 MIF developer",
            MaterialCategory::Developer,
            StockQuantity::new(1.8, QuantityUnit::Liter),
            StockQuantity::new(0.5, QuantityUnit::Liter),
            location("Chem bay", "Base cabinet", "Bin 4", "Ambient"),
            Some(20260420),
            supplier(
                "Kayaku Advanced Materials",
                "KAM-MIF-8820",
                Some("COA-MIF-8820"),
                20260112,
            ),
            vec![usage(
                20260428,
                "op.demo",
                120.0,
                QuantityUnit::Milliliter,
                "Developed poly lithography monitor wafers",
                vec![FabObjectLink::Lot {
                    lot_id: "L-00042".to_string(),
                }],
            )],
            "Expired material retained for non-product training only.",
        ),
        material_lot(
            "MAT-GAS-0003",
            "CF4 etch gas cylinder",
            MaterialCategory::Gas,
            StockQuantity::new(18.0, QuantityUnit::SccmHour),
            StockQuantity::new(12.0, QuantityUnit::SccmHour),
            location("Gas bunker", "Rack 1", "Position CF4-2", "Ambient"),
            Some(20270115),
            supplier("Airgas", "AG-CF4-2190", Some("CERT-CF4-2190"), 20260222),
            vec![usage(
                20260429,
                "etch.owner",
                2.5,
                QuantityUnit::SccmHour,
                "Poly etch qualification",
                vec![FabObjectLink::ToolRun {
                    run_id: "RUN-ETCH-00051".to_string(),
                }],
            )],
            "Cylinder connected to etcher ETCH-01.",
        ),
        material_lot(
            "MAT-WAF-0100",
            "100 mm p-type silicon wafers",
            MaterialCategory::Substrate,
            StockQuantity::new(42.0, QuantityUnit::Wafer),
            StockQuantity::new(25.0, QuantityUnit::Wafer),
            location("Stockroom", "Wafer cabinet", "Cassette P100", "Ambient N2"),
            None,
            supplier(
                "University Wafer",
                "UW-P100-7712",
                Some("COC-P100-7712"),
                20260302,
            ),
            vec![usage(
                20260426,
                "op.demo",
                25.0,
                QuantityUnit::Wafer,
                "Started demo lot L-00042",
                vec![FabObjectLink::Lot {
                    lot_id: "L-00042".to_string(),
                }],
            )],
            "Primary substrate stock for demo CMOS route.",
        ),
    ]
    .into_iter()
    .map(|lot| (lot.id.clone(), lot))
    .collect();

    Inventory { lots }
}

fn material_lot(
    id: &str,
    material_name: &str,
    category: MaterialCategory,
    stock: StockQuantity,
    reorder_threshold: StockQuantity,
    location: StorageLocation,
    expires_on: Option<u32>,
    supplier: SupplierMetadata,
    usage: Vec<MaterialUsage>,
    notes: &str,
) -> MaterialLot {
    MaterialLot {
        id: MaterialLotId::from(id),
        material_name: material_name.to_string(),
        category,
        stock,
        reorder_threshold,
        location,
        expires_on,
        supplier,
        usage,
        notes: notes.to_string(),
    }
}

fn location(area: &str, cabinet: &str, bin: &str, temperature: &str) -> StorageLocation {
    StorageLocation {
        area: area.to_string(),
        cabinet: cabinet.to_string(),
        bin: bin.to_string(),
        temperature: temperature.to_string(),
    }
}

fn supplier(
    supplier: &str,
    supplier_lot: &str,
    certificate_id: Option<&str>,
    received_date: u32,
) -> SupplierMetadata {
    SupplierMetadata {
        supplier: supplier.to_string(),
        supplier_lot: supplier_lot.to_string(),
        certificate_id: certificate_id.map(str::to_string),
        certificate_url: certificate_id.map(|id| format!("fabos://certificates/{id}")),
        received_by: "materials.owner".to_string(),
        received_date,
    }
}

fn usage(
    timestamp: u32,
    actor: &str,
    amount: f64,
    unit: QuantityUnit,
    note: &str,
    links: Vec<FabObjectLink>,
) -> MaterialUsage {
    MaterialUsage {
        timestamp,
        actor: actor.to_string(),
        quantity: StockQuantity::new(amount, unit),
        links,
        note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validation_context() -> InventoryValidationContext {
        InventoryValidationContext::from_mes_and_shape_ids(&crate::mes::FabMesData::sample(), [42])
    }

    fn has_validation_error(findings: &[InventoryValidationFinding], needle: &str) -> bool {
        findings.iter().any(|finding| {
            finding.severity == InventoryValidationSeverity::Error
                && finding.message.contains(needle)
        })
    }

    fn has_validation_warning(findings: &[InventoryValidationFinding], needle: &str) -> bool {
        findings.iter().any(|finding| {
            finding.severity == InventoryValidationSeverity::Warning
                && finding.message.contains(needle)
        })
    }

    #[test]
    fn sample_inventory_validates_with_mes_context() {
        let inventory = sample_inventory();
        let findings = inventory.validate_with_context(&validation_context());

        assert!(findings.is_empty(), "{findings:?}");
        assert!(inventory.is_valid());
    }

    #[test]
    fn inventory_validation_rejects_mismatched_key_and_bad_quantities() {
        let mut inventory = sample_inventory();
        let original_id = MaterialLotId::from("MAT-PR-0007");
        let mut lot = inventory.lots.remove(&original_id).unwrap();
        lot.stock.amount = -1.0;
        lot.reorder_threshold.unit = QuantityUnit::Liter;
        lot.usage[0].quantity.unit = QuantityUnit::Wafer;
        inventory
            .lots
            .insert(MaterialLotId::from("MAT-KEY-MISMATCH"), lot);

        let findings = inventory.validate();

        assert!(has_validation_error(
            &findings,
            "inventory lot key MAT-KEY-MISMATCH does not match material lot id MAT-PR-0007"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-KEY-MISMATCH stock amount must be positive"
        ));
        assert!(has_validation_error(
            &findings,
            "stock unit mL does not match reorder unit L"
        ));
        assert!(has_validation_error(
            &findings,
            "usage unit wafers is incompatible with stock unit mL"
        ));
    }

    #[test]
    fn inventory_date_validation_rejects_impossible_calendar_days() {
        assert!(parse_date(20240229).is_some());
        assert!(parse_date(20260229).is_none());
        assert!(parse_date(20260431).is_none());
        assert!(parse_date(10101).is_none());

        let mut inventory = sample_inventory();
        let lot = inventory
            .lots
            .get_mut(&MaterialLotId::from("MAT-PR-0007"))
            .unwrap();
        lot.expires_on = Some(20260229);
        lot.supplier.received_date = 20260431;
        lot.usage[0].timestamp = 20260230;

        let findings = inventory.validate();

        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 has invalid expiration date 20260229"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 has invalid received date 20260431"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 has invalid usage date 20260230"
        ));
    }

    #[test]
    fn inventory_validation_rejects_incomplete_receiving_and_traceability_metadata() {
        let mut inventory = sample_inventory();
        let lot = inventory
            .lots
            .get_mut(&MaterialLotId::from("MAT-PR-0007"))
            .unwrap();
        lot.location.area.clear();
        lot.supplier.received_by.clear();
        lot.supplier.certificate_id = Some(" ".to_string());
        lot.supplier.certificate_url = Some(" ".to_string());
        lot.usage[0].timestamp = 20260401;
        lot.usage[0].links = vec![
            FabObjectLink::Lot {
                lot_id: String::new(),
            },
            FabObjectLink::Wafer {
                lot_id: "L-00042".to_string(),
                wafer_id: String::new(),
            },
            FabObjectLink::LayoutShape { shape_id: 0 },
            FabObjectLink::ToolRun {
                run_id: String::new(),
            },
            FabObjectLink::Lot {
                lot_id: "L-00042".to_string(),
            },
            FabObjectLink::Lot {
                lot_id: "L-00042".to_string(),
            },
        ];

        let findings = inventory.validate();

        assert!(has_validation_warning(
            &findings,
            "inventory material lot MAT-PR-0007 has incomplete storage location"
        ));
        assert!(has_validation_warning(
            &findings,
            "inventory material lot MAT-PR-0007 has no receiving actor"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 certificate id is empty"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 certificate URL is empty"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 has usage before received date"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 has lot traceability link with empty lot id"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 has wafer traceability link with empty wafer id"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 has layout shape traceability link with shape id 0"
        ));
        assert!(has_validation_error(
            &findings,
            "inventory material lot MAT-PR-0007 has an empty tool run link"
        ));
        assert!(has_validation_warning(
            &findings,
            "inventory material lot MAT-PR-0007 repeats a usage traceability link"
        ));
    }

    #[test]
    fn inventory_validation_rejects_missing_context_references() {
        let mut inventory = sample_inventory();
        let lot = inventory
            .lots
            .get_mut(&MaterialLotId::from("MAT-PR-0007"))
            .unwrap();
        lot.usage[0].links = vec![
            FabObjectLink::Lot {
                lot_id: "L-MISSING".to_string(),
            },
            FabObjectLink::Wafer {
                lot_id: "L-00042".to_string(),
                wafer_id: "W99".to_string(),
            },
            FabObjectLink::LayoutShape { shape_id: 999 },
            FabObjectLink::ToolRun {
                run_id: String::new(),
            },
        ];

        let findings = inventory.validate_with_context(&validation_context());

        assert!(has_validation_error(
            &findings,
            "references missing lot L-MISSING"
        ));
        assert!(has_validation_error(
            &findings,
            "references missing wafer L-00042/W99"
        ));
        assert!(has_validation_error(
            &findings,
            "references missing layout shape 999"
        ));
        assert!(has_validation_error(
            &findings,
            "has an empty tool run link"
        ));
    }

    #[test]
    fn sample_inventory_contains_alertable_and_traceable_materials() {
        let inventory = sample_inventory();

        assert_eq!(inventory.lots.len(), 4);
        assert!(
            inventory
                .lots
                .values()
                .any(|lot| lot.usage.iter().any(|usage| !usage.links.is_empty()))
        );

        let alerts = inventory.alerts(20260508);
        assert!(
            alerts
                .iter()
                .any(|alert| alert.kind == InventoryAlertKind::LowStock
                    && alert.lot_id == MaterialLotId::from("MAT-PR-0007"))
        );
        assert!(
            alerts
                .iter()
                .any(|alert| alert.kind == InventoryAlertKind::Expired
                    && alert.lot_id == MaterialLotId::from("MAT-DEV-0012"))
        );
        assert!(
            alerts
                .iter()
                .any(|alert| alert.kind == InventoryAlertKind::ExpiringSoon
                    && alert.lot_id == MaterialLotId::from("MAT-PR-0007"))
        );
    }

    #[test]
    fn alerts_do_not_flag_future_stable_stock() {
        let inventory = sample_inventory();
        let gas_lot = inventory.lot(&MaterialLotId::from("MAT-GAS-0003")).unwrap();

        assert!(!gas_lot.is_low_stock());
        assert!(!gas_lot.is_expired(20260508));
        assert!(!gas_lot.expires_within_days(20260508, 30));
    }
}
