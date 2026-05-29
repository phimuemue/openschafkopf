mod websocket;
pub mod gamephase;

fn main() -> Result<(), openschafkopf_util::SStringifiedError> {
    websocket::run(
        &clap::Command::new("openschafkopf-websocket")
            .about("Play in the browser")
            .arg(openschafkopf_shared_args::ruleset_arg())
            .arg(clap::Arg::new("with-bots")
                .long("with-bots")
                .help("Allow playing against bots")
            )
            .get_matches()
    )
}
