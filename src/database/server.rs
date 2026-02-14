use sqlx::{Postgres, Transaction};
use crate::database::{pool, ServerHistory, ServerInfo};
use crate::logger;

pub async fn insert_servers(servers: Vec<(ServerInfo, ServerHistory)>) -> Result<(), sqlx::Error> {
    let pool = pool::get_pool();

    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    for (server, history) in servers {
        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO servers (server_ip, server_port, last_seen, discovered, bedrock, country)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (server_ip, server_port)
            DO UPDATE SET last_seen = EXCLUDED.last_seen
            RETURNING server_id
            "#
        )
            .bind(&server.server_ip)
            .bind(server.server_port as i32)
            .bind(server.last_seen)
            .bind(server.discovered)
            .bind(server.bedrock)
            .bind(&server.country)
            .fetch_one(&mut *tx)
            .await?;

        let server_id = row.0;

        sqlx::query(
            r#"
            INSERT INTO server_history (
                server_id, seen, description, plain_description, icon,
                player_online, player_max, player_sample,
                version_name, version_protocol, enforces_secure_chat,
                is_modded_server, mods, mod_loader,
                players, plugins, software,
                kick_message, cracked, whitelist, latency
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            "#
        )
            .bind(server_id)
            .bind(history.seen)
            .bind(&history.description)
            .bind(&history.plain_description)
            .bind(&history.icon)
            .bind(history.player_online)
            .bind(history.player_max)
            .bind(serde_json::to_value(&history.player_sample).ok())
            .bind(&history.version_name)
            .bind(history.version_protocol)
            .bind(history.enforces_secure_chat)
            .bind(history.is_modded_server)
            .bind(serde_json::to_value(&history.mods).ok())
            .bind(serde_json::to_value(&history.mod_loader).ok())
            .bind(serde_json::to_value(&history.players).ok())
            .bind(serde_json::to_value(&history.plugins).ok())
            .bind(serde_json::to_value(&history.software).ok())
            .bind(&history.kick_message)
            .bind(history.cracked)
            .bind(history.whitelist)
            .bind(history.latency)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}