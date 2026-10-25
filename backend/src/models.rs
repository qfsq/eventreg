use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

pub const MAX_TITLE_CHARS: usize = 255;
pub const MAX_CONTENT_CHARS: usize = 20_000;
pub const MAX_METADATA_BYTES: usize = 16 * 1024;

#[derive(Debug, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "event_category", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum EventCategory {
    Chore,
    Work,
    Emotion,
    Thinking,
    Writing,
    Inventing,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub user_id: Uuid,
    pub category: EventCategory,
    pub title: String,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl CreateEventRequest {
    /// Normalizes user input and rejects payloads that break the schema contract.
    pub fn validate(mut self) -> Result<Self, ApiError> {
        self.title = self.title.trim().to_owned();
        if self.title.is_empty() {
            return Err(ApiError::BadRequest("title must not be empty".to_owned()));
        }
        if self.title.chars().count() > MAX_TITLE_CHARS {
            return Err(ApiError::BadRequest(format!(
                "title must be at most {MAX_TITLE_CHARS} characters"
            )));
        }

        self.content = self
            .content
            .map(|content| content.trim().to_owned())
            .filter(|content| !content.is_empty());
        if let Some(content) = &self.content {
            if content.chars().count() > MAX_CONTENT_CHARS {
                return Err(ApiError::BadRequest(format!(
                    "content must be at most {MAX_CONTENT_CHARS} characters"
                )));
            }
        }

        if let Some(metadata) = &self.metadata {
            if !metadata.is_object() {
                return Err(ApiError::BadRequest(
                    "metadata must be a JSON object".to_owned(),
                ));
            }
            let bytes = serde_json::to_vec(metadata)
                .map_err(|error| ApiError::internal("serialize metadata", error))?;
            if bytes.len() > MAX_METADATA_BYTES {
                return Err(ApiError::BadRequest(format!(
                    "metadata must be at most {MAX_METADATA_BYTES} bytes"
                )));
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EventResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: EventCategory,
    pub title: String,
    pub content: Option<String>,
    pub metadata: serde_json::Value,
    pub logged_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SummaryResponse {
    pub category: EventCategory,
    pub total_count: i64,
    pub last_logged_at: Option<DateTime<Utc>>,
    pub streak_days: i32,
    pub aggregate_metrics: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(title: &str) -> CreateEventRequest {
        CreateEventRequest {
            user_id: Uuid::nil(),
            category: EventCategory::Writing,
            title: title.to_owned(),
            content: None,
            metadata: None,
        }
    }

    #[test]
    fn trims_title_and_content() {
        let mut request = request("  Drafted chapter  ");
        request.content = Some("  notes  ".to_owned());

        let validated = request.validate().expect("valid request");
        assert_eq!(validated.title, "Drafted chapter");
        assert_eq!(validated.content.as_deref(), Some("notes"));
    }

    #[test]
    fn rejects_blank_title() {
        assert!(request("   ").validate().is_err());
    }

    #[test]
    fn rejects_overlong_title() {
        assert!(request(&"x".repeat(MAX_TITLE_CHARS + 1)).validate().is_err());
    }

    #[test]
    fn drops_blank_content() {
        let mut request = request("Draft");
        request.content = Some("   ".to_owned());

        assert_eq!(request.validate().unwrap().content, None);
    }

    #[test]
    fn rejects_non_object_metadata() {
        let mut request = request("Draft");
        request.metadata = Some(serde_json::json!(["not", "an", "object"]));

        assert!(request.validate().is_err());
    }

    #[test]
    fn serializes_categories_as_lowercase() {
        let json = serde_json::to_string(&EventCategory::Inventing).unwrap();
        assert_eq!(json, "\"inventing\"");
    }
}
