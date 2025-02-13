use std::{
    env::{self},
    fs,
    path::Path,
    process::{exit, Command},
};

fn main() {
    println!("cargo:warning={}", "Building shaders.");

    let out_dir = env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .display()
        .to_string();

    // gather shaders
    let shaders = std::fs::read_dir("shaders")
        .expect("Could not read dir")
        .filter(|file| {
            let f = file.as_ref().unwrap();
            f.path().is_file()
                && !f
                    .file_name()
                    .to_str()
                    .expect("Could not convert file_name OsString to string slice")
                    .starts_with("descriptor_set")
        })
        .map(|file| file.as_ref().unwrap().path())
        .collect::<Vec<_>>();

    shaders.iter().for_each(|file| {
        let input = std::env::current_dir().unwrap().join(file);
        let mut output = file.clone();
        output.set_extension(
            output
                .extension()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap()
                + ".spv",
        );

        let output = output.file_name().expect("No file name");

        println!("Executing glslc for {}", file.display());

        let glslc_status = Command::new("glslc")
            .arg("--target-env=vulkan1.3")
            .arg(format!("{}", input.display()))
            .arg("-o")
            .arg(format!(
                "{}",
                output.to_str().expect("Could not convert OsStr to str")
            ))
            .current_dir(env::var("OUT_DIR").expect("No OUT_DIR env var."))
            .status()
            .expect(format!("Failed for shader {}", file.display()).as_str());

        let code = glslc_status.code().unwrap();
        if code != 0 {
            exit(code);
        }

        let target_file_dir = Path::new(&target_dir).join(output);
        let from = Path::new(&out_dir).join(output);
        println!(
            "Copying shader {} to target dir {}.",
            output.to_str().unwrap(),
            target_file_dir.display()
        );
        let copy_result = fs::copy(from, &target_file_dir);
        match copy_result {
            Ok(_) => (),
            Err(_) => exit(1),
        }
    });
}
