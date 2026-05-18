use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use geometry_core::{Coord, DBU_PER_MICRON, Point, Polygon, Rect, Vector};
use tracing::warn;

use crate::{
    Cell, CellId, CellInstance, Document, InstanceArray, InstanceId, LayerId, NetId, ProcessLayer,
    Shape, ShapeId, ShapeKind, TechnologyError, TechnologyFile, Transform,
};

#[path = "gdsii_codec/mod.rs"]
mod gdsii_codec;
pub use gdsii_codec::*;
#[allow(unused_imports)]
pub(crate) use gdsii_codec::*;

#[cfg(test)]
#[path = "gdsii_codec/tests/mod.rs"]
mod tests;
