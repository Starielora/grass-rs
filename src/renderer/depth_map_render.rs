use crate::vkutils_new;
use ash::vk;

struct CommandBuffers {
    shadow_map: [vk::CommandBuffer; 2],
    depth_display: [vk::CommandBuffer; 2],
    imgui: [vk::CommandBuffer; 2],
}

struct Semaphores {
    pub shadow_map_draw_finished: vk::Semaphore,
    pub depth_map_render_finished: vk::Semaphore,
    pub gui_finished: vk::Semaphore,
}

pub struct DepthMapRender {
    command_buffers: CommandBuffers,
    semaphores: Semaphores,
    device: ash::Device,
}

impl DepthMapRender {
    pub fn new(
        ctx: &mut vkutils_new::context::VulkanContext,
        shadow_map_command_buffers: [vk::CommandBuffer; 2],
        shadow_map_display_command_buffers: [vk::CommandBuffer; 2],
        imgui_command_buffers: [vk::CommandBuffer; 2],
    ) -> Self {
        Self {
            command_buffers: CommandBuffers {
                shadow_map: shadow_map_command_buffers,
                depth_display: shadow_map_display_command_buffers,
                imgui: imgui_command_buffers,
            },
            semaphores: Semaphores {
                shadow_map_draw_finished: ctx.create_semaphore_vk(),
                depth_map_render_finished: ctx.create_semaphore_vk(),
                gui_finished: ctx.create_semaphore_vk(),
            },
            device: ctx.device.clone(),
        }
    }

    // TODO maybe in the future I can prerecord this as well, but currently I'm too bad at
    // lifetimes
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
        let depth_display_command_buffers = [self.command_buffers.depth_display[image_index]];
        let depth_display_finishied = [self.semaphores.depth_map_render_finished];
        let imgui_command_buffers = [self.command_buffers.imgui[image_index]]; // TODO double buffering
        let swapchain_present_wait = [self.semaphores.gui_finished];

        let submits = [
            vk::SubmitInfo::default()
                .wait_semaphores(&swapchain_acquire)
                .command_buffers(&shadow_map_command_buffers)
                .signal_semaphores(&shadow_map_finished)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::VERTEX_SHADER]),
            vk::SubmitInfo::default()
                .wait_semaphores(&shadow_map_finished)
                .command_buffers(&depth_display_command_buffers)
                .signal_semaphores(&depth_display_finishied)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]),
            vk::SubmitInfo::default()
                .wait_semaphores(&depth_display_finishied)
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
