macro_rules! embed_textures {
    ([ $($path:literal),* $(,)? ]) => {
            [
                $(
                    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
                ),*
            ]
    };
}

pub static SKYBOX1: [&'static [u8]; 6] = embed_textures!([
    "assets/skybox/daylight/Daylight Box_Right.png",
    "assets/skybox/daylight/Daylight Box_Left.png",
    "assets/skybox/daylight/Daylight Box_Top.png",
    "assets/skybox/daylight/Daylight Box_Bottom.png",
    "assets/skybox/daylight/Daylight Box_Front.png",
    "assets/skybox/daylight/Daylight Box_Back.png",
]);

pub static SKYBOX2: [&'static [u8]; 6] = embed_textures!([
    "assets/skybox/learnopengl/right.png",
    "assets/skybox/learnopengl/left.png",
    "assets/skybox/learnopengl/top.png",
    "assets/skybox/learnopengl/bottom.png",
    "assets/skybox/learnopengl/front.png",
    "assets/skybox/learnopengl/back.png",
]);
