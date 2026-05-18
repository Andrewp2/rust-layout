use std::time::{Duration, Instant};

#[cfg(test)]
use glassworks_studio::build_layout_3d_batch_with_options;
use glassworks_studio::{
    GlassworksApp, LayoutCanvasResources, StartupOptions, StartupView, ToolMode,
    Viewport3dCanvasResources, render_layout_2d_canvas_with_size, render_layout_3d_canvas,
};
#[cfg(test)]
use geometry_core::{Point, Rect};
#[cfg(test)]
use layout_model::Document;
use operad::input::{RawInputEvent, RawPointerEvent};
use operad::native::{
    NativeCanvasInput, NativeKeyboardInput, NativeRawMouseMotion, NativeWgpuCanvasRenderContext,
    NativeWgpuCanvasRenderRegistry, NativeWindowHooks, NativeWindowMetrics, NativeWindowOptions,
    run_app_with_canvas_renderers_and_hooks,
};
use operad::platform::{CursorGrabMode, CursorRequest, PixelSize, PlatformRequest};
use operad::renderer::{CanvasRenderOutput, RenderError};
use operad::{
    KeyCode, KeyModifiers, PointerButton, PointerEventKind, UiContent, UiDocument, UiInputEvent,
    UiNodeId, UiPoint, UiRect, UiSize, WidgetAction, WidgetActionBinding, WidgetActionKind,
};
const DEFAULT_WIDTH: u32 = 1440;
const DEFAULT_HEIGHT: u32 = 920;
const DOUBLE_CLICK_MAX_INTERVAL: Duration = Duration::from_millis(400);
const DOUBLE_CLICK_MAX_DISTANCE: f32 = 5.0;
const CANVAS_LINE_SCROLL_POINTS: f32 = 36.0;

pub fn run(options: StartupOptions) -> Result<(), Box<dyn std::error::Error>> {
    let state = GlassworksNativeState::new(options);
    let native_options = NativeWindowOptions::new("Glassworks")
        .with_size(DEFAULT_WIDTH as f32, DEFAULT_HEIGHT as f32)
        .with_min_size(720.0, 480.0);
    let mut canvas_renderers = NativeWgpuCanvasRenderRegistry::new();
    canvas_renderers.register(
        "glassworks.layout.viewport.2d",
        render_native_layout_2d_canvas,
    );
    canvas_renderers.register(
        "glassworks.layout.viewport.3d",
        render_native_layout_3d_canvas,
    );

    run_app_with_canvas_renderers_and_hooks(
        native_options,
        state,
        update_native_state,
        view_native_state,
        canvas_renderers,
        native_hooks(),
    )?;
    Ok(())
}

