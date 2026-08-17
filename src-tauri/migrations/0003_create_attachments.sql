CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    original_name TEXT NOT NULL,
    operation_type TEXT NOT NULL, -- 'COPY', 'MOVE', 'LINK'
    local_path TEXT NOT NULL, -- UUID file name OR absolute path for LINK
    mime_type TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);