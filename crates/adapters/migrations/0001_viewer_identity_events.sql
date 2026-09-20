CREATE TABLE viewers (
    id UUID PRIMARY KEY,
    scope_id TEXT NOT NULL,
    current_alias TEXT,
    alias_observed_at_ms BIGINT,
    created_at_ms BIGINT NOT NULL,
    UNIQUE (scope_id, id),
    CHECK (created_at_ms >= 0),
    CHECK (alias_observed_at_ms IS NULL OR alias_observed_at_ms >= 0)
);

CREATE TABLE viewer_identities (
    scope_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    namespace TEXT NOT NULL,
    id_kind TEXT NOT NULL,
    external_id TEXT NOT NULL,
    viewer_id UUID NOT NULL,
    last_confirmed_at_ms BIGINT NOT NULL,
    PRIMARY KEY (scope_id, platform, namespace, id_kind, external_id),
    FOREIGN KEY (scope_id, viewer_id) REFERENCES viewers (scope_id, id),
    CHECK (last_confirmed_at_ms >= 0)
);

CREATE TABLE viewer_aliases (
    scope_id TEXT NOT NULL,
    viewer_id UUID NOT NULL,
    alias TEXT NOT NULL,
    first_seen_at_ms BIGINT NOT NULL,
    last_seen_at_ms BIGINT NOT NULL,
    PRIMARY KEY (scope_id, viewer_id, alias),
    FOREIGN KEY (scope_id, viewer_id) REFERENCES viewers (scope_id, id),
    CHECK (first_seen_at_ms >= 0),
    CHECK (last_seen_at_ms >= first_seen_at_ms)
);

CREATE TABLE live_sessions (
    scope_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    first_received_at_ms BIGINT NOT NULL,
    PRIMARY KEY (scope_id, session_id),
    CHECK (first_received_at_ms >= 0)
);

CREATE TABLE viewer_events (
    scope_id TEXT NOT NULL,
    source TEXT NOT NULL,
    event_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    viewer_id UUID,
    viewer_alias TEXT NOT NULL,
    occurred_at_ms BIGINT NOT NULL,
    received_at_ms BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    PRIMARY KEY (scope_id, source, event_id),
    FOREIGN KEY (scope_id, session_id) REFERENCES live_sessions (scope_id, session_id),
    FOREIGN KEY (scope_id, viewer_id) REFERENCES viewers (scope_id, id),
    CHECK (occurred_at_ms >= 0),
    CHECK (received_at_ms >= 0),
    CHECK (event_type IN ('chat', 'gift'))
);

CREATE INDEX viewer_aliases_scope_alias_idx ON viewer_aliases (scope_id, alias);
CREATE INDEX viewer_events_scope_received_idx ON viewer_events (scope_id, received_at_ms DESC, source, event_id);
