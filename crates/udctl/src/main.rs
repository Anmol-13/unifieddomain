use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use ud_common::logging;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about = "UnifiedDomain control CLI")]
struct Cli {
    /// Udd base URL
    #[arg(long, default_value = "https://localhost:8443")]
    server: String,
    /// Admin bearer token (falls back to UD_ADMIN_TOKEN env)
    #[arg(long, env = "UD_ADMIN_TOKEN")]
    admin_token: Option<String>,
    /// Allow invalid TLS certificates (dev only)
    #[arg(long, default_value_t = false)]
    insecure: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Health,
    Login {
        username: String,
        password: String,
    },
    Bootstrap {
        admin_username: String,
        admin_password: String,
        #[arg(long)]
        display_name: Option<String>,
    },
    CreateUser {
        username: String,
        display_name: String,
        password: String,
        #[arg(long)]
        ssh_key: Option<String>,
    },
    CreateGroup { name: String },
    AddMember {
        group_id: Uuid,
        user_id: Uuid,
    },
    EnrollDevice {
        name: String,
        #[arg(long)]
        device_type: String,
        #[arg(long, default_values_t = Vec::<String>::new())]
        tags: Vec<String>,
        #[arg(long)]
        host_fingerprint: Option<String>,
        #[arg(long)]
        pubkey_fingerprint: Option<String>,
    },
    CreatePolicy {
        group_id: Uuid,
        host_tag: String,
        #[arg(long, default_value = "allow")]
        effect: String,
        #[arg(long)]
        description: Option<String>,
    },
    ListAudit {
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    KerberosUser {
        username: String,
        #[arg(long, default_value = "UD.INTERNAL")]
        realm: String,
    },
    KerberosHost {
        hostname: String,
        #[arg(long, default_value = "UD.INTERNAL")]
        realm: String,
        #[arg(long, default_value = "/etc/krb5.keytab")]
        keytab: String,
    },
    KerberosSyncUser {
        user_id: Uuid,
    },
    KerberosSyncDevice {
        device_id: Uuid,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    logging::init_json_logging_stdout()?;
    let cli = Cli::parse();
    let client = client(&cli)?;
    let token = cli.admin_token.clone();

    match cli.command {
        Commands::Health => {
            let url = format!("{}/health", cli.server);
            let resp = client.get(url).send().await?;
            let text = resp.text().await?;
            println!("{}", text);
        }
        Commands::Bootstrap {
            admin_username,
            admin_password,
            display_name,
        } => {
            let url = format!("{}/v1/bootstrap", cli.server);
            let body = BootstrapRequest {
                admin_username,
                admin_password,
                display_name,
            };
            let resp: BootstrapResponse = client
                .post(url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            println!("admin_token={}", resp.admin_token);
        }
        Commands::Login { username, password } => {
            let url = format!("{}/v1/login", cli.server);
            let body = LoginRequest { username, password };
            let resp: LoginResponse = client
                .post(url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            println!("{}", resp.result);
        }
        Commands::CreateUser {
            username,
            display_name,
            password,
            ssh_key,
        } => {
            let url = format!("{}/v1/users", cli.server);
            let keys = ssh_key.map(|k| vec![k]);
            let body = CreateUserRequest {
                username,
                display_name,
                password,
                ssh_public_keys: keys,
            };
            let resp: UserResponse = client
                .post(url)
                .apply_auth(&token)?
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            println!("created user {} id={} status={} keys={}", resp.username, resp.id, resp.status, resp.ssh_public_keys.len());
        }
        Commands::CreateGroup { name } => {
            let url = format!("{}/v1/groups", cli.server);
            let body = CreateGroupRequest { name };
            let resp: GroupResponse = client
                .post(url)
                .apply_auth(&token)?
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            println!("created group {} id={}", resp.name, resp.id);
        }
        Commands::AddMember { group_id, user_id } => {
            let url = format!("{}/v1/groups/{}/members", cli.server, group_id);
            let body = AddGroupMemberRequest { user_id };
            client
                .post(url)
                .apply_auth(&token)?
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
            println!("added user {} to group {}", user_id, group_id);
        }
        Commands::EnrollDevice {
            name,
            device_type,
            tags,
            host_fingerprint,
            pubkey_fingerprint,
        } => {
            let url = format!("{}/v1/devices/enroll", cli.server);
            let body = EnrollDeviceRequest {
                name,
                device_type,
                tags,
                host_fingerprint,
                pubkey_fingerprint,
            };
            let resp: EnrollDeviceResponse = client
                .post(url)
                .apply_auth(&token)?
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            println!("enrolled device id={} state={}", resp.device_id, resp.trust_state);
            println!("--- device certificate (PEM) ---\n{}", resp.device_cert_pem.trim());
            println!("--- device private key (PEM) ---\n{}", resp.device_key_pem.trim());
            if let Some(ca) = resp.ca_cert_pem.as_ref() {
                println!("--- ca certificate (PEM) ---\n{}", ca.trim());
            }
        }
        Commands::CreatePolicy {
            group_id,
            host_tag,
            effect,
            description,
        } => {
            let url = format!("{}/v1/policies", cli.server);
            let body = CreatePolicyRequest {
                group_id,
                host_tag,
                effect,
                description,
            };
            let resp: PolicyResponse = client
                .post(url)
                .apply_auth(&token)?
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            println!("policy {} host_tag={} effect={}", resp.id, resp.host_tag, resp.effect);
        }
        Commands::ListAudit { limit } => {
            let url = format!("{}/v1/audit?limit={}", cli.server, limit);
            let resp: Vec<AuditRecord> = client
                .get(url)
                .apply_auth(&token)?
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            for r in resp {
                println!("{} {} {} {} {}", r.id, r.created_at, r.actor_username.unwrap_or_default(), r.action, r.result);
            }
        }
        Commands::KerberosUser { username, realm } => {
            render_kerberos_user(&username, &realm);
        }
        Commands::KerberosHost {
            hostname,
            realm,
            keytab,
        } => {
            render_kerberos_host(&hostname, &realm, &keytab);
        }
        Commands::KerberosSyncUser { user_id } => {
            let url = format!("{}/v1/kerberos/users/{}/commands", cli.server, user_id);
            let resp: KerberosCommandResponse = client
                .post(url)
                .apply_auth(&token)?
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            for cmd in resp.commands {
                println!("{}", cmd);
            }
        }
        Commands::KerberosSyncDevice { device_id } => {
            let url = format!("{}/v1/kerberos/devices/{}/commands", cli.server, device_id);
            let resp: KerberosCommandResponse = client
                .post(url)
                .apply_auth(&token)?
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            for cmd in resp.commands {
                println!("{}", cmd);
            }
        }
    }

    Ok(())
}

fn client(cli: &Cli) -> Result<Client> {
    let mut builder = Client::builder().user_agent("udctl/0.1");
    if cli.insecure {
        builder = builder.danger_accept_invalid_certs(true);
    }
    builder.build().context("build http client")
}

trait AuthRequest {
    fn apply_auth(self, token: &Option<String>) -> Result<reqwest::RequestBuilder>;
}

impl AuthRequest for reqwest::RequestBuilder {
    fn apply_auth(self, token: &Option<String>) -> Result<reqwest::RequestBuilder> {
        if let Some(t) = token {
            Ok(self.bearer_auth(t))
        } else {
            Err(anyhow::anyhow!("admin token required"))
        }
    }
}

#[derive(Debug, Serialize)]
struct BootstrapRequest {
    admin_username: String,
    admin_password: String,
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct BootstrapResponse {
    admin_token: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    result: String,
}

#[derive(Debug, Serialize)]
struct CreateUserRequest {
    username: String,
    display_name: String,
    password: String,
    ssh_public_keys: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct UserResponse {
    id: Uuid,
    username: String,
    display_name: String,
    status: String,
    ssh_public_keys: Vec<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct CreateGroupRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GroupResponse {
    id: Uuid,
    name: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct AddGroupMemberRequest {
    user_id: Uuid,
}

#[derive(Debug, Serialize)]
struct EnrollDeviceRequest {
    name: String,
    device_type: String,
    tags: Vec<String>,
    host_fingerprint: Option<String>,
    pubkey_fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EnrollDeviceResponse {
    device_id: Uuid,
    trust_state: String,
    device_cert_pem: String,
    device_key_pem: String,
    ca_cert_pem: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KerberosCommandResponse {
    commands: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CreatePolicyRequest {
    group_id: Uuid,
    host_tag: String,
    effect: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PolicyResponse {
    id: Uuid,
    group_id: Uuid,
    host_tag: String,
    effect: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuditRecord {
    id: i64,
    created_at: String,
    actor_username: Option<String>,
    action: String,
    result: String,
}

fn render_kerberos_user(username: &str, realm: &str) {
    println!("# Run inside KDC host");
    println!("kadmin.local -q \"addprinc -randkey {}@{}\"", username, realm);
    println!("kadmin.local -q \"ktadd -k /keytabs/{}.keytab {}@{}\"", username, username, realm);
}

fn render_kerberos_host(hostname: &str, realm: &str, keytab: &str) {
    let principal = format!("host/{hostname}@{realm}");
    println!("# Run inside KDC host");
    println!("kadmin.local -q \"addprinc -randkey {principal}\"");
    println!("kadmin.local -q \"ktadd -k {keytab} {principal}\"");
}
