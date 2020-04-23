use openschafkopf_util::*;

use std::{
    env,
    fs::File,
    io::Write,
    path::Path,
    process::Command,
};

fn main() {
    // adapted from https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation
    let execute_external = |cmd: &mut Command| {
        let output = debug_verify!(cmd.output()).unwrap();
        assert!(output.status.success(), output);
        output
    };
    let path_resources = Path::new(&debug_verify!(env::var("CARGO_MANIFEST_DIR")).unwrap()) // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
        .join("tools");
    let str_env_var_out_dir = debug_verify!(env::var("OUT_DIR")).unwrap();
    let path_out_dir = Path::new(&str_env_var_out_dir); // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // TODO can we avoid lessc depencency?
    debug_verify!(
        debug_verify!(
            File::create(&path_out_dir.join("css.css"))
        ).unwrap()
            .write_all(&execute_external(
                Command::new("lessc")
                    .arg(path_resources.join("css.less"))
            ).stdout)
    ).unwrap();
    // TODO can we avoid inkscape depencency?
    execute_external(
        Command::new("inkscape")
            .arg(path_resources.join("cards.svg"))
            .arg(format!("--export-png={}", debug_verify!(path_out_dir.join("cards.png").to_str()).unwrap()))
    );
}

