use ash::vk;

use crate::vkutils::shaders;

pub fn create_pipeline(
    vk: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    shader: &shaders::ShaderData,
    subgroup_size: u32,
) -> vk::Pipeline {
    let cs_module = shaders::create_shader_module(vk, shader.spv).unwrap();
    let cs_name = unsafe { std::ffi::CStr::from_ptr(shader.entry_point_name()) };

    // TODO probably could hide this behind checking for VK_EXT_subgroup_size_control support or VK >= 1.3
    let mut required = vk::PipelineShaderStageRequiredSubgroupSizeCreateInfo::default()
        .required_subgroup_size(subgroup_size);
    let stage = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(cs_module)
        .name(cs_name)
        .push_next(&mut required);

    let create_info = vk::ComputePipelineCreateInfo::default()
        .layout(pipeline_layout)
        .stage(stage);

    let pipeline = unsafe {
        vk.create_compute_pipelines(vk::PipelineCache::null(), &[create_info], None)
            .expect("Failed to create compute pipeline")[0]
    };

    unsafe {
        vk.destroy_shader_module(cs_module, None);
    }

    pipeline
}
