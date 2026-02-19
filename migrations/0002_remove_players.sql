ALTER TABLE player_history DROP CONSTRAINT IF EXISTS player_history_uuid_fkey;
DROP TABLE IF EXISTS players;
CREATE INDEX IF NOT EXISTS idx_player_history_uuid ON player_history(uuid);