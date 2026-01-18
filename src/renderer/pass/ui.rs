use ash::vk;

use crate::{gui, vkutils};

pub struct UiPass {
    pub command_buffers: Vec<vk::CommandBuffer>,

    timestamp_query: vkutils::timestamp_query::TimestampQuery,
}

impl UiPass {
    pub fn new(ctx: &mut vkutils::context::VulkanContext) -> Self {
        let command_buffers = ctx.graphics_command_pool.allocate_command_buffers(
            vk::CommandBufferLevel::PRIMARY,
            ctx.swapchain.images.len().try_into().unwrap(),
        );

        let timestamp_query = vkutils::timestamp_query::TimestampQuery::new(&ctx, 2);

        Self {
            command_buffers,
            timestamp_query,
        }
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

        self.timestamp_query.reset(command_buffer);
        self.timestamp_query
            .cmd_write(0, vk::PipelineStageFlags::TOP_OF_PIPE, command_buffer);

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

        self.timestamp_query
            .cmd_write(1, vk::PipelineStageFlags::BOTTOM_OF_PIPE, command_buffer);

        unsafe { device.cmd_end_rendering(command_buffer) };

        unsafe { device.end_command_buffer(command_buffer) }
            .expect("Failed to end command buffer???");
    }

    pub fn get_pass_total_time(&mut self) -> std::time::Duration {
        let timestamp_period = self.timestamp_query.timestamp_period();
        let query_results = self.timestamp_query.get_results();
        // hope f32 to u64 won't blow up
        let t1_ns = query_results.iter().nth(0).unwrap() * timestamp_period as u64;
        let t2_ns = query_results.iter().nth(1).unwrap() * timestamp_period as u64;

        std::time::Duration::from_nanos(t2_ns - t1_ns)
    }
}
