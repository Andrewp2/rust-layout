pub mod gpu;
pub mod shader;

use std::{
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    fmt,
    hash::{Hash, Hasher},
};

use geometry_core::{Point, Rect};
use layout_model::{
    CellId, Document, InstanceId, LayerFillStyle, LayerId, LayerLineStyle, LayoutIndex, Shape,
    ShapeId, ShapeKind, ShapeKindView, ShapeOccurrenceId, ShapeView, Transform,
};
use tracing::warn;
use web_time::Instant;

#[path = "batch/mod.rs"]
mod batch_parts;
pub use batch_parts::*;
#[allow(unused_imports)]
pub(crate) use batch_parts::*;

#[cfg(test)]
#[path = "batch/tests/mod.rs"]
mod tests;
