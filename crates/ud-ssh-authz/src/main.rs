use anyhow::{anyhow, Context, Result};
use clap::Parser;
use reqwest::Client;
use std::{fs, path::PathBuf};
use tracing::info;
use ud_common::logging;

#[derive(Parser, Debug)]
#[command(version, about = "UnifiedDomain SSH authorization helper")]
struct Cli {
    /// Username requested by sshd
    #[arg(short, long)]
    user: String,
    /// Host fingerprint as seen by sshd
    #[arg(short = 'f', long)]
    host_fingerprint: String,
    /// Udd base URL
    #[arg(long, default_value = "https://localhost:8443")]
    server: String,
    /// Allow invalid TLS certificates (dev only)
    #[arg(long, default_value_t = false)]
    insecure: bool,
    /// Device certificate (PEM) for mTLS
    #[arg(long)]
    device_cert: PathBuf,
    /// Device private key (PEM) for mTLS
    #[arg(long)]
    device_key: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    logging::init_json_logging_stdout()?;
    let cli = Cli::parse();

    let identity = load_identity(&cli.device_cert, &cli.device_key)?;

    let mut builder = Client::builder()
        .danger_accept_invalid_certs(cli.insecure)
        .user_agent("ud-ssh-authz/0.1")
        .identity(identity);

    let client = builder.build().context("build http client")?;

    let url = format!("{}/v1/ssh/authorized_keys", cli.server);
    let resp = client
        .get(url)
        .query(&[("username", &cli.user), ("host_fingerprint", &cli.host_fingerprint)])
        .send()
        .await?
        .error_for_status()?;

    let body = resp.text().await?;
    info!(keys_len = body.len(), "authorized_keys fetched");
    print!("{}", body);
    Ok(())
}

fn load_identity(cert: &PathBuf, key: &PathBuf) -> Result<reqwest::Identity> {
    let mut pem = Vec::new();
    pem.extend(fs::read(cert).context("read device cert")?);
    pem.extend(fs::read(key).context("read device key")?);
    reqwest::Identity::from_pem(&pem).context("load identity from pem")
}