struct GlassworksNativeState {
    app: GlassworksApp,
    layout_canvas: LayoutCanvasResources,
    viewport_3d_canvas: Viewport3dCanvasResources,
    layout_pan_drag: Option<UiPoint>,
    last_left_click: Option<(UiPoint, Instant)>,
    modifiers: KeyModifiers,
    flycam_keys: FlycamKeyState,
    last_flycam_tick: Instant,
    native_flycam_captured: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FlycamKeyState {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

impl FlycamKeyState {
    fn set_key(&mut self, key: KeyCode, pressed: bool) -> bool {
        let slot = match key {
            KeyCode::ArrowUp | KeyCode::Character('w' | 'W') => &mut self.forward,
            KeyCode::ArrowDown | KeyCode::Character('s' | 'S') => &mut self.backward,
            KeyCode::ArrowLeft | KeyCode::Character('a' | 'A') => &mut self.left,
            KeyCode::ArrowRight | KeyCode::Character('d' | 'D') => &mut self.right,
            KeyCode::Character('e' | 'E' | ' ') => &mut self.up,
            KeyCode::Character('q' | 'Q') => &mut self.down,
            _ => return false,
        };
        if *slot == pressed {
            return false;
        }
        *slot = pressed;
        true
    }

    fn any(self) -> bool {
        self.forward || self.backward || self.left || self.right || self.up || self.down
    }
}

impl GlassworksNativeState {
    fn new(options: StartupOptions) -> Self {
        Self {
            app: GlassworksApp::new_with_options(options),
            layout_canvas: LayoutCanvasResources::default(),
            viewport_3d_canvas: Viewport3dCanvasResources::default(),
            layout_pan_drag: None,
            last_left_click: None,
            modifiers: KeyModifiers::NONE,
            flycam_keys: FlycamKeyState::default(),
            last_flycam_tick: Instant::now(),
            native_flycam_captured: false,
        }
    }

    fn build_document(&self, viewport: UiSize) -> UiDocument {
        let mut document = self
            .app
            .build_operad_document(viewport)
            .expect("Glassworks document should build for native window");
        attach_default_pointer_actions(&mut document);
        document
    }

    fn handle_widget_action(&mut self, action: WidgetAction) -> bool {
        if !matches!(action.kind, WidgetActionKind::Activate(_)) {
            return false;
        }
        let Some(action_id) = action.binding.action_id() else {
            return false;
        };
        let previous_view = self.app.active_view();
        let handled = self.app.apply_clicked_node_name(action_id.as_str());
        if handled && self.app.active_view() != previous_view {
            self.layout_pan_drag = None;
            self.flycam_keys = FlycamKeyState::default();
        }
        handled
    }

    fn handle_canvas_input(&mut self, input: NativeCanvasInput) -> bool {
        match input.key.as_str() {
            "glassworks.layout.viewport.2d" => self.handle_layout_2d_canvas_input(&input),
            "glassworks.layout.viewport.3d" => self.handle_layout_3d_canvas_input(&input),
            _ => false,
        }
    }

    fn handle_layout_2d_canvas_input(&mut self, input: &NativeCanvasInput) -> bool {
        if self.app.active_view() != StartupView::Layout2d {
            return false;
        }

        if let RawInputEvent::Pointer(pointer) = &input.input {
            match pointer.kind {
                PointerEventKind::Move if self.update_layout_pan_drag(pointer.position) => {
                    return true;
                }
                PointerEventKind::Down(PointerButton::Secondary) => {
                    return self.handle_layout_secondary_click(pointer.position, input.rect)
                        || self.begin_layout_pan_drag(pointer.position, input.rect);
                }
                PointerEventKind::Down(PointerButton::Auxiliary) => {
                    return self.begin_layout_pan_drag(pointer.position, input.rect);
                }
                PointerEventKind::Up(PointerButton::Secondary | PointerButton::Auxiliary)
                | PointerEventKind::Cancel => return self.end_layout_pan_drag(),
                PointerEventKind::Down(PointerButton::Primary)
                | PointerEventKind::Up(PointerButton::Primary)
                | PointerEventKind::Move => {}
                PointerEventKind::Down(_) | PointerEventKind::Up(_) => return false,
            }
        }

        let Some(event) = native_canvas_ui_event(input) else {
            return false;
        };
        let mut handled =
            self.app
                .handle_layout_canvas_input_with_modifiers(&event, input.rect, self.modifiers);
        if let RawInputEvent::Pointer(pointer) = &input.input
            && matches!(pointer.kind, PointerEventKind::Up(PointerButton::Primary))
            && self.consume_left_double_click(pointer.position)
        {
            handled |= self
                .app
                .handle_layout_canvas_double_click(pointer.position, input.rect);
        }
        handled
    }

    fn handle_layout_3d_canvas_input(&mut self, input: &NativeCanvasInput) -> bool {
        if self.app.active_view() != StartupView::Layout3d {
            return false;
        }
        if let RawInputEvent::Pointer(pointer) = &input.input {
            match pointer.kind {
                PointerEventKind::Down(PointerButton::Secondary) if self.app.flycam_captured() => {
                    self.set_native_flycam_capture(false);
                    return true;
                }
                PointerEventKind::Down(PointerButton::Auxiliary)
                | PointerEventKind::Up(PointerButton::Auxiliary)
                | PointerEventKind::Down(PointerButton::Secondary)
                | PointerEventKind::Up(PointerButton::Secondary) => return false,
                PointerEventKind::Cancel => {
                    self.set_native_flycam_capture(false);
                    return true;
                }
                _ => {}
            }
        }

        let Some(event) = native_canvas_ui_event(input) else {
            return false;
        };
        let handled = self.app.handle_layout_3d_canvas_input(&event, input.rect);
        if handled && self.app.flycam_captured() {
            self.native_flycam_captured = false;
        }
        handled
    }

    fn handle_keyboard_input(&mut self, input: NativeKeyboardInput) -> bool {
        self.modifiers = key_modifiers(input.modifiers);
        let Some(key) = input.key_code else {
            return false;
        };

        if self.app.flycam_captured() {
            let key_state_changed = self.update_flycam_key_state(key, input.pressed);
            if input.pressed && key == KeyCode::Escape && !has_command_modifier(self.modifiers) {
                self.set_native_flycam_capture(false);
                return true;
            }
            if input.pressed && self.app.handle_layout_3d_key(key, self.modifiers) {
                return true;
            }
            return key_state_changed || self.app.active_view() == StartupView::Layout3d;
        }

        if input.pressed {
            if self.app.active_view() == StartupView::Layout2d
                && self.app.layout_browser_search_active()
                && self
                    .app
                    .handle_layout_browser_search_key(key, self.modifiers)
            {
                return true;
            }
            if self.app.active_view() == StartupView::Layout2d
                && self.app.layout_browser_replace_active()
                && self
                    .app
                    .handle_layout_browser_replace_key(key, self.modifiers)
            {
                return true;
            }
            self.handle_keyboard_shortcut(key, self.modifiers)
        } else {
            false
        }
    }

    fn handle_keyboard_shortcut(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        if key == KeyCode::Escape && !has_command_modifier(modifiers) {
            return self.app.dismiss_transient_ui();
        }

        if primary_shortcut(modifiers)
            && (matches_shortcut_char(key, 'k') || matches_shortcut_char(key, 'p'))
        {
            return self
                .app
                .apply_clicked_node_name("glassworks.menu.item.tools.palette");
        }

        if self.handle_view_cycle_shortcut(key, modifiers) {
            return true;
        }

        if self.handle_editor_shortcut(key, modifiers) {
            return true;
        }

        if self.app.active_view() == StartupView::Layout2d
            && primary_shortcut(modifiers)
            && matches_shortcut_char(key, 'f')
        {
            return self.app.begin_layout_browser_search();
        }

        if modifiers.alt
            && !modifiers.ctrl
            && !modifiers.meta
            && let KeyCode::Character(character) = key
            && let Some(slug) = menu_slug_for_hotkey(character)
        {
            return self
                .app
                .apply_clicked_node_name(&format!("glassworks.menu.{slug}"));
        }

        false
    }

    fn handle_view_cycle_shortcut(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        if key != KeyCode::Tab || !primary_shortcut(modifiers) {
            return false;
        }

        let views = glassworks_studio::StartupView::ALL;
        let current = views
            .iter()
            .position(|view| *view == self.app.active_view())
            .unwrap_or(0);
        let next = if modifiers.shift {
            current.checked_sub(1).unwrap_or(views.len() - 1)
        } else {
            (current + 1) % views.len()
        };
        self.app.set_active_view(views[next]);
        self.layout_pan_drag = None;
        self.flycam_keys = FlycamKeyState::default();
        true
    }

    fn handle_editor_shortcut(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        if !matches!(
            self.app.active_view(),
            glassworks_studio::StartupView::Layout2d | glassworks_studio::StartupView::Layout3d
        ) {
            return false;
        }

        if primary_shortcut(modifiers) && matches_shortcut_char(key, 'c') {
            return self
                .app
                .apply_clicked_node_name("glassworks.menu.item.edit.copy");
        }
        if primary_shortcut(modifiers) && matches_shortcut_char(key, 'v') {
            return self
                .app
                .apply_clicked_node_name("glassworks.menu.item.edit.paste");
        }
        if primary_shortcut(modifiers) && matches_shortcut_char(key, 'd') {
            return self
                .app
                .apply_clicked_node_name("glassworks.menu.item.edit.duplicate");
        }
        if primary_shortcut(modifiers) && matches_shortcut_char(key, 'z') {
            let action = if modifiers.shift {
                "glassworks.menu.item.edit.redo"
            } else {
                "glassworks.menu.item.edit.undo"
            };
            return self.app.apply_clicked_node_name(action);
        }
        if primary_shortcut(modifiers) && matches_shortcut_char(key, 'y') {
            return self
                .app
                .apply_clicked_node_name("glassworks.menu.item.edit.redo");
        }
        if self.app.active_view() == glassworks_studio::StartupView::Layout3d
            && self.app.handle_layout_3d_key(key, modifiers)
        {
            return true;
        }
        if !has_command_modifier(modifiers) && self.app.active_view() == StartupView::Layout2d {
            if let Some(node) = layout_tool_shortcut_node(key) {
                return self.app.apply_clicked_node_name(node);
            }
            if matches_shortcut_char(key, 'r') {
                return self
                    .app
                    .apply_clicked_node_name("glassworks.menu.item.edit.rotate90");
            }
            if matches_shortcut_char(key, 'h') {
                return self
                    .app
                    .apply_clicked_node_name("glassworks.menu.item.edit.mirror_x");
            }
            if matches_shortcut_char(key, 'v') {
                return self
                    .app
                    .apply_clicked_node_name("glassworks.menu.item.edit.mirror_y");
            }
        }
        if self.app.handle_layout_editor_key(key, modifiers) {
            return true;
        }
        if key == KeyCode::Delete && !has_command_modifier(modifiers) {
            return self
                .app
                .apply_clicked_node_name("glassworks.menu.item.edit.delete");
        }

        false
    }

    fn handle_raw_mouse_motion(&mut self, input: NativeRawMouseMotion) -> bool {
        if !self.app.flycam_captured() {
            return false;
        }
        self.app.apply_layout_3d_mouse_delta(
            UiPoint::new(input.delta.0 as f32, input.delta.1 as f32),
            true,
        )
    }

    fn before_render(&mut self) {
        if matches!(
            self.app.active_view(),
            StartupView::Layout2d | StartupView::Layout3d
        ) && self.app.live_layout_fps_meter()
        {
            self.app.record_layout_frame_tick(Instant::now());
        }
        if self.app.active_view() != StartupView::Layout3d {
            self.flycam_keys = FlycamKeyState::default();
            self.last_flycam_tick = Instant::now();
            return;
        }
        if !self.app.flycam_captured() || !self.flycam_keys.any() {
            self.last_flycam_tick = Instant::now();
            return;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_flycam_tick).as_secs_f32();
        self.last_flycam_tick = now;
        self.app.advance_layout_3d_flycam(
            self.flycam_keys.forward,
            self.flycam_keys.backward,
            self.flycam_keys.left,
            self.flycam_keys.right,
            self.flycam_keys.up,
            self.flycam_keys.down,
            self.modifiers.shift,
            dt,
        );
    }

    fn idle_redraw(&self) -> bool {
        if matches!(
            self.app.active_view(),
            StartupView::Layout2d | StartupView::Layout3d
        ) && self.app.live_layout_fps_meter()
        {
            return true;
        }
        self.app.flycam_captured() && self.flycam_keys.any()
    }

    fn platform_requests(&mut self) -> Vec<PlatformRequest> {
        let should_capture =
            self.app.active_view() == StartupView::Layout3d && self.app.flycam_captured();
        if self.native_flycam_captured == should_capture {
            return Vec::new();
        }
        self.native_flycam_captured = should_capture;
        if should_capture {
            vec![
                PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::Locked)),
                PlatformRequest::Cursor(CursorRequest::SetVisible(false)),
            ]
        } else {
            vec![
                PlatformRequest::Cursor(CursorRequest::SetGrab(CursorGrabMode::None)),
                PlatformRequest::Cursor(CursorRequest::SetVisible(true)),
            ]
        }
    }

