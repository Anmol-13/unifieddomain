use std::path::PathBuf;

use anyhow::Result;
use config::{Environment, File};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
    #[serde_as(as = "DisplayFromStr")]
    pub tls_cert_path: PathBuf,
    #[serde_as(as = "DisplayFromStr")]
    pub tls_key_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub admin_token: String,
    pub mtls_ca_cert_path: Option<String>,
    pub mtls_ca_key_path: Option<String>,
    #[serde(default = "default_true")]
    pub admin_token_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KerberosConfig {
    #[serde(default)]
    pub enabled: bool,
    pub realm: Option<String>,
    pub kadmin_path: Option<PathBuf>,
    pub keytab_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnifiedDomainConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    pub domain: Option<String>,
    pub kerberos: Option<KerberosConfig>,
}

fn default_true() -> bool {
    true
}

impl UnifiedDomainConfig {
    pub fn load() -> Result<Self> {
        let builder = config::Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(
                Environment::with_prefix("UD")
                    .separator("__")
                    .list_separator(","),
            );

        let cfg = builder.build()?;
        cfg.try_deserialize().map_err(Into::into)
    }
}
