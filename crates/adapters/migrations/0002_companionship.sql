CREATE TABLE viewer_companionship (
 scope_id TEXT NOT NULL, viewer_id UUID NOT NULL, familiarity_milli INTEGER NOT NULL DEFAULT 0 CHECK(familiarity_milli BETWEEN 0 AND 100000), affinity_milli INTEGER NOT NULL DEFAULT 0 CHECK(affinity_milli BETWEEN 0 AND 100000), last_seen_at_ms BIGINT NOT NULL, medal_level BIGINT, guard_level BIGINT, platform_observed_at_ms BIGINT,
 PRIMARY KEY(scope_id,viewer_id), FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE
);
CREATE TABLE viewer_presence (
 scope_id TEXT NOT NULL,viewer_id UUID NOT NULL,day BIGINT NOT NULL, PRIMARY KEY(scope_id,viewer_id,day), FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE
);
CREATE TABLE viewer_observed_sessions (
 scope_id TEXT NOT NULL,viewer_id UUID NOT NULL,session_id TEXT NOT NULL,PRIMARY KEY(scope_id,viewer_id,session_id),FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE
);
CREATE TABLE gift_ledger (
 scope_id TEXT NOT NULL,viewer_id UUID NOT NULL,source TEXT NOT NULL,event_id TEXT NOT NULL,day BIGINT NOT NULL,occurred_at_ms BIGINT NOT NULL,payload JSONB NOT NULL,value_cents BIGINT CHECK(value_cents>=0),value_kind TEXT NOT NULL DEFAULT 'unknown',
 PRIMARY KEY(scope_id,source,event_id),FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE
);
CREATE INDEX gift_viewer_idx ON gift_ledger(scope_id,viewer_id,day);
CREATE TABLE affinity_ledger (
 id UUID PRIMARY KEY,scope_id TEXT NOT NULL,viewer_id UUID NOT NULL,kind TEXT NOT NULL,day BIGINT NOT NULL,computed_delta_milli INTEGER NOT NULL,applied_delta_milli INTEGER NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,created_at_ms BIGINT NOT NULL,request_key TEXT NOT NULL,fingerprint TEXT NOT NULL,reversed_ledger_id UUID,
 UNIQUE(scope_id,request_key), UNIQUE(scope_id,reversed_ledger_id),FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE
);
CREATE INDEX affinity_viewer_idx ON affinity_ledger(scope_id,viewer_id,created_at_ms DESC);
CREATE TABLE companionship_replies (scope_id TEXT NOT NULL,speech_id TEXT NOT NULL,generated_at_ms BIGINT NOT NULL,event_keys TEXT NOT NULL,completed_at_ms BIGINT,PRIMARY KEY(scope_id,speech_id));
CREATE TABLE companionship_reply_events (
 scope_id TEXT NOT NULL,speech_id TEXT NOT NULL,source TEXT NOT NULL,event_id TEXT NOT NULL,viewer_id UUID NOT NULL,day BIGINT NOT NULL,chat_body TEXT,
 PRIMARY KEY(scope_id,speech_id,source,event_id),FOREIGN KEY(scope_id,speech_id) REFERENCES companionship_replies(scope_id,speech_id) ON DELETE CASCADE,FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE
);
CREATE TABLE companionship_completed_events (
 scope_id TEXT NOT NULL,source TEXT NOT NULL,event_id TEXT NOT NULL,viewer_id UUID NOT NULL,PRIMARY KEY(scope_id,source,event_id),FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE
);
CREATE TABLE companionship_daily_chats (
 scope_id TEXT NOT NULL,viewer_id UUID NOT NULL,day BIGINT NOT NULL,body_hash TEXT NOT NULL,PRIMARY KEY(scope_id,viewer_id,day,body_hash),FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE
);
