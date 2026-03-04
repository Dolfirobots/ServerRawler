use sqlx::{Postgres, Transaction};
use crate::database::{parse_database_player, pool, PlayerHistory};

pub async fn insert_players(player_data: &Vec<PlayerHistory>, tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    for history in player_data {
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


async fn get_player_by_field(field: &str, value: &str) -> Result<Option<Vec<PlayerHistory>>, sqlx::Error> {
    let pool = pool::get_pool();

    let query_str = format!(
        "SELECT * FROM player_history WHERE {} = $1 ORDER BY history_id DESC",
        field
    );

    let rows = sqlx::query(&query_str)
        .bind(value)
        .fetch_all(pool)
        .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let history: Vec<PlayerHistory> = rows
        .iter()
        .map(|r| parse_database_player(r))
        .collect();

    Ok(Some(history))
}

pub async fn get_player_by_username(username: &str) -> Result<Option<Vec<PlayerHistory>>, sqlx::Error> {
    get_player_by_field("username", username).await
}

pub async fn get_player_by_uuid(uuid: &str) -> Result<Option<Vec<PlayerHistory>>, sqlx::Error> {
    get_player_by_field("uuid", uuid).await
}