use std::collections::{BTreeMap, BTreeSet};

use geometry_core::{Coord, Point, Rect};
use rstar::{AABB, RTree, RTreeObject};

use crate::{
    Document, LayerId, NetId, ProcessLayer, Shape, ShapeKind, ShapeOccurrenceId, TechnologyError,
    TechnologyFile,
};

#[path = "connectivity_model/mod.rs"]
mod connectivity_model;
pub use connectivity_model::*;
#[allow(unused_imports)]
pub(crate) use connectivity_model::*;

#[cfg(test)]
#[path = "connectivity_model/tests/mod.rs"]
mod tests;
