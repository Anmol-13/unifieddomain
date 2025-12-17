use anyhow::Result;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub fn init_json_logging<W>(writer: W) -> Result<()>
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer()
        .with_writer(writer)
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_timer(fmt::time::UtcTime::rfc_3339());

    let subscriber = Registry::default().with(env_filter).with(fmt_layer);
    subscriber.try_init().map_err(Into::into)
}

/// Initialize logging to stdout using structured JSON.
pub fn init_json_logging_stdout() -> Result<()> {
    init_json_logging(std::io::stdout)
}
