#[cfg(feature = "logging")]
use std::sync::Once;

#[cfg(feature = "logging")]
use tracing_subscriber::{EnvFilter, fmt};

#[cfg(feature = "logging")]
static INIT_LOGGING: Once = Once::new();

pub fn init_cli_logging() {
    #[cfg(feature = "logging")]
    INIT_LOGGING.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("telfhash_rs=info"));
        let _ = fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .try_init();
    });
}

#[cfg(feature = "logging")]
pub(crate) use tracing::{debug, info, warn};

#[cfg(not(feature = "logging"))]
macro_rules! debug_log {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "logging"))]
macro_rules! info_log {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "logging"))]
macro_rules! warn_log {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "logging"))]
pub(crate) use debug_log as debug;
#[cfg(not(feature = "logging"))]
pub(crate) use info_log as info;
#[cfg(not(feature = "logging"))]
pub(crate) use warn_log as warn;
