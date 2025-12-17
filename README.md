# UnifiedDomain v0.1 (Preview) — **Do NOT Use in Production**

> **Status:** Experimental preview. Security-sensitive software. **Do not deploy to production.** Expect breaking changes. No support or warranty. Use only in isolated lab environments you fully control.

## Purpose and Scope
UnifiedDomain is a Linux-first, offline-capable identity and trust control plane. It is **not** a drop-in AD/LDAP replacement. v0.1 deliberately limits scope to a minimal, auditable path:
- Core identity store: users, groups, devices (PostgreSQL + sqlx; migrations included).
- Device enrollment: admin-driven issuance of device X.509 certs from an internal CA.
- SSH authorization: OpenSSH `AuthorizedKeysCommand` helper (`ud-ssh-authz`) fetches allowed keys per (user, host) with policy enforced in `udd`.
- Authentication: password auth for bootstrap; optional Kerberos (MIT krb5) for authentication only—authorization remains in UnifiedDomain.
- Audit logging: every authz decision and trust change is logged with reason.
- CLI (`udctl`) for lifecycle management and Kerberos sync helpers.

## Non-Goals (explicitly out of scope)
- Production readiness: no HA, no hardening, no support, no SLAs.
- AD/LDAP compatibility; Windows login; GPO; SSO/OIDC/SAML; cross-realm/federation.
- macOS login, Wi-Fi/VPN integration, web UI.

## Security Posture (Preview)
- TLS everywhere; mTLS required for host helper/device flows. Admin token exists for bootstrap; can be disabled post-setup (`auth.admin_token_enabled = false`).
- Device trust is explicit: enrolled → trusted → revoked. SSH authz requires a trusted device cert and matching host fingerprint.
- Kerberos automation (optional): `kadmin.local` invoked by `udd` to create principals/keytabs. Keytabs are stored locally; **you** must distribute and protect them. Not suitable for production.
- Secrets (admin tokens, CA keys, keytabs) must be protected with strict file permissions. No secret management is provided. No CRLs/OCSP; revocation is via DB trust_state enforced at authz time.

## Components
- `crates/udd`: HTTP+JSON API daemon with TLS/mTLS, SQLx/PostgreSQL, audit logging, SSH policy evaluation.
- `crates/udctl`: CLI for bootstrap and lifecycle ops (users, groups, devices, policies, Kerberos sync).
- `crates/ud-ssh-authz`: OpenSSH `AuthorizedKeysCommand` helper; uses mTLS with device cert to fetch authorized keys.
- `crates/ud-common`: shared config/logging/types.
- `deploy/docker-compose.yml`: Postgres + krb5-kdc + udd for local/demo.
- `deploy/udd.Dockerfile`: builds release udd with krb5 admin tools installed for automation.
- `deploy/systemd/*.service`: sample systemd units for udd and ud-ssh-authz.
- `deploy/kerberos-sync.sh`: helper to run kadmin commands inside the KDC container (for inspection/manual use).
- `docs/`: quickstart, architecture snapshot, threat model.

## Data Model (simplified)
- User: id (UUID), username (unique), display_name, status (active/disabled), password_hash (argon2), ssh_public_keys, created_at.
- Group: id, name (unique), created_at.
- GroupMembership: user_id, group_id.
- Device: id, name, type (workstation/server), tags, trust_state (enrolled/trusted/revoked), host_fingerprint, device_cert_fingerprint, created_at.
- Policy: group_id, host_tag, effect (allow/deny), description.
- AuditLog: request_id, actor_username, device_id, action, target, result, reason, details, created_at.

## API (minimum, HTTP+JSON over TLS)
- POST /v1/bootstrap — one-time admin bootstrap (admin token issuance).
- POST /v1/users — create user (admin token or admin mTLS).
- GET  /v1/users/:id — fetch user.
- POST /v1/groups — create group.
- POST /v1/groups/:id/members — add member.
- POST /v1/devices/enroll — issue device cert, return bundle.
- POST /v1/devices/:id/trust — set trust_state (enrolled|trusted|revoked).
- POST /v1/policies — create SSH policy.
- GET  /v1/ssh/authorized_keys?username=...&host_fingerprint=... — returns authorized keys (requires mTLS device cert, trusted device, matching host fingerprint, policy allow).
- GET  /v1/audit — list audit events (admin auth).
- POST /v1/kerberos/users/:id/commands — emit kadmin commands (admin auth).
- POST /v1/kerberos/devices/:id/commands — emit kadmin commands (admin auth).

