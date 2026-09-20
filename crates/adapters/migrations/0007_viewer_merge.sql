ALTER TABLE viewers ADD COLUMN merged_into UUID;
ALTER TABLE viewers ADD FOREIGN KEY(scope_id,merged_into) REFERENCES viewers(scope_id,id);
ALTER TABLE affinity_ledger ADD COLUMN original_viewer_id UUID;
CREATE TABLE viewer_merge_audit(scope_id TEXT NOT NULL,request_key TEXT NOT NULL,source_viewer_id UUID NOT NULL,target_viewer_id UUID NOT NULL,expected_revision BIGINT NOT NULL,preview_fingerprint TEXT NOT NULL,request_fingerprint TEXT NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,created_at_ms BIGINT NOT NULL,result_revision BIGINT NOT NULL,PRIMARY KEY(scope_id,request_key));
