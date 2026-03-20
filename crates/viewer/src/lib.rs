//! 3D mesh viewer and GUI application for the CAD kernel.
//!
//! Provides two modes of operation:
//!
//! - [`view_mesh`] — lightweight window that renders a single mesh with orbit
//!   camera controls (used by the `view` CLI command).
//! - [`run_gui`] — full desktop CAD application with egui panels, menu bar,
//!   model tree, properties inspector, and shape creation dialogs.

mod app;
pub mod command;
mod gui;
pub mod nav;
pub mod picking;
mod render;

pub use nav::{NavConfig, NavStyle};
pub use render::{
    Camera, DisplayMode, Projection, StandardView, Uniforms, Vertex, compute_bounds,
    mesh_to_vertices,
};

use cadkernel_io::Mesh;
use nav::{NavAction, NavConfig as NavCfg};
use render::{DisplayMode as DM, GpuState, GridConfig, MouseState};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// ---------------------------------------------------------------------------
// Simple viewer (backward-compatible)
// ---------------------------------------------------------------------------

struct ViewerApp {
    title: String,
    vertices: Vec<Vertex>,
    gpu: Option<GpuState>,
    camera: Camera,
    mouse: MouseState,
    nav: NavCfg,
    grid_config: GridConfig,
}

impl ApplicationHandler for ViewerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(&self.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
                )
                .unwrap(),
        );
        let size = window.inner_size();
        self.camera.aspect = size.width as f32 / size.height.max(1) as f32;
        self.gpu = Some(pollster::block_on(GpuState::new(window, &self.vertices)));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed => match key_event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                PhysicalKey::Code(KeyCode::Digit5 | KeyCode::Numpad5) => {
                    self.camera.toggle_projection();
                    if let Some(gpu) = &self.gpu {
                        gpu.window.request_redraw();
                    }
                }
                _ => {}
            },

            WindowEvent::Resized(new_size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(new_size, &mut self.camera);
                    gpu.window.request_redraw();
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                let s = mods.state();
                self.mouse.shift_held = s.shift_key();
                self.mouse.ctrl_held = s.control_key();
                self.mouse.alt_held = s.alt_key();
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.mouse.left_pressed = pressed,
                    MouseButton::Middle => self.mouse.middle_pressed = pressed,
                    MouseButton::Right => self.mouse.right_pressed = pressed,
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some((lx, ly)) = self.mouse.last_pos {
                    let dx = position.x - lx;
                    let dy = position.y - ly;

                    let action = self.nav.resolve_drag(
                        self.mouse.left_pressed,
                        self.mouse.middle_pressed,
                        self.mouse.right_pressed,
                        self.mouse.shift_held,
                        self.mouse.ctrl_held,
                    );

                    match action {
                        NavAction::Orbit => {
                            self.camera.yaw -= dx as f32 * self.nav.orbit_sensitivity;
                            self.camera.pitch -= dy as f32 * self.nav.orbit_sensitivity;
                        }
                        NavAction::Pan => {
                            let speed = self.camera.distance * self.nav.pan_sensitivity;
                            let r = self.camera.screen_right();
                            let u = self.camera.screen_up();
                            let mx = -dx as f32 * speed;
                            let my = dy as f32 * speed;
                            self.camera.target[0] += r[0] * mx + u[0] * my;
                            self.camera.target[1] += r[1] * mx + u[1] * my;
                            self.camera.target[2] += r[2] * mx + u[2] * my;
                        }
                        NavAction::Zoom => {
                            let factor = self.nav.scroll_zoom_factor(-dy as f32 * 0.05);
                            self.camera.distance *= factor;
                            self.camera.distance = self.camera.distance.max(0.01);
                        }
                        NavAction::None => {}
                    }

                    if action != NavAction::None {
                        if let Some(gpu) = &self.gpu {
                            gpu.window.request_redraw();
                        }
                    }
                }
                self.mouse.last_pos = Some((position.x, position.y));
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                };
                let factor = self.nav.scroll_zoom_factor(scroll);
                self.camera.distance *= factor;
                self.camera.distance = self.camera.distance.max(0.01);
                if let Some(gpu) = &self.gpu {
                    gpu.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                if self.grid_config.update_for_camera(self.camera.distance) {
                    if let Some(gpu) = &mut self.gpu {
                        gpu.rebuild_grid(&self.grid_config);
                    }
                }
                if let Some(gpu) = &self.gpu {
                    gpu.render(&self.camera, DM::Shading, true);
                }
            }

            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Opens a window and renders the given mesh in 3D.
///
/// The window supports:
/// - **Left-drag** — orbit rotate
/// - **Right-drag** — pan
/// - **Scroll** — zoom
/// - **Escape** — close
pub fn view_mesh(mesh: &Mesh, title: &str) {
    let vertices = mesh_to_vertices(mesh);
    if vertices.is_empty() {
        eprintln!("Empty mesh, nothing to display.");
        return;
    }

    let (min, max) = compute_bounds(&vertices);
    let mut camera = Camera::new(16.0 / 9.0);
    camera.fit_to_bounds(min, max);

    let dx = max[0] - min[0];
    let dy = max[1] - min[1];
    let dz = max[2] - min[2];
    let obj_extent = dx.max(dy).max(dz);

    let mut grid_config = GridConfig::new();
    grid_config.set_object_extent(obj_extent);
    grid_config.force_rebuild();
    grid_config.update_for_camera(camera.distance);

    let event_loop = EventLoop::new().unwrap();
    let mut app = ViewerApp {
        title: title.to_string(),
        vertices,
        gpu: None,
        camera,
        mouse: MouseState::new(),
        nav: NavCfg::new(),
        grid_config,
    };
    event_loop.run_app(&mut app).unwrap();
}

/// Launches the full CAD GUI application.
pub fn run_gui() {
    app::run_gui();
}
