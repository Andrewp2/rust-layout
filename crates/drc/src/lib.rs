use std::collections::{BTreeMap, BTreeSet};

use geometry_core::{Coord, Point, Rect};
use layout_model::{
    Document, LayerId, MarkerState, Shape, ShapeId, ShapeKind, ShapeOccurrenceId, TechnologyError,
    TechnologyFile, default_technology,
};
use serde::{Deserialize, Serialize};

#[path = "engine/mod.rs"]
mod engine;
pub use engine::*;
#[allow(unused_imports)]
pub(crate) use engine::*;

#[cfg(test)]
#[path = "engine/tests/mod.rs"]
mod tests;
