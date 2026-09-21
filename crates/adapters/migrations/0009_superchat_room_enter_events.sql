ALTER TABLE viewer_events
    DROP CONSTRAINT viewer_events_event_type_check;

ALTER TABLE viewer_events
    ADD CONSTRAINT viewer_events_event_type_check
    CHECK (event_type IN ('chat', 'gift', 'super_chat', 'room_enter'));
