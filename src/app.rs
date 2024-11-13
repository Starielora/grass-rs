use ash::vk;
use winit::application::ApplicationHandler;
use winit::event::ElementState;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;

use crate::camera;
use crate::cube;
use crate::drawable;
use crate::grid;
use crate::gui;
use crate::vkutils;

pub struct App {
    camera: camera::Camera,
    gui: Option<std::rc::Rc<std::cell::RefCell<gui::Gui>>>,
    drawables: std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn drawable::Drawable>>>,
    vkctx: Option<vkutils::Context>,
    window: Option<winit::window::Window>,
    last_frame: std::time::Instant,
    keyboard_modifiers_state: winit::event::Modifiers,
    cursor_visible: bool,
}

impl App {
    pub fn new() -> App {
        let mut camera = camera::Camera::new();
        camera.look_around(0.0, 0.0);

        Self {
            window: Option::None,
            gui: Option::None,
            drawables: std::vec::Vec::new(),
            camera,
            vkctx: Option::None,
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
        let window = event_loop
            .create_window(window_attrs)
            .expect("Could not create window");

        window.set_cursor_visible(self.cursor_visible);

        let vkctx = vkutils::Context::new(&window);

        let grid = grid::Grid::new(
            &vkctx.device,
            &vkctx.window_extent,
            &vkctx.graphics_pipeline.pipeline_layout,
            &vkctx.graphics_pipeline.descriptor_set,
            &vkctx.render_pass,
        )
        .expect("Could not create grid pipeline");

        let cube = std::rc::Rc::new(std::cell::RefCell::new(cube::Cube::new(
            vkctx.device.clone(),
            vkctx.graphics_pipeline.descriptor_set,
            vkctx.graphics_pipeline.descriptor_set_layout,
            vkctx.window_extent,
            vkctx.render_pass,
        )));

        let gui = std::rc::Rc::new(std::cell::RefCell::new(gui::Gui::new(
            &window,
            &vkctx,
            cube.borrow_mut().model.clone(),
        )));

        self.drawables.push(cube);
        self.drawables
            .push(std::rc::Rc::new(std::cell::RefCell::new(grid)));
        self.drawables.push(gui.clone());

        self.gui = Some(gui);
        self.window = Some(window);
        self.vkctx = Some(vkctx);
        self.last_frame = std::time::Instant::now();
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let camera = &mut self.camera;
        let window = self.window.as_ref().unwrap();
        let vkctx = &mut self.vkctx.as_mut().unwrap();

        camera.update_pos();

        let (image_index, _success) = unsafe {
            vkctx.swapchain_loader.acquire_next_image(
                vkctx.swapchain,
                !0,
                vkctx.acquire_semaphore,
                vk::Fence::null(),
            )
        }
        .expect("Could not acquire image");

        let command_buffer = vkctx.command_buffers.first().unwrap();

        unsafe {
            vkctx
                .device
                .reset_command_buffer(*command_buffer, vk::CommandBufferResetFlags::empty())
        }
        .expect("Failed to reset command buffer");

        vkctx
            .camera
            .data_slice
            .copy_from_slice(&[camera::CameraData {
                pos: cgmath::Vector4::new(0.0, 0.0, 0.0, 0.0),
                projview: camera.get_projection_view(
                    vkctx.window_extent.width as f32,
                    vkctx.window_extent.height as f32,
                ),
            }]);

        let descriptor_buffer_info = vk::DescriptorBufferInfo {
            buffer: vkctx.camera.buffer,
            offset: 0,
            range: vk::WHOLE_SIZE,
        };

        let descriptor_buffer_infos = [descriptor_buffer_info];
        let descriptor_writes = [vk::WriteDescriptorSet::default()
            .dst_set(vkctx.graphics_pipeline.descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&descriptor_buffer_infos)];

        unsafe { vkctx.device.update_descriptor_sets(&descriptor_writes, &[]) };

        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };

        unsafe {
            vkctx
                .device
                .begin_command_buffer(*command_buffer, &begin_info)
        }
        .expect("Failed to begin command buffer");
        let clear_color = vk::ClearColorValue {
            float32: [153.0 / 255.0, 204.0 / 255.0, 255.0 / 255.0, 1.0],
        };

        let depth_clear_value = vk::ClearDepthStencilValue {
            depth: 1.0,
            stencil: 0,
        };

        unsafe {
            vkctx.device.cmd_bind_descriptor_sets(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                vkctx.graphics_pipeline.pipeline_layout,
                0,
                &[vkctx.graphics_pipeline.descriptor_set],
                &[],
            )
        };

        let clear_values = [
            vk::ClearValue { color: clear_color },
            vk::ClearValue {
                depth_stencil: depth_clear_value,
            },
        ];
        let render_pass_begin = vk::RenderPassBeginInfo::default()
            .render_pass(vkctx.render_pass)
            .framebuffer(vkctx.framebuffers[image_index as usize])
            .render_area(vk::Rect2D {
                extent: vkctx.window_extent,
                offset: vk::Offset2D { x: 0, y: 0 },
            })
            .clear_values(&clear_values);

        unsafe {
            vkctx.device.cmd_begin_render_pass(
                *command_buffer,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            )
        };

        self.gui
            .as_mut()
            .unwrap()
            .borrow_mut()
            .prepare_frame(&window);

        for d in self.drawables.iter_mut() {
            d.borrow_mut().cmd_draw(&command_buffer);
        }

        unsafe { vkctx.device.cmd_end_render_pass(*command_buffer) };
        unsafe { vkctx.device.end_command_buffer(*command_buffer) }
            .expect("Failed to end command buffer???");

        let acquire_semaphores = [vkctx.acquire_semaphore];
        let command_buffers = [*command_buffer];
        let wait_semaphores = [vkctx.wait_semaphore];
        let submits = [vk::SubmitInfo::default()
            .wait_semaphores(&acquire_semaphores)
            .command_buffers(&command_buffers)
            .signal_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])];
        unsafe {
            vkctx
                .device
                .queue_submit(vkctx.present_queue, &submits, vk::Fence::null())
        }
        .expect("Failed to submit");

        let swapchains = [vkctx.swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .wait_semaphores(&wait_semaphores)
            .image_indices(&image_indices);

        unsafe {
            vkctx
                .swapchain_loader
                .queue_present(vkctx.present_queue, &present_info)
        }
        .expect("Failed to queue present");

        unsafe { vkctx.device.device_wait_idle() }.expect("Failed to wait");
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
                .borrow_mut()
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
        let mut gui = self.gui.as_mut().unwrap().borrow_mut();

        gui.handle_winit_window_event(window, window_id, &event);

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
