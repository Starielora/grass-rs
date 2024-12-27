use ash::vk;
use winit::application::ApplicationHandler;
use winit::event::ElementState;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;

use crate::camera;
use crate::dir_light;
use crate::drawable;
use crate::grid;
use crate::gui;
use crate::gui_scene_node::GuiSceneNode;
use crate::mesh;
use crate::push_constants::GPUPushConstants;
use crate::skybox;
use crate::vkutils;

pub struct App {
    camera: camera::Camera,
    gui: Option<std::rc::Rc<std::cell::RefCell<gui::Gui>>>,
    mesh_pipeline: Option<mesh::pipeline::Pipeline>,
    drawables: std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn drawable::Drawable>>>,
    scene_nodes: std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn GuiSceneNode>>>,
    dir_light: Option<std::rc::Rc<std::cell::RefCell<dir_light::DirLight>>>,
    meshes: std::vec::Vec<mesh::mesh_data::MeshData>,
    vkctx: Option<vkutils::Context>,
    window: Option<winit::window::Window>,
    last_frame: std::time::Instant,
    keyboard_modifiers_state: winit::event::Modifiers,
    cursor_visible: bool,
    push_constants: Option<GPUPushConstants>,
}

impl App {
    pub fn new() -> App {
        let mut camera = camera::Camera::new();
        camera.look_around(0.0, 0.0);

        Self {
            window: Option::None,
            gui: Option::None,
            mesh_pipeline: Option::None,
            drawables: std::vec::Vec::new(),
            scene_nodes: std::vec::Vec::new(),
            camera,
            vkctx: Option::None,
            last_frame: std::time::Instant::now(),
            keyboard_modifiers_state: winit::event::Modifiers::default(),
            cursor_visible: false,
            push_constants: Option::None,
            dir_light: Option::None,
            meshes: std::vec::Vec::new(),
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
            vkctx.surface_format.format,
            vkctx.depth_image_format,
        )
        .expect("Could not create grid pipeline");

        let mesh_pipeline = mesh::pipeline::Pipeline::new(&vkctx);
        let cube_mesh = mesh::mesh_data::MeshData::new("assets/cube.gltf", &vkctx);

        let cube = std::rc::Rc::new(std::cell::RefCell::new(mesh::Mesh::new(
            &cube_mesh,
            &mesh_pipeline,
            &vkctx,
            "Cube",
        )));

        let cube2 = std::rc::Rc::new(std::cell::RefCell::new(mesh::Mesh::new(
            &cube_mesh,
            &mesh_pipeline,
            &vkctx,
            "Floor",
        )));

        // set init transformations. Technically I could move these to cube constructor
        {
            cube.as_ref().borrow_mut().set_transformation(
                glm::make_vec3(&[3.0, 2.0, 1.0]),
                glm::make_vec3(&[0.0, 0.0, 0.0]),
                glm::make_vec3(&[1.0, 1.0, 1.0]),
            );

            cube2.as_ref().borrow_mut().set_transformation(
                glm::make_vec3(&[0.0, 0.0, 0.0]),
                glm::make_vec3(&[0.0, 0.0, 0.0]),
                glm::make_vec3(&[10.0, 0.5, 10.0]),
            );
        }

        let skybox = std::rc::Rc::new(std::cell::RefCell::new(skybox::Skybox::new(
            &vkctx,
            cube_mesh.vertex_buffer,
            cube_mesh.index_buffer,
            cube_mesh.indices_count,
        )));

        let gui = std::rc::Rc::new(std::cell::RefCell::new(gui::Gui::new(&window, &vkctx)));

        let init_dir_light_data = dir_light::GPUDirLight {
            dir: glm::make_vec4(&[1.0, -0.5, 0.5, 0.0]),
            color: glm::make_vec4(&[1.0, 1.0, 1.0, 0.0]),
        };

        let dir_light = std::rc::Rc::new(std::cell::RefCell::new(dir_light::DirLight::new(
            init_dir_light_data,
            &vkctx,
        )));

        self.scene_nodes.push(dir_light.clone());
        self.scene_nodes.push(cube.clone());
        self.scene_nodes.push(cube2.clone());
        self.scene_nodes.push(skybox.clone());

        self.dir_light = Some(dir_light.clone());

        self.push_constants = Some(GPUPushConstants {
            cube_vertex: 0, //cube.borrow().vertex_buffer_device_address,
            cube_model: 0,  //cube.borrow().model_buffer_device_address,
            camera_data_buffer_address: vkctx.camera.buffer_address,
            dir_light_buffer_address: self
                .dir_light
                .as_ref()
                .unwrap()
                .borrow()
                .buffer_device_address,
            current_skybox: 0,
        });

        self.drawables.push(cube);
        self.drawables.push(cube2);
        self.drawables.push(skybox);
        self.drawables
            .push(std::rc::Rc::new(std::cell::RefCell::new(grid)));
        self.drawables.push(gui.clone());

        self.gui = Some(gui);
        self.mesh_pipeline = Some(mesh_pipeline);
        self.window = Some(window);
        self.vkctx = Some(vkctx);
        self.last_frame = std::time::Instant::now();

        self.meshes.push(cube_mesh);
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
            .copy_from_slice(&[camera::GPUCameraData {
                pos: glm::make_vec4(&[camera.pos.x, camera.pos.y, camera.pos.z, 0.0]),
                projview: camera.get_projection_view(
                    vkctx.window_extent.width as f32,
                    vkctx.window_extent.height as f32,
                ),
            }]);

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

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [153.0 / 255.0, 204.0 / 255.0, 255.0 / 255.0, 1.0],
            },
        };

        let depth_clear_value = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };

        let color_attachments = [vk::RenderingAttachmentInfo::default()
            .image_view(vkctx.color_image.view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(color_clear_value)];

        let depth_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(vkctx.depth_image.view)
            .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(depth_clear_value);

        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                extent: vkctx.window_extent,
                offset: vk::Offset2D { x: 0, y: 0 },
            })
            .layer_count(1)
            .color_attachments(&color_attachments)
            .depth_attachment(&depth_attachment);

        let color_subresource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(vk::REMAINING_ARRAY_LAYERS);

        vkctx.image_barrier(
            *command_buffer,
            vkctx.color_image.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::ALL_GRAPHICS,
            color_subresource_range,
        );

        unsafe {
            vkctx
                .device
                .cmd_begin_rendering(*command_buffer, &rendering_info);
        }

        self.gui
            .as_mut()
            .unwrap()
            .borrow_mut()
            .prepare_frame(&window, &mut self.scene_nodes);

        for d in self.drawables.iter_mut() {
            d.borrow_mut()
                .cmd_draw(&command_buffer, self.push_constants.as_ref().unwrap());
        }

        unsafe { vkctx.device.cmd_end_rendering(*command_buffer) };

        vkctx.image_barrier(
            *command_buffer,
            vkctx.color_image.image,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::TRANSFER,
            color_subresource_range,
        );

        vkctx.image_barrier(
            *command_buffer,
            vkctx.swapchain_images.images[image_index as usize],
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::TRANSFER,
            color_subresource_range,
        );

        let copy_subresource = vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1);

        let image_copy = [vk::ImageCopy::default()
            .src_subresource(copy_subresource)
            .dst_subresource(copy_subresource)
            .extent(
                vk::Extent3D::default()
                    .width(vkctx.window_extent.width)
                    .height(vkctx.window_extent.height)
                    .depth(1),
            )];

        unsafe {
            vkctx.device.cmd_copy_image(
                *command_buffer,
                vkctx.color_image.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vkctx.swapchain_images.images[image_index as usize],
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &image_copy,
            );
        }

        vkctx.image_barrier(
            *command_buffer,
            vkctx.swapchain_images.images[image_index as usize],
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            color_subresource_range,
        );

        unsafe { vkctx.device.end_command_buffer(*command_buffer) }
            .expect("Failed to end command buffer???");

        let acquire_semaphores = [vkctx.acquire_semaphore];
        let command_buffers = [*command_buffer];
        let wait_semaphores = [vkctx.wait_semaphore];
        let submits = [vk::SubmitInfo::default()
            .wait_semaphores(&acquire_semaphores)
            .command_buffers(&command_buffers)
            .signal_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::BOTTOM_OF_PIPE])];
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
