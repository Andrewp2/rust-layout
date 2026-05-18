#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_drag_world(
    start: Point,
    target: Point,
    modifiers: operad::KeyModifiers,
) -> Point {
    if modifiers.shift {
        axis_constrained_point(start, target)
    } else {
        target
    }
}

pub(crate) fn rect_size(rect: UiRect) -> UiSize {
    UiSize::new(rect.width, rect.height)
}

pub(crate) fn local_canvas_point(point: UiPoint, rect: UiRect) -> UiPoint {
    UiPoint::new(point.x - rect.x, point.y - rect.y)
}

pub(crate) fn fixed_height_layout(style: LayoutStyle, height: f32) -> LayoutStyle {
    layout::with_min_size(
        layout::with_flex(style, 0.0, 0.0, layout::px(height)),
        layout::px(0.0),
        layout::px(height),
    )
}

pub(crate) fn fixed_row_child_layout(style: LayoutStyle, width: f32, height: f32) -> LayoutStyle {
    layout::with_min_size(
        layout::with_flex(style, 0.0, 0.0, layout::px(width)),
        layout::px(width),
        layout::px(height),
    )
}

pub(crate) fn layout_occurrence_label(occurrence: &ShapeOccurrenceId) -> String {
    if occurrence.is_top_level() {
        format!("shape #{}", occurrence.source_shape_id().0)
    } else {
        format!("shape #{} instance", occurrence.source_shape_id().0)
    }
}

pub(crate) fn format_physical_length_with_options(
    length_dbu: f64,
    dbu_per_micron: Coord,
    unit_display: UnitDisplay,
    precision: usize,
) -> String {
    let dbu_per_micron = dbu_per_micron.max(1) as f64;
    match unit_display {
        UnitDisplay::Dbu => format!("{length_dbu:.0} dbu"),
        UnitDisplay::Nanometers => {
            let nm = length_dbu / dbu_per_micron * 1000.0;
            format!("{nm:.precision$} nm")
        }
        UnitDisplay::Microns => {
            let um = length_dbu / dbu_per_micron;
            format!("{um:.precision$} um")
        }
        UnitDisplay::Auto => {
            let nm = length_dbu / dbu_per_micron * 1000.0;
            if nm.abs() >= 1000.0 {
                let um = nm / 1000.0;
                format!("{um:.precision$} um")
            } else {
                format!("{nm:.precision$} nm")
            }
        }
    }
}

pub(crate) fn layout_rect_end(
    start: Point,
    target: Point,
    modifiers: operad::KeyModifiers,
) -> Point {
    if modifiers.shift {
        square_constrained_point(start, target)
    } else {
        target
    }
}

pub(crate) fn layout_vertex_drag_position(
    original_position: Point,
    start: Point,
    target: Point,
    modifiers: operad::KeyModifiers,
) -> Point {
    let constrained = layout_drag_world(start, target, modifiers);
    original_position.translated(Vector::new(
        constrained.x - start.x,
        constrained.y - start.y,
    ))
}

pub(crate) fn axis_constrained_point(start: Point, target: Point) -> Point {
    let dx = target.x - start.x;
    let dy = target.y - start.y;
    if dx.abs() >= dy.abs() {
        Point::new(target.x, start.y)
    } else {
        Point::new(start.x, target.y)
    }
}

pub(crate) fn square_constrained_point(start: Point, target: Point) -> Point {
    let dx = target.x - start.x;
    let dy = target.y - start.y;
    let side = dx.abs().max(dy.abs());
    Point::new(
        start.x + if dx < 0 { -side } else { side },
        start.y + if dy < 0 { -side } else { side },
    )
}

pub(crate) fn parse_layout_origin_point(input: &str) -> Option<Point> {
    let normalized = input.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let x = parts.next()?.parse::<Coord>().ok()?;
    let y = parts.next()?.parse::<Coord>().ok()?;
    parts.next().is_none().then_some(Point::new(x, y))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShapeTransform {
    RotateCw90,
    MirrorX,
    MirrorY,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutShapeClipboardBoolean {
    And,
    Or,
    Not,
    Xor,
}

impl LayoutShapeClipboardBoolean {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
            Self::Not => "NOT",
            Self::Xor => "XOR",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutAlignEdge {
    Left,
    Right,
    Top,
    Bottom,
    CenterX,
    CenterY,
    OriginX,
    OriginY,
}

impl std::fmt::Display for LayoutAlignEdge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::CenterX => "center x",
            Self::CenterY => "center y",
            Self::OriginX => "origin x",
            Self::OriginY => "origin y",
        })
    }
}

pub(crate) fn rect_center_x(rect: Rect) -> Coord {
    rect.min.x + (rect.max.x - rect.min.x) / 2
}

pub(crate) fn rect_center_y(rect: Rect) -> Coord {
    rect.min.y + (rect.max.y - rect.min.y) / 2
}

pub(crate) fn transform_shape_kind(
    kind: &ShapeKind,
    center: Point,
    transform: ShapeTransform,
) -> ShapeKind {
    let mut transformed = kind.clone();
    match &mut transformed {
        ShapeKind::Rectangle(rect) => {
            let points = rect
                .corners()
                .into_iter()
                .map(|point| transform_point(point, center, transform))
                .collect::<Vec<_>>();
            *rect = Rect::from_points(&points).unwrap_or(*rect);
        }
        ShapeKind::Polygon(poly) => {
            for point in &mut poly.points {
                *point = transform_point(*point, center, transform);
            }
        }
        ShapeKind::Path { points, .. } => {
            for point in points {
                *point = transform_point(*point, center, transform);
            }
        }
        ShapeKind::Via {
            center: via_center, ..
        }
        | ShapeKind::Label {
            position: via_center,
            ..
        } => {
            *via_center = transform_point(*via_center, center, transform);
        }
        ShapeKind::Measurement { a, b, .. } => {
            *a = transform_point(*a, center, transform);
            *b = transform_point(*b, center, transform);
        }
    }
    transformed
}

