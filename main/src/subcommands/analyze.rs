use crate::util::*;
use crate::game_analysis::*;
use crate::sauspiel::*;
use std::io::Read;

pub fn analyze<
    'str_sauspiel_html_file,
    ItStrSauspielHtmlFile: Iterator<Item=&'str_sauspiel_html_file str>,
>(path_analysis: &std::path::Path, itstr_sauspiel_html_file: ItStrSauspielHtmlFile) -> Result<(), Error> {
    let mut vecanalyzeparams = Vec::new();
    for str_file_sauspiel_html in itstr_sauspiel_html_file {
        let itglobresult = glob::glob(str_file_sauspiel_html)?;
        for globresult in itglobresult {
            match globresult {
                Ok(path) => {
                    println!("Opening {:?}", path);
                    vecanalyzeparams.push(SAnalyzeParamsWithDesc{
                        str_description: path.to_string_lossy().into_owned(),
                        str_link: format!("file://{}", path.to_string_lossy()),
                        resanalyzeparams: analyze_html(&via_out_param_result(|str_html|
                            std::fs::File::open(&path)?.read_to_string(str_html)
                        )?.0),
                    });
                },
                Err(e) => {
                    println!("Error: {:?}. Trying to continue.", e);
                },
            }
        }
    }
    analyze_games(
        path_analysis,
        /*fn_link*/|str_description: &str| str_description.to_string(),
        vecanalyzeparams.into_iter(),
    )
}
