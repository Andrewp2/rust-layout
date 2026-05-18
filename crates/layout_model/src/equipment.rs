use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{mes::FabMesData, recipe::RecipeCatalog};

#[path = "equipment_model/mod.rs"]
mod equipment_model;
pub use equipment_model::*;
#[allow(unused_imports)]
pub(crate) use equipment_model::*;

#[cfg(test)]
#[path = "equipment_model/tests/mod.rs"]
mod tests;
