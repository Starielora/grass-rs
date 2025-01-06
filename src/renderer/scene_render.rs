use crate::vkutils_new;
use ash::vk;

struct CommandBuffers {
    shadow_map: [vk::CommandBuffer; 2],
    scene: [vk::CommandBuffer; 2],
    imgui: [vk::CommandBuffer; 2],
}

struct Semaphores {
    pub shadow_map_draw_finished: vk::Semaphore,
    pub scene_render_finished: vk::Semaphore,
    pub gui_finished: vk::Semaphore,
}

pub struct ColorSceneRender {
    command_buffers: CommandBuffers,
    semaphores: Semaphores,
}

impl ColorSceneRender {
    pub fn new(
        ctx: &mut vkutils_new::context::VulkanContext,
        shadow_map_command_buffers: [vk::CommandBuffer; 2],
        scene_command_buffers: [vk::CommandBuffer; 2],
        imgui_command_buffers: [vk::CommandBuffer; 2],
    ) -> Self {
        Self {
            command_buffers: CommandBuffers {
                shadow_map: shadow_map_command_buffers,
                scene: scene_command_buffers,
                imgui: imgui_command_buffers,
            },
            semaphores: Semaphores {
                shadow_map_draw_finished: ctx.create_semaphore_vk(),
                scene_render_finished: ctx.create_semaphore_vk(),
                gui_finished: ctx.create_semaphore_vk(),
            },
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
        let shadow_map_command_buffers = [self.command_buffers.shadow_map[image_index]];
        let shadow_map_finished = [self.semaphores.shadow_map_draw_finished];
        let scene_command_buffers = [self.command_buffers.scene[image_index]];
        let scene_render_finishied = [self.semaphores.scene_render_finished];
        let imgui_command_buffers = [self.command_buffers.imgui[image_index]];
        let swapchain_present_wait = [self.semaphores.gui_finished];

        let submits = [
            vk::SubmitInfo::default()
                .wait_semaphores(&swapchain_acquire)
                .command_buffers(&shadow_map_command_buffers)
                .signal_semaphores(&shadow_map_finished)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::VERTEX_SHADER]),
            vk::SubmitInfo::default()
                .wait_semaphores(&shadow_map_finished)
                .command_buffers(&scene_command_buffers)
                .signal_semaphores(&scene_render_finishied)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]),
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