## CLI (udctl) essentials
- `bootstrap <user> <pass>` — initialize admin; prints admin_token.
- `create-user`, `create-group`, `add-member`, `create-policy` — lifecycle.
- `enroll-device` — issues device cert/key/ca bundle.
- `kerberos-sync-user --user-id ...` / `kerberos-sync-device --device-id ...` — fetch kadmin commands (useful when automation disabled).
- Flags: `--server`, `--admin-token`, `--insecure` (dev TLS).

## SSH Authorization Model
- Default deny. Policies match (group_id, host_tag) with allow/deny; deny wins on conflict.
- Device must be `trusted`, present mTLS cert whose fingerprint matches stored device_cert_fingerprint, and host_fingerprint must match request.
- AuthorizedKeysCommand response is empty string on deny; audit logs record allow/deny with reason.

## Kerberos (Lab Only)
- Optional; enable with `kerberos.enabled=true` (config/env). udd then runs `kadmin.local` inside its container to ensure principals and keytabs on user create/bootstrap and device trust promotion.
- Config: realm, kadmin_path, keytab_dir. Defaults in `config/default.toml` are for compose.
- Keytabs are written locally (e.g., /var/lib/udd/keytabs). Distribution/permissions are up to the operator (chmod 600, correct owner).
- Compose KDC: `mitkrb5/mit-krb5-kdc:1.20`; admin password from env; change it.
- CI disables Kerberos; flows are untested there.

## Demo Deployment (Lab)
1) Generate dev TLS + CA in `deploy/certs` (see docs/quickstart.md).
2) `docker compose -f deploy/docker-compose.yml up --build` (Postgres + KDC + udd). Kerberos automation enabled via env in compose; disable by setting `UD__KERBEROS__ENABLED=false`.
3) Bootstrap admin with `udctl bootstrap`; export admin token.
4) Create user/group/membership/policy for host tag `server`.
5) Enroll device (tags include `server`, set host_fingerprint); receive cert/key/ca.
6) Promote device to `trusted` (admin auth).
7) On target host: install cert/key/ca, configure sshd `AuthorizedKeysCommand` to run `ud-ssh-authz --server ... --user %u --host-fingerprint %f --cert ... --key ... --ca ...`; restart sshd; test allow/deny.
8) Kerberos (optional): with automation enabled, udd generates principals/keytabs; otherwise use `udctl kerberos-sync-*` or `deploy/kerberos-sync.sh` to run kadmin commands in the KDC container.

## Threat Model (Condensed)
- Network attacker on LAN: mitigated by TLS/mTLS.
- Device impersonation: mitigated by device cert fingerprint + host fingerprint + trust_state.
- Policy abuse: default deny; audited decisions.
- Secrets at rest: rely on file permissions; no HSM/KMS. Protect CA keys, admin tokens, keytabs; rotate.
- Residual: no DoS controls; no CRL/OCSP; admin token allowed unless disabled; Kerberos admin password/ACLs are operator-managed.

## Testing & CI
- `cargo test` covers unit + happy-path integration (requires DB env). CI runs Postgres-backed tests; Kerberos disabled there.
- No CI coverage for OpenSSH or kadmin flows; validate locally with compose.

## Operational Warnings
- **Do not use in production.** Missing hardening, monitoring, HA, backup/restore, secret management, rate limiting, supply-chain review.
- Host deployment (sshd, cert/key placement, keytab distribution) is manual and security-critical; mistakes can lock you out or leak credentials.
- Admin token enabled by default; disable post-bootstrap for mTLS-only admin access.

## License
Apache-2.0. Use at your own risk.
