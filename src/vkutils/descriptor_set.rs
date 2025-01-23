pub mod bindless {

    use ash::vk;

    use crate::vkutils::push_constants;
    use crate::vkutils::vk_destroy;

    pub const CUBE_SAMPLER_BINDING: u32 = 0;
    pub const DEPTH_SAMPLER_BINDING: u32 = 1;

    const CUBE_SAMPLER_COUNT: u32 = 2;
    const DEPTH_SAMPLER_COUNT: u32 = 4;

    pub struct DescriptorSet {
        pool: vk::DescriptorPool,
        pub layout: vk::DescriptorSetLayout,
        pub handle: vk::DescriptorSet,
        pub pipeline_layout: vk::PipelineLayout,
        device: ash::Device,
    }

    impl DescriptorSet {
        pub fn new(device: ash::Device) -> Self {
            let descriptor_pool = create_descriptor_pool(&device);
            let descriptor_set_layout = create_descriptor_set_layout(&device);
            let descriptor_set =
                allocate_descriptor_set(descriptor_pool, descriptor_set_layout, &device);
            let pipeline_layout = create_pipeline_layout(&device, descriptor_set_layout);

            Self {
                pool: descriptor_pool,
                layout: descriptor_set_layout,
                handle: descriptor_set,
                pipeline_layout,
                device,
            }
        }

        pub fn update_sampler2d(
            &self,
            image_view: vk::ImageView,
            sampler: vk::Sampler,
            image_layout: vk::ImageLayout,
            dst_array_element: u32,
        ) {
            let descriptor_image_info = [vk::DescriptorImageInfo::default()
                .sampler(sampler)
                .image_view(image_view)
                .image_layout(image_layout)];

            let descriptor_writes = [vk::WriteDescriptorSet::default()
                .dst_set(self.handle)
                .dst_binding(DEPTH_SAMPLER_BINDING)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .dst_array_element(dst_array_element)
                .image_info(&descriptor_image_info)];

            let descriptor_copies = [];
            unsafe {
                self.device
                    .update_descriptor_sets(&descriptor_writes, &descriptor_copies)
            };
        }

        pub fn cmd_bind(
            &self,
            command_buffer: vk::CommandBuffer,
            bind_point: vk::PipelineBindPoint,
        ) {
            unsafe {
                self.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    bind_point,
                    self.pipeline_layout,
                    0,
                    &[self.handle],
                    &[],
                );
            }
        }
    }

    impl vk_destroy::VkDestroy for DescriptorSet {
        fn vk_destroy(&self) {
            unsafe {
                self.device
                    .destroy_pipeline_layout(self.pipeline_layout, None);
                self.device.destroy_descriptor_set_layout(self.layout, None);
                self.device.destroy_descriptor_pool(self.pool, None);
            }
        }
    }

    fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
        let descriptor_pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(CUBE_SAMPLER_COUNT),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(DEPTH_SAMPLER_COUNT),
        ];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .max_sets(1)
            .pool_sizes(&descriptor_pool_sizes);

        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&descriptor_pool_create_info, None)
                .expect("Failed to create descriptor pool.")
        };

        descriptor_pool
    }

    fn create_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(CUBE_SAMPLER_BINDING)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(CUBE_SAMPLER_COUNT)
                .stage_flags(vk::ShaderStageFlags::ALL),
            vk::DescriptorSetLayoutBinding::default()
                .binding(DEPTH_SAMPLER_BINDING)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(DEPTH_SAMPLER_COUNT)
                .stage_flags(vk::ShaderStageFlags::ALL),
        ];

        let binding_flags = [
            vk::DescriptorBindingFlags::PARTIALLY_BOUND
                | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            vk::DescriptorBindingFlags::PARTIALLY_BOUND
                | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
        ];
        let mut binding_flags_create_info =
            vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&binding_flags);

        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::default()
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .bindings(&bindings)
            .push_next(&mut binding_flags_create_info);

        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                .expect("Failed to create descriptor set layout")
        };

        descriptor_set_layout
    }

    fn allocate_descriptor_set(
        pool: vk::DescriptorPool,
        layout: vk::DescriptorSetLayout,
        device: &ash::Device,
    ) -> vk::DescriptorSet {
        let set_layouts = [layout];

        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&set_layouts);

        let descriptor_set = unsafe {
            device
                .allocate_descriptor_sets(&descriptor_set_allocate_info)
                .expect("Failed to allocate descriptor set")
        };

        descriptor_set[0]
    }

    fn create_pipeline_layout(
        device: &ash::Device,
        set_layout: vk::DescriptorSetLayout,
    ) -> vk::PipelineLayout {
        let set_layouts = [set_layout];
        let push_constants_range = push_constants::get_range();
        let create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constants_range);
        unsafe {
            device
                .create_pipeline_layout(&create_info, None)
                .expect("Failed to create pipeline layout")
        }
    }
}
