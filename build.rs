use std::{
    env,
    fs::File,
    io::Write,
    path::Path,
    process::Command,
};

fn main() {
    // adapted from https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation
    let execute_external = |cmd: &mut Command| cmd.output().unwrap();
    let path_resources = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()) // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
        .join("tools");
    let str_env_var_out_dir = env::var("OUT_DIR").unwrap();
    let path_out_dir = Path::new(&str_env_var_out_dir); // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    let output_lessc = execute_external(
        Command::new("lessc")
            .arg(path_resources.join("css.less"))
    );
    assert!(output_lessc.status.success(), output_lessc);
    File::create(&path_out_dir.join("css.css"))
        .unwrap()
        .write_all(&output_lessc.stdout).unwrap();
    let output_inkscape = execute_external(
        Command::new("inkscape")
            .arg(path_resources.join("cards.svg"))
            .arg(format!("--export-png={}", path_out_dir.join("cards.png").to_str().unwrap()))
    );
    assert!(output_inkscape.status.success(), output_inkscape);
}