    fn set_native_flycam_capture(&mut self, captured: bool) -> bool {
        let changed = self.app.set_layout_3d_flycam_capture(captured);
        if !captured {
            self.flycam_keys = FlycamKeyState::default();
        }
        changed
    }

    fn update_flycam_key_state(&mut self, key: KeyCode, pressed: bool) -> bool {
        if self.app.active_view() != StartupView::Layout3d {
            return false;
        }
        self.flycam_keys.set_key(key, pressed)
    }

    fn handle_layout_secondary_click(&mut self, point: UiPoint, canvas_rect: UiRect) -> bool {
        if !matches!(self.app.active_tool(), ToolMode::Polygon | ToolMode::Path) {
            return false;
        }
        self.app
            .handle_layout_canvas_secondary_click(point, canvas_rect)
    }

    fn begin_layout_pan_drag(&mut self, point: UiPoint, canvas_rect: UiRect) -> bool {
        if !canvas_rect.contains_point(point) || self.app.active_view() != StartupView::Layout2d {
            return false;
        }
        self.layout_pan_drag = Some(point);
        true
    }

    fn update_layout_pan_drag(&mut self, point: UiPoint) -> bool {
        let Some(previous) = self.layout_pan_drag else {
            return false;
        };
        let delta = UiPoint::new(point.x - previous.x, point.y - previous.y);
        self.layout_pan_drag = Some(point);
        self.app.pan_layout_canvas_by(delta)
    }

