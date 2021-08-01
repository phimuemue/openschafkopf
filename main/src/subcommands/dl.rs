use futures::prelude::*;
use num::BigUint;

use crate::util::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::App {
    clap::SubCommand::with_name(str_subcommand)
        .about("Download played games from Sauspiel")
        .arg(clap::Arg::with_name("START")
            .required(true)
            .index(1)
        )
        .arg(clap::Arg::with_name("END")
            .required(true)
            .index(2)
        )
        .arg(clap::Arg::with_name("user")
            .long("user")
            .takes_value(true)
            .required(true)
        )
        .arg(clap::Arg::with_name("pass")
            .long("pass")
            .takes_value(true)
            .required(true)
        )
        .arg(clap::Arg::with_name("jobs")
            .short("j")
            .long("jobs")
            .takes_value(true)
            .required(false)
        )
}

async fn internal_run(
    i_lo: BigUint,
    i_hi: BigUint,
    str_user: &str,
    str_pass: &str,
    n_jobs: usize,
) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let n_count = i_hi.clone() - i_lo.clone();
    let path_dst = std::path::PathBuf::from(&format!("downloaded/{}_{}_{}",
        i_lo,
        i_hi,
        chrono::Local::now().format("%Y%m%d%H%M%S"),
    ));
    unwrap!(std::fs::create_dir_all(&path_dst));
    futures::stream::iter(
        num::range(i_lo.clone(), i_hi).map(|i| {
            let (i_lo, path_dst, n_count, client) = (i_lo.clone(), path_dst.clone(), n_count.clone(), client.clone());
            let str_url = format!("https://www.sauspiel.de/spiele/{}", i);
            async move {
                println!("{}/{}: {}", i.clone() - i_lo.clone(), n_count, str_url);
                let mut file = unwrap!(async_std::fs::File::create(path_dst.join(&format!("{}.html", i))).await);
                loop {
                    match unwrap!(client.get(&str_url).basic_auth(str_user, Some(str_pass)).try_clone()).send().await {
                        Ok(response) => {
                            unwrap!(file.write_all(unwrap!(response.text().await).as_bytes()).await);
                            break;
                        },
                        Err(e) => {
                            println!("Error at {}: {:?}", i, e);
                        },
                    }
                }
            }
        })
    ).buffer_unordered(n_jobs).for_each(|_| async {()}).await;
    Ok(())
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let parse_bigint = |str_key| unwrap!(clapmatches.value_of(str_key)).parse();

    unwrap!(
        // adapted from https://docs.rs/tokio/1.9.0/tokio/attr.main.html#using-the-multi-thread-runtime
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
    )
        .block_on(internal_run(
            /*i_lo*/parse_bigint("START")?,
            /*i_hi*/parse_bigint("END")?,
            unwrap!(clapmatches.value_of("user")),
            unwrap!(clapmatches.value_of("pass")),
            unwrap!(clapmatches.value_of("jobs").unwrap_or("20").parse()),
        ))
}
