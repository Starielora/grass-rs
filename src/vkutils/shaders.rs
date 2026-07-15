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

macro_rules! embed_shader_data {
    ($name:literal, $entry:ident) => {
        ShaderData {
            spv: include_bytes!(concat!(env!("OUT_DIR"), "/", $name)),
            entry_point_name: $entry,
        }
    };
}

pub static MAIN: &[u8] = b"main\0";

pub static GRID_VERT: ShaderData = embed_shader_data!("grid2.vert.spv", MAIN);
pub static GRID_FRAG: ShaderData = embed_shader_data!("grid2.frag.spv", MAIN);

pub static SKYBOX_VERT: ShaderData = embed_shader_data!("skybox2.vert.spv", MAIN);
pub static SKYBOX_FRAG: ShaderData = embed_shader_data!("skybox2.frag.spv", MAIN);

pub static MESHLET_MESH: ShaderData = embed_shader_data!("meshlet2.mesh.spv", MAIN);
pub static MESHLET_TASK: ShaderData = embed_shader_data!("meshlet2.task.spv", MAIN);
pub static MESHLET_FRAG: ShaderData = embed_shader_data!("meshlet2.frag.spv", MAIN);

pub static FRUSTUM_VERT: ShaderData = embed_shader_data!("frustum2.vert.spv", MAIN);
pub static FRUSTUM_FRAG: ShaderData = embed_shader_data!("frustum2.frag.spv", MAIN);

pub static BOUNDING_SPHERE_MESH: ShaderData = embed_shader_data!("bounding_sphere.mesh.spv", MAIN);
pub static BOUNDING_SPHERE_TASK_OBJECT: ShaderData =
    embed_shader_data!("bounding_sphere.task.object_variant.spv", MAIN);
pub static BOUNDING_SPHERE_TASK_MESHLET: ShaderData =
    embed_shader_data!("bounding_sphere.task.meshlet_variant.spv", MAIN);

pub static COMPUTE_VISIBLE_MESHLETS_COMP: ShaderData =
    embed_shader_data!("compute_visible_meshlets.comp.spv", MAIN);
pub static PREP_DRAW_MESH_TASKS_COMMAND: ShaderData =
    embed_shader_data!("prep_draw_mesh_tasks_command.comp.spv", MAIN);
