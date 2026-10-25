use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::{CreateEventRequest, EventResponse, SummaryResponse},
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub async fn health() -> &'static str {
    "OK"
}

pub async fn ready(State(state): State<AppState>) -> Result<&'static str, ApiError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await?;
    Ok("READY")
}

pub async fn create_event(
    State(state): State<AppState>,
    payload: Result<Json<CreateEventRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<EventResponse>), ApiError> {
    let Json(payload) =
        payload.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let payload = payload.validate()?;
    let mut tx = state.db.begin().await?;

    let now = Utc::now();
    let metadata = payload
        .metadata
        .unwrap_or_else(|| serde_json::json!({}));

    let event = sqlx::query_as::<_, EventResponse>(
        r#"
        INSERT INTO events (id, user_id, category, title, content, metadata, logged_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        RETURNING id, user_id, category, title, content, metadata, logged_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(payload.user_id)
    .bind(payload.category)
    .bind(payload.title)
    .bind(payload.content)
    .bind(metadata)
    .bind(now)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23503") => {
            ApiError::BadRequest("unknown user_id".to_owned())
        }
        _ => ApiError::internal("insert event", error),
    })?;

    sqlx::query(
        r#"
        INSERT INTO user_summaries (
            user_id, category, total_count, last_logged_at, streak_days,
            aggregate_metrics, updated_at
        )
        VALUES ($1, $2, 1, $3, 1, '{}'::jsonb, $3)
        ON CONFLICT (user_id, category)
        DO UPDATE SET
            total_count = user_summaries.total_count + 1,
            streak_days = CASE
                WHEN user_summaries.last_logged_at IS NULL THEN 1
                WHEN (user_summaries.last_logged_at AT TIME ZONE 'UTC')::date
                    = (EXCLUDED.last_logged_at AT TIME ZONE 'UTC')::date
                    THEN user_summaries.streak_days
                WHEN (user_summaries.last_logged_at AT TIME ZONE 'UTC')::date
                    = (EXCLUDED.last_logged_at AT TIME ZONE 'UTC')::date - 1
                    THEN user_summaries.streak_days + 1
                ELSE 1
            END,
            last_logged_at = GREATEST(user_summaries.last_logged_at, EXCLUDED.last_logged_at),
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(payload.user_id)
    .bind(payload.category)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn get_summaries(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<SummaryResponse>>, ApiError> {
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::BadRequest("user_id must be a UUID".to_owned()))?;

    let user_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM users WHERE id = $1)",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    if !user_exists {
        return Err(ApiError::NotFound("user not found".to_owned()));
    }

    // Left join against the enum so every category is present, even before the
    // user has logged anything. This keeps the dashboard stable across clients.
    let summaries = sqlx::query_as::<_, SummaryResponse>(
        r#"
        SELECT
            c.category AS category,
            COALESCE(s.total_count, 0) AS total_count,
            s.last_logged_at,
            COALESCE(s.streak_days, 0) AS streak_days,
            COALESCE(s.aggregate_metrics, '{}'::jsonb) AS aggregate_metrics
        FROM unnest(enum_range(NULL::event_category)) WITH ORDINALITY AS c(category, ord)
        LEFT JOIN user_summaries s
            ON s.user_id = $1
            AND s.category = c.category
        ORDER BY c.ord
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(summaries))
}
