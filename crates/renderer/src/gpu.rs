use std::{error::Error, time::Instant};

use crate::{GpuRectSlabInstance, PickBatch, RenderBatch, RenderBatch3d, shader};
use geometry_core::{Coord, Point, Rect};
use layout_model::{Document, LayoutIndex, ShapeOccurrenceId};
use tracing::{error, warn};

#[path = "gpu_backend/mod.rs"]
mod gpu_backend;
pub use gpu_backend::*;
#[allow(unused_imports)]
pub(crate) use gpu_backend::*;

#[cfg(test)]
#[path = "gpu_backend/tests/mod.rs"]
mod tests;
