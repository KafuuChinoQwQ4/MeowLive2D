-- Viewer scope cleanup must remove dependent runtime memory rows atomically.
ALTER TABLE memory_jobs DROP CONSTRAINT memory_jobs_scope_id_viewer_id_fkey;
ALTER TABLE memory_jobs ADD FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE;
ALTER TABLE memories DROP CONSTRAINT memories_scope_id_viewer_id_fkey;
ALTER TABLE memories ADD FOREIGN KEY(scope_id,viewer_id) REFERENCES viewers(scope_id,id) ON DELETE CASCADE;
ALTER TABLE memory_evidence DROP CONSTRAINT memory_evidence_scope_id_memory_id_fkey;
ALTER TABLE memory_evidence ADD FOREIGN KEY(scope_id,memory_id) REFERENCES memories(scope_id,id) ON DELETE CASCADE;
ALTER TABLE memory_vectors DROP CONSTRAINT memory_vectors_scope_id_memory_id_fkey;
ALTER TABLE memory_vectors ADD FOREIGN KEY(scope_id,memory_id) REFERENCES memories(scope_id,id) ON DELETE CASCADE;
