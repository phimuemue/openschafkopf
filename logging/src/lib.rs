extern crate fern;
extern crate dirs;
extern crate itertools;
extern crate log;
extern crate chrono;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate openschafkopf_util;
use crate::itertools::Itertools;

pub use log::{trace, debug, info, warn, error};

pub fn init_logging() -> Result<(), failure::Error> {
    fern::Dispatch::new()
        .format(|formatcallback, fmtarguments_msg, logrecord| {
            formatcallback.finish(format_args!(
                "[{} {}({:?}) {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                logrecord.target(), // TODO needed? desired?
                std::thread::current().id(),
                logrecord.level(),
                fmtarguments_msg,
            ))
        })
        .level(if_dbg_else!({log::LevelFilter::Trace}{log::LevelFilter::Info}))
        .chain(fern::log_file({
            let mut path_log = std::env::current_exe()?;
            if !path_log.set_extension("log") {
                bail!("set_extension error");
            }
            dirs::home_dir().ok_or_else(||format_err!("home_dir error"))?
                .join(path_log
                    .file_name()
                    .ok_or_else(||format_err!("file_name error"))?
                )
        })?)
        .apply()?;
    let fn_panic_handler_original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panicinfo| {
        error!("panic: {}", panicinfo);
        fn_panic_handler_original(panicinfo)
    }));
    info!("Started: {}", std::env::args()
        .format_with(
            /*sep*/" ",
            |str_arg, formatter| {
                formatter(&format_args!("\"{}\"", str_arg))
            },
        )
    );
    Ok(())
}
