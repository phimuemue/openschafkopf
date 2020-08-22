use openschafkopf_util::*;
use image;
use image::GenericImageView;
use as_num::*;

use std::{env, fs::File, io::Write, path::Path, process::Command};

fn main() {
    // adapted from https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation
    let execute_external = |path_in: &Path, cmd: &mut Command| {
        println!("cargo:rerun-if-changed={}", debug_verify!(path_in.to_str()).unwrap());
        let output = debug_verify!(cmd.output()).unwrap();
        assert!(output.status.success(), "{:?}: {:?}", cmd, output);
        output
    };
    let path_resources = Path::new("tools");
    let str_env_var_out_dir = debug_verify!(env::var("OUT_DIR")).unwrap();
    let path_out_dir = Path::new(&str_env_var_out_dir); // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    // TODO can we avoid lessc depencency?
    let path_css_in = path_resources.join("css.less");
    debug_verify!(
        debug_verify!(
            File::create(&path_out_dir.join("css.css"))
        ).unwrap()
            .write_all(&execute_external(
                &path_css_in,
                Command::new("lessc")
                    .arg(&path_css_in)
            ).stdout)
    ).unwrap();
    // TODO can we avoid inkscape depencency?
    let path_svg_in = path_resources.join("cards.svg");
    execute_external(
        &path_svg_in,
        Command::new("inkscape")
            .arg(&path_svg_in)
            .arg(format!("--export-filename={}", debug_verify!(path_out_dir.join("cards.png").to_str()).unwrap()))
    );
    let path_svg_3dpi = path_out_dir.join("cards_3dpi.png");
    execute_external(
        &path_svg_in,
        Command::new("inkscape")
            .arg(&path_svg_in)
            .arg(format!("--export-filename={}", debug_verify!(path_svg_3dpi.to_str()).unwrap()))
            .arg(format!("--export-dpi={}", 3*/*default DPI*/96))
    );
    let img = /*TODO debug_verify!*/image::open(path_svg_3dpi).unwrap();
    let (n_width, n_height) = img.dimensions();
    let str_efarbe = "EGHS";
    let str_eschlag = "AZKOU987";
    assert_eq!(n_width % str_eschlag.len().as_num::<u32>(), 0);
    let n_width_card = n_width / str_eschlag.len().as_num::<u32>();
    assert_eq!(n_height % str_efarbe.len().as_num::<u32>(), 0);
    let n_height_card = n_height / str_efarbe.len().as_num::<u32>();
    for (i_efarbe, ch_efarbe) in str_efarbe.chars().enumerate() {
        for (i_eschlag, ch_eschlag) in str_eschlag.chars().enumerate() {
            debug_verify!(
                img.view(
                    /*x*/n_width_card * i_eschlag.as_num::<u32>(),
                    /*y*/n_height_card * i_efarbe.as_num::<u32>(),
                    n_width_card,
                    n_height_card,
                )
                    .to_image()
                    .save(
                        path_resources // TODO allowed to write into this directory?
                            .join("site")
                            .join("img")
                            .join(format!("{}{}.png", ch_efarbe, ch_eschlag))
                    )
            ).unwrap();
        }
    }
}

