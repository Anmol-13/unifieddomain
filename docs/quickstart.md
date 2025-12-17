- Example audit log entries (JSON):
```
{"timestamp":"2024-01-01T00:00:00Z","request_id":"...","action":"device_trust_update","device_id":"...","result":"allow","reason":"trusted"}
{"timestamp":"2024-01-01T00:01:00Z","request_id":"...","action":"ssh_authorize","actor_username":"alice","device_id":"...","result":"allow","reason":"policy allow"}
{"timestamp":"2024-01-01T00:02:00Z","request_id":"...","action":"ssh_authorize","actor_username":"bob","device_id":"...","result":"deny","reason":"no matching allow"}
```
# Quickstart

Prerequisites: Docker, Docker Compose, Rust stable, OpenSSL or `step` for test certificates.

1) TLS materials (dev self-signed)
```bash
mkdir -p deploy/certs
openssl req -x509 -nodes -newkey rsa:4096 -keyout deploy/certs/udd-key.pem -out deploy/certs/udd.pem -days 365 -subj "/CN=udd"
cp deploy/certs/udd.pem deploy/certs/ca.pem
cp deploy/certs/udd-key.pem deploy/certs/ca-key.pem
chmod 600 deploy/certs/udd-key.pem
```

2) Config
```bash
cp config/default.toml config/local.toml
# edit config/local.toml: set a strong auth.admin_token and adjust DB URL if needed
```

3) Start stack (Postgres, KDC, udd build)
```bash
docker compose -f deploy/docker-compose.yml up --build
# or run udd locally:
cargo run -p udd
```

4) Health check
```bash
curl -k https://localhost:8443/health
```

5) Bootstrap admin (one-time)
```bash
cargo run -p udctl -- --server https://localhost:8443 bootstrap \
   --display-name "UD Admin" admin supersecret
# note the admin_token returned; export it for later commands
export UD_ADMIN_TOKEN="<value from response or config>"
```

6) Create user/group/policy and enroll a server
```bash
# create user
cargo run -p udctl -- --server https://localhost:8443 --admin-token "$UD_ADMIN_TOKEN" create-user \
   alice "Alice Ops" "P@ssw0rd" --ssh-key "ssh-ed25519 AAA... alice@host"

# create group and add user
cargo run -p udctl -- --server https://localhost:8443 --admin-token "$UD_ADMIN_TOKEN" create-group ops-admins
cargo run -p udctl -- --server https://localhost:8443 --admin-token "$UD_ADMIN_TOKEN" add-member \
   --group-id <ops-admins-uuid> --user-id <alice-uuid>

# enroll device tagged server (returns device cert/key)
cargo run -p udctl -- --server https://localhost:8443 --admin-token "$UD_ADMIN_TOKEN" enroll-device \
   --device-type server --tags server --host-fingerprint SHA256:demoFP --name demo-server

# mark device trusted (required before ssh authz succeeds)
curl -k -X POST "https://localhost:8443/v1/devices/<device-uuid>/trust" \
  -H "Authorization: Bearer $UD_ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"trust_state":"trusted"}'

# generate Kerberos commands (run inside KDC container)
cargo run -p udctl -- --server https://localhost:8443 --admin-token "$UD_ADMIN_TOKEN" kerberos-sync-user --user-id <alice-uuid>
cargo run -p udctl -- --server https://localhost:8443 --admin-token "$UD_ADMIN_TOKEN" kerberos-sync-device --device-id <device-uuid>

# (optional) enforce mTLS-only admin by disabling tokens after bootstrap
sed -i 's/admin_token_enabled = true/admin_token_enabled = false/' config/local.toml
systemctl restart udd

# automated Kerberos sync (compose)
# enable kerberos in config/local.toml or via env:
#   UD__KERBEROS__ENABLED=true
# udd container has kadmin.local installed and will provision principals/keytabs
# keytabs are written to /var/lib/udd/keytabs (mounted via uddkeytabs volume)

# allow ops-admins on hosts tagged server
cargo run -p udctl -- --server https://localhost:8443 --admin-token "$UD_ADMIN_TOKEN" create-policy \
   --group-id <ops-admins-uuid> --host-tag server --effect allow
```

7) SSH helper (AuthorizedKeysCommand example)
```bash
# on the target host, prefer mTLS using issued device cert
cargo run -p ud-ssh-authz -- \
   --server https://udd:8443 \
   --user alice \
   --host-fingerprint SHA256:demoFP \
   --cert /etc/ud/device.pem \
   --key /etc/ud/device-key.pem \
   --ca /etc/ud/ca.pem
```
Configure sshd:
```
AuthorizedKeysCommand /usr/local/bin/ud-ssh-authz --server https://udd:8443 --host-fingerprint %f --user %u
AuthorizedKeysCommandUser root
```

8) Audit log
```bash
cargo run -p udctl -- --server https://localhost:8443 --admin-token "$UD_ADMIN_TOKEN" list-audit --limit 20
```

9) Kerberos (partial automation)
- Start the KDC service from docker-compose (included).
- Use provisioning helper to print `kadmin.local` commands:
```bash
deploy/kerberos-provision.sh user alice
deploy/kerberos-provision.sh host demo-server.UD.INTERNAL /etc/krb5.keytab
```
- Run the printed commands inside the KDC container: `docker compose exec krb5-kdc bash` then paste.
- Copy keytabs to clients/hosts securely.
- Enable GSSAPI in sshd on the host:
```
GSSAPIAuthentication yes
GSSAPICleanupCredentials yes
```
- Users obtain tickets with `kinit alice@UD.INTERNAL` and connect with `ssh -o GSSAPIAuthentication=yes demo-server`.

Notes
- `udd` auto-runs migrations at startup.
- Admin auth can be bearer token or mTLS admin cert; hosts/helpers must use mTLS.
- Device cert issuance requires `auth.mtls_ca_cert_path` and `auth.mtls_ca_key_path` to be set.
- Kerberos provisioning is manual via helper script; keep KDC volumes protected.