pub(crate) fn transform_point(point: Point, center: Point, transform: ShapeTransform) -> Point {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    match transform {
        ShapeTransform::RotateCw90 => Point::new(center.x + dy, center.y - dx),
        ShapeTransform::MirrorX => Point::new(point.x, center.y - dy),
        ShapeTransform::MirrorY => Point::new(center.x - dx, point.y),
    }
}

pub(crate) fn sized_shape_kind(kind: &ShapeKind, amount: Coord) -> Option<ShapeKind> {
    match kind {
        ShapeKind::Rectangle(rect) => sized_rect(*rect, amount).map(ShapeKind::Rectangle),
        ShapeKind::Polygon(poly) => sized_polygon(poly, amount).map(ShapeKind::Polygon),
        ShapeKind::Path { points, width } => {
            let width = width.checked_add(amount.saturating_mul(2))?;
            (width > 0).then(|| ShapeKind::Path {
                points: points.clone(),
                width,
            })
        }
        ShapeKind::Via {
            center,
            size,
            lower,
            upper,
        } => {
            let size = size.checked_add(amount.saturating_mul(2))?;
            (size > 0).then(|| ShapeKind::Via {
                center: *center,
                size,
                lower: *lower,
                upper: *upper,
            })
        }
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => None,
    }
}

pub(crate) fn sized_rect(rect: Rect, amount: Coord) -> Option<Rect> {
    let rect = rect.expanded(amount);
    (rect.width() > 0 && rect.height() > 0).then_some(rect)
}

#[derive(Clone, Copy)]
pub(crate) struct OffsetLine {
    pub(crate) point: (f64, f64),
    pub(crate) direction: (f64, f64),
}

pub(crate) fn sized_polygon(poly: &Polygon, amount: Coord) -> Option<Polygon> {
    let points = region_points_for_shape_kind(&ShapeKind::Polygon(poly.clone()))?;
    if !polygon_is_simple(&points) {
        return None;
    }
    let orientation = signed_area2_points(&points).signum();
    let offset = amount as f64;
    let mut lines = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        let dx = (b.x - a.x) as f64;
        let dy = (b.y - a.y) as f64;
        let length = (dx * dx + dy * dy).sqrt();
        if length <= f64::EPSILON {
            return None;
        }
        let (normal_x, normal_y) = if orientation >= 0 {
            (dy / length, -dx / length)
        } else {
            (-dy / length, dx / length)
        };
        lines.push(OffsetLine {
            point: (
                a.x as f64 + normal_x * offset,
                a.y as f64 + normal_y * offset,
            ),
            direction: (dx, dy),
        });
    }

    let mut sized_points = Vec::with_capacity(lines.len());
    for index in 0..lines.len() {
        let previous = lines[(index + lines.len() - 1) % lines.len()];
        let current = lines[index];
        sized_points.push(intersect_offset_lines(previous, current)?);
    }

    let sized_points = normalize_polygon_points(sized_points)?;
    if signed_area2_points(&sized_points).signum() != orientation
        || !polygon_is_simple(&sized_points)
    {
        return None;
    }
    Some(Polygon::new(sized_points))
}

pub(crate) fn polygon_is_simple(points: &[Point]) -> bool {
    if points.len() < 3 || signed_area2_points(points) == 0 {
        return false;
    }
    for edge_index in 0..points.len() {
        let a = points[edge_index];
        let b = points[(edge_index + 1) % points.len()];
        for other_index in edge_index + 1..points.len() {
            if other_index == edge_index
                || other_index == (edge_index + 1) % points.len()
                || edge_index == (other_index + 1) % points.len()
            {
                continue;
            }
            let c = points[other_index];
            let d = points[(other_index + 1) % points.len()];
            if segments_intersect(a, b, c, d) {
                return false;
            }
        }
    }
    true
}

pub(crate) fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    let ab_c = cross_points(a, b, c);
    let ab_d = cross_points(a, b, d);
    let cd_a = cross_points(c, d, a);
    let cd_b = cross_points(c, d, b);

    if ab_c == 0 && point_on_segment(c, a, b) {
        return true;
    }
    if ab_d == 0 && point_on_segment(d, a, b) {
        return true;
    }
    if cd_a == 0 && point_on_segment(a, c, d) {
        return true;
    }
    if cd_b == 0 && point_on_segment(b, c, d) {
        return true;
    }

    ab_c.signum() != ab_d.signum() && cd_a.signum() != cd_b.signum()
}

pub(crate) fn point_on_segment(point: Point, start: Point, end: Point) -> bool {
    point.x >= start.x.min(end.x)
        && point.x <= start.x.max(end.x)
        && point.y >= start.y.min(end.y)
        && point.y <= start.y.max(end.y)
}

pub(crate) fn intersect_offset_lines(a: OffsetLine, b: OffsetLine) -> Option<Point> {
    let denominator = a.direction.0 * b.direction.1 - a.direction.1 * b.direction.0;
    if denominator.abs() <= f64::EPSILON {
        return None;
    }
    let delta_x = b.point.0 - a.point.0;
    let delta_y = b.point.1 - a.point.1;
    let t = (delta_x * b.direction.1 - delta_y * b.direction.0) / denominator;
    let x = a.point.0 + t * a.direction.0;
    let y = a.point.1 + t * a.direction.1;
    Some(Point::new(coord_from_f64(x)?, coord_from_f64(y)?))
}

