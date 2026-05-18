#![allow(unused_imports)]
use super::*;

pub(crate) fn push_transformed_pick_fan(
    batch: &mut PickBatch,
    points: &[Point],
    transform: Transform,
    pick_id: u32,
) {
    if batch.vertices.len() > u32::MAX as usize - points.len() {
        warn!(
            vertex_count = batch.vertices.len(),
            added_points = points.len(),
            max_indexable_vertices = u32::MAX,
            "transformed pick fan would exceed u32 index range"
        );
        return;
    }
    let base = batch.vertices.len() as u32;
    for point in points {
        let point = transform.apply_point(*point);
        batch.vertices.push(PickVertex {
            position: [point.x as f32, point.y as f32],
            pick_id,
        });
    }
    for index in 1..points.len().saturating_sub(1) {
        batch
            .indices
            .extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
    }
}

pub(crate) fn push_pick_segment_as_rect(
    batch: &mut PickBatch,
    a: Point,
    b: Point,
    width: i64,
    pick_id: u32,
) {
    if a.x == b.x {
        let half = width / 2;
        push_pick_rect(
            batch,
            Rect::new(Point::new(a.x - half, a.y), Point::new(b.x + half, b.y)),
            pick_id,
        );
    } else if a.y == b.y {
        let half = width / 2;
        push_pick_rect(
            batch,
            Rect::new(Point::new(a.x, a.y - half), Point::new(b.x, b.y + half)),
            pick_id,
        );
    }
}
