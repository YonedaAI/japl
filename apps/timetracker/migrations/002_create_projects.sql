CREATE TABLE IF NOT EXISTS projects (
    id          VARCHAR(36) PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    owner_id    VARCHAR(36) NOT NULL REFERENCES users(id),
    created_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    archived    BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_projects_owner ON projects(owner_id);