pub(crate) fn coord_from_f64(value: f64) -> Option<Coord> {
    if !value.is_finite() || value < Coord::MIN as f64 || value > Coord::MAX as f64 {
        return None;
    }
    Some(value.round() as Coord)
}

pub(crate) fn region_points_for_shape_kind(kind: &ShapeKind) -> Option<Vec<Point>> {
    match kind {
        ShapeKind::Rectangle(rect) if rect.width() > 0 && rect.height() > 0 => {
            Some(rect.corners().to_vec())
        }
        ShapeKind::Polygon(poly) => normalize_polygon_points(poly.points.clone()),
        _ => None,
    }
}

pub(crate) fn convex_region_points_for_shape_kind(kind: &ShapeKind) -> Option<Vec<Point>> {
    let points = region_points_for_shape_kind(kind)?;
    is_convex_polygon(&points).then_some(points)
}

pub(crate) fn convex_region_parts_for_shape_kind(kind: &ShapeKind) -> Option<Vec<Vec<Point>>> {
    let points = region_points_for_shape_kind(kind)?;
    if is_convex_polygon(&points) {
        return Some(vec![points]);
    }
    triangulate_simple_polygon(&points)
}

pub(crate) fn clip_shape_kind_to_region(
    kind: &ShapeKind,
    clip_points: &[Point],
) -> Option<ShapeKind> {
    let subject_points = region_points_for_shape_kind(kind)?;
    let clipped_points = clip_polygon_to_convex_region(&subject_points, clip_points)?;
    shape_kind_from_region_points(clipped_points)
}

pub(crate) fn subtract_convex_regions_from_shape_kind<'a, I>(
    kind: &ShapeKind,
    cuts: I,
) -> Vec<ShapeKind>
where
    I: IntoIterator<Item = &'a [Point]>,
{
    let cuts = cuts.into_iter().collect::<Vec<_>>();
    if let ShapeKind::Rectangle(rect) = kind {
        let cut_rects = cuts
            .iter()
            .filter_map(|points| rect_from_polygon_points(points))
            .collect::<Vec<_>>();
        if cut_rects.len() == cuts.len() {
            return subtract_rects(*rect, &cut_rects)
                .into_iter()
                .map(ShapeKind::Rectangle)
                .collect();
        }
    }
    let Some(initial_points) = convex_region_points_for_shape_kind(kind) else {
        return Vec::new();
    };
    let mut fragments = vec![initial_points];
    for cut in cuts {
        let mut next = Vec::new();
        for fragment in fragments {
            next.extend(subtract_convex_region_points(&fragment, cut));
        }
        fragments = next;
        if fragments.is_empty() {
            break;
        }
    }
    fragments
        .into_iter()
        .filter_map(shape_kind_from_region_points)
        .collect()
}

pub(crate) fn subtract_convex_regions_from_region_parts<'a, I>(
    kind: &ShapeKind,
    cuts: I,
) -> Vec<ShapeKind>
where
    I: IntoIterator<Item = &'a [Point]>,
{
    let cuts = cuts.into_iter().collect::<Vec<_>>();
    let Some(parts) = convex_region_parts_for_shape_kind(kind) else {
        return Vec::new();
    };
    let mut fragments = Vec::new();
    for part in parts {
        let Some(part_kind) = shape_kind_from_region_points(part) else {
            continue;
        };
        fragments.extend(subtract_convex_regions_from_shape_kind(
            &part_kind,
            cuts.iter().copied(),
        ));
    }
    fragments
}

pub(crate) fn subtract_convex_region_points(
    subject: &[Point],
    cut_points: &[Point],
) -> Vec<Vec<Point>> {
    let Some(mut remaining) = normalize_polygon_points(subject.to_vec()) else {
        return Vec::new();
    };
    let Some(cut_points) = normalize_polygon_points(cut_points.to_vec()) else {
        return vec![remaining];
    };
    if !is_convex_polygon(&remaining) || !is_convex_polygon(&cut_points) {
        return Vec::new();
    }
    let clip_orientation = signed_area2_points(&cut_points).signum();
    let mut fragments = Vec::new();

    for edge_index in 0..cut_points.len() {
        let edge_start = cut_points[edge_index];
        let edge_end = cut_points[(edge_index + 1) % cut_points.len()];
        if let Some(outside) =
            clip_polygon_to_half_plane(&remaining, edge_start, edge_end, clip_orientation, false)
        {
            fragments.push(outside);
        }
        let Some(inside) =
            clip_polygon_to_half_plane(&remaining, edge_start, edge_end, clip_orientation, true)
        else {
            return fragments;
        };
        remaining = inside;
    }

    fragments
}

pub(crate) fn clip_polygon_to_convex_region(
    subject: &[Point],
    clip_points: &[Point],
) -> Option<Vec<Point>> {
    let mut output = normalize_polygon_points(subject.to_vec())?;
    let clip_points = normalize_polygon_points(clip_points.to_vec())?;
    if !is_convex_polygon(&clip_points) {
        return None;
    }
    let clip_orientation = signed_area2_points(&clip_points).signum();

    for edge_index in 0..clip_points.len() {
        let edge_start = clip_points[edge_index];
        let edge_end = clip_points[(edge_index + 1) % clip_points.len()];
        let input = output;
        output = Vec::new();

        let mut previous = *input.last()?;
        let mut previous_inside =
            point_inside_clip_edge(previous, edge_start, edge_end, clip_orientation);
        for current in input {
            let current_inside =
                point_inside_clip_edge(current, edge_start, edge_end, clip_orientation);
            match (previous_inside, current_inside) {
                (true, true) => push_unique_polygon_point(&mut output, current),
                (true, false) => {
                    if let Some(point) =
                        segment_line_intersection(previous, current, edge_start, edge_end)
                    {
                        push_unique_polygon_point(&mut output, point);
                    }
                }
                (false, true) => {
                    if let Some(point) =
                        segment_line_intersection(previous, current, edge_start, edge_end)
                    {
                        push_unique_polygon_point(&mut output, point);
                    }
                    push_unique_polygon_point(&mut output, current);
                }
                (false, false) => {}
            }
            previous = current;
            previous_inside = current_inside;
        }

        output = cleanup_polygon_points(output);
        if output.is_empty() {
            return None;
        }
    }

    normalize_polygon_points(output)
}

