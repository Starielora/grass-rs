use ash::vk;

use crate::{gui, vkutils};

pub struct UiPass {
    pub command_buffers: [vk::CommandBuffer; 2],
}

impl UiPass {
    pub fn new(ctx: &mut vkutils::context::VulkanContext) -> Self {
        let command_buffers = ctx
            .graphics_command_pool
            .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 2);

        let command_buffers = [command_buffers[0], command_buffers[1]];

        Self { command_buffers }
    }

    pub fn record(
        &self,
        image_index: u32,
        ctx: &vkutils::context::VulkanContext,
        src_image: (vk::Image, vk::ImageView),
        resolve_image: (vk::Image, vk::ImageView),
        gui: &mut gui::Gui,
    ) {
        let device = ctx.device.clone();
        let command_buffer = self.command_buffers[image_index as usize];

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

        vkutils::image_barrier(
            &device,
            command_buffer,
            resolve_image.0,
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
            .image_view(src_image.1)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE)
            .resolve_mode(vk::ResolveModeFlags::AVERAGE)
            .resolve_image_view(resolve_image.1)
            .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                extent: ctx.swapchain.extent,
                offset: vk::Offset2D { x: 0, y: 0 },
            })
            .layer_count(1)
            .color_attachments(&color_attachments);

        let color_subresource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(vk::REMAINING_ARRAY_LAYERS);

        vkutils::image_barrier(
            &device,
            command_buffer,
            src_image.0,
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
            device.cmd_begin_rendering(command_buffer, &rendering_info);
        }

        gui.cmd_draw(command_buffer);

        unsafe { device.cmd_end_rendering(command_buffer) };

        vkutils::image_barrier(
            &device,
            command_buffer,
            resolve_image.0,
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

        unsafe { device.end_command_buffer(command_buffer) }
            .expect("Failed to end command buffer???");
    }
}
