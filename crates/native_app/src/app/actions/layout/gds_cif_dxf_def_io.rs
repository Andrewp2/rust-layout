#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_json_as_top_cell_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Layout JSON top-cell import failed: {error}");
                return true;
            }
        };
        let mut document = match serde_json::from_str::<Document>(&contents) {
            Ok(document) => document,
            Err(error) => {
                self.status_message = format!("Layout JSON top-cell import failed: {error}");
                return true;
            }
        };
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("Layout JSON top-cell import failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let shape_count = document.flattened_shape_count_estimate();
        let source_cell_count = document.cells.len();
        let import = match self.prepare_layout_json_cell_import(&document, &import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("Layout JSON top-cell import failed: {error}");
                return true;
            }
        };
        let LayoutJsonCellImportPlan {
            redo_operations,
            undo_operations,
            first_shape,
            instance_id,
            root_cell,
            root_cell_name,
            added_layers,
        } = import;
        let redo_operations = redo_operations
            .into_iter()
            .filter(|operation| {
                !matches!(operation, Operation::AddInstance { instance, .. } if instance.id == instance_id)
            })
            .collect::<Vec<_>>();
        let undo_operations = undo_operations
            .into_iter()
            .filter(|operation| {
                !matches!(operation, Operation::DeleteInstance { id, .. } if *id == instance_id)
            })
            .collect::<Vec<_>>();
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("Layout JSON top-cell import failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: redo_operations,
            },
            Operation::Batch {
                operations: undo_operations,
            },
        );
        self.layout_view_top_cell = root_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = first_shape;
        self.selected_layout_occurrence = first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("layout_json", path);
        self.status_message = format!(
            "Imported layout JSON {} as extra top cell {} ({} shape(s), {} source cell(s), {} layer(s))",
            path.display(),
            root_cell_name,
            shape_count,
            source_cell_count,
            added_layers
        );
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_json_as_cell_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Layout JSON cell import failed: {error}");
                return true;
            }
        };
        let mut document = match serde_json::from_str::<Document>(&contents) {
            Ok(document) => document,
            Err(error) => {
                self.status_message = format!("Layout JSON cell import failed: {error}");
                return true;
            }
        };
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("Layout JSON cell import failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let shape_count = document.flattened_shape_count_estimate();
        let source_cell_count = document.cells.len();
        let import = match self.prepare_layout_json_cell_import(&document, &import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("Layout JSON cell import failed: {error}");
                return true;
            }
        };
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("Layout JSON cell import failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import
            .first_shape
            .map(|id| ShapeOccurrenceId::from_instance_path(id, &[import.instance_id]));
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("layout_json", path);
        self.status_message = format!(
            "Imported layout JSON {} as cell {} ({} shape(s), {} source cell(s), {} layer(s))",
            path.display(),
            import.root_cell_name,
            shape_count,
            source_cell_count,
            import.added_layers
        );
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn prepare_layout_json_cell_import(
        &mut self,
        document: &Document,
        import_name: &str,
    ) -> Result<LayoutJsonCellImportPlan, String> {
        let (layer_map, added_layers) = self.prepare_layout_json_import_layers(document)?;

        let source_cells = document.cells.values().cloned().collect::<Vec<_>>();
        if source_cells.is_empty() {
            return Err("imported layout has no cells".to_string());
        }
        let mut cell_map = BTreeMap::new();
        for cell in &source_cells {
            cell_map.insert(cell.id, self.workspace.document.allocate_cell_id());
        }
        let root_cell = *cell_map
            .get(&document.top_cell)
            .ok_or_else(|| "imported layout has no top cell".to_string())?;

        let mut imported_cells = Vec::new();
        let mut first_shape = None;
        for cell in &source_cells {
            let mapped_cell = *cell_map
                .get(&cell.id)
                .ok_or_else(|| format!("missing cell mapping for C{}", cell.id.0))?;
            let mut imported_cell = Cell::new(
                mapped_cell,
                if cell.id == document.top_cell {
                    format!(
                        "import {} root {}",
                        compact_button_label(import_name, 18),
                        mapped_cell.0
                    )
                } else {
                    format!(
                        "import {} {} {}",
                        compact_button_label(import_name, 12),
                        compact_button_label(&cell.name, 18),
                        mapped_cell.0
                    )
                },
            );
            imported_cell.properties = cell.properties.clone();
            imported_cell
                .properties
                .insert("import.source_document".to_string(), document.name.clone());
            imported_cell
                .properties
                .insert("import.source_cell".to_string(), cell.name.clone());

            let mut source_shapes = cell.shapes.values().collect::<Vec<_>>();
            if cell.id == document.top_cell {
                source_shapes.extend(document.shapes.values());
            }
            source_shapes.sort_by_key(|shape| shape.id);
            for shape in source_shapes {
                let imported_shape = self.remap_layout_json_import_shape(shape, &layer_map)?;
                if cell.id == document.top_cell && first_shape.is_none() {
                    first_shape = Some(imported_shape.id);
                }
                imported_cell
                    .shapes
                    .insert(imported_shape.id, imported_shape);
            }

            let mut instances = cell.instances.values().collect::<Vec<_>>();
            instances.sort_by_key(|instance| instance.id);
            for instance in instances {
                let target = *cell_map.get(&instance.cell).ok_or_else(|| {
                    format!(
                        "imported instance I{} references missing cell C{}",
                        instance.id.0, instance.cell.0
                    )
                })?;
                let mut imported_instance = instance;
                imported_instance.id = self.workspace.document.allocate_instance_id();
                imported_instance.cell = target;
                imported_cell
                    .instances
                    .insert(imported_instance.id, imported_instance);
            }
            imported_cells.push(imported_cell);
        }

        let origin = self
            .layout_canvas_size
            .map(|size| {
                self.snap_layout_point(self.layout_canvas_to_world_local(
                    UiPoint::new(size.width * 0.5, size.height * 0.5),
                    size,
                ))
            })
            .unwrap_or_else(|| self.snap_layout_point(Point::new(0, 0)));
        let origin = Point::new(
            origin.x.saturating_add(self.layout_import_offset.dx),
            origin.y.saturating_add(self.layout_import_offset.dy),
        );
        let instance_id = self.workspace.document.allocate_instance_id();
        let root_cell_name = imported_cells
            .iter()
            .find(|cell| cell.id == root_cell)
            .map(|cell| cell.name.clone())
            .unwrap_or_else(|| format!("import {}", compact_button_label(import_name, 18)));
        let top_cell = self.workspace.document.top_cell;
        let root_instance = CellInstance {
            id: instance_id,
            name: Some(format!("{} inst", root_cell_name)),
            cell: root_cell,
            transform: Transform::translate(origin.x, origin.y),
            array: InstanceArray::single(),
            properties: BTreeMap::new(),
        };

        let mut redo_operations = Vec::new();
        for layer in &added_layers {
            redo_operations.push(Operation::AddLayer {
                layer: layer.clone(),
            });
        }
        for cell in &imported_cells {
            redo_operations.push(Operation::AddCell { cell: cell.clone() });
        }
        redo_operations.push(Operation::AddInstance {
            parent: top_cell,
            instance: root_instance,
        });

        let mut undo_operations = vec![Operation::DeleteInstance {
            parent: top_cell,
            id: instance_id,
        }];
        for cell in imported_cells.iter().rev() {
            undo_operations.push(Operation::DeleteCell { id: cell.id });
        }
        for layer in added_layers.iter().rev() {
            undo_operations.push(Operation::DeleteLayer { id: layer.id });
        }

        Ok(LayoutJsonCellImportPlan {
            redo_operations,
            undo_operations,
            first_shape,
            instance_id,
            root_cell,
            root_cell_name,
            added_layers: added_layers.len(),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn prepare_layout_json_import_layers(
        &mut self,
        document: &Document,
    ) -> Result<(BTreeMap<LayerId, LayerId>, Vec<Layer>), String> {
        let mut layer_map = BTreeMap::new();
        let mut added_layers = Vec::new();
        let mut allocated_layer_ids = BTreeSet::new();
        let target_layers_by_name = self
            .workspace
            .document
            .layers
            .values()
            .map(|layer| (layout_import_layer_name_key(&layer.name), layer.id))
            .collect::<BTreeMap<_, _>>();

        for layer in document.layers.values() {
            let mapped_id = match self.layout_import_layer_policy {
                LayoutImportLayerPolicy::Id
                    if self.workspace.document.layers.contains_key(&layer.id) =>
                {
                    Some(layer.id)
                }
                LayoutImportLayerPolicy::Name => target_layers_by_name
                    .get(&layout_import_layer_name_key(&layer.name))
                    .copied(),
                LayoutImportLayerPolicy::Id | LayoutImportLayerPolicy::Copy => None,
            };
            let mapped_id = match mapped_id {
                Some(mapped_id) => mapped_id,
                None => self.allocate_layout_json_import_layer(
                    layer,
                    &mut allocated_layer_ids,
                    &mut added_layers,
                )?,
            };
            layer_map.insert(layer.id, mapped_id);
        }
        Ok((layer_map, added_layers))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn allocate_layout_json_import_layer(
        &mut self,
        layer: &Layer,
        allocated_layer_ids: &mut BTreeSet<LayerId>,
        added_layers: &mut Vec<Layer>,
    ) -> Result<LayerId, String> {
        if let Some(candidate) = self.preferred_layout_json_import_layer_id(layer.id)
            && !self.workspace.document.layers.contains_key(&candidate)
            && !allocated_layer_ids.contains(&candidate)
        {
            return self.add_layout_json_import_layer_copy(
                layer,
                candidate,
                allocated_layer_ids,
                added_layers,
            );
        }

        let mut candidate = self.workspace.document.next_layer_id.max(1);
        while self
            .workspace
            .document
            .layers
            .contains_key(&LayerId(candidate))
            || allocated_layer_ids.contains(&LayerId(candidate))
        {
            candidate = candidate
                .checked_add(1)
                .ok_or_else(|| "imported layout cannot allocate another layer id".to_string())?;
        }
        let mapped_id = LayerId(candidate);
        self.add_layout_json_import_layer_copy(layer, mapped_id, allocated_layer_ids, added_layers)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn preferred_layout_json_import_layer_id(
        &self,
        source_layer: LayerId,
    ) -> Option<LayerId> {
        if self.layout_import_layer_id_offset == 0 {
            return None;
        }
        let candidate = i64::from(source_layer.0) + i64::from(self.layout_import_layer_id_offset);
        (1..=i64::from(u32::MAX))
            .contains(&candidate)
            .then_some(LayerId(candidate as u32))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn add_layout_json_import_layer_copy(
        &mut self,
        layer: &Layer,
        mapped_id: LayerId,
        allocated_layer_ids: &mut BTreeSet<LayerId>,
        added_layers: &mut Vec<Layer>,
    ) -> Result<LayerId, String> {
        let next_layer_id = mapped_id
            .0
            .checked_add(1)
            .ok_or_else(|| "imported layout cannot allocate another layer id".to_string())?;
        self.workspace.document.next_layer_id =
            self.workspace.document.next_layer_id.max(next_layer_id);
        allocated_layer_ids.insert(mapped_id);
        let mut imported_layer = layer.clone();
        imported_layer.id = mapped_id;
        imported_layer.name = format!("Imported {}", imported_layer.name);
        added_layers.push(imported_layer);
        Ok(mapped_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn remap_layout_json_import_shape(
        &mut self,
        mut shape: Shape,
        layer_map: &BTreeMap<LayerId, LayerId>,
    ) -> Result<Shape, String> {
        shape.id = self.workspace.document.allocate_shape_id();
        shape.layer = remap_layout_json_import_layer(shape.layer, layer_map)?;
        if let ShapeKind::Via { lower, upper, .. } = &mut shape.kind {
            *lower = remap_layout_json_import_layer(*lower, layer_map)?;
            *upper = remap_layout_json_import_layer(*upper, layer_map)?;
        }
        Ok(shape)
    }

    pub(crate) fn export_layout_gds(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_gds_exchange_path();
            return self.export_layout_gds_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "GDS export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_gds_to_path(&mut self, path: &Path) -> bool {
        let result = match export_gdsii_with_report(
            &self.workspace.document,
            self.active_layout_technology(),
        ) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("GDS export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &result.bytes) {
            self.status_message = format!("GDS export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("gds", path);
        self.status_message = format!(
            "Exported GDS {} ({} structure(s), {} element(s), {} warning(s))",
            path.display(),
            result.report.structure_count,
            result.report.element_count(),
            result.report.warnings.len()
        );
        true
    }

    pub(crate) fn import_layout_gds(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_gds_exchange_path();
            return self.import_layout_gds_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "GDS import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_gds_from_path(&mut self, path: &Path) -> bool {
        let bytes = match read_native_bytes_file_maybe_gzip(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("GDS import failed: {error}");
                return true;
            }
        };
        let result = match import_gdsii_with_report(&bytes, self.active_layout_technology()) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("GDS import failed: {error}");
                return true;
            }
        };
        let report = result.report;
        self.workspace.document = result.document;
        self.reset_layout_document_state();
        self.active_view = StartupView::Layout2d;
        self.status_message = format!(
            "Imported GDS {} ({} structure(s), {} element(s), {} generated layer(s), {} warning(s), {} skipped)",
            path.display(),
            report.structure_count,
            report.element_count,
            report.generated_layers.len(),
            report.warnings.len(),
            report.skipped_elements.len()
        );
        self.record_recent_layout_file("gds", path);
        true
    }

    pub(crate) fn merge_layout_gds(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_gds_exchange_path();
            return self.merge_layout_gds_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "GDS merge is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_gds_from_path(&mut self, path: &Path) -> bool {
        let bytes = match read_native_bytes_file_maybe_gzip(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("GDS merge failed: {error}");
                return true;
            }
        };
        let result = match import_gdsii_with_report(&bytes, self.active_layout_technology()) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("GDS merge failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("GDS merge failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let import = match self.prepare_layout_json_merge_import(&document) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("GDS merge failed: {error}");
                return true;
            }
        };
        if import.imported_shapes == 0 {
            self.status_message = format!("GDS merge found no shapes in {import_name}");
            return true;
        }
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("GDS merge failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import.first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("gds", path);
        self.status_message = format!(
            "Merged GDS {} into top cell ({} shape(s), {} layer(s), {} warning(s), {} skipped)",
            path.display(),
            import.imported_shapes,
            import.added_layers,
            report.warnings.len(),
            report.skipped_elements.len()
        );
        true
    }

    pub(crate) fn merge_layout_gds_hierarchy(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_gds_exchange_path();
            return self.merge_layout_gds_hierarchy_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "GDS hierarchy merge is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_gds_hierarchy_from_path(&mut self, path: &Path) -> bool {
        let bytes = match read_native_bytes_file_maybe_gzip(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("GDS hierarchy merge failed: {error}");
                return true;
            }
        };
        let result = match import_gdsii_with_report(&bytes, self.active_layout_technology()) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("GDS hierarchy merge failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("GDS hierarchy merge failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        self.apply_layout_hierarchy_merge_document(
            &document,
            &import_name,
            path,
            "GDS",
            "gds",
            format!(
                ", {} warning(s), {} skipped",
                report.warnings.len(),
                report.skipped_elements.len()
            ),
        )
    }

    pub(crate) fn import_layout_gds_as_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_gds_exchange_path();
            return self.import_layout_gds_as_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "GDS cell import is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn import_layout_gds_as_top_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_gds_exchange_path();
            return self.import_layout_gds_as_top_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "GDS top-cell import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_gds_as_cell_from_path(&mut self, path: &Path) -> bool {
        let bytes = match read_native_bytes_file_maybe_gzip(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("GDS cell import failed: {error}");
                return true;
            }
        };
        let result = match import_gdsii_with_report(&bytes, self.active_layout_technology()) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("GDS cell import failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("GDS cell import failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let shape_count = document.flattened_shape_count_estimate();
        let source_cell_count = document.cells.len();
        let import = match self.prepare_layout_json_cell_import(&document, &import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("GDS cell import failed: {error}");
                return true;
            }
        };
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("GDS cell import failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import
            .first_shape
            .map(|id| ShapeOccurrenceId::from_instance_path(id, &[import.instance_id]));
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("gds", path);
        self.status_message = format!(
            "Imported GDS {} as cell {} ({} shape(s), {} source cell(s), {} layer(s), {} warning(s), {} skipped)",
            path.display(),
            import.root_cell_name,
            shape_count,
            source_cell_count,
            import.added_layers,
            report.warnings.len(),
            report.skipped_elements.len()
        );
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_gds_as_top_cell_from_path(&mut self, path: &Path) -> bool {
        let bytes = match read_native_bytes_file_maybe_gzip(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("GDS top-cell import failed: {error}");
                return true;
            }
        };
        let result = match import_gdsii_with_report(&bytes, self.active_layout_technology()) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("GDS top-cell import failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("GDS top-cell import failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let shape_count = document.flattened_shape_count_estimate();
        let source_cell_count = document.cells.len();
        let import = match self.prepare_layout_json_cell_import(&document, &import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("GDS top-cell import failed: {error}");
                return true;
            }
        };
        let LayoutJsonCellImportPlan {
            redo_operations,
            undo_operations,
            first_shape,
            instance_id,
            root_cell,
            root_cell_name,
            added_layers,
        } = import;
        let redo_operations = redo_operations
            .into_iter()
            .filter(|operation| {
                !matches!(operation, Operation::AddInstance { instance, .. } if instance.id == instance_id)
            })
            .collect::<Vec<_>>();
        let undo_operations = undo_operations
            .into_iter()
            .filter(|operation| {
                !matches!(operation, Operation::DeleteInstance { id, .. } if *id == instance_id)
            })
            .collect::<Vec<_>>();
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("GDS top-cell import failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: redo_operations,
            },
            Operation::Batch {
                operations: undo_operations,
            },
        );
        self.layout_view_top_cell = root_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = first_shape;
        self.selected_layout_occurrence = first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("gds", path);
        self.status_message = format!(
            "Imported GDS {} as extra top cell {} ({} shape(s), {} source cell(s), {} layer(s), {} warning(s), {} skipped)",
            path.display(),
            root_cell_name,
            shape_count,
            source_cell_count,
            added_layers,
            report.warnings.len(),
            report.skipped_elements.len()
        );
        true
    }

    pub(crate) fn export_layout_cif(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_cif_exchange_path();
            return self.export_layout_cif_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "CIF export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_cif_to_path(&mut self, path: &Path) -> bool {
        let result = match export_cif_with_report(&self.workspace.document) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("CIF export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, result.text.as_bytes()) {
            self.status_message = format!("CIF export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("cif", path);
        self.status_message = format!(
            "Exported CIF {} ({} layer(s), {} element(s), {} warning(s))",
            path.display(),
            result.report.layer_count,
            result.report.element_count(),
            result.report.warnings.len()
        );
        true
    }

    pub(crate) fn import_layout_cif(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_cif_exchange_path();
            return self.import_layout_cif_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "CIF import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_cif_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("CIF import failed: {error}");
                return true;
            }
        };
        let result = match import_cif_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("CIF import failed: {error}");
                return true;
            }
        };
        if let Err(error) = Self::validate_layout_document_snapshot(&result.document) {
            self.status_message = format!("CIF import failed: {error}");
            return true;
        }
        let report = result.report;
        self.workspace.document = result.document;
        self.reset_layout_document_state();
        self.active_view = StartupView::Layout2d;
        self.status_message = format!(
            "Imported CIF {} ({} shape(s), {} layer(s), {} generated layer(s), {} skipped)",
            path.display(),
            report.shape_count,
            report.layer_count,
            report.generated_layers.len(),
            report.skipped_commands.len()
        );
        self.record_recent_layout_file("cif", path);
        true
    }

    pub(crate) fn merge_layout_cif(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_cif_exchange_path();
            return self.merge_layout_cif_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "CIF merge is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_cif_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("CIF merge failed: {error}");
                return true;
            }
        };
        let result = match import_cif_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("CIF merge failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("CIF merge failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let import = match self.prepare_layout_json_merge_import(&document) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("CIF merge failed: {error}");
                return true;
            }
        };
        if import.imported_shapes == 0 {
            self.status_message = format!("CIF merge found no shapes in {import_name}");
            return true;
        }
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("CIF merge failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import.first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("cif", path);
        self.status_message = format!(
            "Merged CIF {} into top cell ({} shape(s), {} layer(s), {} skipped)",
            path.display(),
            import.imported_shapes,
            import.added_layers,
            report.skipped_commands.len()
        );
        true
    }

    pub(crate) fn merge_layout_cif_hierarchy(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_cif_exchange_path();
            return self.merge_layout_cif_hierarchy_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "CIF hierarchy merge is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_cif_hierarchy_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("CIF hierarchy merge failed: {error}");
                return true;
            }
        };
        let result = match import_cif_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("CIF hierarchy merge failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("CIF hierarchy merge failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        self.apply_layout_hierarchy_merge_document(
            &document,
            &import_name,
            path,
            "CIF",
            "cif",
            format!(", {} skipped", report.skipped_commands.len()),
        )
    }

    pub(crate) fn import_layout_cif_as_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_cif_exchange_path();
            return self.import_layout_cif_as_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "CIF cell import is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn import_layout_cif_as_top_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_cif_exchange_path();
            return self.import_layout_cif_as_top_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "CIF top-cell import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_cif_as_cell_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("CIF cell import failed: {error}");
                return true;
            }
        };
        let result = match import_cif_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("CIF cell import failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("CIF cell import failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let shape_count = document.flattened_shape_count_estimate();
        let source_cell_count = document.cells.len();
        let import = match self.prepare_layout_json_cell_import(&document, &import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("CIF cell import failed: {error}");
                return true;
            }
        };
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("CIF cell import failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import
            .first_shape
            .map(|id| ShapeOccurrenceId::from_instance_path(id, &[import.instance_id]));
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("cif", path);
        self.status_message = format!(
            "Imported CIF {} as cell {} ({} shape(s), {} source cell(s), {} layer(s), {} skipped)",
            path.display(),
            import.root_cell_name,
            shape_count,
            source_cell_count,
            import.added_layers,
            report.skipped_commands.len()
        );
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_cif_as_top_cell_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("CIF top-cell import failed: {error}");
                return true;
            }
        };
        let result = match import_cif_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("CIF top-cell import failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("CIF top-cell import failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let shape_count = document.flattened_shape_count_estimate();
        let source_cell_count = document.cells.len();
        let import = match self.prepare_layout_json_cell_import(&document, &import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("CIF top-cell import failed: {error}");
                return true;
            }
        };
        let LayoutJsonCellImportPlan {
            redo_operations,
            undo_operations,
            first_shape,
            instance_id,
            root_cell,
            root_cell_name,
            added_layers,
        } = import;
        let redo_operations = redo_operations
            .into_iter()
            .filter(|operation| {
                !matches!(operation, Operation::AddInstance { instance, .. } if instance.id == instance_id)
            })
            .collect::<Vec<_>>();
        let undo_operations = undo_operations
            .into_iter()
            .filter(|operation| {
                !matches!(operation, Operation::DeleteInstance { id, .. } if *id == instance_id)
            })
            .collect::<Vec<_>>();
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("CIF top-cell import failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: redo_operations,
            },
            Operation::Batch {
                operations: undo_operations,
            },
        );
        self.layout_view_top_cell = root_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = first_shape;
        self.selected_layout_occurrence = first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("cif", path);
        self.status_message = format!(
            "Imported CIF {} as extra top cell {} ({} shape(s), {} source cell(s), {} layer(s), {} skipped)",
            path.display(),
            root_cell_name,
            shape_count,
            source_cell_count,
            added_layers,
            report.skipped_commands.len()
        );
        true
    }

    pub(crate) fn export_layout_dxf(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_dxf_exchange_path();
            return self.export_layout_dxf_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DXF export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_dxf_to_path(&mut self, path: &Path) -> bool {
        let result = match export_dxf_with_report(&self.workspace.document) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DXF export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, result.text.as_bytes()) {
            self.status_message = format!("DXF export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("dxf", path);
        self.status_message = format!(
            "Exported DXF {} ({} layer(s), {} entity/entities, {} warning(s))",
            path.display(),
            result.report.layer_count,
            result.report.entity_count(),
            result.report.warnings.len()
        );
        true
    }

    pub(crate) fn import_layout_dxf(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_dxf_exchange_path();
            return self.import_layout_dxf_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DXF import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_dxf_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DXF import failed: {error}");
                return true;
            }
        };
        let result = match import_dxf_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DXF import failed: {error}");
                return true;
            }
        };
        if let Err(error) = Self::validate_layout_document_snapshot(&result.document) {
            self.status_message = format!("DXF import failed: {error}");
            return true;
        }
        let report = result.report;
        self.workspace.document = result.document;
        self.reset_layout_document_state();
        self.active_view = StartupView::Layout2d;
        self.status_message = format!(
            "Imported DXF {} ({} shape(s), {} layer(s), {} generated layer(s), {} skipped)",
            path.display(),
            report.shape_count,
            report.layer_count,
            report.generated_layers.len(),
            report.skipped_entities.len()
        );
        self.record_recent_layout_file("dxf", path);
        true
    }

    pub(crate) fn merge_layout_dxf(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_dxf_exchange_path();
            return self.merge_layout_dxf_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DXF merge is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_dxf_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DXF merge failed: {error}");
                return true;
            }
        };
        let result = match import_dxf_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DXF merge failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("DXF merge failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let import = match self.prepare_layout_json_merge_import(&document) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("DXF merge failed: {error}");
                return true;
            }
        };
        if import.imported_shapes == 0 {
            self.status_message = format!("DXF merge found no shapes in {import_name}");
            return true;
        }
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("DXF merge failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import.first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("dxf", path);
        self.status_message = format!(
            "Merged DXF {} into top cell ({} shape(s), {} layer(s), {} skipped)",
            path.display(),
            import.imported_shapes,
            import.added_layers,
            report.skipped_entities.len()
        );
        true
    }

    pub(crate) fn merge_layout_dxf_hierarchy(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_dxf_exchange_path();
            return self.merge_layout_dxf_hierarchy_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DXF hierarchy merge is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_dxf_hierarchy_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DXF hierarchy merge failed: {error}");
                return true;
            }
        };
        let result = match import_dxf_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DXF hierarchy merge failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("DXF hierarchy merge failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        self.apply_layout_hierarchy_merge_document(
            &document,
            &import_name,
            path,
            "DXF",
            "dxf",
            format!(", {} skipped", report.skipped_entities.len()),
        )
    }

    pub(crate) fn import_layout_dxf_as_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_dxf_exchange_path();
            return self.import_layout_dxf_as_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DXF cell import is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn import_layout_dxf_as_top_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_dxf_exchange_path();
            return self.import_layout_dxf_as_top_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DXF top-cell import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_dxf_as_cell_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DXF cell import failed: {error}");
                return true;
            }
        };
        let result = match import_dxf_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DXF cell import failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("DXF cell import failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let shape_count = document.flattened_shape_count_estimate();
        let source_cell_count = document.cells.len();
        let import = match self.prepare_layout_json_cell_import(&document, &import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("DXF cell import failed: {error}");
                return true;
            }
        };
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("DXF cell import failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import
            .first_shape
            .map(|id| ShapeOccurrenceId::from_instance_path(id, &[import.instance_id]));
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("dxf", path);
        self.status_message = format!(
            "Imported DXF {} as cell {} ({} shape(s), {} source cell(s), {} layer(s), {} skipped)",
            path.display(),
            import.root_cell_name,
            shape_count,
            source_cell_count,
            import.added_layers,
            report.skipped_entities.len()
        );
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_dxf_as_top_cell_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DXF top-cell import failed: {error}");
                return true;
            }
        };
        let result = match import_dxf_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DXF top-cell import failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("DXF top-cell import failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let report = result.report;
        let shape_count = document.flattened_shape_count_estimate();
        let source_cell_count = document.cells.len();
        let import = match self.prepare_layout_json_cell_import(&document, &import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("DXF top-cell import failed: {error}");
                return true;
            }
        };
        let LayoutJsonCellImportPlan {
            redo_operations,
            undo_operations,
            first_shape,
            instance_id,
            root_cell,
            root_cell_name,
            added_layers,
        } = import;
        let redo_operations = redo_operations
            .into_iter()
            .filter(|operation| {
                !matches!(operation, Operation::AddInstance { instance, .. } if instance.id == instance_id)
            })
            .collect::<Vec<_>>();
        let undo_operations = undo_operations
            .into_iter()
            .filter(|operation| {
                !matches!(operation, Operation::DeleteInstance { id, .. } if *id == instance_id)
            })
            .collect::<Vec<_>>();
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("DXF top-cell import failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: redo_operations,
            },
            Operation::Batch {
                operations: undo_operations,
            },
        );
        self.layout_view_top_cell = root_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = first_shape;
        self.selected_layout_occurrence = first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("dxf", path);
        self.status_message = format!(
            "Imported DXF {} as extra top cell {} ({} shape(s), {} source cell(s), {} layer(s), {} skipped)",
            path.display(),
            root_cell_name,
            shape_count,
            source_cell_count,
            added_layers,
            report.skipped_entities.len()
        );
        true
    }

    pub(crate) fn export_layout_def(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_def_exchange_path();
            return self.export_layout_def_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DEF export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_def_to_path(&mut self, path: &Path) -> bool {
        let result = match export_def_with_report(&self.workspace.document) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DEF export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, result.text.as_bytes()) {
            self.status_message = format!("DEF export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("def", path);
        self.status_message = format!(
            "Exported DEF {} ({} layer(s), {} object(s), {} warning(s))",
            path.display(),
            result.report.layer_count,
            result.report.object_count(),
            result.report.warnings.len()
        );
        true
    }

    pub(crate) fn import_layout_def(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_def_exchange_path();
            return self.import_layout_def_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DEF import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_def_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DEF import failed: {error}");
                return true;
            }
        };
        let result = match import_def_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DEF import failed: {error}");
                return true;
            }
        };
        if let Err(error) = Self::validate_layout_document_snapshot(&result.document) {
            self.status_message = format!("DEF import failed: {error}");
            return true;
        }
        let report = result.report;
        self.workspace.document = result.document;
        self.reset_layout_document_state();
        self.active_view = StartupView::Layout2d;
        self.status_message = format!(
            "Imported DEF {} ({} shape(s), {} layer(s), {} generated layer(s), {} skipped)",
            path.display(),
            report.shape_count,
            report.layer_count,
            report.generated_layers.len(),
            report.skipped_items.len()
        );
        self.record_recent_layout_file("def", path);
        true
    }

    pub(crate) fn merge_layout_def(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_def_exchange_path();
            return self.merge_layout_def_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DEF merge is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn merge_layout_def_hierarchy(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_def_exchange_path();
            return self.merge_layout_def_hierarchy_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DEF hierarchy merge is available in the native app".to_string();
            true
        }
    }
}
