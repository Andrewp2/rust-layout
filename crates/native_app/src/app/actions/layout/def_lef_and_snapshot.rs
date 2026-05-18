#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_def_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DEF merge failed: {error}");
                return true;
            }
        };
        let result = match import_def_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DEF merge failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("DEF merge failed: {error}");
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
                self.status_message = format!("DEF merge failed: {error}");
                return true;
            }
        };
        if import.imported_shapes == 0 {
            self.status_message = format!("DEF merge found no shapes in {import_name}");
            return true;
        }
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("DEF merge failed: {error}");
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
        self.record_recent_layout_file("def", path);
        self.status_message = format!(
            "Merged DEF {} into top cell ({} shape(s), {} layer(s), {} skipped)",
            path.display(),
            import.imported_shapes,
            import.added_layers,
            report.skipped_items.len()
        );
        true
    }

    pub(crate) fn import_layout_def_as_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_def_exchange_path();
            return self.import_layout_def_as_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DEF cell import is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn import_layout_def_as_top_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_def_exchange_path();
            return self.import_layout_def_as_top_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DEF top-cell import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_def_as_cell_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DEF cell import failed: {error}");
                return true;
            }
        };
        let result = match import_def_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DEF cell import failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("DEF cell import failed: {error}");
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
                self.status_message = format!("DEF cell import failed: {error}");
                return true;
            }
        };
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("DEF cell import failed: {error}");
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
        self.record_recent_layout_file("def", path);
        self.status_message = format!(
            "Imported DEF {} as cell {} ({} shape(s), {} source cell(s), {} layer(s), {} skipped)",
            path.display(),
            import.root_cell_name,
            shape_count,
            source_cell_count,
            import.added_layers,
            report.skipped_items.len()
        );
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_def_as_top_cell_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DEF top-cell import failed: {error}");
                return true;
            }
        };
        let result = match import_def_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("DEF top-cell import failed: {error}");
                return true;
            }
        };
        let mut document = result.document;
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("DEF top-cell import failed: {error}");
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
                self.status_message = format!("DEF top-cell import failed: {error}");
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
            self.status_message = format!("DEF top-cell import failed: {error}");
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
        self.record_recent_layout_file("def", path);
        self.status_message = format!(
            "Imported DEF {} as extra top cell {} ({} shape(s), {} source cell(s), {} layer(s), {} skipped)",
            path.display(),
            root_cell_name,
            shape_count,
            source_cell_count,
            added_layers,
            report.skipped_items.len()
        );
        true
    }

    pub(crate) fn export_layout_lef(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_lef_exchange_path();
            return self.export_layout_lef_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "LEF export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_lef_to_path(&mut self, path: &Path) -> bool {
        let result = match export_lef_with_report(&self.workspace.document) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("LEF export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, result.text.as_bytes()) {
            self.status_message = format!("LEF export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("lef", path);
        self.status_message = format!(
            "Exported LEF {} ({} layer(s), {} object(s), {} warning(s))",
            path.display(),
            result.report.layer_count,
            result.report.object_count(),
            result.report.warnings.len()
        );
        true
    }

    pub(crate) fn import_layout_lef(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_lef_exchange_path();
            return self.import_layout_lef_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "LEF import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_lef_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("LEF import failed: {error}");
                return true;
            }
        };
        let result = match import_lef_with_report(&contents) {
            Ok(result) => result,
            Err(error) => {
                self.status_message = format!("LEF import failed: {error}");
                return true;
            }
        };
        if let Err(error) = Self::validate_layout_document_snapshot(&result.document) {
            self.status_message = format!("LEF import failed: {error}");
            return true;
        }
        let report = result.report;
        self.workspace.document = result.document;
        self.reset_layout_document_state();
        self.active_view = StartupView::Layout2d;
        self.status_message = format!(
            "Imported LEF {} ({} shape(s), {} layer(s), {} generated layer(s), {} skipped)",
            path.display(),
            report.shape_count,
            report.layer_count,
            report.generated_layers.len(),
            report.skipped_items.len()
        );
        self.record_recent_layout_file("lef", path);
        true
    }

    pub(crate) fn export_ui_screenshot(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_screenshot_export_path();
            return self.export_ui_screenshot_to_path(
                &path,
                UI_SCREENSHOT_EXPORT_WIDTH,
                UI_SCREENSHOT_EXPORT_HEIGHT,
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Screenshot export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_ui_screenshot_to_path(
        &mut self,
        path: &Path,
        width: u32,
        height: u32,
    ) -> bool {
        let ui_scale = UiScale::new(self.app_options.appearance.ui_scale);
        let report = match self.render_operad_snapshot_scaled(width, height, ui_scale) {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("Screenshot export failed: {error}");
                return true;
            }
        };
        if let Err(error) = write_operad_snapshot_rgba(path, &report) {
            self.status_message = format!("Screenshot export failed: {error}");
            return true;
        }
        let snapshot = report
            .render
            .snapshot
            .as_ref()
            .map(|image| format!("{}x{}", image.size.width, image.size.height))
            .unwrap_or_else(|| format!("{width}x{height}"));
        self.status_message = format!("Exported screenshot {} ({snapshot} RGBA)", path.display());
        true
    }

    pub(crate) fn export_ui_screenshot_ppm(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_screenshot_ppm_export_path();
            return self.export_ui_screenshot_ppm_to_path(
                &path,
                UI_SCREENSHOT_EXPORT_WIDTH,
                UI_SCREENSHOT_EXPORT_HEIGHT,
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Screenshot PPM export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_ui_screenshot_ppm_to_path(
        &mut self,
        path: &Path,
        width: u32,
        height: u32,
    ) -> bool {
        let ui_scale = UiScale::new(self.app_options.appearance.ui_scale);
        let report = match self.render_operad_snapshot_scaled(width, height, ui_scale) {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("Screenshot PPM export failed: {error}");
                return true;
            }
        };
        if let Err(error) = write_operad_snapshot_ppm(path, &report) {
            self.status_message = format!("Screenshot PPM export failed: {error}");
            return true;
        }
        let snapshot = report
            .render
            .snapshot
            .as_ref()
            .map(|image| format!("{}x{}", image.size.width, image.size.height))
            .unwrap_or_else(|| format!("{width}x{height}"));
        self.status_message = format!("Exported screenshot {} ({snapshot} PPM)", path.display());
        true
    }

    pub(crate) fn secondary_panel_label(&self) -> &'static str {
        match self.active_view {
            StartupView::Metrology => "Map",
            StartupView::MaskPrep | StartupView::CrossSection | StartupView::Layout2d => "Layers",
            StartupView::Layout3d => "3D Stack",
            StartupView::Experiment => "Responses",
            StartupView::Notebook => "Links",
            _ => "Panel",
        }
    }

    pub(crate) fn has_inspector_panel(&self) -> bool {
        !matches!(
            self.active_view,
            StartupView::FabControl | StartupView::Yield | StartupView::Notebook
        )
    }

    pub(crate) fn has_secondary_panel(&self) -> bool {
        matches!(
            self.active_view,
            StartupView::Layout2d
                | StartupView::Layout3d
                | StartupView::Metrology
                | StartupView::MaskPrep
                | StartupView::CrossSection
                | StartupView::Experiment
                | StartupView::Notebook
        )
    }

    pub fn build_operad_document(&self, viewport: UiSize) -> Result<UiDocument, String> {
        self.build_operad_document_scaled(viewport, UiScale::new(1.0))
    }

    pub fn build_operad_document_scaled(
        &self,
        viewport: UiSize,
        ui_scale: UiScale,
    ) -> Result<UiDocument, String> {
        let mut document = UiDocument::new(root_style(viewport.width, viewport.height));
        document.set_node_visual(document.root(), UiVisual::panel(COLOR_APP_BG, None, 0.0));

        let app = document.add_child(
            document.root(),
            UiNode::container(
                "glassworks.app",
                UiNodeStyle::new(layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::percent(1.0),
                ))
                .with_clip(ClipBehavior::Clip),
            ),
        );

        if self.active_view == StartupView::Layout3d && self.viewport_fullscreen {
            add_fullscreen_3d_view(&mut document, app, viewport, ui_scale);
            document
                .compute_layout(viewport, &mut ApproxTextMeasurer)
                .map_err(|err| format!("UI layout failed: {err}"))?;
            attach_default_pointer_actions(&mut document);
            return Ok(document);
        }

        let menu_bar = document.add_child(
            app,
            UiNode::container(
                "glassworks.menu_bar",
                layout::with_padding_all(
                    layout::with_gap_all(
                        layout::with_size(
                            layout::row(),
                            layout::percent(1.0),
                            layout::px(ui_scale.value(30.0)),
                        ),
                        ui_scale.value(4.0),
                    ),
                    ui_scale.value(4.0),
                ),
            )
            .with_visual(UiVisual::panel(
                COLOR_CHROME_BG,
                Some(StrokeStyle::new(COLOR_PANEL_STROKE, 1.0)),
                0.0,
            )),
        );

        let compact_menu = viewport.width < ui_scale.value(900.0);
        let visible_menus: &[AppMenu] = if compact_menu {
            &[AppMenu::File, AppMenu::Edit, AppMenu::View, AppMenu::More]
        } else {
            &AppMenu::ALL
        };
        for menu in visible_menus.iter().copied() {
            add_menu_button(
                &mut document,
                menu_bar,
                menu,
                self.active_menu == Some(menu),
                ui_scale,
            );
        }
        document.add_child(
            menu_bar,
            UiNode::container(
                "glassworks.menu_spacer",
                layout::with_flex(layout::row(), 1.0, 1.0, layout::px(0.0)),
            ),
        );
        add_text(
            &mut document,
            menu_bar,
            "glassworks.user_id",
            USER_ID_LABEL,
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::auto(), layout::px(ui_scale.value(22.0))),
        );

        if matches!(
            self.active_view,
            StartupView::Layout2d | StartupView::Layout3d
        ) {
            add_tool_strip(&mut document, app, self, viewport.width, ui_scale);
        } else {
            add_tool_strip_spacer(&mut document, app, viewport.width, ui_scale);
        }

        if let Some(menu) = self.active_menu {
            add_menu_panel(
                &mut document,
                app,
                menu,
                self.active_view,
                self,
                viewport.width,
                ui_scale,
            );
        }
        pad_pre_shell_node_ids(&mut document, app);

        let shell = document.add_child(
            app,
            UiNode::container(
                "glassworks.shell",
                UiNodeStyle::new(layout::with_min_size(
                    layout::with_size(
                        layout::with_flex(layout::row(), 1.0, 1.0, layout::px(0.0)),
                        layout::percent(1.0),
                        layout::auto(),
                    ),
                    layout::px(0.0),
                    layout::px(0.0),
                ))
                .with_clip(ClipBehavior::Clip),
            ),
        );

        let shell_metrics = ShellMetrics::new(ShellMetricsOptions {
            viewport_width: viewport.width,
            ui_scale,
            has_secondary_panel: self.has_secondary_panel(),
            show_layers: self.show_layers,
            inline_side_panels: matches!(
                self.active_view,
                StartupView::Layout2d | StartupView::Layout3d
            ) && viewport.width >= ui_scale.value(980.0),
            show_inspector: self.show_inspector,
            has_inspector_panel: self.has_inspector_panel(),
        });
        let nav_metrics = shell_metrics.nav;
        let nav = document.add_child(
            shell,
            UiNode::container(
                "glassworks.nav",
                UiNodeStyle::new(layout::with_min_size(
                    layout::with_padding_all(
                        layout::with_gap_all(
                            layout::with_size(
                                layout::column(),
                                layout::px(nav_metrics.width),
                                layout::percent(1.0),
                            ),
                            nav_metrics.gap,
                        ),
                        nav_metrics.padding,
                    ),
                    layout::px(0.0),
                    layout::px(0.0),
                ))
                .with_clip(ClipBehavior::Clip),
            )
            .with_visual(UiVisual::panel(
                COLOR_CHROME_BG,
                Some(StrokeStyle::new(COLOR_PANEL_STROKE, 1.0)),
                0.0,
            ))
            .with_scroll(ScrollAxes::VERTICAL),
        );

        for view in StartupView::ALL
            .into_iter()
            .filter(|view| self.nav_rail_views.contains(view))
        {
            let selected = view == self.active_view;
            add_button_with_accessibility_label(
                &mut document,
                nav,
                format!("glassworks.nav.action.{}", view.slug()),
                view.nav_label(),
                view.label(),
                selected,
                layout::size(layout::percent(1.0), layout::px(nav_metrics.button_height)),
                nav_metrics.button_text_scale,
            );
        }

        let body_frame = document.add_child(
            shell,
            UiNode::container(
                "glassworks.body_frame",
                UiNodeStyle::new(layout::with_min_size(
                    layout::with_flex(layout::row(), 1.0, 1.0, layout::px(0.0)),
                    layout::px(0.0),
                    layout::px(0.0),
                ))
                .with_clip(ClipBehavior::Clip),
            )
            .with_visual(UiVisual::panel(COLOR_APP_BG, None, 0.0)),
        );

        let body = document.add_child(
            body_frame,
            UiNode::container(
                "glassworks.body",
                UiNodeStyle::new(layout::with_min_size(
                    layout::with_padding_all(
                        layout::with_gap_all(
                            layout::with_flex(layout::column(), 1.0, 1.0, layout::px(0.0)),
                            ui_scale.value(10.0),
                        ),
                        ui_scale.value(10.0),
                    ),
                    layout::px(0.0),
                    layout::px(0.0),
                ))
                .with_clip(ClipBehavior::Clip),
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );
        add_scrollbar_rail(
            &mut document,
            body_frame,
            "glassworks.body_scrollbar",
            ui_scale,
        );

        add_text(
            &mut document,
            body,
            "glassworks.header.title",
            self.active_view.label(),
            text_style(ui_scale.value(22.0), FontWeight::NORMAL, COLOR_TEXT),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(30.0))),
        );

        if matches!(
            self.active_view,
            StartupView::Layout2d | StartupView::Layout3d
        ) {
            add_primary_view_panel(
                &mut document,
                body,
                self,
                viewport,
                ui_scale,
                shell_metrics.compact_primary_rows,
                shell_metrics.wide_primary_rows,
                shell_metrics.body_width,
            );
        } else {
            add_view_controls(
                &mut document,
                body,
                self,
                ui_scale,
                shell_metrics.compact_primary_rows,
                shell_metrics.body_width,
            );
            add_primary_view_panel(
                &mut document,
                body,
                self,
                viewport,
                ui_scale,
                shell_metrics.compact_primary_rows,
                shell_metrics.wide_primary_rows,
                shell_metrics.body_width,
            );
        }

        if !matches!(
            self.active_view,
            StartupView::Layout2d | StartupView::Layout3d
        ) {
            add_detail_sections(&mut document, body, self, ui_scale);
        }

        if self.show_inspector && self.has_inspector_panel() {
            add_inspector_panel(&mut document, shell, self, ui_scale);
        }
        if shell_metrics.show_secondary_panel {
            add_secondary_panel(&mut document, shell, self, ui_scale);
        }
        if self.show_command_palette {
            add_command_palette_panel(&mut document, app, self, viewport, ui_scale);
        }
        if self.show_sidebar_modules {
            add_sidebar_modules_panel(&mut document, app, self, viewport, ui_scale);
        }
        if self.show_options_panel {
            add_options_panel(&mut document, app, self, viewport, ui_scale);
        }
        if self.show_diagnostics_panel {
            add_diagnostics_panel(&mut document, app, self, viewport, ui_scale);
        }

        document
            .compute_layout(viewport, &mut ApproxTextMeasurer)
            .map_err(|err| format!("UI layout failed: {err}"))?;
        attach_default_pointer_actions(&mut document);
        Ok(document)
    }

    pub fn audit_operad_document(&self, viewport: UiSize) -> Result<OperadAuditReport, String> {
        self.audit_operad_document_scaled(viewport, UiScale::new(1.0))
    }

    pub fn audit_operad_document_scaled(
        &self,
        viewport: UiSize,
        ui_scale: UiScale,
    ) -> Result<OperadAuditReport, String> {
        let document = self.build_operad_document_scaled(viewport, ui_scale)?;
        let paint_items = document.paint_list().items.len();
        Ok(OperadAuditReport {
            operad_version: OPERAD_UI_RUNTIME_VERSION,
            view: self.active_view,
            viewport,
            paint_items,
            layout_warnings: document.audit_layout().len(),
            document_shapes: self.workspace.document.flattened_shape_count_estimate(),
            workspace_lots: self.workspace.mes.lots.len(),
            equipment_tools: self.workspace.equipment.tools().count(),
        })
    }

    pub fn render_operad_snapshot(
        &self,
        width: u32,
        height: u32,
    ) -> Result<OperadSnapshotReport, String> {
        self.render_operad_snapshot_scaled(width, height, UiScale::new(1.0))
    }

    pub fn render_operad_snapshot_scaled(
        &self,
        width: u32,
        height: u32,
        ui_scale: UiScale,
    ) -> Result<OperadSnapshotReport, String> {
        let width = width.max(1);
        let height = height.max(1);
        let viewport = UiSize::new(width as f32, height as f32);
        let document = self.build_operad_document_scaled(viewport, ui_scale)?;
        let paint = document.paint_list();
        let audit = OperadAuditReport {
            operad_version: OPERAD_UI_RUNTIME_VERSION,
            view: self.active_view,
            viewport,
            paint_items: paint.items.len(),
            layout_warnings: document.audit_layout().len(),
            document_shapes: self.workspace.document.flattened_shape_count_estimate(),
            workspace_lots: self.workspace.mes.lots.len(),
            equipment_tools: self.workspace.equipment.tools().count(),
        };
        let request = RenderFrameRequest::new(
            RenderTarget::snapshot(PixelSize::new(width, height)),
            viewport,
            paint,
        )
        .options(RenderOptions {
            clear_color: COLOR_APP_BG,
            ..Default::default()
        });
        let mut renderer = WgpuRenderer::new();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut layout_canvas = LayoutCanvasResources::default();
            let mut viewport_3d_canvas = Viewport3dCanvasResources::default();
            render_layout_canvas_requests(
                self,
                &mut renderer,
                &request,
                &mut layout_canvas,
                &mut viewport_3d_canvas,
            )?;
        }
        let render = renderer
            .render_frame(request, &operad::testing::EmptyResourceResolver)
            .map_err(|err| format!("snapshot render failed: {err}"))?;
        Ok(OperadSnapshotReport { audit, render })
    }
}
