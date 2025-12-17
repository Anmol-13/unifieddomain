-- Initial schema for UnifiedDomain v0.1
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    ssh_public_keys TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE groups (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE group_memberships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, group_id)
);

CREATE TABLE devices (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    device_type TEXT NOT NULL CHECK (device_type IN ('workstation', 'server')),
    tags TEXT[] NOT NULL DEFAULT '{}',
    trust_state TEXT NOT NULL CHECK (trust_state IN ('enrolled', 'trusted', 'revoked')),
    pubkey_fingerprint TEXT,
    host_fingerprint TEXT UNIQUE,
    device_cert_pem TEXT,
    device_cert_fingerprint TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE ssh_policies (
    id UUID PRIMARY KEY,
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    host_tag TEXT NOT NULL,
    effect TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    description TEXT
);
CREATE INDEX ON ssh_policies (group_id, host_tag);

CREATE TABLE audit_logs (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    request_id UUID,
    actor_username TEXT,
    device_id UUID,
    action TEXT NOT NULL,
    target TEXT,
    result TEXT NOT NULL,
    reason TEXT,
    details JSONB
);
