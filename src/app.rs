use ash::vk;
use winit::application::ApplicationHandler;
use winit::event::ElementState;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;

use crate::camera;
use crate::depth_map_display_pipeline;
use crate::depth_map_display_pipeline::DepthMapDisplayPipeline;
use crate::dir_light;
use crate::drawable::Drawable;
use crate::grid;
use crate::gui;
use crate::gui_scene_node::GuiSceneNode;
use crate::mesh;
use crate::push_constants::GPUPushConstants;
use crate::skybox;
use crate::vkutils;
use crate::vkutils_new;

pub struct App {
    camera: camera::Camera,
    gui: Option<std::rc::Rc<std::cell::RefCell<gui::Gui>>>,
    mesh_pipeline: Option<vkutils_new::pipeline::Pipeline>,
    mesh_depth_pipeline: Option<vkutils_new::pipeline::Pipeline>,
    depth_map_display_pipeline: Option<DepthMapDisplayPipeline>,
    grid: Option<grid::Grid>,
    skybox: Option<std::rc::Rc<std::cell::RefCell<skybox::Skybox>>>,
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
            mesh_depth_pipeline: Option::None,
            depth_map_display_pipeline: None,
            grid: None,
            skybox: None,
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

fn record_imgui_commands(
    vkctx: &vkutils::Context,
    window: &winit::window::Window,
    gui: &mut gui::Gui,
    scene_nodes: &mut std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn GuiSceneNode>>>,
    push_constants: &mut GPUPushConstants,
    resolve_image: vk::Image,
    resolve_image_view: vk::ImageView,
    command_buffer: vk::CommandBuffer,
    pipeline: vk::Pipeline,
) {
    let device = vkctx.device.clone();

    let begin_info = vk::CommandBufferBeginInfo {
        ..Default::default()
    };

    unsafe {
        device
            .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
            .expect("Failed to reset imgui command buffer");
        device.begin_command_buffer(command_buffer, &begin_info)
    }
    .expect("Failed to begin command buffer");

    let color_subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(vk::REMAINING_ARRAY_LAYERS);

    vkutils_new::image_barrier(
        &vkctx.device,
        command_buffer,
        resolve_image,
        (
            vk::ImageLayout::UNDEFINED,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ),
        (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        color_subresource_range,
    );

    let color_attachments = [vk::RenderingAttachmentInfo::default()
        .image_view(vkctx.color_image.view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::LOAD)
        .store_op(vk::AttachmentStoreOp::STORE)
        .resolve_mode(vk::ResolveModeFlags::AVERAGE)
        .resolve_image_view(resolve_image_view)
        .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

    let rendering_info = vk::RenderingInfo::default()
        .render_area(vk::Rect2D {
            extent: vkctx.swapchain.extent,
            offset: vk::Offset2D { x: 0, y: 0 },
        })
        .layer_count(1)
        .color_attachments(&color_attachments);

    let color_subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(vk::REMAINING_ARRAY_LAYERS);

    vkutils_new::image_barrier(
        &vkctx.device,
        command_buffer,
        vkctx.color_image.handle,
        (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::COLOR_ATTACHMENT_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ),
        color_subresource_range,
    );

    unsafe {
        vkctx
            .device
            .cmd_begin_rendering(command_buffer, &rendering_info);
    }

    // let window = self.window.as_ref().unwrap();
    // TODO fix this in the future... is it possible to prerecord?
    gui.prepare_frame(&window, scene_nodes);

    gui.cmd_draw(command_buffer, pipeline, push_constants);

    unsafe { vkctx.device.cmd_end_rendering(command_buffer) };

    vkutils_new::image_barrier(
        &vkctx.device,
        command_buffer,
        resolve_image,
        (
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        ),
        color_subresource_range,
    );

    unsafe { vkctx.device.end_command_buffer(command_buffer) }
        .expect("Failed to end command buffer???");
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attrs = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(1440, 900));
        let window = event_loop
            .create_window(window_attrs)
            .expect("Could not create window");

        window.set_cursor_visible(self.cursor_visible);

        let mut vkctx = vkutils::Context::new(&window);

        let grid = grid::Grid::new(
            &vkctx.device,
            &vkctx.swapchain.extent,
            vkctx.swapchain.surface_format.format,
            vkctx.depth_image.format,
            vkctx.bindless_descriptor_set.pipeline_layout,
        )
        .expect("Could not create grid pipeline");

        let mesh_pipeline = mesh::pipeline::new(&vkctx);
        let mesh_depth_pipeline = mesh::pipeline::new_shadow_map(&vkctx);
        let cube_mesh = mesh::mesh_data::MeshData::new("assets/cube.gltf", &mut vkctx);

        let cube = std::rc::Rc::new(std::cell::RefCell::new(mesh::Mesh::new(
            &cube_mesh, &vkctx, "Cube",
        )));

        let cube2 = std::rc::Rc::new(std::cell::RefCell::new(mesh::Mesh::new(
            &cube_mesh, &vkctx, "Floor",
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

        let init_dir_light_data = dir_light::GPUDirLight {
            dir: glm::make_vec4(&[1.0, -0.5, 0.5, 0.0]),
            color: glm::make_vec4(&[1.0, 1.0, 1.0, 0.0]),
        };

        let dir_light = dir_light::DirLight::new(init_dir_light_data, &vkctx);

        let mut push_constants = GPUPushConstants {
            mesh_data: 0, // initialized later
            camera_data_buffer_address: vkctx.camera_buffer.device_address.unwrap(),
            dir_light_buffer_address: dir_light.buffer_device_address,
            skybox_data: 0, //initialized later
        };

        let skybox = skybox::Skybox::new(
            &mut vkctx,
            cube_mesh.vertex_buffer.handle,
            cube_mesh.index_buffer.handle,
            cube_mesh.indices_count,
        );

        let gui = std::rc::Rc::new(std::cell::RefCell::new(gui::Gui::new(&window, &vkctx)));

        let depth_map_display_pipeline = depth_map_display_pipeline::DepthMapDisplayPipeline::new(
            &vkctx,
            dir_light.depth_image.view,
        );

        vkctx.render_shadow_map_to_swapchain_image(
            &depth_map_display_pipeline,
            (dir_light.depth_image.handle, dir_light.depth_image.view),
        );

        // TODO it's own type for this particular pipeline
        let meshes: [std::rc::Rc<std::cell::RefCell<dyn Drawable>>; 2] =
            [cube.clone(), cube2.clone()];

        vkctx.render_scene(
            &skybox,
            &grid,
            &mut push_constants,
            mesh_pipeline.handle,
            &meshes,
        );

        vkctx.render_shadow_map(
            &mut push_constants,
            mesh_depth_pipeline.handle,
            (dir_light.depth_image.handle, dir_light.depth_image.view),
            dir_light.camera_buffer.device_address.unwrap(),
            &meshes,
        );

        push_constants.camera_data_buffer_address = vkctx.camera_buffer.device_address.unwrap();

        let dir_light = std::rc::Rc::new(std::cell::RefCell::new(dir_light));

        let skybox = std::rc::Rc::new(std::cell::RefCell::new(skybox));
        self.scene_nodes.push(dir_light.clone());
        self.scene_nodes.push(cube.clone());
        self.scene_nodes.push(cube2.clone());
        self.scene_nodes.push(skybox.clone());

        self.dir_light = Some(dir_light.clone());

        self.push_constants = Some(push_constants);
        self.grid = Some(grid);
        self.skybox = Some(skybox);

        self.gui = Some(gui);
        self.mesh_pipeline = Some(mesh_pipeline);
        self.mesh_depth_pipeline = Some(mesh_depth_pipeline);
        self.depth_map_display_pipeline = Some(depth_map_display_pipeline);
        self.window = Some(window);
        self.vkctx = Some(vkctx);
        self.last_frame = std::time::Instant::now();

        self.meshes.push(cube_mesh);
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let camera = &mut self.camera;
        let vkctx = &mut self.vkctx.as_mut().unwrap();
        let device = vkctx.device.clone();

        camera.update_pos();

        let image_index = vkctx.swapchain.acquire_next_image(
            !0,
            vkctx.acquire_semaphore.handle,
            vk::Fence::null(),
        );

        let imgui_command_buffer = vkctx.imgui_command_buffer;

        vkctx
            .camera_buffer
            .update_contents(&[camera::GPUCameraData {
                pos: glm::make_vec4(&[camera.pos.x, camera.pos.y, camera.pos.z, 0.0]),
                projview: camera.get_projection_view(
                    vkctx.swapchain.extent.width as f32,
                    vkctx.swapchain.extent.height as f32,
                ),
            }]);

        record_imgui_commands(
            &vkctx,
            self.window.as_ref().unwrap(),
            &mut self.gui.as_mut().unwrap().borrow_mut(),
            &mut self.scene_nodes,
            self.push_constants.as_mut().unwrap(),
            vkctx.swapchain.images[image_index as usize],
            vkctx.swapchain.views[image_index as usize],
            imgui_command_buffer,
            self.mesh_pipeline.as_mut().unwrap().handle,
        );

        // vkctx.submit_and_present_shadow_map(image_index);
        vkctx.submit_and_present_scene(image_index);

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
