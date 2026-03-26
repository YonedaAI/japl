CREATE TABLE IF NOT EXISTS time_entries (
    id               VARCHAR(36) PRIMARY KEY,
    user_id          VARCHAR(36) NOT NULL REFERENCES users(id),
    project_id       VARCHAR(36) NOT NULL REFERENCES projects(id),
    description      TEXT NOT NULL DEFAULT '',
    started_at       TIMESTAMP NOT NULL,
    stopped_at       TIMESTAMP,
    duration_seconds INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_entries_user ON time_entries(user_id);
CREATE INDEX idx_entries_project ON time_entries(project_id);
CREATE INDEX idx_entries_started ON time_entries(started_at);
