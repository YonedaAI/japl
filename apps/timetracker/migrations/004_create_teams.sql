CREATE TABLE IF NOT EXISTS teams (
    id          VARCHAR(36) PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS team_members (
    team_id     VARCHAR(36) NOT NULL REFERENCES teams(id),
    user_id     VARCHAR(36) NOT NULL REFERENCES users(id),
    role        VARCHAR(20) NOT NULL DEFAULT 'member',
    joined_at   TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (team_id, user_id)
);

CREATE INDEX idx_team_members_user ON team_members(user_id);