    fn end_layout_pan_drag(&mut self) -> bool {
        self.layout_pan_drag.take().is_some()
    }

    fn consume_left_double_click(&mut self, point: UiPoint) -> bool {
        let now = Instant::now();
        let double_click = self.last_left_click.is_some_and(|(previous, at)| {
            now.duration_since(at) <= DOUBLE_CLICK_MAX_INTERVAL
                && ui_point_distance(previous, point) <= DOUBLE_CLICK_MAX_DISTANCE
        });
        self.last_left_click = if double_click {
            None
        } else {
            Some((point, now))
        };
        double_click
    }
}

fn native_hooks() -> NativeWindowHooks<GlassworksNativeState> {
    NativeWindowHooks::new()
        .with_scale_factor(|_state: &GlassworksNativeState, metrics| native_ui_scale(metrics))
        .with_keyboard_input(|state: &mut GlassworksNativeState, input| {
            state.handle_keyboard_input(input)
        })
        .with_raw_mouse_motion(|state: &mut GlassworksNativeState, input| {
            state.handle_raw_mouse_motion(input)
        })
        .with_canvas_input(|state: &mut GlassworksNativeState, input| {
            state.handle_canvas_input(input)
        })
        .with_platform_requests(|state: &mut GlassworksNativeState, _metrics| {
            state.platform_requests()
        })
        .with_before_render(|state: &mut GlassworksNativeState, _metrics| state.before_render())
        .with_idle_redraw(|state: &GlassworksNativeState| state.idle_redraw())
}

fn update_native_state(state: &mut GlassworksNativeState, action: WidgetAction) {
    state.handle_widget_action(action);
}

fn view_native_state(state: &GlassworksNativeState, viewport: UiSize) -> UiDocument {
    state.build_document(viewport)
}

fn render_native_layout_2d_canvas(
    state: &mut GlassworksNativeState,
    context: NativeWgpuCanvasRenderContext<'_>,
) -> Result<CanvasRenderOutput, RenderError> {
    render_layout_2d_canvas_with_size(
        &state.app,
        &mut state.layout_canvas,
        context.surface,
        UiSize::new(context.request.rect.width, context.request.rect.height),
    )
    .map_err(RenderError::Backend)?;
    Ok(layout_canvas_render_output(
        state.app.live_layout_fps_meter(),
    ))
}

fn render_native_layout_3d_canvas(
    state: &mut GlassworksNativeState,
    context: NativeWgpuCanvasRenderContext<'_>,
) -> Result<CanvasRenderOutput, RenderError> {
    render_layout_3d_canvas(&state.app, &mut state.viewport_3d_canvas, context.surface)
        .map_err(RenderError::Backend)?;
    Ok(layout_canvas_render_output(
        state.app.live_layout_fps_meter(),
    ))
}

fn layout_canvas_render_output(live_fps: bool) -> CanvasRenderOutput {
    CanvasRenderOutput::new().repaint_requested(live_fps)
}

fn attach_default_pointer_actions(document: &mut UiDocument) {
    for index in 0..document.node_count() {
        let id = UiNodeId::from_index(index);
        let action = {
            let node = document.node(id);
            let is_canvas = matches!(node.content(), UiContent::Canvas(_));
            (node.action().is_none() && node.input().pointer && !is_canvas)
                .then(|| node.name().to_string())
        };
        if let Some(action) = action {
            document.set_node_action(id, WidgetActionBinding::action(action));
        }
    }
}

fn native_canvas_ui_event(input: &NativeCanvasInput) -> Option<UiInputEvent> {
    input.input.to_ui_input_event_with_wheel_scale(
        CANVAS_LINE_SCROLL_POINTS,
        UiSize::new(input.rect.width, input.rect.height),
    )
}

fn key_modifiers(modifiers: winit::keyboard::ModifiersState) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.shift_key(),
        ctrl: modifiers.control_key(),
        alt: modifiers.alt_key(),
        meta: modifiers.super_key(),
    }
}

