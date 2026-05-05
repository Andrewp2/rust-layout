use serde::{Deserialize, Serialize};

pub type Coord = i64;

pub const DBU_PER_MICRON: Coord = 1_000;
pub const DEFAULT_GRID: Coord = 10;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Point {
    pub x: Coord,
    pub y: Coord,
}

impl Point {
    pub const ZERO: Self = Self { x: 0, y: 0 };

    pub const fn new(x: Coord, y: Coord) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Self) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn snap(self, grid: Coord) -> Self {
        if grid <= 1 {
            return self;
        }
        Self {
            x: snap_coord(self.x, grid),
            y: snap_coord(self.y, grid),
        }
    }

    pub fn translated(self, delta: Vector) -> Self {
        Self {
            x: self.x + delta.dx,
            y: self.y + delta.dy,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Vector {
    pub dx: Coord,
    pub dy: Coord,
}

impl Vector {
    pub const ZERO: Self = Self { dx: 0, dy: 0 };

    pub const fn new(dx: Coord, dy: Coord) -> Self {
        Self { dx, dy }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn new(a: Point, b: Point) -> Self {
        Self {
            min: Point::new(a.x.min(b.x), a.y.min(b.y)),
            max: Point::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn from_min_size(min: Point, width: Coord, height: Coord) -> Self {
        Self {
            min,
            max: Point::new(min.x + width.max(0), min.y + height.max(0)),
        }
    }

    pub fn from_points(points: &[Point]) -> Option<Self> {
        let mut iter = points.iter().copied();
        let first = iter.next()?;
        let mut rect = Self::new(first, first);
        for point in iter {
            rect = rect.union(Self::new(point, point));
        }
        Some(rect)
    }

    pub fn width(self) -> Coord {
        self.max.x - self.min.x
    }

    pub fn height(self) -> Coord {
        self.max.y - self.min.y
    }

    pub fn center(self) -> Point {
        Point::new((self.min.x + self.max.x) / 2, (self.min.y + self.max.y) / 2)
    }

    pub fn area(self) -> Coord {
        self.width().max(0) * self.height().max(0)
    }

    pub fn contains_point(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn contains_rect(self, other: Self) -> bool {
        self.contains_point(other.min) && self.contains_point(other.max)
    }

    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }
        Some(Self {
            min: Point::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y)),
            max: Point::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y)),
        })
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            min: Point::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Point::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    pub fn expanded(self, amount: Coord) -> Self {
        Self {
            min: Point::new(self.min.x - amount, self.min.y - amount),
            max: Point::new(self.max.x + amount, self.max.y + amount),
        }
    }

    pub fn translated(self, delta: Vector) -> Self {
        Self {
            min: self.min.translated(delta),
            max: self.max.translated(delta),
        }
    }

    pub fn corners(self) -> [Point; 4] {
        [
            self.min,
            Point::new(self.max.x, self.min.y),
            self.max,
            Point::new(self.min.x, self.max.y),
        ]
    }

    pub fn distance_to_rect(self, other: Self) -> f64 {
        let dx = if self.max.x < other.min.x {
            other.min.x - self.max.x
        } else if other.max.x < self.min.x {
            self.min.x - other.max.x
        } else {
            0
        };
        let dy = if self.max.y < other.min.y {
            other.min.y - self.max.y
        } else if other.max.y < self.min.y {
            self.min.y - other.max.y
        } else {
            0
        };
        let dx = dx as f64;
        let dy = dy as f64;
        (dx * dx + dy * dy).sqrt()
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(Point::ZERO, Point::ZERO)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Polygon {
    pub points: Vec<Point>,
}

impl Polygon {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    pub fn bounds(&self) -> Option<Rect> {
        Rect::from_points(&self.points)
    }

    pub fn signed_area2(&self) -> i128 {
        let len = self.points.len();
        if len < 3 {
            return 0;
        }
        let mut sum = 0i128;
        for index in 0..len {
            let a = self.points[index];
            let b = self.points[(index + 1) % len];
            sum += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
        }
        sum
    }

    pub fn area(&self) -> f64 {
        self.signed_area2().unsigned_abs() as f64 * 0.5
    }

    pub fn translate(&mut self, delta: Vector) {
        for point in &mut self.points {
            *point = point.translated(delta);
        }
    }

    pub fn contains_point(&self, point: Point) -> bool {
        let len = self.points.len();
        if len < 3 {
            return false;
        }
        let mut inside = false;
        let mut j = len - 1;
        for i in 0..len {
            let pi = self.points[i];
            let pj = self.points[j];
            let crosses = (pi.y > point.y) != (pj.y > point.y);
            if crosses {
                let x_intersection = (pj.x - pi.x) as f64 * (point.y - pi.y) as f64
                    / (pj.y - pi.y) as f64
                    + pi.x as f64;
                if (point.x as f64) < x_intersection {
                    inside = !inside;
                }
            }
            j = i;
        }
        inside
    }

    pub fn min_edge_length(&self) -> Option<f64> {
        if self.points.len() < 2 {
            return None;
        }
        let mut best = f64::INFINITY;
        for index in 0..self.points.len() {
            let a = self.points[index];
            let b = self.points[(index + 1) % self.points.len()];
            best = best.min(a.distance_to(b));
        }
        Some(best)
    }
}

pub fn snap_coord(value: Coord, grid: Coord) -> Coord {
    if grid <= 1 {
        return value;
    }
    let half = grid / 2;
    if value >= 0 {
        ((value + half) / grid) * grid
    } else {
        ((value - half) / grid) * grid
    }
}

pub fn distance_point_to_segment(point: Point, a: Point, b: Point) -> f64 {
    let px = point.x as f64;
    let py = point.y as f64;
    let ax = a.x as f64;
    let ay = a.y as f64;
    let bx = b.x as f64;
    let by = b.y as f64;
    let abx = bx - ax;
    let aby = by - ay;
    let len2 = abx * abx + aby * aby;
    if len2 <= f64::EPSILON {
        return point.distance_to(a);
    }
    let t = (((px - ax) * abx + (py - ay) * aby) / len2).clamp(0.0, 1.0);
    let cx = ax + t * abx;
    let cy = ay + t * aby;
    let dx = px - cx;
    let dy = py - cy;
    (dx * dx + dy * dy).sqrt()
}

pub fn point_segment_hit(point: Point, a: Point, b: Point, tolerance: Coord) -> bool {
    distance_point_to_segment(point, a, b) <= tolerance as f64
}

pub fn manhattan(a: Point, b: Point) -> Coord {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_distance_is_zero_when_overlapping() {
        let a = Rect::from_min_size(Point::new(0, 0), 10, 10);
        let b = Rect::from_min_size(Point::new(5, 5), 10, 10);
        assert_eq!(a.distance_to_rect(b), 0.0);
    }

    #[test]
    fn polygon_area_uses_shoelace_formula() {
        let poly = Polygon::new(vec![
            Point::new(0, 0),
            Point::new(10, 0),
            Point::new(10, 10),
            Point::new(0, 10),
        ]);
        assert_eq!(poly.area(), 100.0);
    }
}
