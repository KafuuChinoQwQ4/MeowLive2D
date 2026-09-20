CREATE TABLE relationship_facts (
 scope_id TEXT NOT NULL,id UUID NOT NULL,version BIGINT NOT NULL DEFAULT 1,
 source_kind TEXT NOT NULL,source_id TEXT NOT NULL,target_kind TEXT NOT NULL,target_id TEXT NOT NULL,
 kind TEXT NOT NULL,confirmation TEXT NOT NULL,evidence JSONB NOT NULL,expires_at_ms BIGINT,
 deleted BOOLEAN NOT NULL DEFAULT false,created_at_ms BIGINT NOT NULL,updated_at_ms BIGINT NOT NULL,
 PRIMARY KEY(scope_id,id),CHECK(version>0),CHECK(confirmation IN ('claimed','confirmed'))
);
CREATE INDEX relationship_source ON relationship_facts(scope_id,source_kind,source_id);
CREATE INDEX relationship_target ON relationship_facts(scope_id,target_kind,target_id);
CREATE TABLE relationship_audit(scope_id TEXT NOT NULL,request_key TEXT NOT NULL,fingerprint TEXT NOT NULL,fact_id UUID,reason TEXT NOT NULL,actor TEXT NOT NULL DEFAULT 'single_admin',created_at_ms BIGINT NOT NULL,result JSONB NOT NULL,PRIMARY KEY(scope_id,request_key));
CREATE TABLE relationship_outbox(scope_id TEXT NOT NULL,fact_id UUID NOT NULL,version BIGINT NOT NULL,state TEXT NOT NULL DEFAULT 'pending',attempts INT NOT NULL DEFAULT 0,available_at_ms BIGINT NOT NULL,created_at_ms BIGINT NOT NULL,lease_until_ms BIGINT,token UUID,PRIMARY KEY(scope_id,fact_id),FOREIGN KEY(scope_id,fact_id) REFERENCES relationship_facts(scope_id,id) ON DELETE CASCADE);
CREATE INDEX relationship_outbox_ready ON relationship_outbox(scope_id,state,available_at_ms);
