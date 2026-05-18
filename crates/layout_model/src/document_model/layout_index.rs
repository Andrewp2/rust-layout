#![allow(unused_imports)]
use super::*;

impl LayoutIndex {
    pub fn rebuild(document: &Document) -> Self {
        let mut entries = Vec::with_capacity(document.shapes.len());
        entries.extend(
            document
                .shapes
                .live_rows()
                .filter(|&row| {
                    document
                        .layers
                        .get(&document.shapes.row_layer(row))
                        .is_some_and(|layer| layer.visible)
                })
                .map(|row| IndexedShape {
                    id: ShapeOccurrenceId::top_level(document.shapes.row_id(row)),
                    bounds: document.shapes.row_bounds(row),
                }),
        );
        Self::bulk_load(entries)
    }

    pub fn rebuild_hierarchical(document: &Document) -> Self {
        Self::rebuild_hierarchical_with_max_depth(document, None)
    }

    pub fn rebuild_hierarchical_with_max_depth(
        document: &Document,
        max_depth: Option<usize>,
    ) -> Self {
        Self::rebuild_hierarchical_for_cell_with_depth_range(
            document,
            document.top_cell,
            0,
            max_depth,
        )
    }

    pub fn rebuild_hierarchical_for_cell(
        document: &Document,
        root_cell: CellId,
        max_depth: Option<usize>,
    ) -> Self {
        Self::rebuild_hierarchical_for_cell_with_depth_range(document, root_cell, 0, max_depth)
    }

    pub fn rebuild_hierarchical_for_cell_with_depth_range(
        document: &Document,
        root_cell: CellId,
        min_depth: usize,
        max_depth: Option<usize>,
    ) -> Self {
        let mut entries = Vec::with_capacity(document.flattened_shape_count_estimate());
        document.append_visible_index_entries(root_cell, &mut entries, min_depth, max_depth);
        Self::bulk_load(entries)
    }

    pub fn bulk_load(entries: Vec<IndexedShape>) -> Self {
        let mut refs = Vec::with_capacity(entries.len());
        for (entry, shape) in entries.iter().enumerate() {
            let min_x = shape.bounds.min.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
            let max_x = shape.bounds.max.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
            let min_y = shape.bounds.min.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
            let max_y = shape.bounds.max.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    refs.push(LayoutIndexRef {
                        key: LayoutIndexTileKey { x, y },
                        entry,
                    });
                }
            }
        }
        refs.sort_unstable_by_key(|reference| reference.key);

        let mut buckets = Vec::new();
        let mut start = 0;
        while start < refs.len() {
            let key = refs[start].key;
            let mut end = start + 1;
            while end < refs.len() && refs[end].key == key {
                end += 1;
            }
            buckets.push(LayoutIndexBucket { key, start, end });
            start = end;
        }

        Self {
            entries,
            refs,
            buckets,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn bounds(&self) -> Option<Rect> {
        self.entries
            .iter()
            .map(|entry| entry.bounds)
            .reduce(|left, right| left.union(right))
    }

    pub fn query_occurrences_into(&self, rect: Rect, out: &mut Vec<ShapeOccurrenceId>) {
        let start_len = out.len();
        let min_x = rect.min.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let max_x = rect.max.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let min_y = rect.min.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let max_y = rect.max.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let key = LayoutIndexTileKey { x, y };
                let Ok(bucket_index) = self.buckets.binary_search_by_key(&key, |bucket| bucket.key)
                else {
                    continue;
                };
                let bucket = self.buckets[bucket_index];
                for reference in &self.refs[bucket.start..bucket.end] {
                    let entry = &self.entries[reference.entry];
                    if entry.bounds.intersects(rect) {
                        out.push(entry.id.clone());
                    }
                }
            }
        }

        out[start_len..].sort_unstable();
        let mut write = start_len;
        for read in start_len..out.len() {
            if read == start_len || out[read] != out[write - 1] {
                if write != read {
                    out[write] = out[read].clone();
                }
                write += 1;
            }
        }
        out.truncate(write);
    }

    pub fn query_rect(&self, rect: Rect) -> Vec<ShapeId> {
        self.query_occurrences(rect)
            .into_iter()
            .map(|id| id.source_shape_id())
            .collect()
    }

    pub fn query_occurrences(&self, rect: Rect) -> Vec<ShapeOccurrenceId> {
        let mut out = Vec::new();
        self.query_occurrences_into(rect, &mut out);
        out
    }

    pub fn query_occurrences_limited(&self, rect: Rect, limit: usize) -> Vec<ShapeOccurrenceId> {
        let mut out = Vec::with_capacity(limit.min(1024));
        self.query_occurrences_limited_into(rect, limit, &mut out);
        out
    }

    pub fn query_occurrences_limited_into(
        &self,
        rect: Rect,
        limit: usize,
        out: &mut Vec<ShapeOccurrenceId>,
    ) {
        if limit == 0 {
            return;
        }

        let start_len = out.len();
        let min_x = rect.min.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let max_x = rect.max.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let min_y = rect.min.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let max_y = rect.max.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let mut seen_entries = BTreeSet::new();
        let mut limit_reached = false;
        'tiles: for y in min_y..=max_y {
            for x in min_x..=max_x {
                let key = LayoutIndexTileKey { x, y };
                let Ok(bucket_index) = self.buckets.binary_search_by_key(&key, |bucket| bucket.key)
                else {
                    continue;
                };
                let bucket = self.buckets[bucket_index];
                for reference in &self.refs[bucket.start..bucket.end] {
                    let entry = &self.entries[reference.entry];
                    if entry.bounds.intersects(rect) && seen_entries.insert(reference.entry) {
                        out.push(entry.id.clone());
                        if out.len() - start_len >= limit {
                            limit_reached = true;
                            break 'tiles;
                        }
                    }
                }
            }
        }

        if limit_reached {
            warn!(
                limit,
                returned = out.len() - start_len,
                "layout index query reached result limit; results were truncated"
            );
        }

        out[start_len..].sort_unstable();
    }

    pub fn hit_test(&self, point: Point, tolerance: Coord) -> Option<ShapeId> {
        self.hit_test_occurrence(point, tolerance)
            .map(|id| id.source_shape_id())
    }

    pub fn hit_test_occurrence(&self, point: Point, tolerance: Coord) -> Option<ShapeOccurrenceId> {
        let query = Rect::new(point, point).expanded(tolerance);
        self.query_occurrences(query).into_iter().next()
    }
}

