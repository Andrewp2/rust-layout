#![allow(unused_imports)]
use super::*;

#[path = "detail_sections.rs"]
mod detail_sections;
pub use detail_sections::*;
pub(crate) use detail_sections::*;
#[path = "view_detail_sections.rs"]
mod view_detail_sections;
pub use view_detail_sections::*;
pub(crate) use view_detail_sections::*;
#[path = "panels.rs"]
mod panels;
pub use panels::*;
pub(crate) use panels::*;