fn native_ui_scale(metrics: NativeWindowMetrics) -> f32 {
    let env_scale = std::env::var("GLASSWORKS_UI_SCALE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(1.0);
    metrics
        .dpi_scale
        .max(monitor_ui_scale(metrics.physical_size))
        .max(env_scale)
}

fn monitor_ui_scale(size: operad::platform::PixelSize) -> f32 {
    ((size.width as f32 / 1920.0).min(size.height as f32 / 1080.0)).clamp(1.0, 2.0)
}

fn ui_point_distance(a: UiPoint, b: UiPoint) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn has_command_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.ctrl || modifiers.alt || modifiers.meta
}

fn primary_shortcut(modifiers: KeyModifiers) -> bool {
    (modifiers.ctrl || modifiers.meta) && !modifiers.alt
}

fn matches_shortcut_char(key: KeyCode, expected: char) -> bool {
    matches!(
        key,
        KeyCode::Character(character)
            if character.eq_ignore_ascii_case(&expected)
    )
}

fn layout_tool_shortcut_node(key: KeyCode) -> Option<&'static str> {
    let KeyCode::Character(character) = key else {
        return None;
    };
    match character {
        '1' => Some("glassworks.tool.select"),
        '2' => Some("glassworks.tool.rect"),
        '3' => Some("glassworks.tool.poly"),
        '4' => Some("glassworks.tool.path"),
        '5' => Some("glassworks.tool.measure"),
        '6' => Some("glassworks.tool.route"),
        '7' => Some("glassworks.tool.label"),
        '8' => Some("glassworks.tool.trace"),
        _ => None,
    }
}

