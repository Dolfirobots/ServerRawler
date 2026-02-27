use crate::discord::{Context, Error};

/// Search a player in the database
#[poise::command(slash_command,
    description_localized("de", "Suche einen Spieler in der Datenbank")
)]
pub async fn search_player(
    ctx: Context<'_>,
    #[description = "Username of the player"]
    #[description_localized("de", "Spielername von dem Spieler")]
    username: String
) -> Result<(), Error> {
    Ok(())
}