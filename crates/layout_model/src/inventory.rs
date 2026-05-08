use std::{collections::BTreeMap, fmt};

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
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
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
