use sqlx::{Postgres, Row, Transaction};
use crate::database::{parse_players, pool, Player, PlayerHistory, ServerHistory, ServerInfo};

pub async fn insert_servers(results: &Vec<(ServerInfo, ServerHistory)>) -> Result<(), sqlx::Error> {
    let pool = pool::get_pool();

    for chunk in results.chunks(100) {
        let mut tx = pool.begin().await?;

        for (server, server_history) in chunk {
            let (server_id, ): (i32,) = sqlx::query_as(
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
                .bind(server_history.seen)
                .bind(&server_history.description)
                .bind(&server_history.plain_description)
                .bind(&server_history.icon)
                .bind(server_history.player_online)
                .bind(server_history.player_max)
                .bind(serde_json::to_value(&server_history.player_sample).unwrap_or(serde_json::Value::Null))
                .bind(&server_history.version_name)
                .bind(server_history.version_protocol)
                .bind(server_history.enforces_secure_chat)
                .bind(server_history.is_modded_server)
                .bind(serde_json::to_value(&server_history.mods).unwrap_or(serde_json::Value::Null))
                .bind(serde_json::to_value(&server_history.mod_loader).unwrap_or(serde_json::Value::Null))
                .bind(serde_json::to_value(&server_history.players).unwrap_or(serde_json::Value::Null))
                .bind(serde_json::to_value(&server_history.plugins).unwrap_or(serde_json::Value::Null))
                .bind(serde_json::to_value(&server_history.software).unwrap_or(serde_json::Value::Null))
                .bind(&server_history.kick_message)
                .bind(server_history.cracked)
                .bind(server_history.whitelist)
                .bind(server_history.latency)
                .execute(&mut *tx)
                .await?;

            let players = parse_players(server_id, server_history);
            insert_players(&players, &mut tx).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

pub async fn insert_players(player_data: &Vec<(Player, PlayerHistory)>, tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    for (player, history) in player_data {
        sqlx::query(
            r#"
            INSERT INTO player_history (uuid, username, server_id, seen)
            VALUES ($1, $2, $3, $4)
            "#
        )
            .bind(&history.uuid)
            .bind(&history.username)
            .bind(history.server_id)
            .bind(history.seen)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

pub async fn get_total_servers() -> Result<Vec<ServerInfo>, sqlx::Error> {
    let pool = pool::get_pool();

    let rows = sqlx::query("SELECT server_id, server_ip, server_port, last_seen, discovered, bedrock, country FROM servers")
        .fetch_all(pool)
        .await?;

    let servers = rows.into_iter().map(|row| {
        ServerInfo {
            server_id: Some(row.get::<i32, _>("server_id") as i64),
            server_ip: row.get::<String, _>("server_ip"),
            server_port: row.get::<i32, _>("server_port") as u16,
            last_seen: row.get::<i64, _>("last_seen"),
            discovered: row.get::<i64, _>("discovered"),
            bedrock: row.get::<bool, _>("bedrock"),
            country: row.get::<Option<String>, _>("country"),
        }
    }).collect();
    Ok(servers)
}