pub(crate) fn clip_polygon_to_half_plane(
    subject: &[Point],
    edge_start: Point,
    edge_end: Point,
    clip_orientation: i128,
    keep_inside: bool,
) -> Option<Vec<Point>> {
    let input = normalize_polygon_points(subject.to_vec())?;
    let mut output = Vec::new();

    let mut previous = *input.last()?;
    let mut previous_kept = point_kept_by_half_plane(
        previous,
        edge_start,
        edge_end,
        clip_orientation,
        keep_inside,
    );
    for current in input {
        let current_kept =
            point_kept_by_half_plane(current, edge_start, edge_end, clip_orientation, keep_inside);
        match (previous_kept, current_kept) {
            (true, true) => push_unique_polygon_point(&mut output, current),
            (true, false) => {
                if let Some(point) =
                    segment_line_intersection(previous, current, edge_start, edge_end)
                {
                    push_unique_polygon_point(&mut output, point);
                }
            }
            (false, true) => {
                if let Some(point) =
                    segment_line_intersection(previous, current, edge_start, edge_end)
                {
                    push_unique_polygon_point(&mut output, point);
                }
                push_unique_polygon_point(&mut output, current);
            }
            (false, false) => {}
        }
        previous = current;
        previous_kept = current_kept;
    }

    normalize_polygon_points(output)
}

pub(crate) fn point_kept_by_half_plane(
    point: Point,
    edge_start: Point,
    edge_end: Point,
    clip_orientation: i128,
    keep_inside: bool,
) -> bool {
    let cross = cross_points(edge_start, edge_end, point);
    if keep_inside {
        if clip_orientation >= 0 {
            cross >= 0
        } else {
            cross <= 0
        }
    } else if clip_orientation >= 0 {
        cross < 0
    } else {
        cross > 0
    }
}

pub(crate) fn point_inside_clip_edge(
    point: Point,
    edge_start: Point,
    edge_end: Point,
    clip_orientation: i128,
) -> bool {
    let cross = cross_points(edge_start, edge_end, point);
    if clip_orientation >= 0 {
        cross >= 0
    } else {
        cross <= 0
    }
}

pub(crate) fn segment_line_intersection(
    start: Point,
    end: Point,
    line_start: Point,
    line_end: Point,
) -> Option<Point> {
    let sx = (end.x - start.x) as f64;
    let sy = (end.y - start.y) as f64;
    let lx = (line_end.x - line_start.x) as f64;
    let ly = (line_end.y - line_start.y) as f64;
    let denominator = sx * ly - sy * lx;
    if denominator.abs() <= f64::EPSILON {
        return None;
    }
    let t =
        ((line_start.x - start.x) as f64 * ly - (line_start.y - start.y) as f64 * lx) / denominator;
    Some(Point::new(
        (start.x as f64 + t * sx).round() as Coord,
        (start.y as f64 + t * sy).round() as Coord,
    ))
}

pub(crate) fn shape_kind_from_region_points(points: Vec<Point>) -> Option<ShapeKind> {
    let points = normalize_polygon_points(points)?;
    if let Some(rect) = rect_from_polygon_points(&points) {
        return Some(ShapeKind::Rectangle(rect));
    }
    Some(ShapeKind::Polygon(Polygon::new(points)))
}

pub(crate) fn rect_from_polygon_points(points: &[Point]) -> Option<Rect> {
    if points.len() != 4 {
        return None;
    }
    let rect = Rect::from_points(points)?;
    if rect.width() <= 0 || rect.height() <= 0 {
        return None;
    }
    let corners = rect.corners();
    points
        .iter()
        .all(|point| corners.contains(point))
        .then_some(rect)
}

pub(crate) fn normalize_polygon_points(points: Vec<Point>) -> Option<Vec<Point>> {
    let points = cleanup_polygon_points(points);
    (points.len() >= 3 && signed_area2_points(&points) != 0).then_some(points)
}

pub(crate) fn cleanup_polygon_points(points: Vec<Point>) -> Vec<Point> {
    let mut deduped = Vec::with_capacity(points.len());
    for point in points {
        push_unique_polygon_point(&mut deduped, point);
    }
    while deduped.len() > 1 && deduped.first() == deduped.last() {
        deduped.pop();
    }

    let mut simplified = deduped;
    loop {
        if simplified.len() < 3 {
            return simplified;
        }
        let mut changed = false;
        let mut next = Vec::with_capacity(simplified.len());
        for index in 0..simplified.len() {
            let previous = simplified[(index + simplified.len() - 1) % simplified.len()];
            let current = simplified[index];
            let following = simplified[(index + 1) % simplified.len()];
            if cross_points(previous, current, following) == 0 {
                changed = true;
                continue;
            }
            push_unique_polygon_point(&mut next, current);
        }
        while next.len() > 1 && next.first() == next.last() {
            next.pop();
        }
        simplified = next;
        if !changed {
            return simplified;
        }
    }
}

pub(crate) fn push_unique_polygon_point(points: &mut Vec<Point>, point: Point) {
    if points.last().copied() != Some(point) {
        points.push(point);
    }
}

