#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn viewport_3d_target_size_rounds_fractional_high_dpi_extents_up() {
    assert_eq!(viewport_3d_target_size([320.25, 200.5], 1.5), [481, 301]);
    assert_eq!(viewport_3d_target_size([320.0, 200.0], 2.0), [640, 400]);
}

#[test]
pub(crate) fn viewport_3d_target_size_sanitizes_empty_or_invalid_inputs() {
    assert_eq!(viewport_3d_target_size([0.0, -4.0], 2.0), [1, 1]);
    assert_eq!(viewport_3d_target_size([f32::NAN, 10.0], f32::NAN), [1, 10]);
}

#[test]
pub(crate) fn rect_slab_template_contains_dynamic_visible_face_slots() {
    let template = rect_slab_template_bytes();
    assert_eq!(template.len(), RECT_SLAB_TEMPLATE_VERTEX_COUNT * 16);

    let vertices = template
        .chunks_exact(16)
        .map(|vertex| {
            [
                f32::from_ne_bytes(vertex[0..4].try_into().unwrap()),
                f32::from_ne_bytes(vertex[4..8].try_into().unwrap()),
                f32::from_ne_bytes(vertex[8..12].try_into().unwrap()),
                f32::from_ne_bytes(vertex[12..16].try_into().unwrap()),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(vertices.len(), 18);
    assert_eq!(
        vertices.iter().filter(|vertex| vertex[3] == 0.0).count(),
        6,
        "rect slab template should include one dynamic top/bottom cap slot"
    );
    assert_eq!(
        vertices.iter().filter(|vertex| vertex[3] == 1.0).count(),
        6,
        "rect slab template should include one dynamic X-side slot"
    );
    assert_eq!(
        vertices.iter().filter(|vertex| vertex[3] == 2.0).count(),
        6,
        "rect slab template should include one dynamic Y-side slot"
    );
}
