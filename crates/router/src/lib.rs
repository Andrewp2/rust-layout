use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

use geometry_core::{Coord, Point, Rect, manhattan};
use layout_model::{Document, LayerId, NetId, ShapeId, ShapeKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RouterConfig {
    pub grid: Coord,
    pub bend_cost: i64,
    pub max_expansions: usize,
    pub obstacle_margin: Coord,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            grid: 80,
            bend_cost: 18,
            max_expansions: 250_000,
            obstacle_margin: 80,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteRequest {
    pub start: Point,
    pub goal: Point,
    pub layer: LayerId,
    pub wire_width: Coord,
    pub bounds: Option<Rect>,
    #[serde(default)]
    pub allowed_net: Option<NetId>,
    #[serde(default)]
    pub ignored_shape_ids: Vec<ShapeId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub points: Vec<Point>,
    pub cost: i64,
    pub visited_nodes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteError {
    NoPath { visited_nodes: usize },
    InvalidGrid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Cell {
    x: i64,
    y: i64,
    dir: Direction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Direction {
    Start,
    East,
    West,
    North,
    South,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QueueNode {
    estimated_total: i64,
    cost: i64,
    cell: Cell,
}

impl Ord for QueueNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimated_total
            .cmp(&self.estimated_total)
            .then_with(|| other.cost.cmp(&self.cost))
    }
}

impl PartialOrd for QueueNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn route(
    document: &Document,
    request: RouteRequest,
    config: RouterConfig,
) -> Result<Route, RouteError> {
    if config.grid <= 0 {
        return Err(RouteError::InvalidGrid);
    }
    let start = world_to_cell(request.start, config.grid);
    let goal = world_to_cell(request.goal, config.grid);
    let route_bounds = request.bounds.unwrap_or_else(|| {
        Rect::new(request.start, request.goal)
            .expanded(8_000)
            .expanded(request.wire_width + config.obstacle_margin)
    });
    let min_cell = world_to_cell(route_bounds.min, config.grid);
    let max_cell = world_to_cell(route_bounds.max, config.grid);
    let ignored_shape_ids = request
        .ignored_shape_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let blocked = build_blocked_cells(
        document,
        request.layer,
        config.grid,
        request.wire_width + config.obstacle_margin,
        request.allowed_net,
        &ignored_shape_ids,
    );

    let mut open = BinaryHeap::new();
    let start_cell = Cell {
        x: start.0,
        y: start.1,
        dir: Direction::Start,
    };
    open.push(QueueNode {
        estimated_total: heuristic(start, goal, config.grid),
        cost: 0,
        cell: start_cell,
    });

    let mut best: HashMap<Cell, i64> = HashMap::new();
    let mut came_from: HashMap<Cell, Cell> = HashMap::new();
    best.insert(start_cell, 0);
    let mut visited_nodes = 0usize;
    let goal_xy = (goal.0, goal.1);
    let mut goal_cell = None;

    while let Some(node) = open.pop() {
        visited_nodes += 1;
        if visited_nodes > config.max_expansions {
            break;
        }
        if (node.cell.x, node.cell.y) == goal_xy {
            goal_cell = Some(node.cell);
            break;
        }
        for (nx, ny, direction) in neighbors(node.cell.x, node.cell.y) {
            if nx < min_cell.0 || nx > max_cell.0 || ny < min_cell.1 || ny > max_cell.1 {
                continue;
            }
            if blocked.contains(&(nx, ny)) && (nx, ny) != goal_xy && (nx, ny) != (start.0, start.1)
            {
                continue;
            }
            let step_cost = config.grid
                + if node.cell.dir != Direction::Start && node.cell.dir != direction {
                    config.bend_cost
                } else {
                    0
                };
            let next_cost = node.cost + step_cost;
            let next = Cell {
                x: nx,
                y: ny,
                dir: direction,
            };
            if best.get(&next).is_some_and(|known| *known <= next_cost) {
                continue;
            }
            best.insert(next, next_cost);
            came_from.insert(next, node.cell);
            open.push(QueueNode {
                estimated_total: next_cost + heuristic((nx, ny), goal, config.grid),
                cost: next_cost,
                cell: next,
            });
        }
    }

    let Some(goal_cell) = goal_cell else {
        return Err(RouteError::NoPath { visited_nodes });
    };
    let cost = best.get(&goal_cell).copied().unwrap_or_default();
    let mut cells = Vec::new();
    let mut current = goal_cell;
    cells.push(current);
    while current != start_cell {
        let Some(prev) = came_from.get(&current).copied() else {
            break;
        };
        current = prev;
        cells.push(current);
    }
    cells.reverse();

    let mut points: Vec<Point> = cells
        .into_iter()
        .map(|cell| cell_to_world((cell.x, cell.y), config.grid))
        .collect();
    simplify_orthogonal_route(&mut points);
    if let Some(first) = points.first_mut() {
        *first = request.start.snap(config.grid);
    }
    if let Some(last) = points.last_mut() {
        *last = request.goal.snap(config.grid);
    }
    Ok(Route {
        points,
        cost,
        visited_nodes,
    })
}

fn build_blocked_cells(
    document: &Document,
    route_layer: LayerId,
    grid: Coord,
    margin: Coord,
    allowed_net: Option<NetId>,
    ignored_shape_ids: &HashSet<ShapeId>,
) -> HashSet<(i64, i64)> {
    let mut blocked = HashSet::new();
    for flattened in document.flattened_shapes() {
        if ignored_shape_ids.contains(&flattened.source_shape_id()) {
            continue;
        }
        let shape = flattened.transformed_shape();
        if shape.layer != route_layer {
            continue;
        }
        if allowed_net.is_some() && shape.net == allowed_net {
            continue;
        }
        if matches!(
            shape.kind,
            ShapeKind::Label { .. } | ShapeKind::Measurement { .. }
        ) {
            continue;
        }
        let bounds = shape.kind.bounds().expanded(margin);
        let min = world_to_cell(bounds.min, grid);
        let max = world_to_cell(bounds.max, grid);
        for x in min.0..=max.0 {
            for y in min.1..=max.1 {
                blocked.insert((x, y));
            }
        }
    }
    blocked
}

fn world_to_cell(point: Point, grid: Coord) -> (i64, i64) {
    (point.x.div_euclid(grid), point.y.div_euclid(grid))
}

fn cell_to_world(cell: (i64, i64), grid: Coord) -> Point {
    Point::new(cell.0 * grid, cell.1 * grid)
}

fn heuristic(a: (i64, i64), b: (i64, i64), grid: Coord) -> i64 {
    manhattan(cell_to_world(a, grid), cell_to_world(b, grid))
}

fn neighbors(x: i64, y: i64) -> [(i64, i64, Direction); 4] {
    [
        (x + 1, y, Direction::East),
        (x - 1, y, Direction::West),
        (x, y + 1, Direction::North),
        (x, y - 1, Direction::South),
    ]
}

fn simplify_orthogonal_route(points: &mut Vec<Point>) {
    if points.len() < 3 {
        return;
    }
    let mut out = Vec::with_capacity(points.len());
    out.push(points[0]);
    for window in points.windows(3) {
        let a = window[0];
        let b = window[1];
        let c = window[2];
        let same_x = a.x == b.x && b.x == c.x;
        let same_y = a.y == b.y && b.y == c.y;
        if !same_x && !same_y {
            out.push(b);
        }
    }
    out.push(*points.last().unwrap());
    *points = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_core::Rect;
    use layout_model::{Document, NetId, Operation, ProcessLayer, ShapeKind, Transform};

    #[test]
    fn routes_around_blocker() {
        let mut doc = Document::new("route");
        let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        doc.insert_shape(
            layer,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, -100), 400, 200)),
        );
        let route = route(
            &doc,
            RouteRequest {
                start: Point::new(0, 0),
                goal: Point::new(900, 0),
                layer,
                wire_width: 80,
                bounds: Some(Rect::from_min_size(Point::new(-200, -600), 1400, 1200)),
                allowed_net: None,
                ignored_shape_ids: Vec::new(),
            },
            RouterConfig::default(),
        )
        .unwrap();
        assert!(route.points.len() > 2);
    }

    #[test]
    fn routes_around_hidden_flattened_instance_blocker() {
        let mut doc = Document::new("route hierarchy");
        let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = doc.create_cell("blocker");
        doc.insert_shape_in_cell(
            child,
            layer,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, -100), 400, 200)),
        )
        .unwrap();
        doc.insert_instance_in_top(child, Transform::translate(200, 0))
            .unwrap();
        doc.apply_operation_without_log(&Operation::SetLayerVisibility {
            layer,
            visible: false,
        });

        let route = route(
            &doc,
            RouteRequest {
                start: Point::new(0, 0),
                goal: Point::new(900, 0),
                layer,
                wire_width: 80,
                bounds: Some(Rect::from_min_size(Point::new(-200, -600), 1400, 1200)),
                allowed_net: None,
                ignored_shape_ids: Vec::new(),
            },
            RouterConfig::default(),
        )
        .unwrap();

        assert!(route.points.len() > 2);
    }

    #[test]
    fn allowed_net_shapes_do_not_block_route() {
        let mut doc = Document::new("route same net");
        let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let blocker = doc.insert_shape(
            layer,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, -100), 400, 200)),
        );
        doc.shapes.get_mut(&blocker).unwrap().net = Some(NetId(7));

        let route = route(
            &doc,
            RouteRequest {
                start: Point::new(0, 0),
                goal: Point::new(900, 0),
                layer,
                wire_width: 80,
                bounds: Some(Rect::from_min_size(Point::new(-200, -600), 1400, 1200)),
                allowed_net: Some(NetId(7)),
                ignored_shape_ids: Vec::new(),
            },
            RouterConfig::default(),
        )
        .unwrap();

        assert_eq!(
            route.points,
            vec![Point::new(0, 0), Point::new(880, 0)],
            "same-net geometry should be treated as usable route context"
        );
    }
}