pub(crate) fn is_convex_polygon(points: &[Point]) -> bool {
    if points.len() < 3 || signed_area2_points(points) == 0 {
        return false;
    }
    let mut turn_direction = 0;
    for index in 0..points.len() {
        let previous = points[index];
        let current = points[(index + 1) % points.len()];
        let following = points[(index + 2) % points.len()];
        let cross = cross_points(previous, current, following);
        if cross == 0 {
            continue;
        }
        let sign = cross.signum();
        if turn_direction == 0 {
            turn_direction = sign;
        } else if turn_direction != sign {
            return false;
        }
    }
    turn_direction != 0
}

pub(crate) fn triangulate_simple_polygon(points: &[Point]) -> Option<Vec<Vec<Point>>> {
    let mut remaining = normalize_polygon_points(points.to_vec())?;
    if remaining.len() == 3 {
        return Some(vec![remaining]);
    }
    let orientation = signed_area2_points(&remaining).signum();
    if orientation == 0 {
        return None;
    }
    let mut triangles = Vec::new();
    let mut guard = 0usize;
    while remaining.len() > 3 {
        guard += 1;
        if guard > points.len().saturating_mul(points.len()).max(16) {
            return None;
        }
        let mut ear_index = None;
        for index in 0..remaining.len() {
            if polygon_vertex_is_ear(&remaining, index, orientation) {
                ear_index = Some(index);
                break;
            }
        }
        let index = ear_index?;
        let previous = remaining[(index + remaining.len() - 1) % remaining.len()];
        let current = remaining[index];
        let next = remaining[(index + 1) % remaining.len()];
        triangles.push(vec![previous, current, next]);
        remaining.remove(index);
    }
    triangles.push(remaining);
    Some(triangles)
}

pub(crate) fn polygon_vertex_is_ear(points: &[Point], index: usize, orientation: i128) -> bool {
    let previous = points[(index + points.len() - 1) % points.len()];
    let current = points[index];
    let next = points[(index + 1) % points.len()];
    if cross_points(previous, current, next).signum() != orientation {
        return false;
    }
    for (candidate_index, candidate) in points.iter().enumerate() {
        if candidate_index == index
            || candidate_index == (index + points.len() - 1) % points.len()
            || candidate_index == (index + 1) % points.len()
        {
            continue;
        }
        if point_in_or_on_triangle(*candidate, previous, current, next) {
            return false;
        }
    }
    true
}

pub(crate) fn point_in_or_on_triangle(point: Point, a: Point, b: Point, c: Point) -> bool {
    let area = cross_points(a, b, c);
    if area == 0 {
        return false;
    }
    let orientation = area.signum();
    [
        cross_points(a, b, point),
        cross_points(b, c, point),
        cross_points(c, a, point),
    ]
    .into_iter()
    .all(|cross| cross == 0 || cross.signum() == orientation)
}

pub(crate) fn signed_area2_points(points: &[Point]) -> i128 {
    let mut sum = 0i128;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        sum += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
    }
    sum
}

pub(crate) fn cross_points(a: Point, b: Point, c: Point) -> i128 {
    let abx = (b.x - a.x) as i128;
    let aby = (b.y - a.y) as i128;
    let acx = (c.x - a.x) as i128;
    let acy = (c.y - a.y) as i128;
    abx * acy - aby * acx
}

pub(crate) fn chamfered_rect_polygon(
    rect: Rect,
    amount: Coord,
) -> Option<(geometry_core::Polygon, Coord)> {
    let amount = amount.abs();
    let max_chamfer = rect.width().min(rect.height()).saturating_sub(1) / 2;
    let chamfer = amount.min(max_chamfer);
    if chamfer <= 0 {
        return None;
    }
    Some((
        geometry_core::Polygon::new(vec![
            Point::new(rect.min.x + chamfer, rect.min.y),
            Point::new(rect.max.x - chamfer, rect.min.y),
            Point::new(rect.max.x, rect.min.y + chamfer),
            Point::new(rect.max.x, rect.max.y - chamfer),
            Point::new(rect.max.x - chamfer, rect.max.y),
            Point::new(rect.min.x + chamfer, rect.max.y),
            Point::new(rect.min.x, rect.max.y - chamfer),
            Point::new(rect.min.x, rect.min.y + chamfer),
        ]),
        chamfer,
    ))
}

pub(crate) fn chamfered_shape_polygon(
    kind: &ShapeKind,
    amount: Coord,
) -> Option<(geometry_core::Polygon, Coord)> {
    match kind {
        ShapeKind::Rectangle(rect) => chamfered_rect_polygon(*rect, amount),
        ShapeKind::Polygon(poly) => chamfered_polygon(poly, amount),
        _ => None,
    }
}

pub(crate) fn chamfered_polygon(
    poly: &geometry_core::Polygon,
    amount: Coord,
) -> Option<(geometry_core::Polygon, Coord)> {
    let points = region_points_for_shape_kind(&ShapeKind::Polygon(poly.clone()))?;
    let amount = polygon_corner_amount(&points, amount)?;
    let mut chamfered = Vec::with_capacity(points.len() * 2);
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        push_unique_polygon_point(&mut chamfered, point_toward(current, previous, amount)?);
        push_unique_polygon_point(&mut chamfered, point_toward(current, next, amount)?);
    }
    Some((
        geometry_core::Polygon::new(normalize_polygon_points(chamfered)?),
        amount,
    ))
}

