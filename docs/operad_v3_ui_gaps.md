# Operad UI Migration Notes

Fabricad now depends on Operad 3.0.0 from `https://github.com/Andrewp2/Operad.git` at rev `8762dbf`.

## Running on Operad 3.0

- Safety dashboard central view.
- Sidebar Modules customization window.
- Inventory Tracker central view.
- Maintenance and Calibration central view.
- Process-flow Designer central view.
- Fab Workflow central view.
- Cleanroom Environment central view.
- Lot Traceability central view.
- SPC / FDC central view.
- Layout Diff Review central view.
- Scheduler / Dispatch central view.
- Run-to-Run Control central view.
- DOE Planner dashboard central view.
- Reticle Prep central view.
- Lab Notebook preview dashboard central view.
- Fab Control Room central view.
- Left navigation rail.
- Shared Operad sidecar renderer for module context panels, including interactive action rows.
- Module context panels for Workflow, Scheduler / Dispatch, Metrology, Reticle Prep, Layout Diff, Inventory, Maintenance, Environment, Safety, Lot Traceability, SPC / FDC, Run-to-Run Control, DOE Planner, Process-flow Designer, Lab Notebook, and Process Cross-Section.
- Read-only drawer summaries for Mask Layers and the 3D Stack.

## Remaining Fabricad 3.0 Integration Gaps

- The native bridge in `crates/native_app/src/operad_egui.rs` paints the legacy primitives plus Operad 3 rich rectangles, scene text, paths, and deterministic placeholders for image/canvas resources. `PaintKind::Image`, `PaintKind::ImagePlacement`, and `PaintKind::Canvas` still need application resource/canvas callbacks before they can render real resource content in Fabricad's native host.
- The 2D mask layout viewport is still a custom GPU/tiled renderer with layout-specific picking and editing behavior. Full ownership by Operad needs Fabricad to wire a canvas or render-callback host without losing pointer routing.
- The 3D viewport is still a wgpu scene with fly-camera controls, fullscreen viewport behavior, and frame timing. Full ownership by Operad needs Fabricad to wire native viewport/canvas embedding with keyboard capture and pointer-lock style input.
- Metrology remains egui because its central workflow is an interactive wafer map: die rectangles, hover/selection hit testing, defect overlays, vector overlays, annotations, histograms, radial profiles, and review-image drawing. Moving it cleanly needs Fabricad integration for Operad canvas/render callbacks, chart primitives, and pointer routing that can return domain hit targets.
- Yield Dashboard remains egui because the primary review surface is also an interactive wafer map plus dense failure-mode tables, wafer drill-down, lot comparison tables, process-measurement tables, and correlation views. Replacing this view without losing the actual yield-analysis workflow needs Fabricad integration for Operad canvas, chart/table, combo/listbox, and filter-control primitives.
- The app shell still uses egui panels and windows as the host runtime. Operad owns more content inside those hosts, but replacing `TopBottomPanel`, `SidePanel`, `CentralPanel`, and floating windows requires Fabricad to adopt the Operad host/runtime shell layer.
- Inspector and secondary drawers still use egui windows/scroll hosts, but most module context content inside them now renders through Operad sidecars. The menu/toolbar chrome, command palette shell, MES side panel, layer/property editors, metrology controls, notebook filters, experiment response entry, and cross-section controls still use egui host widgets. They should move after Fabricad wires Operad shell layout, overlays, dockable panels, and text/list inputs rather than only embedded content documents.
- Text-edit-heavy workflows should move after Fabricad integrates Operad text input with IME, clipboard, multiline editing, selection, and egui event routing deeply enough to replace notebook and recipe editors.
- Combo boxes and listbox-style controls should move after Fabricad has a shared Operad overlay/listbox routing pattern for popup ownership and keyboard focus.
- Scheduler / Dispatch now renders policy controls, filter toggles, next actions, bottleneck summaries, tool matching, queue priority, conflicts, and estimated completion through Operad rows. The old egui timeline painter, free-text filter, slider, and combo widgets remain fallback-only until Fabricad wires Operad canvas, text input, and listbox/slider paths into the host.
- Run-to-Run Control now renders loop selection, control health, recommendation queue, selected action details, transition actions, guardrails, metrology feedback, yield links, and audit trail through Operad rows. The old egui target-vs-measured chart and detailed table painters remain fallback-only until Fabricad wires Operad chart/canvas paths into the host.
- DOE Planner now renders experiment controls, factor splits, response plan, split lots, run matrix, selected run detail, readiness, response summary, and main effects through Operad rows. The old egui response capture form remains the editing path until Fabricad wires Operad text/numeric input, combo/listbox overlays, and form focus routing deeply enough for measurement entry.
- Reticle Prep now renders route sync controls, reticle summary, fields, exposure blocks, mask layers, issue grouping, and paged mask-check rows through Operad. The old egui lot combo and issue search field remain fallback-only until Fabricad wires complete text input and overlay/listbox routing.
- Lab Notebook now renders the preview dashboard, filter/action rows, timeline, selected-entry summary, structured links, and related entries through Operad. Markdown edit mode still intentionally falls back to egui until Fabricad wires robust multiline editing, clipboard/IME, selection, and focus routing.
- Fab Control Room now renders fleet status, selected tool details, recipe load rows, host commands, run state, alarms, sensors, and logs through Operad rows. The old egui fallback still has richer inline recipe combos, progress bars, sensor sparklines, and detailed grid density; those should move after Fabricad wires Operad listbox, progress, and chart/canvas paths.
- Process-flow Designer now renders the core route, controls, dependencies, findings, and MES export preview through Operad rows. The older detailed egui timeline/card painter remains fallback-only; richer timeline visualization should move after Fabricad wires Operad canvas or richer list primitives.
- Fab Workflow now renders its central workflow, focus-lot selector rows, production focus, route operations, and cross-links through Operad rows. The old egui lot combo remains fallback-only; a richer Operad combo/listbox overlay would be useful for compact lot switching.
- Cleanroom Environment now renders status, sensor trend summaries, alarm console rows, sensor matrix rows, zones, events, and process correlations through Operad. The old egui plots and sparklines remain fallback-only until Fabricad wires Operad chart/canvas paths.
- Lot Traceability now renders the central genealogy, wafer flow, selected trace node, excursion impact, process history, material ancestry, and audit trail through Operad rows. The old egui search field and combo picker path remains fallback-only until Fabricad wires complete text input and overlay/listbox routing.
- SPC / FDC now renders monitor triage, filter controls, chart and trace selection, selected chart/trace summaries, equipment alarm context, and capped findings through Operad rows. The old egui control-chart and sensor-trace plots remain fallback-only until Fabricad wires Operad chart/canvas paths.
- Layout Diff Review now renders comparison setup, source actions, summary metrics, layer summaries, paged change review, and disposition actions through Operad rows. The old egui search box and combo widgets remain fallback-only until Fabricad wires Operad text editing and listbox overlays.
- Process Cross-Section now has an Operad context sidecar, but its central value remains egui because it is a custom cross-section painter with mask overlays, dimension guides, material picking, risk markers, and step slider controls. Moving it cleanly needs Fabricad to wire an Operad canvas/render callback plus slider/toggle primitives through the host.
- Recipe Manager remains egui because it is primarily an editable parameter form: decimal/integer drag values, boolean toggles, text fields, choice combos, draft/apply state, validation messages, approval transitions, and version diff selection. It should move after Fabricad wires Operad form controls, text editing, and listbox overlays well enough to preserve recipe-editing ergonomics.
