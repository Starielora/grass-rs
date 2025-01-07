mod depth_map_render;
mod pass;
mod scene_render;

use crate::{
    camera::GPUCameraData,
    dir_light::{self, GPUDirLight},
    grid, gui,
    gui_scene_node::GuiSceneNode,
    mesh, skybox,
    vkutils_new::{self, vk_destroy::VkDestroy},
};
use ash::vk;

struct Passes {
    _shadow_map: pass::shadow_map::ShadowMapPass,
    scene: pass::scene::SceneColorPass,
    depth_display: pass::depth_map_display::DepthMapDisplayPass,
    ui: pass::ui::UiPass,
}

struct Submits {
    shadow_map_render: depth_map_render::DepthMapRender,
    scene_color_render: scene_render::ColorSceneRender,
}

pub struct Renderer {
    pub camera_data_buffer: vkutils_new::buffer::Buffer,

    pub gui_scene_nodes: std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn GuiSceneNode>>>,
    _cube_mesh_data: mesh::mesh_data::MeshData,
    passes: Passes,
    submits: Submits,

    _grid: grid::Grid,
    // TODO
    scene_pass: bool,
}

impl std::ops::Drop for Renderer {
    fn drop(&mut self) {
        self.camera_data_buffer.vk_destroy();
    }
}

impl Renderer {
    pub fn new(ctx: &mut vkutils_new::context::VulkanContext) -> Self {
        let camera_data_buffer = ctx.create_bar_buffer(
            size_of::<GPUCameraData>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let cube_mesh_data = mesh::mesh_data::MeshData::new("assets/cube.gltf", &ctx);
        let mut meshes = vec![
            mesh::Mesh::new(&cube_mesh_data, &ctx, "Cube"),
            mesh::Mesh::new(&cube_mesh_data, &ctx, "Floor"),
        ];

        // set init transformations. Technically I could move these to cube constructor
        {
            meshes[0].set_transformation(
                glm::make_vec3(&[3.0, 2.0, 1.0]),
                glm::make_vec3(&[0.0, 0.0, 0.0]),
                glm::make_vec3(&[1.0, 1.0, 1.0]),
            );

            meshes[1].set_transformation(
                glm::make_vec3(&[0.0, 0.0, 0.0]),
                glm::make_vec3(&[0.0, 0.0, 0.0]),
                glm::make_vec3(&[10.0, 0.5, 10.0]),
            );
        }

        let dir_light = dir_light::DirLight::new(
            GPUDirLight {
                dir: glm::make_vec4(&[1.0, -0.5, 0.5, 0.0]),
                color: glm::make_vec4(&[1.0, 1.0, 1.0, 0.0]),
            },
            &ctx,
        );

        let ui_pass = pass::ui::UiPass::new(ctx);

        // shadow map
        let shadow_map_pass = pass::shadow_map::ShadowMapPass::new(
            ctx,
            dir_light.camera_buffer.device_address.unwrap(),
            &meshes,
        );

        let depth_map_display_pass = pass::depth_map_display::DepthMapDisplayPass::new(
            ctx,
            (
                shadow_map_pass.output_depth_image.handle,
                shadow_map_pass.output_depth_image.view,
            ),
        );

        let shadow_map_render = depth_map_render::DepthMapRender::new(
            ctx,
            shadow_map_pass.command_buffers.clone(),
            depth_map_display_pass.command_buffers.clone(),
            ui_pass.command_buffers.clone(),
        );

        let skybox = skybox::Skybox::new(
            &ctx,
            cube_mesh_data.vertex_buffer.handle,
            cube_mesh_data.index_buffer.handle,
            cube_mesh_data.indices_count,
        );
        let grid = grid::Grid::new(
            &ctx.device,
            &ctx.swapchain.extent,
            ctx.swapchain.surface_format.format,
            ctx.depth_format,
            ctx.bindless_descriptor_set.pipeline_layout,
        )
        .expect("Failed to create Grid");

        let scene_pass = pass::scene::SceneColorPass::new(
            ctx,
            &skybox,
            &grid,
            camera_data_buffer.device_address.unwrap(),
            dir_light.buffer_device_address,
            &meshes,
        );

        let scene_render = scene_render::ColorSceneRender::new(
            ctx,
            shadow_map_pass.command_buffers.clone(),
            scene_pass.command_buffers.clone(),
            ui_pass.command_buffers.clone(),
        );

        let mut gui_scene_nodes: std::vec::Vec<std::rc::Rc<std::cell::RefCell<dyn GuiSceneNode>>> =
            vec![];

        {
            gui_scene_nodes.push(std::rc::Rc::new(std::cell::RefCell::new(dir_light)));
            gui_scene_nodes.push(std::rc::Rc::new(std::cell::RefCell::new(skybox)));

            for mesh in meshes {
                gui_scene_nodes.push(std::rc::Rc::new(std::cell::RefCell::new(mesh)));
            }
        }

        Self {
            camera_data_buffer,
            _cube_mesh_data: cube_mesh_data,
            passes: Passes {
                _shadow_map: shadow_map_pass,
                scene: scene_pass,
                depth_display: depth_map_display_pass,
                ui: ui_pass,
            },
            submits: Submits {
                shadow_map_render,
                scene_color_render: scene_render,
            },
            _grid: grid,
            scene_pass: true,
            gui_scene_nodes,
        }
    }

    pub fn record_imgui_pass(
        &self,
        image_index: u32,
        ctx: &vkutils_new::context::VulkanContext,
        gui: &mut gui::Gui,
    ) {
        let src_image = if self.scene_pass {
            let img = &self.passes.scene.render_target;
            (img.handle, img.view)
        } else {
            let img = &self.passes.depth_display.render_target;
            (img.handle, img.view)
        };

        let swapchain_image = (
            ctx.swapchain.images[image_index as usize],
            ctx.swapchain.views[image_index as usize],
        );

        self.passes
            .ui
            .record(image_index, &ctx, src_image, swapchain_image, gui)
    }

    pub fn submit(
        &self,
        device: &ash::Device,
        queue: vk::Queue,
        image_index: u32,
        swapchain_acquire_semaphore: vk::Semaphore,
    ) -> vk::Semaphore {
        if self.scene_pass {
            self.submits.scene_color_render.submit(
                device,
                queue,
                swapchain_acquire_semaphore,
                image_index as usize,
            )
        } else {
            self.submits.shadow_map_render.submit(
                device,
                queue,
                swapchain_acquire_semaphore,
                image_index as usize,
            )
        }
    }
}
