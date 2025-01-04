use crate::vkutils;
use crate::vkutils_new;

pub mod color;
pub mod shadow_map;

pub fn new(ctx: &vkutils::Context) -> vkutils_new::pipeline::Pipeline {
    let layout = ctx.bindless_descriptor_set.pipeline_layout;
    let pipeline = color::create(
        &ctx.device,
        &ctx.swapchain.extent,
        &layout,
        ctx.swapchain.surface_format.format,
        ctx.depth_image.format,
    );

    vkutils_new::pipeline::Pipeline::new(ctx.device.clone(), pipeline)
}

pub fn new_shadow_map(ctx: &vkutils::Context) -> vkutils_new::pipeline::Pipeline {
    let layout = ctx.bindless_descriptor_set.pipeline_layout;
    let pipeline = shadow_map::create(
        &ctx.device,
        &ctx.swapchain.extent,
        &layout,
        ctx.depth_image.format,
    );

    vkutils_new::pipeline::Pipeline::new(ctx.device.clone(), pipeline)
}
