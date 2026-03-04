use sqlx::Postgres;
use crate::database::{parse_database_server_history, parse_database_server_info, parse_players, pool, ServerHistory, ServerInfo};
use crate::database::player::insert_players;
use crate::discord::actions::server_filter::{NumberFilter, SearchFilters, StringFilter};

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
                .bind(&server_history.mod_loader)
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

pub async fn get_total_servers() -> Result<Vec<ServerInfo>, sqlx::Error> {
    let pool = pool::get_pool();

    let rows = sqlx::query("SELECT * FROM servers")
        .fetch_all(pool)
        .await?;

    let servers = rows.into_iter().map(|row| {
        parse_database_server_info(&row)
    }).collect();
    Ok(servers)
}

pub async fn get_server_by_address(ip: String, port: u16) -> Result<Option<(ServerInfo, ServerHistory)>, sqlx::Error> {
    let pool = pool::get_pool();

    let row = sqlx::query(
        r#"
        SELECT s.*, h.*
        FROM servers s
        LEFT JOIN server_history h ON s.server_id = h.server_id
        WHERE s.server_ip = $1 AND s.server_port = $2
        ORDER BY h.seen DESC
        LIMIT 1
        "#
    )
        .bind(ip)
        .bind(port as i32)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        let server_info = parse_database_server_info(&r);
        let server_history = parse_database_server_history(&r);

        Ok(Some((server_info, server_history)))
    } else {
        Ok(None)
    }
}

pub async fn search_servers(filters: SearchFilters, limit: i64) -> Result<Option<Vec<(ServerInfo, ServerHistory)>>, sqlx::Error> {
    let pool = pool::get_pool();

    let mut query_builder: sqlx::QueryBuilder<'_, Postgres> = sqlx::QueryBuilder::new(
        r#"
        SELECT DISTINCT ON (s.server_id) s.*, h.*
        FROM servers s
        JOIN server_history h ON s.server_id = h.server_id
        WHERE 1=1
        "#
    );

    fn push_string_filter(builder: &mut sqlx::QueryBuilder<'_, Postgres>, field: &str, filter: Option<StringFilter>) {
        if let Some(f) = filter {
            match f {
                StringFilter::Contains(val) => {
                    builder.push(format!(" AND {} ILIKE ", field));
                    builder.push_bind(format!("%{}%", val));
                }
                StringFilter::Equals(val) => {
                    builder.push(format!(" AND {} ILIKE ", field));
                    builder.push_bind(val);
                }
            }
        }
    }

    fn push_number_filter(builder: &mut sqlx::QueryBuilder<'_, Postgres>, field: &str, filter: Option<NumberFilter>) {
        if let Some(f) = filter {
            match f {
                NumberFilter::Less(n) => {
                    builder.push(format!(" AND {} < ", field));
                    builder.push_bind(n);
                }
                NumberFilter::Greater(n) => {
                    builder.push(format!(" AND {} > ", field));
                    builder.push_bind(n);
                }
                NumberFilter::Equal(n) => {
                    builder.push(format!(" AND {} = ", field));
                    builder.push_bind(n);
                }
                NumberFilter::Range(a, b) => {
                    builder.push(format!(" AND {} BETWEEN ", field));
                    builder.push_bind(a);
                    builder.push(" AND ");
                    builder.push_bind(b);
                }
            }
        }
    }

    push_string_filter(&mut query_builder, "h.plain_description", filters.description);
    push_string_filter(&mut query_builder, "h.version_name", filters.version_name);
    push_string_filter(&mut query_builder, "h.software->>'name'", filters.software_name);
    push_string_filter(&mut query_builder, "h.kick_message", filters.kick_message);

    push_number_filter(&mut query_builder, "h.player_online", filters.players_online);
    push_number_filter(&mut query_builder, "h.player_max", filters.players_max);

    if let Some(protocol) = filters.version_protocol {
        query_builder.push(" AND h.version_protocol = ");
        query_builder.push_bind(protocol);
    }

    if let Some(enforce) = filters.enforces_secure_chat {
        query_builder.push(" AND h.enforces_secure_chat = ");
        query_builder.push_bind(enforce);
    }

    if let Some(modded) = filters.is_modded {
        query_builder.push(" AND h.is_modded_server = ");
        query_builder.push_bind(modded);
    }

    if let Some(cracked) = filters.cracked {
        query_builder.push(" AND h.cracked = ");
        query_builder.push_bind(cracked);
    }

    if let Some(whitelist) = filters.whitelist {
        query_builder.push(" AND h.whitelist = ");
        query_builder.push_bind(whitelist);
    }

    if let Some(plugin) = filters.plugin_name {
        query_builder.push(" AND h.plugins @> ");
        query_builder.push_bind(serde_json::json!([{ "name": plugin }]));
    }

    if let Some(m_id) = filters.mod_id {
        query_builder.push(" AND h.mods @> ");
        query_builder.push_bind(serde_json::json!([{ "mod_id": m_id }]));
    }

    query_builder.push(" ORDER BY s.server_id, h.seen DESC");

    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);

    let rows = query_builder.build().fetch_all(pool).await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let results = rows.iter().map(|row| {
        (
            parse_database_server_info(row),
            parse_database_server_history(row)
        )
    }).collect();

    Ok(Some(results))
}