pub(crate) fn rounded_rect_polygon(
    rect: Rect,
    amount: Coord,
) -> Option<(geometry_core::Polygon, Coord)> {
    let amount = amount.abs();
    let max_radius = rect.width().min(rect.height()).saturating_sub(1) / 2;
    let radius = amount.min(max_radius);
    if radius <= 0 {
        return None;
    }
    let diagonal = ((radius as i128 * 293 + 500) / 1_000).clamp(1, radius as i128) as Coord;
    Some((
        geometry_core::Polygon::new(vec![
            Point::new(rect.min.x + radius, rect.min.y),
            Point::new(rect.max.x - radius, rect.min.y),
            Point::new(rect.max.x - diagonal, rect.min.y + diagonal),
            Point::new(rect.max.x, rect.min.y + radius),
            Point::new(rect.max.x, rect.max.y - radius),
            Point::new(rect.max.x - diagonal, rect.max.y - diagonal),
            Point::new(rect.max.x - radius, rect.max.y),
            Point::new(rect.min.x + radius, rect.max.y),
            Point::new(rect.min.x + diagonal, rect.max.y - diagonal),
            Point::new(rect.min.x, rect.max.y - radius),
            Point::new(rect.min.x, rect.min.y + radius),
            Point::new(rect.min.x + diagonal, rect.min.y + diagonal),
        ]),
        radius,
    ))
}

pub(crate) fn rounded_shape_polygon(
    kind: &ShapeKind,
    amount: Coord,
) -> Option<(geometry_core::Polygon, Coord)> {
    match kind {
        ShapeKind::Rectangle(rect) => rounded_rect_polygon(*rect, amount),
        ShapeKind::Polygon(poly) => rounded_polygon(poly, amount),
        _ => None,
    }
}

pub(crate) fn rounded_polygon(
    poly: &geometry_core::Polygon,
    amount: Coord,
) -> Option<(geometry_core::Polygon, Coord)> {
    let points = region_points_for_shape_kind(&ShapeKind::Polygon(poly.clone()))?;
    let radius = polygon_corner_amount(&points, amount)?;
    let diagonal = ((radius as i128 * 293 + 500) / 1_000).clamp(1, radius as i128) as Coord;
    let mut rounded = Vec::with_capacity(points.len() * 3);
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        push_unique_polygon_point(&mut rounded, point_toward(current, previous, radius)?);
        push_unique_polygon_point(
            &mut rounded,
            rounded_corner_midpoint(current, previous, next, diagonal)?,
        );
        push_unique_polygon_point(&mut rounded, point_toward(current, next, radius)?);
    }
    Some((
        geometry_core::Polygon::new(normalize_polygon_points(rounded)?),
        radius,
    ))
}

pub(crate) fn polygon_corner_amount(points: &[Point], amount: Coord) -> Option<Coord> {
    let amount = amount.abs();
    if amount <= 0 || points.len() < 3 {
        return None;
    }
    let mut max_amount = Coord::MAX;
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let shortest =
            segment_length_floor(current, previous).min(segment_length_floor(current, next));
        max_amount = max_amount.min(shortest.saturating_sub(1) / 2);
    }
    let amount = amount.min(max_amount);
    (amount > 0).then_some(amount)
}

pub(crate) fn segment_length_floor(a: Point, b: Point) -> Coord {
    let dx = (b.x - a.x) as f64;
    let dy = (b.y - a.y) as f64;
    ((dx * dx + dy * dy).sqrt().floor()).clamp(0.0, Coord::MAX as f64) as Coord
}

pub(crate) fn point_toward(from: Point, toward: Point, distance: Coord) -> Option<Point> {
    let dx = (toward.x - from.x) as f64;
    let dy = (toward.y - from.y) as f64;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f64::EPSILON {
        return None;
    }
    let scale = distance as f64 / length;
    Some(Point::new(
        coord_from_f64(from.x as f64 + dx * scale)?,
        coord_from_f64(from.y as f64 + dy * scale)?,
    ))
}

pub(crate) fn rounded_corner_midpoint(
    current: Point,
    previous: Point,
    next: Point,
    diagonal: Coord,
) -> Option<Point> {
    let prev_dx = (previous.x - current.x) as f64;
    let prev_dy = (previous.y - current.y) as f64;
    let next_dx = (next.x - current.x) as f64;
    let next_dy = (next.y - current.y) as f64;
    let prev_length = (prev_dx * prev_dx + prev_dy * prev_dy).sqrt();
    let next_length = (next_dx * next_dx + next_dy * next_dy).sqrt();
    if prev_length <= f64::EPSILON || next_length <= f64::EPSILON {
        return None;
    }
    let x = current.x as f64 + (prev_dx / prev_length + next_dx / next_length) * diagonal as f64;
    let y = current.y as f64 + (prev_dy / prev_length + next_dy / next_length) * diagonal as f64;
    Some(Point::new(coord_from_f64(x)?, coord_from_f64(y)?))
}

pub(crate) fn positive_area_rect(min: Point, max: Point) -> Option<Rect> {
    let rect = Rect::new(min, max);
    (rect.width() > 0 && rect.height() > 0).then_some(rect)
}

