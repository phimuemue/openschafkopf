use itertools::Itertools;
use crate::util::if_dbg_else;
pub use log::{debug, error, info, trace, warn};

#[derive(Debug)]
pub enum VInitLoggingError {
    HomeDir,
    FernLogFile,
    FernSetLoggerError,
}
impl std::fmt::Display for VInitLoggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for VInitLoggingError {}

pub fn init_logging(str_log_basename: &str) -> Result<(), VInitLoggingError> {
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
            dirs::home_dir()
                .ok_or(VInitLoggingError::HomeDir)?
                .join(format!("{str_log_basename}.log"))
        }).map_err(|_| VInitLoggingError::FernLogFile)?)
        .apply().map_err(|_| VInitLoggingError::FernSetLoggerError)?;
    let fn_panic_handler_original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panicinfo| {
        error!("panic: {}", panicinfo);
        fn_panic_handler_original(panicinfo)
    }));
    info!(
        "Started: {}",
        std::env::args().format_with(/*sep*/ " ", |str_arg, formatter| {
            formatter(&format_args!("\"{}\"", str_arg))
        }),
    );
    Ok(())
}
