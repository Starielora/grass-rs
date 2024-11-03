use ash::vk;
use imgui_winit_support::HiDpiMode;
use winit::application::ApplicationHandler;
use winit::event::ElementState;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;

use crate::camera;
use crate::vkutils;

pub struct App {
    camera: camera::Camera,
    imguictx: Option<imgui::Context>,
    imgui_renderer: Option<imgui_rs_vulkan_renderer::Renderer>,
    vkctx: Option<vkutils::Context>,
    platform: Option<imgui_winit_support::WinitPlatform>,
    window: Option<winit::window::Window>,
    last_frame: std::time::Instant,
}

impl App {
    pub fn new() -> App {
        let mut camera = camera::Camera::new();
        camera.look_around(0.0, 0.0);

        // yeah, gg winit/Rust, well played. Do not initialize shit in constructor, just fucking
        // wrap in optional and initialize later
        Self {
            window: Option::None,
            camera,
            platform: Option::None,
            vkctx: Option::None,
            imguictx: Option::None,
            imgui_renderer: Option::None,
            last_frame: std::time::Instant::now(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(winit::window::WindowAttributes::default())
            .expect("Could not create window");

        let vkctx = vkutils::Context::new(&window);

        let mut imguictx = imgui::Context::create();
        imguictx.set_ini_filename(None);

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imguictx);
        platform.attach_window(imguictx.io_mut(), &window, HiDpiMode::Rounded);

        let imgui_renderer = imgui_rs_vulkan_renderer::Renderer::with_default_allocator(
            &vkctx.instance,
            vkctx.physical_device,
            vkctx.device.clone(),
            vkctx.present_queue,
            vkctx.command_pool,
            vkctx.render_pass,
            &mut imguictx,
            Some(imgui_rs_vulkan_renderer::Options {
                in_flight_frames: 1,
                sample_count: vk::SampleCountFlags::TYPE_8,
                ..Default::default()
            }),
        )
        .expect("Could not create imgui renderer");

        self.window = Some(window);
        self.vkctx = Some(vkctx);
        self.imguictx = Some(imguictx);
        self.platform = Some(platform);
        self.imgui_renderer = Some(imgui_renderer);
        self.last_frame = std::time::Instant::now();
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let camera = &mut self.camera;
        let window = self.window.as_ref().unwrap();
        let vkctx = &mut self.vkctx.as_mut().unwrap();
        let imguictx = &mut self.imguictx.as_mut().unwrap();
        let platform = self.platform.as_ref().unwrap();
        let imgui_renderer = &mut self.imgui_renderer.as_mut().unwrap();

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

        unsafe {
            vkctx.device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                vkctx.graphics_pipeline.pipeline,
            )
        };
        unsafe { vkctx.device.cmd_draw(*command_buffer, 3, 1, 0, 0) };

        unsafe {
            vkctx.device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                vkctx.grid_pipeline,
            )
        };

        unsafe { vkctx.device.cmd_draw(*command_buffer, 6, 1, 0, 0) };

        platform
            .prepare_frame(imguictx.io_mut(), &window)
            .expect("Failed to prepare frame.");
        let ui = imguictx.frame();
        ui.window("Hello world")
            .size([300.0, 110.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text_wrapped("Hello world!");
                ui.text_wrapped("こんにちは世界！");
                ui.button("This...is...imgui-rs!");
                ui.separator();
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
            });

        imgui_renderer
            .cmd_draw(*command_buffer, imguictx.render())
            .expect("Could not draw imgui");

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
        if self.imguictx.is_some() {
            let now = std::time::Instant::now();
            self.imguictx
                .as_mut()
                .unwrap()
                .io_mut()
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
                camera.look_around(delta.0 as f32, delta.1 as f32);
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
        let imguictx = &mut self.imguictx.as_mut().unwrap();
        let platform = self.platform.as_mut().unwrap();

        // FFFFFFFFFFFFFFFFFFFFFFFFFFFUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUU
        // handle_window_event is private so I have to wrap this shit, even though handle_event
        // calls only handle_window_event
        let ev = winit::event::Event::<()>::WindowEvent {
            window_id,
            event: event.clone(),
        };
        platform.handle_event(imguictx.io_mut(), &window, &ev);

        match event {
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                let (key, state) = (event.physical_key, event.state);
                match key {
                    PhysicalKey::Code(key_code) => match (key_code, state) {
                        (KeyCode::Escape, ElementState::Pressed) => event_loop.exit(),
                        (KeyCode::KeyA, _) => camera.set_move_left(state == ElementState::Pressed),
                        (KeyCode::KeyD, _) => camera.set_move_right(state == ElementState::Pressed),
                        (KeyCode::KeyW, _) => {
                            camera.set_move_forward(state == ElementState::Pressed)
                        }
                        (KeyCode::KeyS, _) => {
                            camera.set_move_backward(state == ElementState::Pressed)
                        }
                        (KeyCode::KeyF, _) => {
                            window.set_cursor_visible(state == ElementState::Pressed)
                        }
                        (KeyCode::KeyQ, _) => camera.set_move_down(state == ElementState::Pressed),
                        (KeyCode::KeyE, _) => camera.set_move_up(state == ElementState::Pressed),
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
            _ => (),
        }
    }
}
