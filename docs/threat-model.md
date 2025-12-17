# Threat Model (v0.1)

## Scope
- Assets: user credentials (password hashes), device credentials (certs/keys), SSH authorization decisions, audit logs, Kerberos principals/keytabs, admin tokens.
- Trusted boundary: `udd` API over TLS with mandatory mTLS for hosts/helpers; admin token allowed only for bootstrap/admin flows.
- Environment: single realm `UD.INTERNAL`, Linux-only, operator-controlled network; assume LAN can be monitored/modified.

## Assumptions
- PostgreSQL host and filesystem permissions protect data/keys at rest.
- TLS private keys, device CA keys, and Kerberos KDC files are stored with strict OS perms; no multi-tenant sharing.
- Operators run `udd` behind a firewall; DNS/IP routing is controlled.
- Time is roughly synchronized across nodes (TLS/Kerberos validity).

## Major Threats & Mitigations
- **Traffic sniffing/tampering:** All API/SSH helper calls require TLS; hosts/helpers use mTLS with device certs; reject when client cert missing/mismatched.
- **Stolen admin token:** Token used only for bootstrap/admin; encourage short lifetime and file permissions; mTLS preferred for day-2 ops.
- **Device impersonation:** Device cert fingerprint bound at enrollment; authorized_keys enforces certificate fingerprint and host_fingerprint matching; trust_state must be `trusted`.
- **Privilege escalation via policy:** Policies are explicit; evaluation defaults deny; audit logs capture allow/deny with reason.
- **DB compromise:** Passwords hashed with Argon2id; device keys not stored; only cert/key pair returned at enrollment; audit logs enable forensics.
- **Kerberos realm takeover:** KDC isolated in its container; provisioning commands run via `kadmin.local`; keytabs transferred out-of-band; document to restrict access to KDC volume.

## Residual Risks / TODO
- No automatic CRL/OCSP for device certs; revocation is via DB `trust_state` and enforced at authorized_keys.
- No rate limiting or DoS protections on API endpoints.
- mTLS not yet enforced for admin CLI flows (token still accepted); tighten in later releases.
- KDC admin password/ACLs rely on deployer; no automation for rotation.
