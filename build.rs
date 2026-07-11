use std::{
    env::{self},
    fs::{self},
    path::{Path, PathBuf},
    process::Command,
};

const STAGE_EXTENSIONS: &[&str] = &["vert", "frag", "comp", "geom", "mesh", "task"];

struct ShaderVariant {
    suffix: &'static str,
    defines: &'static [&'static str],
}

const DEFAULT_VARIANT: &[ShaderVariant] = &[ShaderVariant {
    suffix: "",
    defines: &[],
}];

fn shader_variants(file_name: &str) -> &'static [ShaderVariant] {
    match file_name {
        "bounding_sphere.task" => &[
            ShaderVariant {
                suffix: "object_variant",
                defines: &["MESHLET_BOUNDING_SPHERE=0"],
            },
            ShaderVariant {
                suffix: "meshlet_variant",
                defines: &["MESHLET_BOUNDING_SPHERE=1"],
            },
        ],
        _ => DEFAULT_VARIANT,
    }
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let shader_dir = Path::new("shaders");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", shader_dir.display());

    for entry in fs::read_dir(shader_dir).expect("Failed to read shaders/") {
        let src = entry.unwrap().path();
        let is_stage_shader_file = src
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| STAGE_EXTENSIONS.contains(&ext));

        if !is_stage_shader_file {
            continue;
        }

        let name = src.file_name().unwrap().to_str().unwrap();

        for variant in shader_variants(name) {
            let out_name = if variant.suffix.is_empty() {
                format!("{name}.spv")
            } else {
                format!("{name}.{}.spv", variant.suffix)
            };

            let spv = out_dir.join(&out_name);
            let dep = out_dir.join(format!("{out_name}.d"));

            if up_to_date(&spv, &dep) {
                continue;
            }

            println!("cargo:warning=compiling {out_name}");

            let mut cmd = Command::new("glslc");
            cmd.args(["-O", "-g", "--target-env=vulkan1.3", "-MD"])
                .arg("-MF")
                .arg(&dep);
            for def in variant.defines {
                cmd.arg(format!("-D{def}"));
            }
            let status = cmd
                .arg(&src)
                .arg("-o")
                .arg(&spv)
                .status()
                .unwrap_or_else(|e| panic!("glslc failed to start for {name}: {e}"));

            assert!(status.success(), "glslc failed for {name}");
        }
    }
}

fn up_to_date(spv: &Path, dep: &Path) -> bool {
    let Ok(spv_time) = fs::metadata(spv).and_then(|metadata| metadata.modified()) else {
        return false; // never compiled
    };

    let Ok(text) = fs::read_to_string(dep) else {
        return false; // no dep info, regenerate
    };

    let Some((_, deps)) = text.split_once(":") else {
        return false;
    };

    deps.split_whitespace().all(|dep| {
        fs::metadata(dep)
            .and_then(|metadata| metadata.modified())
            .map(|time| time <= spv_time)
            .unwrap_or(false)
    })
}