fn menu_slug_for_hotkey(character: char) -> Option<&'static str> {
    match character.to_ascii_lowercase() {
        'f' => Some("file"),
        'e' => Some("edit"),
        'v' => Some("view"),
        'b' => Some("bookmarks"),
        'd' => Some("display"),
        'o' => Some("options"),
        't' => Some("tools"),
        'm' => Some("macros"),
        'h' => Some("help"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_key_input(key_code: KeyCode, modifiers: KeyModifiers) -> NativeKeyboardInput {
        let logical_key = match key_code {
            KeyCode::Character(character) => {
                winit::keyboard::Key::Character(character.to_string().into())
            }
            KeyCode::Backspace => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace),
            KeyCode::Delete => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Delete),
            KeyCode::Enter => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter),
            KeyCode::Escape => winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            KeyCode::ArrowUp => winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp),
            KeyCode::ArrowDown => winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown),
            KeyCode::ArrowLeft => winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft),
            KeyCode::ArrowRight => {
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight)
            }
            _ => winit::keyboard::Key::Unidentified(winit::keyboard::NativeKey::Unidentified),
        };
        let mut native_modifiers = winit::keyboard::ModifiersState::empty();
        if modifiers.shift {
            native_modifiers |= winit::keyboard::ModifiersState::SHIFT;
        }
        if modifiers.ctrl {
            native_modifiers |= winit::keyboard::ModifiersState::CONTROL;
        }
        if modifiers.alt {
            native_modifiers |= winit::keyboard::ModifiersState::ALT;
        }
        if modifiers.meta {
            native_modifiers |= winit::keyboard::ModifiersState::SUPER;
        }
        NativeKeyboardInput {
            logical_key,
            physical_key: winit::keyboard::PhysicalKey::Unidentified(
                winit::keyboard::NativeKeyCode::Unidentified,
            ),
            key_code: Some(key_code),
            modifiers: native_modifiers,
            state: winit::event::ElementState::Pressed,
            pressed: true,
            repeat: false,
            text: None,
        }
    }

    #[test]
    fn monitor_scale_uses_readable_4k_fallback() {
        assert_eq!(monitor_ui_scale(PixelSize::new(1920, 1080)), 1.0);
        assert!((monitor_ui_scale(PixelSize::new(2560, 1440)) - 1.333).abs() < 0.01);
        assert_eq!(monitor_ui_scale(PixelSize::new(3840, 2160)), 2.0);
        assert_eq!(monitor_ui_scale(PixelSize::new(7680, 4320)), 2.0);
    }

    #[test]
    fn native_document_assigns_button_actions_for_operad_runner() {
        let state = GlassworksNativeState::new(StartupOptions::default());
        let document = state.build_document(UiSize::new(1024.0, 720.0));
        let nav = document
            .nodes()
            .iter()
            .find(|node| node.name() == "glassworks.nav.action.layout2d")
            .expect("layout nav node should be present");
        assert_eq!(
            nav.action().and_then(|action| action.action_id()),
            Some(&operad::WidgetActionId::new("glassworks.nav.action.layout2d"))
        );
    }

    #[test]
    fn native_widget_actions_route_through_existing_node_names() {
        let mut state = GlassworksNativeState::new(StartupOptions::default());
        assert!(state.handle_widget_action(WidgetAction::activate(
            UiNodeId::ROOT,
            "glassworks.nav.action.layout2d"
        )));
        assert_eq!(state.app.active_view(), StartupView::Layout2d);
    }

    #[test]
    fn native_layout_canvas_requests_repaint_for_live_fps_meter() {
        assert!(layout_canvas_render_output(true).repaint_requested);
        assert!(!layout_canvas_render_output(false).repaint_requested);
    }

    #[test]
    fn native_layout_views_idle_redraw_and_tick_fps_without_dirtying_scene() {
        let workflow = GlassworksNativeState::new(StartupOptions::default());
        assert!(
            !workflow.idle_redraw(),
            "non-layout views should not redraw only for the layout FPS meter"
        );

        let mut layout = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        assert!(layout.idle_redraw());

        let revision = layout.app.layout_revision();
        layout.app.record_layout_frame_tick(
            Instant::now()
                .checked_sub(Duration::from_millis(16))
                .expect("test timestamp should be representable"),
        );
        layout.before_render();

        assert_eq!(layout.app.layout_revision(), revision);
        assert!(
            layout.app.layout_fps_frame_ms().is_some(),
            "native layout frame tick should refresh only the FPS sample"
        );
    }

    #[test]
    fn native_pointer_events_route_to_layout_canvas_editor() {
        let mut state = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            zoom: Some(0.1),
            ..Default::default()
        });
        assert!(state.app.apply_clicked_node_name("glassworks.tool.rect"));
        let document = state.build_document(UiSize::new(1024.0, 720.0));
        let canvas = layout_canvas_rect(&document).expect("layout canvas should exist");
        let shape_count = state.app.workspace().document.shapes.len();
        let start = UiPoint::new(canvas.x + canvas.width * 0.5 - 30.0, canvas.y + 80.0);
        let end = UiPoint::new(canvas.x + canvas.width * 0.5 + 30.0, canvas.y + 140.0);

        assert!(state.handle_canvas_input(pointer_canvas_input(
            "glassworks.layout.viewport.2d",
            canvas,
            PointerEventKind::Down(PointerButton::Primary),
            start,
        )));
        assert!(state.handle_canvas_input(pointer_canvas_input(
            "glassworks.layout.viewport.2d",
            canvas,
            PointerEventKind::Move,
            end,
        )));
        assert!(state.handle_canvas_input(pointer_canvas_input(
            "glassworks.layout.viewport.2d",
            canvas,
            PointerEventKind::Up(PointerButton::Primary),
            end,
        )));

        assert_eq!(state.app.workspace().document.shapes.len(), shape_count + 1);
        assert!(
            state.app.selected_layout_shape().is_some(),
            "canvas-created geometry should become the layout selection"
        );
    }

    #[test]
    fn native_middle_or_right_drag_pans_layout_canvas() {
        let mut state = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            zoom: Some(0.1),
            ..Default::default()
        });
        let document = state.build_document(UiSize::new(1024.0, 720.0));
        let canvas = layout_canvas_rect(&document).expect("layout canvas should exist");
        let start = UiPoint::new(
            canvas.x + canvas.width * 0.5,
            canvas.y + canvas.height * 0.5,
        );
        let end = UiPoint::new(start.x + 50.0, start.y - 25.0);

        let before = state.app.layout_pan();
        assert!(state.handle_canvas_input(pointer_canvas_input(
            "glassworks.layout.viewport.2d",
            canvas,
            PointerEventKind::Down(PointerButton::Auxiliary),
            start,
        )));
        assert!(state.handle_canvas_input(pointer_canvas_input(
            "glassworks.layout.viewport.2d",
            canvas,
            PointerEventKind::Move,
            end,
        )));
        assert!(state.handle_canvas_input(pointer_canvas_input(
            "glassworks.layout.viewport.2d",
            canvas,
            PointerEventKind::Up(PointerButton::Auxiliary),
            end,
        )));
        assert_eq!(state.app.layout_pan(), [before[0] + 50.0, before[1] - 25.0]);
    }

    #[test]
    fn native_right_click_finishes_layout_polyline_instead_of_panning() {
        let mut state = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            zoom: Some(0.1),
            ..Default::default()
        });
        let document = state.build_document(UiSize::new(1024.0, 720.0));
        let canvas = layout_canvas_rect(&document).expect("layout canvas should exist");
        let shape_count = state.app.workspace().document.shapes.len();
        assert!(state.app.apply_clicked_node_name("glassworks.tool.poly"));

        for point in [
            UiPoint::new(canvas.x + 220.0, canvas.y + 220.0),
            UiPoint::new(canvas.x + 280.0, canvas.y + 220.0),
            UiPoint::new(canvas.x + 280.0, canvas.y + 280.0),
        ] {
            assert!(state.handle_canvas_input(pointer_canvas_input(
                "glassworks.layout.viewport.2d",
                canvas,
                PointerEventKind::Down(PointerButton::Primary),
                point,
            )));
            assert!(state.handle_canvas_input(pointer_canvas_input(
                "glassworks.layout.viewport.2d",
                canvas,
                PointerEventKind::Up(PointerButton::Primary),
                point,
            )));
        }
        assert!(state.handle_canvas_input(pointer_canvas_input(
            "glassworks.layout.viewport.2d",
            canvas,
            PointerEventKind::Down(PointerButton::Secondary),
            UiPoint::new(canvas.x + 280.0, canvas.y + 280.0),
        )));

        assert_eq!(state.app.workspace().document.shapes.len(), shape_count + 1);
        assert_eq!(state.layout_pan_drag, None);
    }

    #[test]
    fn native_3d_canvas_click_captures_flycam_and_wasd_advances() {
        let mut state = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout3d),
            ..Default::default()
        });
        let document = state.build_document(UiSize::new(1024.0, 720.0));
        let canvas = layout_canvas_rect(&document).expect("3D canvas should exist");
        let point = UiPoint::new(
            canvas.x + canvas.width * 0.5,
            canvas.y + canvas.height * 0.5,
        );

        assert!(state.handle_canvas_input(pointer_canvas_input(
            "glassworks.layout.viewport.3d",
            canvas,
            PointerEventKind::Down(PointerButton::Primary),
            point,
        )));
        assert!(state.app.flycam_captured());
        assert!(!state.platform_requests().is_empty());
        assert!(state.native_flycam_captured);

        let before = state
            .app
            .workspace()
            .document
            .flattened_shape_count_estimate();
        assert!(before > 0);
        assert!(state.update_flycam_key_state(KeyCode::Character('w'), true));
        assert!(state.app.advance_layout_3d_flycam(
            state.flycam_keys.forward,
            state.flycam_keys.backward,
            state.flycam_keys.left,
            state.flycam_keys.right,
            state.flycam_keys.up,
            state.flycam_keys.down,
            false,
            1.0 / 60.0,
        ));

        assert!(state.set_native_flycam_capture(false));
        assert!(!state.app.flycam_captured());
    }

    #[test]
    fn native_alt_hotkeys_open_underlined_top_menus() {
        let mut state = GlassworksNativeState::new(StartupOptions::default());
        let alt = KeyModifiers {
            alt: true,
            ..KeyModifiers::NONE
        };

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('v'), alt));
        assert_eq!(state.app.active_menu(), Some(glassworks_studio::AppMenu::View));

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('D'), alt));
        assert_eq!(
            state.app.active_menu(),
            Some(glassworks_studio::AppMenu::Display)
        );
    }

    #[test]
    fn native_primary_palette_shortcuts_open_command_palette() {
        let ctrl = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        };
        let ctrl_shift = KeyModifiers {
            ctrl: true,
            shift: true,
            ..KeyModifiers::NONE
        };

        for (key, modifiers, label) in [
            (KeyCode::Character('k'), ctrl, "Ctrl/Cmd+K"),
            (KeyCode::Character('p'), ctrl, "Ctrl/Cmd+P"),
            (KeyCode::Character('P'), ctrl_shift, "Ctrl/Cmd+Shift+P"),
        ] {
            let mut state = GlassworksNativeState::new(StartupOptions::default());
            assert!(state.handle_keyboard_shortcut(key, modifiers));
            let document = state.build_document(UiSize::new(1024.0, 720.0));
            assert!(
                document
                    .nodes()
                    .iter()
                    .any(|node| node.name() == "glassworks.command_palette"),
                "{label} should expose the command palette"
            );
        }
    }

    #[test]
    fn native_layout_browser_search_shortcut_captures_text_before_tool_shortcuts() {
        let mut state = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        let ctrl = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        };
        assert!(state.handle_keyboard_shortcut(KeyCode::Character('f'), ctrl));
        assert!(state.app.layout_browser_search_active());

        assert!(state.handle_keyboard_input(native_key_input(
            KeyCode::Character('2'),
            KeyModifiers::NONE,
        )));
        assert_eq!(state.app.layout_browser_search(), "2");
        assert_eq!(
            state.app.active_tool(),
            ToolMode::Select,
            "active browser search should capture numeric keys before tool shortcuts"
        );

        assert!(
            state.handle_keyboard_input(native_key_input(KeyCode::Backspace, KeyModifiers::NONE,))
        );
        assert_eq!(state.app.layout_browser_search(), "");
        assert!(
            state.handle_keyboard_input(native_key_input(KeyCode::Escape, KeyModifiers::NONE,))
        );
        assert!(!state.app.layout_browser_search_active());

        assert!(state.app.begin_layout_browser_replace());
        assert!(state.handle_keyboard_input(native_key_input(
            KeyCode::Character('3'),
            KeyModifiers::NONE,
        )));
        assert_eq!(state.app.layout_browser_replace(), "3");
        assert_eq!(
            state.app.active_tool(),
            ToolMode::Select,
            "active browser replace should capture numeric keys before tool shortcuts"
        );
        assert!(
            state.handle_keyboard_input(native_key_input(KeyCode::Escape, KeyModifiers::NONE,))
        );
        assert!(!state.app.layout_browser_replace_active());
    }

    #[test]
    fn native_escape_closes_keyboard_opened_shell_ui() {
        let mut state = GlassworksNativeState::new(StartupOptions::default());
        let alt = KeyModifiers {
            alt: true,
            ..KeyModifiers::NONE
        };
        let ctrl = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        };

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('f'), alt));
        assert_eq!(state.app.active_menu(), Some(glassworks_studio::AppMenu::File));
        assert!(state.handle_keyboard_shortcut(KeyCode::Escape, KeyModifiers::NONE));
        assert_eq!(state.app.active_menu(), None);

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('k'), ctrl));
        assert!(state.handle_keyboard_shortcut(KeyCode::Escape, KeyModifiers::NONE));
        let document = state.build_document(UiSize::new(1024.0, 720.0));
        assert!(
            !document
                .nodes()
                .iter()
                .any(|node| node.name() == "glassworks.command_palette"),
            "Escape should close the command palette"
        );
    }

    #[test]
    fn native_primary_tab_cycles_between_apps() {
        let mut state = GlassworksNativeState::new(StartupOptions::default());
        let ctrl = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        };
        let ctrl_shift = KeyModifiers {
            ctrl: true,
            shift: true,
            ..KeyModifiers::NONE
        };

        assert_eq!(state.app.active_view(), StartupView::Workflow);
        assert!(state.handle_keyboard_shortcut(KeyCode::Tab, ctrl));
        assert_eq!(state.app.active_view(), StartupView::Layout2d);

        assert!(state.handle_keyboard_shortcut(KeyCode::Tab, ctrl_shift));
        assert_eq!(state.app.active_view(), StartupView::Workflow);
    }

    #[test]
    fn native_editor_shortcuts_route_to_layout_actions() {
        let mut state = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        let ctrl = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        };
        let shape_id = state
            .app
            .workspace()
            .document
            .shapes
            .keys()
            .next()
            .copied()
            .expect("demo document should have shapes");
        assert!(
            state
                .app
                .apply_clicked_node_name(&format!("glassworks.viewctl.layout.shape.{}", shape_id.0))
        );

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('c'), ctrl));
        let shape_count = state.app.workspace().document.shapes.len();

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('v'), ctrl));
        assert_eq!(state.app.workspace().document.shapes.len(), shape_count + 1);

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('d'), ctrl));
        assert_eq!(state.app.workspace().document.shapes.len(), shape_count + 2);

        assert!(state.handle_keyboard_shortcut(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(state.app.workspace().document.shapes.len(), shape_count + 1);
    }

    #[test]
    fn native_editor_shortcuts_route_undo_and_redo() {
        let mut state = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        let ctrl = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::NONE
        };
        let ctrl_shift = KeyModifiers {
            ctrl: true,
            shift: true,
            ..KeyModifiers::NONE
        };
        let shape_id = state
            .app
            .workspace()
            .document
            .shapes
            .keys()
            .next()
            .copied()
            .expect("demo document should have shapes");
        assert!(
            state
                .app
                .apply_clicked_node_name(&format!("glassworks.viewctl.layout.shape.{}", shape_id.0))
        );
        let shape_count = state.app.workspace().document.shapes.len();

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('d'), ctrl));
        assert_eq!(state.app.workspace().document.shapes.len(), shape_count + 1);

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('z'), ctrl));
        assert_eq!(state.app.workspace().document.shapes.len(), shape_count);

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('Z'), ctrl_shift));
        assert_eq!(state.app.workspace().document.shapes.len(), shape_count + 1);

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('z'), ctrl));
        assert_eq!(state.app.workspace().document.shapes.len(), shape_count);

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('y'), ctrl));
        assert_eq!(state.app.workspace().document.shapes.len(), shape_count + 1);
    }

    #[test]
    fn native_layout_single_key_shortcuts_switch_tools_and_transform() {
        let mut state = GlassworksNativeState::new(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        let shape_id = state
            .app
            .workspace()
            .document
            .shapes
            .keys()
            .next()
            .copied()
            .expect("demo document should have shapes");
        assert!(
            state
                .app
                .apply_clicked_node_name(&format!("glassworks.viewctl.layout.shape.{}", shape_id.0))
        );

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('2'), KeyModifiers::NONE));
        assert_eq!(state.app.active_tool(), ToolMode::Rect);
        assert!(state.handle_keyboard_shortcut(KeyCode::Character('6'), KeyModifiers::NONE));
        assert_eq!(state.app.active_tool(), ToolMode::Route);
        assert!(state.handle_keyboard_shortcut(KeyCode::Character('7'), KeyModifiers::NONE));
        assert_eq!(state.app.active_tool(), ToolMode::Label);
        assert!(state.handle_keyboard_shortcut(KeyCode::Character('8'), KeyModifiers::NONE));
        assert_eq!(state.app.active_tool(), ToolMode::Trace);
        assert!(state.handle_keyboard_shortcut(KeyCode::Character('1'), KeyModifiers::NONE));
        assert_eq!(state.app.active_tool(), ToolMode::Select);

        assert!(state.handle_keyboard_shortcut(KeyCode::Character('r'), KeyModifiers::NONE));
        assert!(state.app.status_message().contains("Rotated"));
        assert!(
            state.handle_keyboard_shortcut(KeyCode::Character('h'), KeyModifiers::NONE),
            "H should route to the mirror-X edit action"
        );
    }

    #[test]
    fn layout_3d_batch_covers_demo_shapes() {
        let document = Document::demo();
        let batch = build_layout_3d_batch_with_options(&document, true);
        let validation = batch
            .validate_geometry()
            .expect("demo 3D canvas batch should validate");
        assert!(validation.rect_slabs >= 6, "{validation:?}");
        assert!(validation.guide_segments > 2, "{validation:?}");
    }

    #[test]
    fn fitted_layout_viewport_tracks_canvas_aspect() {
        let viewport = glassworks_studio::fitted_layout_viewport(
            Some(Rect::from_min_size(Point::new(0, 0), 1_000, 500)),
            PixelSize::new(1600, 800),
        );
        assert_eq!(viewport.width(), viewport.height() * 2);
        assert!(viewport.width() > 1_000);
        assert!(viewport.height() > 500);
    }

    fn layout_canvas_rect(document: &UiDocument) -> Option<UiRect> {
        document
            .nodes()
            .iter()
            .find(|node| node.name() == "glassworks.layout.preview")
            .map(|node| node.layout().rect)
    }

    fn pointer_canvas_input(
        key: &'static str,
        rect: UiRect,
        kind: PointerEventKind,
        position: UiPoint,
    ) -> NativeCanvasInput {
        NativeCanvasInput {
            node: UiNodeId::ROOT,
            key: key.to_string(),
            rect,
            local_position: Some(UiPoint::new(position.x - rect.x, position.y - rect.y)),
            input: RawInputEvent::Pointer(RawPointerEvent::new(kind, position, 1)),
        }
    }
}