pub(crate) fn rect_union_rectangles(rects: &[Rect]) -> Vec<Rect> {
    let rects = rects
        .iter()
        .copied()
        .filter(|rect| rect.area() > 0)
        .collect::<Vec<_>>();
    if rects.is_empty() {
        return Vec::new();
    }

    let mut xs = rects
        .iter()
        .flat_map(|rect| [rect.min.x, rect.max.x])
        .collect::<Vec<_>>();
    xs.sort_unstable();
    xs.dedup();
    let mut ys = rects
        .iter()
        .flat_map(|rect| [rect.min.y, rect.max.y])
        .collect::<Vec<_>>();
    ys.sort_unstable();
    ys.dedup();
    if xs.len() < 2 || ys.len() < 2 {
        return Vec::new();
    }

    let mut union = Vec::new();
    let mut open_runs: BTreeMap<(usize, usize), Coord> = BTreeMap::new();
    for y_index in 0..ys.len() - 1 {
        let y0 = ys[y_index];
        let y1 = ys[y_index + 1];
        if y1 <= y0 {
            continue;
        }
        let mut next_open_runs = BTreeMap::new();
        for range in rect_union_row_ranges(&rects, &xs, y0, y1) {
            let start_y = open_runs.remove(&range).unwrap_or(y0);
            next_open_runs.insert(range, start_y);
        }
        for ((start_x, end_x), start_y) in open_runs {
            if let Some(rect) =
                positive_area_rect(Point::new(xs[start_x], start_y), Point::new(xs[end_x], y0))
            {
                union.push(rect);
            }
        }
        open_runs = next_open_runs;
    }
    let final_y = *ys.last().unwrap_or(&0);
    for ((start_x, end_x), start_y) in open_runs {
        if let Some(rect) = positive_area_rect(
            Point::new(xs[start_x], start_y),
            Point::new(xs[end_x], final_y),
        ) {
            union.push(rect);
        }
    }
    union.sort_by_key(|rect| (rect.min.x, rect.min.y, rect.max.x, rect.max.y));
    union
}

pub(crate) fn rect_union_row_ranges(
    rects: &[Rect],
    xs: &[Coord],
    y0: Coord,
    y1: Coord,
) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut x_index = 0;
    while x_index + 1 < xs.len() {
        if rect_union_cell_covered(rects, xs[x_index], xs[x_index + 1], y0, y1) {
            let start = x_index;
            x_index += 1;
            while x_index + 1 < xs.len()
                && rect_union_cell_covered(rects, xs[x_index], xs[x_index + 1], y0, y1)
            {
                x_index += 1;
            }
            ranges.push((start, x_index));
        } else {
            x_index += 1;
        }
    }
    ranges
}

pub(crate) fn rect_union_cell_covered(
    rects: &[Rect],
    x0: Coord,
    x1: Coord,
    y0: Coord,
    y1: Coord,
) -> bool {
    x1 > x0
        && y1 > y0
        && rects.iter().any(|rect| {
            x0 >= rect.min.x && x1 <= rect.max.x && y0 >= rect.min.y && y1 <= rect.max.y
        })
}

pub(crate) fn rect_difference(rect: Rect, cut: Rect) -> Vec<Rect> {
    let Some(overlap) = rect.intersection(cut) else {
        return vec![rect];
    };
    if overlap.width() <= 0 || overlap.height() <= 0 {
        return vec![rect];
    }

    let mut fragments = Vec::new();
    if let Some(fragment) = positive_area_rect(rect.min, Point::new(rect.max.x, overlap.min.y)) {
        fragments.push(fragment);
    }
    if let Some(fragment) = positive_area_rect(Point::new(rect.min.x, overlap.max.y), rect.max) {
        fragments.push(fragment);
    }
    if let Some(fragment) = positive_area_rect(
        Point::new(rect.min.x, overlap.min.y),
        Point::new(overlap.min.x, overlap.max.y),
    ) {
        fragments.push(fragment);
    }
    if let Some(fragment) = positive_area_rect(
        Point::new(overlap.max.x, overlap.min.y),
        Point::new(rect.max.x, overlap.max.y),
    ) {
        fragments.push(fragment);
    }
    fragments
}

pub(crate) fn subtract_rects(rect: Rect, cuts: &[Rect]) -> Vec<Rect> {
    let mut fragments = vec![rect];
    for cut in cuts {
        let mut next = Vec::new();
        for fragment in fragments {
            next.extend(rect_difference(fragment, *cut));
        }
        fragments = next;
        if fragments.is_empty() {
            break;
        }
    }
    fragments
}

pub(crate) fn same_optional_net(shapes: &[Shape]) -> Option<NetId> {
    let first = shapes.first()?.net?;
    shapes
        .iter()
        .all(|shape| shape.net == Some(first))
        .then_some(first)
}

pub(crate) fn editable_vertex_points(kind: &ShapeKind) -> Vec<Point> {
    kind.key_points()
}

pub(crate) fn editable_edges(kind: &ShapeKind) -> Vec<(Point, Point)> {
    let points = editable_vertex_points(kind);
    match kind {
        ShapeKind::Rectangle(_) | ShapeKind::Polygon(_) if !points.is_empty() => points
            .iter()
            .enumerate()
            .map(|(index, point)| (*point, points[(index + 1) % points.len()]))
            .collect(),
        ShapeKind::Path { .. } | ShapeKind::Measurement { .. } => {
            points.windows(2).map(|pair| (pair[0], pair[1])).collect()
        }
        _ => Vec::new(),
    }
}

pub(crate) fn shape_edge_is_draggable(kind: &ShapeKind, edge: usize) -> bool {
    edge < editable_edges(kind).len()
}

pub(crate) fn shape_with_moved_vertex(
    shape: &Shape,
    vertex: usize,
    position: Point,
) -> Option<Shape> {
    let mut shape = shape.clone();
    match &mut shape.kind {
        ShapeKind::Rectangle(rect) => {
            let mut corners = rect.corners();
            *corners.get_mut(vertex)? = position;
            *rect = Rect::from_points(&corners)?;
        }
        ShapeKind::Polygon(poly) => *poly.points.get_mut(vertex)? = position,
        ShapeKind::Path { points, .. } => *points.get_mut(vertex)? = position,
        ShapeKind::Via { center, .. }
        | ShapeKind::Label {
            position: center, ..
        } => {
            if vertex != 0 {
                return None;
            }
            *center = position;
        }
        ShapeKind::Measurement { a, b, .. } => match vertex {
            0 => *a = position,
            1 => *b = position,
            _ => return None,
        },
    }
    Some(shape)
}