impl Document {
    pub(crate) fn append_visible_index_entries(
        &self,
        root_cell: CellId,
        entries: &mut Vec<IndexedShape>,
        min_depth: usize,
        max_depth: Option<usize>,
    ) {
        if root_cell == self.top_cell {
            for row in self.shapes.live_rows() {
                if !self.layer_is_visible(self.shapes.row_layer(row)) {
                    continue;
                }
                if min_depth > 0 {
                    continue;
                }
                entries.push(IndexedShape {
                    id: ShapeOccurrenceId::top_level(self.shapes.row_id(row)),
                    bounds: self.shapes.row_bounds(row),
                });
            }
        }

        let identity = Transform::IDENTITY;
        let mut instance_path = Vec::new();
        let mut array_path = Vec::new();
        if let Some(root) = self.cells.get(&root_cell) {
            for row in root.shapes.live_rows() {
                if !self.layer_is_visible(root.shapes.row_layer(row)) {
                    continue;
                }
                if min_depth > 0 {
                    continue;
                }
                entries.push(IndexedShape {
                    id: ShapeOccurrenceId::top_level(root.shapes.row_id(row)),
                    bounds: root.shapes.row_bounds(row),
                });
            }
            self.append_instance_index_entries(
                root,
                identity,
                min_depth,
                max_depth,
                1,
                &mut instance_path,
                &mut array_path,
                entries,
            );
        }
    }

    pub(crate) fn append_instance_index_entries(
        &self,
        parent: &Cell,
        parent_transform: Transform,
        min_depth: usize,
        max_depth: Option<usize>,
        depth: usize,
        instance_path: &mut Vec<InstanceId>,
        array_path: &mut Vec<ArrayIndex>,
        entries: &mut Vec<IndexedShape>,
    ) {
        if max_depth.is_some_and(|limit| depth > limit) {
            return;
        }

        for instance_row in parent.instances.live_rows() {
            let instance_id = parent.instances.row_id(instance_row);
            if instance_path.contains(&instance_id) {
                continue;
            }
            let Some(cell) = self.cells.get(&parent.instances.row_cell(instance_row)) else {
                continue;
            };
            let instance_transform = parent.instances.row_transform(instance_row);
            let array = parent.instances.row_array(instance_row).normalized();
            let include_array_index = !array.is_single();
            for row in 0..array.rows {
                for column in 0..array.columns {
                    let offset = array.element_offset(column, row);
                    let transform = parent_transform
                        .compose(Transform::from_translation(offset).compose(instance_transform));
                    instance_path.push(instance_id);
                    if include_array_index {
                        array_path.push(ArrayIndex { column, row });
                    }
                    for shape_row in cell.shapes.live_rows() {
                        if !self.layer_is_visible(cell.shapes.row_layer(shape_row)) {
                            continue;
                        }
                        if depth < min_depth {
                            continue;
                        }
                        entries.push(IndexedShape {
                            id: ShapeOccurrenceId::from_instance_array_path(
                                cell.shapes.row_id(shape_row),
                                instance_path,
                                array_path,
                            ),
                            bounds: transform.apply_rect(cell.shapes.row_bounds(shape_row)),
                        });
                    }
                    self.append_instance_index_entries(
                        cell,
                        transform,
                        min_depth,
                        max_depth,
                        depth + 1,
                        instance_path,
                        array_path,
                        entries,
                    );
                    if include_array_index {
                        array_path.pop();
                    }
                    instance_path.pop();
                }
            }
        }
    }

    pub fn flattened_shape_count_estimate(&self) -> usize {
        let mut count = self.shapes.len();
        if let Some(top) = self.cells.get(&self.top_cell) {
            count += top.shapes.len();
            count += self.flattened_instance_shape_count_estimate(top, &mut Vec::new());
        }
        count
    }

    pub(crate) fn flattened_instance_shape_count_estimate(
        &self,
        parent: &Cell,
        instance_path: &mut Vec<InstanceId>,
    ) -> usize {
        let mut count = 0;
        for instance_row in parent.instances.live_rows() {
            let instance_id = parent.instances.row_id(instance_row);
            if instance_path.contains(&instance_id) {
                continue;
            }
            let Some(cell) = self.cells.get(&parent.instances.row_cell(instance_row)) else {
                continue;
            };
            let array = parent.instances.row_array(instance_row).normalized();
            let array_count = array.columns as usize * array.rows as usize;
            count += cell.shapes.len() * array_count;
            instance_path.push(instance_id);
            count +=
                self.flattened_instance_shape_count_estimate(cell, instance_path) * array_count;
            instance_path.pop();
        }
        count
    }
}
