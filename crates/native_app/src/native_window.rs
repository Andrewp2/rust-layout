use std::sync::Arc;

use fabricad_app::{FabricadApp, StartupOptions, UiScale};
use operad::{
    ColorRgba, RenderFrameRequest, RenderOptions, RenderTarget, RendererAdapter, UiDocument,
    UiInputEvent, UiPoint, UiSize, WgpuSurfaceRenderer,
};
use operad_wgpu as wgpu;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

const DEFAULT_WIDTH: u32 = 1440;
const DEFAULT_HEIGHT: u32 = 920;

pub fn run(options: StartupOptions) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = NativeWindowApp::new(options);
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct NativeWindowApp {
    app: FabricadApp,
    window: Option<Arc<Window>>,
    renderer: Option<WgpuSurfaceRenderer<'static>>,
    document: Option<UiDocument>,
    cursor_position: Option<UiPoint>,
}

impl NativeWindowApp {
    fn new(options: StartupOptions) -> Self {
        Self {
            app: FabricadApp::new_with_options(options),
            window: None,
            renderer: None,
            document: None,
            cursor_position: None,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.window.is_some() {
            return Ok(());
        }

        let attributes = WindowAttributes::default()
            .with_title("Fabricad")
            .with_inner_size(LogicalSize::new(
                DEFAULT_WIDTH as f64,
                DEFAULT_HEIGHT as f64,
            ))
            .with_min_inner_size(LogicalSize::new(720.0, 480.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|err| format!("create Fabricad window: {err}"))?,
        );
        self.renderer = Some(pollster::block_on(create_renderer(&window))?);
        window.request_redraw();
        self.window = Some(window);
        Ok(())
    }

    fn render(&mut self) -> Result<(), String> {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
            return Ok(());
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        let viewport = UiSize::new(size.width as f32, size.height as f32);
        let ui_scale = window_ui_scale(window);
        let document = self.app.build_operad_document_scaled(viewport, ui_scale)?;
        let request = RenderFrameRequest::new(
            RenderTarget::window("fabricad.main", viewport),
            viewport,
            document.paint_list(),
        )
        .options(RenderOptions {
            clear_color: ColorRgba::new(14, 18, 22, 255),
            ..Default::default()
        });
        renderer
            .render_frame(request, &operad::EmptyResourceResolver)
            .map_err(|err| format!("render Fabricad window: {err}"))?;
        self.document = Some(document);
        Ok(())
    }

    fn handle_ui_input(&mut self, event: UiInputEvent) -> bool {
        let Some(document) = self.document.as_mut() else {
            return false;
        };
        let result = document.handle_input(event);
        if let Some(clicked) = result.clicked {
            let name = document.node(clicked).name.clone();
            return self.app.apply_clicked_node_name(&name);
        }
        result.hovered.is_some() || result.scrolled.is_some()
    }

    fn handle_error(event_loop: &ActiveEventLoop, error: String) {
        eprintln!("ERROR {error}");
        event_loop.exit();
    }
}

impl ApplicationHandler for NativeWindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.create_window(event_loop) {
            Self::handle_error(event_loop, error);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.id() != window_id)
        {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                let point = point_from_position(position);
                self.cursor_position = Some(point);
                if self.handle_ui_input(UiInputEvent::PointerMove(point))
                    && let Some(window) = &self.window
                {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(point) = self.cursor_position {
                    let event = match state {
                        ElementState::Pressed => UiInputEvent::PointerDown(point),
                        ElementState::Released => UiInputEvent::PointerUp(point),
                    };
                    if self.handle_ui_input(event)
                        && let Some(window) = &self.window
                    {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(point) = self.cursor_position {
                    let ui_scale = self
                        .window
                        .as_ref()
                        .map(|window| window_ui_scale(window).factor())
                        .unwrap_or(1.0);
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            UiPoint::new(x * 36.0 * ui_scale, -y * 36.0 * ui_scale)
                        }
                        MouseScrollDelta::PixelDelta(delta) => {
                            UiPoint::new(delta.x as f32, delta.y as f32)
                        }
                    };
                    if self.handle_ui_input(UiInputEvent::wheel(point, scroll))
                        && let Some(window) = &self.window
                    {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.render() {
                    Self::handle_error(event_loop, error);
                }
            }
            _ => {}
        }
    }
}

fn point_from_position(position: PhysicalPosition<f64>) -> UiPoint {
    UiPoint::new(position.x as f32, position.y as f32)
}

fn window_ui_scale(window: &Window) -> UiScale {
    let os_scale = window.scale_factor() as f32;
    let monitor_scale = window
        .current_monitor()
        .map(|monitor| {
            let size = monitor.size();
            ((size.width as f32 / 2560.0).min(size.height as f32 / 1440.0)).clamp(1.0, 2.0)
        })
        .unwrap_or(1.0);
    let env_scale = std::env::var("FABRICAD_UI_SCALE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(1.0);
    UiScale::new(os_scale.max(monitor_scale).max(env_scale))
}

async fn create_renderer(window: &Arc<Window>) -> Result<WgpuSurfaceRenderer<'static>, String> {
    let size = window.inner_size();
    let width = size.width.max(1);
    let height = size.height.max(1);
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let surface = instance
        .create_surface(Arc::clone(window))
        .map_err(|err| format!("create WGPU surface: {err}"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .map_err(|err| format!("request WGPU adapter: {err}"))?;
    let adapter_features = adapter.features();
    let required_features = if adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
        wgpu::Features::TIMESTAMP_QUERY
    } else {
        wgpu::Features::empty()
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("fabricad-window-device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await
        .map_err(|err| format!("request WGPU device: {err}"))?;
    let surface_config = surface
        .get_default_config(&adapter, width, height)
        .ok_or_else(|| "WGPU adapter cannot present to Fabricad window".to_string())?;
    WgpuSurfaceRenderer::new(surface, device, queue, surface_config)
        .map_err(|err| format!("initialize Operad surface renderer: {err}"))
}
