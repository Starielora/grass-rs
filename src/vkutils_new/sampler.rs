use ash::vk;

use super::vk_destroy;

pub struct Sampler {
    pub handle: vk::Sampler,
    device: ash::Device,
}

impl Sampler {
    pub fn new(device: ash::Device) -> Self {
        let create_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .mip_lod_bias(0.0)
            .max_anisotropy(1.0)
            .min_lod(0.0)
            .max_lod(1.0)
            .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE);

        let sampler = unsafe {
            device
                .create_sampler(&create_info, None)
                .expect("Failed to create sampler.")
        };

        Self {
            handle: sampler,
            device,
        }
    }
}

impl vk_destroy::VkDestroy for Sampler {
    fn vk_destroy(&self) {
        unsafe {
            self.device.destroy_sampler(self.handle, None);
        }
    }
}
