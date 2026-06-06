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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ScoreUpdateResult {
    pub added_team_score: f64,
    pub is_new_record: bool,
    pub score: i64,
    pub rank: String,
    pub team_rank: String,
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
) -> Result<Option<ScoreUpdateResult>, Error> {
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

    let is_new_record = report_score > user.score;
    let current_best = if is_new_record {
        report_score
    } else {
        user.score
    };

    // 2. Update personal best if reported score is higher
    if is_new_record {
        sqlx::query("UPDATE roll_user SET score = ? WHERE openid = ?")
            .bind(report_score)
            .bind(openid)
            .execute(&mut *tx)
            .await?;
    }

    let mut added_team_score = 0.0;

    // 3. Update team score if user is in a team
    if let Some(ref team_name) = user.team_name {
        // Count members in this team
        let member_count_row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM roll_user WHERE team_name = ?")
                .bind(team_name)
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

    // Calculate rank percentage
    let user_rank = get_user_rank(pool, current_best).await.unwrap_or(1);
    let user_count = get_user_count(pool).await.unwrap_or(1);
    let user_rank_percent = if user_count > 0 {
        let denominator = user_count.max(100);
        (user_rank as f64 / denominator as f64 * 100.0).ceil() as i64
    } else {
        100
    };

    // Calculate team rank percentage
    let mut team_rank_str = "-".to_string();
    if let Some(ref team_name) = user.team_name {
        let team_user_rank = get_user_rank_in_team(pool, current_best, team_name)
            .await
            .unwrap_or(1);
        let team_user_count = get_team_member_count(pool, team_name)
            .await
            .unwrap_or(1);
        if team_user_count > 0 {
            let denominator = team_user_count.max(100);
            let team_rank_percent = (team_user_rank as f64 / denominator as f64 * 100.0).ceil() as i64;
            team_rank_str = format!("{}%", team_rank_percent);
        }
    }

    Ok(Some(ScoreUpdateResult {
        added_team_score,
        is_new_record,
        score: current_best,
        rank: format!("{}%", user_rank_percent),
        team_rank: team_rank_str,
    }))
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
