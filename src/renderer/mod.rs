mod depth_map_render;
mod pass;
mod scene_render;

use crate::{
    camera::GPUCameraData,
    dir_light::{self, GPUDirLight},
    grid, gui, mesh, skybox,
    vkutils_new::{self},
};
use ash::vk;

struct Passes {
    shadow_map: pass::shadow_map::ShadowMapPass,
    scene: pass::scene::SceneColorPass,
    depth_display: pass::depth_map_display::DepthMapDisplayPass,
    ui: pass::ui::UiPass,
}

struct Submits {
    shadow_map_render: depth_map_render::DepthMapRender,
    scene_color_render: scene_render::ColorSceneRender,
}

pub struct Renderer {
    camera_data_buffer: vkutils_new::buffer::Buffer,

    cube_mesh_data: mesh::mesh_data::MeshData,
    meshes: std::vec::Vec<mesh::Mesh>,
    dir_light: dir_light::DirLight,
    passes: Passes,
    submits: Submits,

    // TODO do something with this shit
    skybox: skybox::Skybox,
    grid: grid::Grid,
    scene_pass: bool,
}

impl Renderer {
    pub fn new(ctx: &mut vkutils_new::context::VulkanContext) -> Self {
        let camera_data_buffer = ctx.create_bar_buffer(
            size_of::<GPUCameraData>(),
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let cube_mesh_data = mesh::mesh_data::MeshData::new2("assets/cube.gltf", &ctx);
        let mut meshes = vec![
            mesh::Mesh::new2(&cube_mesh_data, &ctx, "Cube"),
            mesh::Mesh::new2(&cube_mesh_data, &ctx, "Floor"),
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

        let dir_light = dir_light::DirLight::new2(
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

        let skybox = skybox::Skybox::new2(
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
            dir_light.buffer_device_address,
            &meshes,
        );

        let scene_render = scene_render::ColorSceneRender::new(
            ctx,
            shadow_map_pass.command_buffers.clone(),
            scene_pass.command_buffers.clone(),
            ui_pass.command_buffers.clone(),
        );

        Self {
            camera_data_buffer,
            cube_mesh_data,
            meshes,
            dir_light,
            passes: Passes {
                shadow_map: shadow_map_pass,
                scene: scene_pass,
                depth_display: depth_map_display_pass,
                ui: ui_pass,
            },
            submits: Submits {
                shadow_map_render,
                scene_color_render: scene_render,
            },
            skybox,
            grid,
            scene_pass: true,
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
