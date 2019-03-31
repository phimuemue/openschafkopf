use std::{
    env,
    fs::File,
    io::Write,
    path::Path,
    process::Command,
};

fn main() {
    // adapted from https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation
    let output_lessc = Command::new("lessc")
        .arg(
            Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()) // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
                .join("tools")
                .join("css.less")
        )
        .output()
        .unwrap();
    assert!(output_lessc.status.success(), output_lessc);
    File::create(
        &Path::new(&env::var("OUT_DIR").unwrap()) // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
            .join("css.css")
    )
        .unwrap()
        .write_all(&output_lessc.stdout).unwrap();
}

