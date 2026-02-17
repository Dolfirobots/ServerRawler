-- Servers
CREATE TABLE IF NOT EXISTS servers (
    server_id SERIAL PRIMARY KEY,
    server_ip TEXT NOT NULL,
    server_port INTEGER NOT NULL,

    last_seen BIGINT NOT NULL,
    discovered BIGINT NOT NULL,

    bedrock BOOLEAN NOT NULL,
    country TEXT
);

CREATE INDEX IF NOT EXISTS idx_servers_ip ON servers(server_ip);
ALTER TABLE servers ADD CONSTRAINT unique_server_address UNIQUE (server_ip, server_port);

-- Server History
CREATE TABLE IF NOT EXISTS server_history (
    history_id BIGSERIAL PRIMARY KEY,
    server_id INTEGER NOT NULL REFERENCES servers(server_id) ON DELETE CASCADE,

    seen BIGINT NOT NULL,
    description TEXT,
    plain_description TEXT,
    icon TEXT,

    player_online INTEGER,
    player_max INTEGER,
    player_sample JSONB,

    version_name TEXT,
    version_protocol INTEGER,

    enforces_secure_chat BOOLEAN,

    is_modded_server BOOLEAN NOT NULL,
    mods JSONB,
    mod_loader TEXT,

    players JSONB,
    plugins JSONB,
    software JSONB,

    kick_message TEXT,
    cracked BOOLEAN,
    whitelist BOOLEAN,

    latency REAL NOT NULL
);

-- Players
CREATE TABLE IF NOT EXISTS players (
    uuid TEXT PRIMARY KEY,
    username TEXT NOT NULL,

    discovered BIGINT NOT NULL,
    last_seen BIGINT NOT NULL
);

-- Player History
CREATE TABLE IF NOT EXISTS player_history (
    history_id BIGSERIAL PRIMARY KEY,
    uuid TEXT NOT NULL REFERENCES players(uuid) ON DELETE CASCADE,
    username TEXT NOT NULL,

    server_id INTEGER NOT NULL REFERENCES servers(server_id) ON DELETE CASCADE,
    seen BIGINT NOT NULL
);