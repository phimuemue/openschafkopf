use openschafkopf_util::*;
use image::GenericImageView;
use as_num::*;

use std::{env, fs::File, io::Write, path::Path, process::Command};

fn main() {
    // adapted from https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation
    fn declare_input_file<P: AsRef<Path>>(path_in: P) -> P {
        println!("cargo:rerun-if-changed={}", unwrap!(path_in.as_ref().to_str()));
        path_in
    }
    let execute_external = |path_in: &Path, cmd: &mut Command| {
        declare_input_file(path_in);
        let output = unwrap!(cmd.output());
        assert!(output.status.success(), "{:?}: {:?}", cmd, output);
        output
    };
    let path_resources = Path::new("tools");
    let str_env_var_out_dir = unwrap!(env::var("OUT_DIR"));
    let path_out_dir = Path::new(&str_env_var_out_dir); // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    unwrap!(std::fs::create_dir_all(&path_out_dir));
    // TODO can we avoid lessc depencency?
    let path_css_in = path_resources.join("css.less");
    unwrap!(
        unwrap!(
            File::create(&path_out_dir.join("css.css"))
        )
            .write_all(&execute_external(
                &path_css_in,
                Command::new("lessc")
                    .arg(&path_css_in)
            ).stdout)
    );
    // SVG rendering adapted from https://github.com/RazrFalcon/resvg/blob/master/examples/minimal.rs
    let svgtree = usvg::Tree::from_data(
        &unwrap!(std::fs::read(declare_input_file(path_resources.join("cards.svg")))),
        &usvg::Options::default().to_ref()
    ).unwrap(/*TODO should implement Debug*/);
    let screensize = svgtree.svg_node().size.to_screen_size();
    let export_cards_png = |path_cards_png, n_factor: u32| {
        let mut pixmap = unwrap!(tiny_skia::Pixmap::new(
            screensize.width() * n_factor,
            screensize.height() * n_factor,
        ));
        unwrap!(resvg::render(
            &svgtree,
            usvg::FitTo::Original,
            tiny_skia::Transform::from_scale(n_factor.as_num::<f32>(), n_factor.as_num::<f32>()),
            pixmap.as_mut()
        ));
        unwrap!(pixmap.save_png(path_cards_png));
    };
    export_cards_png(path_out_dir.join("cards.png"), 1);
    let path_svg_3dpi = path_out_dir.join("cards_3dpi.png");
    export_cards_png(path_svg_3dpi.clone(), 3);
    let img = unwrap!(image::open(path_svg_3dpi));
    let (n_width, n_height) = img.dimensions();
    let str_efarbe = "EGHS";
    let str_eschlag = "AZKOU987";
    assert_eq!(n_width % str_eschlag.len().as_num::<u32>(), 0);
    let n_width_card = n_width / str_eschlag.len().as_num::<u32>();
    assert_eq!(n_height % str_efarbe.len().as_num::<u32>(), 0);
    let n_height_card = n_height / str_efarbe.len().as_num::<u32>();
    for (i_efarbe, ch_efarbe) in str_efarbe.chars().enumerate() {
        for (i_eschlag, ch_eschlag) in str_eschlag.chars().enumerate() {
            unwrap!(
                img.view(
                    /*x*/n_width_card * i_eschlag.as_num::<u32>(),
                    /*y*/n_height_card * i_efarbe.as_num::<u32>(),
                    n_width_card,
                    n_height_card,
                )
                    .to_image()
                    .save({
                        let path_img = path_resources // TODO allowed to write into this directory?
                            .join("site")
                            .join("img");
                        unwrap!(std::fs::create_dir_all(&path_img));
                        path_img.join(format!("{}{}.png", ch_efarbe, ch_eschlag))
                    })
            );
        }
    }
}