pub(crate) fn shape_with_inserted_vertex(
    shape: &Shape,
    edge: usize,
    position: Point,
) -> Option<Shape> {
    let mut shape = shape.clone();
    match &mut shape.kind {
        ShapeKind::Rectangle(rect) => {
            let mut points = rect.corners().to_vec();
            if edge >= points.len() {
                return None;
            }
            points.insert(edge + 1, position);
            shape.kind = ShapeKind::Polygon(geometry_core::Polygon::new(points));
        }
        ShapeKind::Polygon(poly) => {
            if edge >= poly.points.len() {
                return None;
            }
            poly.points.insert(edge + 1, position);
        }
        ShapeKind::Path { points, .. } => {
            if edge + 1 > points.len() {
                return None;
            }
            points.insert(edge + 1, position);
        }
        _ => return None,
    }
    Some(shape)
}

pub(crate) fn shape_with_deleted_vertex(shape: &Shape, vertex: usize) -> Option<Shape> {
    let mut shape = shape.clone();
    match &mut shape.kind {
        ShapeKind::Polygon(poly) if poly.points.len() > 3 => {
            poly.points.remove(vertex);
        }
        ShapeKind::Path { points, .. } if points.len() > 2 => {
            points.remove(vertex);
        }
        _ => return None,
    }
    Some(shape)
}

pub(crate) fn shape_with_moved_edge(shape: &Shape, edge: usize, delta: Vector) -> Option<Shape> {
    let mut shape = shape.clone();
    match &mut shape.kind {
        ShapeKind::Rectangle(rect) => {
            let mut corners = rect.corners();
            let len = corners.len();
            corners[edge % len] = corners[edge % len].translated(delta);
            corners[(edge + 1) % len] = corners[(edge + 1) % len].translated(delta);
            *rect = Rect::from_points(&corners)?;
        }
        ShapeKind::Polygon(poly) => {
            let len = poly.points.len();
            if edge >= len {
                return None;
            }
            poly.points[edge] = poly.points[edge].translated(delta);
            poly.points[(edge + 1) % len] = poly.points[(edge + 1) % len].translated(delta);
        }
        ShapeKind::Path { points, .. } => {
            if edge + 1 >= points.len() {
                return None;
            }
            points[edge] = points[edge].translated(delta);
            points[edge + 1] = points[edge + 1].translated(delta);
        }
        ShapeKind::Measurement { a, b, .. } => {
            if edge != 0 {
                return None;
            }
            *a = a.translated(delta);
            *b = b.translated(delta);
        }
        ShapeKind::Via { .. } | ShapeKind::Label { .. } => return None,
    }
    Some(shape)
}

pub(crate) fn display_document_name(name: &str) -> String {
    name.strip_prefix("Glassworks demo ")
        .map(|suffix| format!("Sample {suffix}"))
        .unwrap_or_else(|| name.to_string())
}

pub(crate) fn format_fps_label(frame_ms: Option<f64>) -> String {
    frame_ms
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| format!("FPS: {:.1}", 1000.0 / value))
        .unwrap_or_else(|| "FPS: --".to_string())
}

pub(crate) fn equipment_tool_label_for_raw_id(workspace: &WorkspaceDataset, id: &str) -> String {
    workspace
        .equipment
        .tools()
        .find(|tool| tool.id.as_str() == id)
        .map(|tool| compact_button_label(&tool.name, 18))
        .unwrap_or_else(|| compact_button_label(id, 18))
}

pub(crate) fn display_spc_finding_detail(
    _workspace: &WorkspaceDataset,
    finding: &layout_model::spc_fdc::MonitorFinding,
) -> String {
    finding.detail.clone()
}

pub(crate) fn drc_violation_is_active(document: &Document, violation: &DrcViolation) -> bool {
    document
        .marker_states
        .get(&violation.stable_key())
        .is_none_or(|state| !state.hidden && !state.waived)
}

pub(crate) fn drc_violation_intersects_region(
    violation: &DrcViolation,
    region_points: &[Point],
) -> bool {
    let mut bounds = violation.bounds;
    if bounds.width() <= 0 || bounds.height() <= 0 {
        bounds = bounds.expanded(1);
    }
    clip_polygon_to_convex_region(&bounds.corners(), region_points).is_some()
}

pub(crate) fn drc_violation_intersects_any_region(
    violation: &DrcViolation,
    region_parts: &[Vec<Point>],
) -> bool {
    region_parts
        .iter()
        .any(|region_points| drc_violation_intersects_region(violation, region_points))
}

pub(crate) fn layout_drc_marker_output_bounds(bounds: Rect, grid: Coord) -> Rect {
    let grid = grid.max(1);
    let pad = grid.max(10) * 2;
    let expanded = bounds.expanded(pad);
    let mut min_x = snap_coord_floor(expanded.min.x, grid);
    let mut min_y = snap_coord_floor(expanded.min.y, grid);
    let mut max_x = snap_coord_ceil(expanded.max.x, grid);
    let mut max_y = snap_coord_ceil(expanded.max.y, grid);
    if max_x <= min_x {
        min_x -= grid;
        max_x += grid;
    }
    if max_y <= min_y {
        min_y -= grid;
        max_y += grid;
    }
    Rect::new(Point::new(min_x, min_y), Point::new(max_x, max_y))
}

pub(crate) fn snap_coord_floor(value: Coord, grid: Coord) -> Coord {
    if grid <= 1 {
        return value;
    }
    value.div_euclid(grid) * grid
}

pub(crate) fn snap_coord_ceil(value: Coord, grid: Coord) -> Coord {
    if grid <= 1 {
        return value;
    }
    let floor = snap_coord_floor(value, grid);
    if floor == value { value } else { floor + grid }
}
