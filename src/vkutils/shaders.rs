use ash::vk;
use std::io::Cursor;

pub fn create_shader_module(
    vk: &ash::Device,
    spv: &[u8],
) -> Result<vk::ShaderModule, Box<dyn std::error::Error>> {
    let code = ash::util::read_spv(&mut Cursor::new(spv))?;
    let shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&code);
    let shader_module = unsafe { vk.create_shader_module(&shader_module_create_info, None) }?;

    Ok(shader_module)
}

pub struct ShaderData {
    pub spv: &'static [u8],
    pub entry_point_name: &'static [u8],
}

impl ShaderData {
    pub fn entry_point_name(&self) -> *const i8 {
        self.entry_point_name.as_ptr() as *const i8
    }
}

macro_rules! shader_data {
    ($name:literal, $entry:ident) => {
        ShaderData {
            spv: include_bytes!(concat!(env!("OUT_DIR"), "/", $name)),
            entry_point_name: $entry,
        }
    };
}

pub static MAIN: &[u8] = b"main\0";

pub static GRID_VERT: ShaderData = shader_data!("grid2.vert.spv", MAIN);
pub static GRID_FRAG: ShaderData = shader_data!("grid2.frag.spv", MAIN);
