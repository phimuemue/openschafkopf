use openschafkopf_util::*;

use std::{env, fs::File, io::Write, path::Path, process::Command};

fn main() {
    // adapted from https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation
    let execute_external = |cmd: &mut Command| {
        let output = debug_verify!(cmd.output()).unwrap();
        assert!(output.status.success(), "{:?}: {:?}", cmd, output);
        output
    };
    let path_resources = Path::new("tools");
    let str_env_var_out_dir = debug_verify!(env::var("OUT_DIR")).unwrap();
    let path_out_dir = Path::new(&str_env_var_out_dir); // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // TODO can we avoid lessc depencency?
    let path_css_in = path_resources.join("css.less");
    println!("cargo:rerun-if-changed={}", debug_verify!(path_css_in.to_str()).unwrap());
    debug_verify!(
        debug_verify!(
            File::create(&path_out_dir.join("css.css"))
        ).unwrap()
            .write_all(&execute_external(
                Command::new("lessc")
                    .arg(path_css_in)
            ).stdout)
    ).unwrap();
    // TODO can we avoid inkscape depencency?
    let path_svg_in = path_resources.join("cards.svg");
    println!("cargo:rerun-if-changed={}", debug_verify!(path_svg_in.to_str()).unwrap());
    execute_external(
        Command::new("inkscape")
            .arg(path_svg_in)
            .arg(format!("--export-filename={}", debug_verify!(path_out_dir.join("cards.png").to_str()).unwrap()))
    );
}

