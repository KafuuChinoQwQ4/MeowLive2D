-- Conservative source-event provenance: correcting/removing a memory invalidates
-- every live relationship using its evidence, rather than retaining stale facts.
CREATE FUNCTION invalidate_memory_relationships() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE changed_at BIGINT;
BEGIN
 IF NOT (NEW.deleted AND NOT OLD.deleted
     OR NEW.status='expired' AND OLD.status IS DISTINCT FROM NEW.status
     OR NEW.value IS DISTINCT FROM OLD.value
     OR NEW.expires_at_ms IS DISTINCT FROM OLD.expires_at_ms
        AND NEW.expires_at_ms IS NOT NULL
        AND NEW.expires_at_ms <= (extract(epoch FROM clock_timestamp())*1000)::bigint) THEN
   RETURN NEW;
 END IF;
 changed_at := GREATEST(NEW.updated_at_ms,OLD.updated_at_ms);
 WITH invalidated AS (
   UPDATE relationship_facts f SET deleted=true,version=f.version+1,updated_at_ms=changed_at
   WHERE f.scope_id=NEW.scope_id AND NOT f.deleted AND EXISTS (
     SELECT 1 FROM memory_evidence me CROSS JOIN LATERAL jsonb_array_elements(f.evidence) e
     WHERE me.scope_id=NEW.scope_id AND me.memory_id=NEW.id
       AND me.source=e->>'source' AND me.event_id=e->>'event_id'
   ) RETURNING f.scope_id,f.id,f.version
 ) INSERT INTO relationship_outbox(scope_id,fact_id,version,available_at_ms,created_at_ms)
 SELECT scope_id,id,version,changed_at,changed_at FROM invalidated
 ON CONFLICT(scope_id,fact_id) DO UPDATE SET version=EXCLUDED.version,state='pending',attempts=0,
   available_at_ms=EXCLUDED.available_at_ms,created_at_ms=EXCLUDED.created_at_ms,
   token=NULL,lease_until_ms=NULL;
 UPDATE memory_scopes SET revision=revision+1 WHERE scope_id=NEW.scope_id;
 RETURN NEW;
END $$;
CREATE TRIGGER memories_invalidate_relationships AFTER UPDATE OF value,deleted,status,expires_at_ms ON memories FOR EACH ROW EXECUTE FUNCTION invalidate_memory_relationships();
