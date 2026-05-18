use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use geometry_core::{Coord, DEFAULT_GRID, Point, Polygon, Rect, Vector};
use loro::{
    Container, ExportMode, LoroDoc, LoroEncodeError, LoroError, LoroMap, LoroValue, PeerID,
    ValueOrContainer,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeMap};
use tracing::warn;
use uuid::Uuid;

pub mod cif;
pub mod connectivity;
pub mod cross_section;
pub mod def;
pub mod dxf;
pub mod environment;
pub mod equipment;
pub mod experiment;
pub mod fab_ref;
pub mod gdsii;
pub mod genealogy;
pub mod inventory;
pub mod layout_diff;
pub mod lef;
pub mod maintenance;
pub mod mask;
pub mod mes;
pub mod metrology;
pub mod notebook;
pub mod process_control;
pub mod process_flow;
pub mod recipe;
pub mod safety;
pub mod scheduler;
pub mod spc_fdc;
pub mod workspace;
pub mod yield_analysis;

#[path = "document_model/mod.rs"]
mod model_core;
pub use model_core::*;
#[allow(unused_imports)]
pub(crate) use model_core::*;

#[cfg(test)]
#[path = "document_model/tests/mod.rs"]
mod tests;
