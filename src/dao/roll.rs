use sqlx::{Error, FromRow, SqlitePool};

#[derive(Debug, serde::Serialize, serde::Deserialize, FromRow)]
pub struct RollUser {
    pub openid: String,
    pub session: String,
    pub team_name: Option<String>,
    pub score: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, FromRow)]
pub struct RollTeam {
    pub name: String,
    pub score: f64,
}

pub async fn get_user_by_openid(
    pool: &SqlitePool,
    openid: &str,
) -> Result<Option<RollUser>, Error> {
    sqlx::query_as("SELECT openid, session, team_name, score FROM roll_user WHERE openid = ?")
        .bind(openid)
        .fetch_optional(pool)
        .await
}

pub async fn upsert_user_session(
    pool: &SqlitePool,
    openid: &str,
    session: &str,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO roll_user (openid, session, score) VALUES (?, ?, 0) ON CONFLICT(openid) DO UPDATE SET session = excluded.session"
    )
    .bind(openid)
    .bind(session)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_team_if_not_exists(pool: &SqlitePool, name: &str) -> Result<(), Error> {
    sqlx::query("INSERT INTO roll_team (name, score) VALUES (?, 0.0) ON CONFLICT(name) DO NOTHING")
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_user_team(
    pool: &SqlitePool,
    openid: &str,
    team_name: Option<&str>,
) -> Result<u64, Error> {
    let result = sqlx::query("UPDATE roll_user SET team_name = ? WHERE openid = ?")
        .bind(team_name)
        .bind(openid)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Reports score and returns the added team score
pub async fn add_user_score(
    pool: &SqlitePool,
    openid: &str,
    report_score: i64,
) -> Result<Option<f64>, Error> {
    let mut tx = pool.begin().await?;

    // 1. Get user and team info
    let user = sqlx::query_as::<_, RollUser>(
        "SELECT openid, session, team_name, score FROM roll_user WHERE openid = ?",
    )
    .bind(openid)
    .fetch_optional(&mut *tx)
    .await?;

    let user = match user {
        Some(u) => u,
        None => return Ok(None),
    };

    // 2. Update personal best if reported score is higher
    if report_score > user.score {
        sqlx::query("UPDATE roll_user SET score = ? WHERE openid = ?")
            .bind(report_score)
            .bind(openid)
            .execute(&mut *tx)
            .await?;
    }

    let mut added_team_score = 0.0;

    // 3. Update team score if user is in a team
    if let Some(team_name) = user.team_name {
        // Count members in this team
        let member_count_row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM roll_user WHERE team_name = ?")
                .bind(&team_name)
                .fetch_one(&mut *tx)
                .await?;

        let member_count = member_count_row.0;
        if member_count > 0 {
            added_team_score = (report_score as f64) / (member_count as f64);

            sqlx::query("UPDATE roll_team SET score = score + ? WHERE name = ?")
                .bind(added_team_score)
                .bind(&team_name)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;

    Ok(Some(added_team_score))
}

pub async fn get_user_rank(pool: &SqlitePool, score: i64) -> Result<i64, Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roll_user WHERE score > ?")
        .bind(score)
        .fetch_one(pool)
        .await?;

    Ok(row.0 + 1)
}

pub async fn get_user_count(pool: &SqlitePool) -> Result<i64, Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roll_user")
        .fetch_one(pool)
        .await?;

    Ok(row.0)
}

pub async fn get_user_rank_in_team(
    pool: &SqlitePool,
    score: i64,
    team_name: &str,
) -> Result<i64, Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roll_user WHERE team_name = ? AND score > ?")
        .bind(team_name)
        .bind(score)
        .fetch_one(pool)
        .await?;

    Ok(row.0 + 1)
}

pub async fn get_team_member_count(pool: &SqlitePool, team_name: &str) -> Result<i64, Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roll_user WHERE team_name = ?")
        .bind(team_name)
        .fetch_one(pool)
        .await?;

    Ok(row.0)
}

pub async fn get_all_teams(pool: &SqlitePool) -> Result<Vec<RollTeam>, Error> {
    let teams = sqlx::query_as("SELECT name, score FROM roll_team ORDER BY score DESC")
        .fetch_all(pool)
        .await?;

    Ok(teams)
}
