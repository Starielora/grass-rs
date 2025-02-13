use crate::vkutils;
use ash::vk;

struct CommandBuffers {
    scene: [vk::CommandBuffer; 2],
    imgui: [vk::CommandBuffer; 2],
}

struct Semaphores {
    pub scene_render_finished: vk::Semaphore,
    pub gui_finished: vk::Semaphore,
}

pub struct MeshletRender {
    command_buffers: CommandBuffers,
    semaphores: Semaphores,
    device: ash::Device,
}

impl std::ops::Drop for MeshletRender {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_semaphore(self.semaphores.scene_render_finished, None);
            self.device
                .destroy_semaphore(self.semaphores.gui_finished, None);
        }
    }
}

impl MeshletRender {
    pub fn new(
        ctx: &mut vkutils::context::VulkanContext,
        scene_command_buffers: [vk::CommandBuffer; 2],
        imgui_command_buffers: [vk::CommandBuffer; 2],
    ) -> Self {
        Self {
            command_buffers: CommandBuffers {
                scene: scene_command_buffers,
                imgui: imgui_command_buffers,
            },
            semaphores: Semaphores {
                scene_render_finished: ctx.create_semaphore_vk(),
                gui_finished: ctx.create_semaphore_vk(),
            },
            device: ctx.device.clone(),
        }
    }

    pub fn submit(
        &self,
        device: &ash::Device,
        queue: vk::Queue,
        swapchain_acquire_semaphore: vk::Semaphore,
        image_index: usize,
    ) -> vk::Semaphore {
        let swapchain_acquire = [swapchain_acquire_semaphore];
        let scene_command_buffers = [self.command_buffers.scene[image_index]];
        let scene_render_finishied = [self.semaphores.scene_render_finished];
        let imgui_command_buffers = [self.command_buffers.imgui[image_index]];
        let swapchain_present_wait = [self.semaphores.gui_finished];

        let submits = [
            vk::SubmitInfo::default()
                .wait_semaphores(&swapchain_acquire)
                .command_buffers(&scene_command_buffers)
                .signal_semaphores(&scene_render_finishied)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::MESH_SHADER_EXT]),
            vk::SubmitInfo::default()
                .wait_semaphores(&scene_render_finishied)
                .command_buffers(&imgui_command_buffers)
                .signal_semaphores(&swapchain_present_wait)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]),
        ];

        unsafe {
            device
                .queue_submit(queue, &submits, vk::Fence::null())
                .expect("Failed to submit shadow map display commands");
        }

        swapchain_present_wait[0]
    }
}
