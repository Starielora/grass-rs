use ash::vk;
use winit::application::ApplicationHandler;
use winit::event::ElementState;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;

use crate::camera;
use crate::gui;
use crate::renderer;
use crate::vkutils;

pub struct App {
    camera: camera::Camera,
    gui: Option<gui::Gui>,
    renderer: Option<renderer::Renderer>,
    vkctx: Option<vkutils::context::VulkanContext>,
    window: Option<std::rc::Rc<winit::window::Window>>,
    last_frame: std::time::Instant,
    keyboard_modifiers_state: winit::event::Modifiers,
    cursor_visible: bool,
}

impl App {
    pub fn new() -> App {
        let mut camera = camera::Camera::new();
        camera.look_around(0.0, 0.0);

        Self {
            gui: Option::None,
            camera,
            renderer: Option::None,
            vkctx: Option::None,
            window: Option::None,
            last_frame: std::time::Instant::now(),
            keyboard_modifiers_state: winit::event::Modifiers::default(),
            cursor_visible: false,
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

        let mut vkctx = vkutils::context::VulkanContext::new(&window);
        let renderer = renderer::Renderer::new(&mut vkctx);

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
        let camera = &mut self.camera;
        let vkctx = self.vkctx.as_mut().unwrap();
        let device = vkctx.device.clone();
        let renderer = self.renderer.as_mut().unwrap();
        let mut gui = self.gui.as_mut().unwrap();

        camera.update_pos();

        let (image_index, acquire_semaphore) =
            vkctx.swapchain.acquire_next_image(!0, vk::Fence::null());

        renderer
            .camera_data_buffer
            .update_contents(&[camera::GPUCameraData {
                pos: glm::make_vec4(&[camera.pos.x, camera.pos.y, camera.pos.z, 0.0]),
                projview: camera.get_projection_view(
                    vkctx.swapchain.extent.width as f32,
                    vkctx.swapchain.extent.height as f32,
                ),
            }]);

        gui.prepare_frame();
        renderer.record_imgui_pass(image_index, &vkctx, &mut gui);

        let queue = vkctx.graphics_present_queue;
        let render_finished_semaphore =
            renderer.submit(&vkctx.device, queue, image_index, acquire_semaphore);

        vkctx
            .swapchain
            .present(image_index, &[render_finished_semaphore], queue);

        unsafe { device.device_wait_idle() }.expect("Failed to wait");
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
        let camera = &mut self.camera;

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
        let camera = &mut self.camera;
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
