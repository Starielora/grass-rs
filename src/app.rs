use ash::vk;
use winit::application::ApplicationHandler;
use winit::event::ElementState;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;

use crate::camera;
use crate::fps_window;
use crate::gui;
use crate::gui_scene_node::GuiCameraNode;
use crate::gui_scene_node::GuiSceneNode;
use crate::renderer;
use crate::vkutils;

const NUM_CAMERAS: usize = 3;

pub struct App {
    cameras: [Option<camera::Camera>; NUM_CAMERAS],
    current_view_camera_index: usize,
    current_control_camera_index: usize,
    gui: Option<gui::Gui>,
    renderer: Option<renderer::Renderer>,
    vkctx: Option<vkutils::context::VulkanContext>,
    window: Option<std::rc::Rc<winit::window::Window>>,
    last_frame: std::time::Instant,
    keyboard_modifiers_state: winit::event::Modifiers,
    cursor_visible: bool,
    previous_frame_timestamp: std::time::Instant,
    frame_number: usize,
}

impl App {
    pub fn new() -> App {
        Self {
            gui: Option::None,
            current_view_camera_index: 0,
            current_control_camera_index: 0,
            cameras: [const { Option::None }; NUM_CAMERAS],
            renderer: Option::None,
            vkctx: Option::None,
            window: Option::None,
            last_frame: std::time::Instant::now(),
            keyboard_modifiers_state: winit::event::Modifiers::default(),
            cursor_visible: false,
            previous_frame_timestamp: std::time::Instant::now(),
            frame_number: 0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attrs = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(1440, 900));
        let window = std::rc::Rc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Could not create window"),
        );

        window.set_cursor_visible(self.cursor_visible);
        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
        let mut vkctx = vkutils::context::VulkanContext::new(&window);
        let renderer = renderer::Renderer::new(&mut vkctx);
        for camera in &mut self.cameras {
            camera
                .insert(camera::Camera::new(
                    vkctx.swapchain.extent.width as f32,
                    vkctx.swapchain.extent.height as f32,
                ))
                .look_around(0.0, 0.0);
        }
        // TODO I don't quite like this dependency gui->renderer->gui
        // i.e. first gui gets nodes from renderer, and then renderer uses gui to render imgui,
        // but atm I don't have any better idea
        let gui = gui::Gui::new(window.clone(), &vkctx, renderer.gui_scene_nodes.clone());

        self.vkctx = Some(vkctx);
        self.renderer = Some(renderer);
        self.gui = Some(gui);
        self.window = Some(window);
        self.last_frame = std::time::Instant::now();
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let (camera_pos, camera_projview) = {
            let camera = self
                .cameras
                .iter_mut()
                .nth(self.current_view_camera_index)
                .unwrap()
                .as_mut()
                .unwrap();
            camera.update_pos();
            (camera.pos(), camera.get_projection_view())
        };

        let (image_index, acquire_semaphore) = {
            let vkctx = self.vkctx.as_mut().unwrap();
            vkctx.swapchain.acquire_next_image(!0, vk::Fence::null())
        };

        let (
            shadow_map_render_duration,
            scene_render_duration,
            meshlet_render_duration,
            ui_render_duration,
        ) = {
            let renderer = self.renderer.as_mut().unwrap();
            renderer
                .camera_data_buffer
                .update_contents(&[camera::GPUCameraData {
                    pos: camera_pos,
                    projview: camera_projview,
                }]);

            if self.frame_number == 0 {
                (
                    std::time::Duration::from_secs(0),
                    std::time::Duration::from_secs(0),
                    std::time::Duration::from_secs(0),
                    std::time::Duration::from_secs(0),
                )
            } else {
                renderer.get_pass_durations()
            }
        };

        let current_timestamp = std::time::Instant::now();
        let cpu_duration = current_timestamp - self.previous_frame_timestamp;
        self.previous_frame_timestamp = current_timestamp;

        // Take gui so self has no live borrows - is that a smell?
        let mut gui = self.gui.take().unwrap();
        gui.prepare_frame(
            self,
            fps_window::FrameDurations {
                cpu: cpu_duration,
                gpu: shadow_map_render_duration + scene_render_duration,
                shadow_map: shadow_map_render_duration,
                color_pass: scene_render_duration,
                meshlet_pass: meshlet_render_duration,
                ui: ui_render_duration,
            },
        );
        self.gui = Some(gui);

        let renderer = self.renderer.as_mut().unwrap();
        let gui = self.gui.as_mut().unwrap();
        let vkctx = self.vkctx.as_mut().unwrap();
        renderer.record_imgui_pass(image_index, &vkctx, gui);

