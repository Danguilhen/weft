-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE node (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_url TEXT NOT NULL UNIQUE,
    public_url TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE node_keys (
    key_id TEXT PRIMARY KEY,
    node_id UUID NOT NULL REFERENCES node (id) ON DELETE CASCADE,
    alg TEXT NOT NULL CHECK (alg = 'Ed25519'),
    public_key BYTEA NOT NULL,
    private_key_ref TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    valid_until TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

CREATE TABLE nodes (
    node_id TEXT PRIMARY KEY,
    public_url TEXT NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    descriptor JSONB,
    status TEXT NOT NULL DEFAULT 'unknown'
        CHECK (status IN ('unknown', 'allowed', 'blocked')),
    first_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_verified_at TIMESTAMPTZ
);

CREATE TABLE blobs (
    hash CHAR(64) PRIMARY KEY,
    size BIGINT NOT NULL CHECK (size > 0),
    mime TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    frames INTEGER,
    duration_ms INTEGER,
    storage_key TEXT NOT NULL,
    thumbnail_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    verified_at TIMESTAMPTZ
);

CREATE TABLE content (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hash CHAR(64) NOT NULL REFERENCES blobs (hash),
    title TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    origin_node TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'active', 'restricted', 'removed', 'rejected', 'failed')),
    licence_spdx TEXT,
    licence_url TEXT,
    licence_claim TEXT,
    licence_status TEXT NOT NULL DEFAULT 'unverified'
        CHECK (licence_status IN ('unverified', 'evidence_provided', 'verified_evidence', 'disputed', 'invalidated')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ,
    removed_at TIMESTAMPTZ,
    tombstone_reason TEXT,
    search TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('simple', coalesce(title, '') || ' ' || coalesce(description, ''))
    ) STORED,
    UNIQUE (hash, origin_node)
);

CREATE TABLE content_tags (
    content_id UUID NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (content_id, tag)
);

CREATE TABLE content_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hash CHAR(64) NOT NULL REFERENCES blobs (hash) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    url TEXT NOT NULL,
    bytes BIGINT,
    verified_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (hash, node_id)
);

CREATE TABLE licence_evidence (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_id UUID NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    fetched_at TIMESTAMPTZ,
    fetched_hash CHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE uploads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hash CHAR(64),
    status TEXT NOT NULL DEFAULT 'received'
        CHECK (status IN ('received', 'validating', 'stored', 'rejected', 'failed')),
    review TEXT NOT NULL DEFAULT 'queued'
        CHECK (review IN ('queued', 'approved', 'rejected', 'not_required')),
    client_hash BYTEA,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at TIMESTAMPTZ
);

CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_id UUID REFERENCES content (id) ON DELETE SET NULL,
    hash CHAR(64),
    reason TEXT NOT NULL
        CHECK (reason IN ('licence', 'illegal', 'spam', 'malware', 'privacy', 'other')),
    details TEXT NOT NULL DEFAULT '',
    contact_hash BYTEA,
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'reviewing', 'actioned', 'dismissed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ
);

CREATE TABLE moderation_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_id UUID REFERENCES content (id) ON DELETE SET NULL,
    hash CHAR(64),
    category TEXT NOT NULL
        CHECK (category IN (
            'CONTENT_REPORTED', 'LICENCE_DISPUTED', 'CONTENT_RESTRICTED',
            'CONTENT_BLOCKED', 'CONTENT_REMOVED', 'CONTENT_REINSTATED',
            'NODE_SUSPENDED'
        )),
    scope TEXT NOT NULL DEFAULT 'local',
    issuer TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    evidence JSONB NOT NULL DEFAULT '[]'::jsonb,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    signature TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);

CREATE TABLE federation_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    activity_id TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    target TEXT NOT NULL,
    payload JSONB NOT NULL,
    state TEXT NOT NULL DEFAULT 'queued'
        CHECK (state IN ('queued', 'sending', 'sent', 'failed', 'dead')),
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at TIMESTAMPTZ
);

CREATE TABLE federation_inbox (
    activity_id TEXT PRIMARY KEY,
    actor TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    digest TEXT,
    result TEXT NOT NULL DEFAULT 'accepted',
    expires_at TIMESTAMPTZ
);

CREATE TABLE node_blocks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind TEXT NOT NULL CHECK (kind IN ('node', 'domain', 'url_prefix', 'tag')),
    value TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NOT NULL,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX content_hash_idx ON content (hash);
CREATE INDEX content_status_idx ON content (status);
CREATE INDEX content_published_idx ON content (published_at DESC);
CREATE INDEX content_search_idx ON content USING GIN (search);
CREATE INDEX content_tags_tag_idx ON content_tags (tag);
CREATE INDEX content_sources_hash_idx ON content_sources (hash);
CREATE INDEX uploads_status_idx ON uploads (status, created_at);
CREATE INDEX reports_status_idx ON reports (status, created_at);
CREATE INDEX moderation_events_content_idx ON moderation_events (content_id, created_at);
CREATE INDEX moderation_events_created_idx ON moderation_events (created_at);
CREATE INDEX federation_outbox_state_idx ON federation_outbox (state, next_attempt_at);
CREATE INDEX federation_inbox_received_idx ON federation_inbox (received_at);
