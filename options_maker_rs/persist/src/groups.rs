use crate::db;

const FAV_GROUP_NAME: &str = "Favorite";

pub async fn is_favorite(ticker: &str) -> sqlx::Result<bool> {
    let groups = sqlx::query!(
        r"
        SELECT group_name FROM symbol_groups
        WHERE symbol=$1
        ",
        ticker,
    )
    .map(|r| r.group_name)
    .fetch_all(db())
    .await?;
    Ok(groups.iter().any(|group| group == FAV_GROUP_NAME))
}

pub async fn add_to_favorite(ticker: &str) -> sqlx::Result<()> {
    sqlx::query!(
        r"
        INSERT INTO symbol_groups (symbol, group_name)
        VALUES ($1, $2)
        ON CONFLICT (symbol, group_name) DO NOTHING
        ",
        ticker,
        FAV_GROUP_NAME,
    )
    .execute(db())
    .await?;
    Ok(())
}

pub async fn remove_from_favorite(ticker: &str) -> sqlx::Result<()> {
    sqlx::query!(
        r"
        DELETE FROM symbol_groups
        WHERE symbol = $1 AND group_name = $2
        ",
        ticker,
        FAV_GROUP_NAME,
    )
    .execute(db())
    .await?;
    Ok(())
}
