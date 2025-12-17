# Architecture snapshot (v0.1 draft)

- One domain/realm: `UD.INTERNAL`.
- `udd`: HTTP+JSON API over TLS, single binary, Linux target, systemd friendly.
- Storage: PostgreSQL (sqlx) with migrations in `crates/udd/migrations`. State: users, groups, memberships, devices, SSH policies, audit log.
- AuthN today: bearer admin token; mTLS for hosts/admins is planned. Kerberos integration is partial: generate `kadmin.local` commands via `udctl` to create user/host principals and keytabs; run them inside the KDC.
- AuthZ: centralized policy evaluated by `udd`. SSH helpers fetch authorization decisions via AuthorizedKeysCommand.
- Audit: every decision writes to `audit_logs` with request_id, actor, device, result, reason.
- Logging: structured JSON, UTC timestamps, includes request context.

Operational notes (v0.1):
- Admin auth: bearer token for bootstrap; mTLS or token accepted for admin endpoints.
- Device lifecycle: enroll issues device cert+key; operators must promote to `trusted` (or `revoked`) via `/v1/devices/:id/trust` before SSH authz will allow.
- SSH: AuthorizedKeysCommand via `ud-ssh-authz` using mTLS with device cert; policy is allow/deny with default deny and audited decisions.
- Kerberos: mit-krb5 in docker-compose; provisioning uses generated `kadmin.local` commands (see deploy/kerberos-provision.sh); authorization stays in `udd`.
- Auditing: every authz decision and trust update is logged.