        let queue = vkctx.graphics_present_queue;
        let render_finished_semaphore =
            renderer.submit(&vkctx.device, queue, image_index, acquire_semaphore);

        vkctx
            .swapchain
            .present(image_index, &[render_finished_semaphore], queue);

        unsafe { vkctx.device.device_wait_idle() }.expect("Failed to wait");

        self.frame_number += 1;
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _cause: winit::event::StartCause,
    ) {
        if self.gui.is_some() {
            let now = std::time::Instant::now();
            self.gui
                .as_mut()
                .unwrap()
                .update_delta_time(now - self.last_frame);
            self.last_frame = now;
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let camera = self
            .cameras
            .iter_mut()
            .nth(self.current_view_camera_index)
            .unwrap()
            .as_mut()
            .unwrap();

        match event {
            winit::event::DeviceEvent::MouseMotion { delta } => {
                if !self.cursor_visible {
                    camera.look_around(delta.0 as f32, delta.1 as f32);
                }
            }
            _ => (),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let camera = self
            .cameras
            .iter_mut()
            .nth(self.current_view_camera_index)
            .unwrap()
            .as_mut()
            .unwrap();
        let window = self.window.as_ref().unwrap();
        let gui = self.gui.as_mut().unwrap();

        gui.handle_winit_window_event(window_id, &event);

        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                let (key, state) = (event.physical_key, event.state);
                match key {
                    PhysicalKey::Code(key_code) => match (key_code, state) {
                        (KeyCode::Escape, ElementState::Pressed) => {
                            self.cursor_visible = !self.cursor_visible;
                            window.set_cursor_visible(self.cursor_visible);
                            window
                                .set_cursor_grab(if self.cursor_visible {
                                    winit::window::CursorGrabMode::None
                                } else {
                                    winit::window::CursorGrabMode::Confined
                                })
                                .unwrap();
                        }
                        (KeyCode::KeyA, _) => camera.set_move_left(state == ElementState::Pressed),
                        (KeyCode::KeyD, _) => camera.set_move_right(state == ElementState::Pressed),
                        (KeyCode::KeyW, _) => {
                            camera.set_move_forward(state == ElementState::Pressed)
                        }
                        (KeyCode::KeyS, _) => {
                            camera.set_move_backward(state == ElementState::Pressed)
                        }
                        (KeyCode::KeyF, _) => {}
                        (KeyCode::KeyQ, _) => camera.set_move_down(state == ElementState::Pressed),
                        (KeyCode::KeyE, _) => camera.set_move_up(state == ElementState::Pressed),
                        (KeyCode::F4, ElementState::Pressed) => {
                            match self.keyboard_modifiers_state.lalt_state() {
                                winit::keyboard::ModifiersKeyState::Pressed => event_loop.exit(),
                                winit::keyboard::ModifiersKeyState::Unknown => {}
                            }
                        }
                        _ => {
                            if let PhysicalKey::Code(key) = key {
                                println!(
                                    "Key {:?}: {}",
                                    state,
                                    winit::platform::scancode::PhysicalKeyExtScancode::to_scancode(
                                        key
                                    )
                                    .unwrap()
                                )
                            }
                        }
                    },
                    PhysicalKey::Unidentified(_) => (),
                }
            }
            winit::event::WindowEvent::ModifiersChanged(state) => {
                self.keyboard_modifiers_state = state;
                println!("Modifiers changed to {:?}", state);
            }
            _ => (),
        }
    }
}

impl GuiSceneNode for App {
    fn update(self: &mut Self, ui: &imgui::Ui) {
        let mut camid = 0;
        for _camera in &self.cameras {
            ui.radio_button(
                format!("View {}", camid),
                &mut self.current_view_camera_index,
                camid,
            );
            ui.same_line();
            ui.radio_button(
                format!("Control {}", camid),
                &mut self.current_control_camera_index,
                camid,
            );
            ui.same_line();
            let mut render_camera_model = false; // in the future will control whether camera model (box) is rendered
            let mut render_camera_furstum = false; // in the future will control whether camera frustum is rendered
            ui.checkbox(format!("Model {}", camid), &mut render_camera_model);
            ui.checkbox(format!("Frustum {}", camid), &mut render_camera_furstum);
            camid += 1;
        }

        self.cameras
            .iter_mut()
            .nth(self.current_view_camera_index)
            .unwrap()
            .as_mut()
            .unwrap()
            .update(ui);
    }
}
