use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Entity not found: {0}")]
    NotFound(u64),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Database(#[from] ents::DatabaseError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Audit error: {0}")]
    Audit(#[from] ents_admin::AuditError),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("{0}")]
    RequestValidation(String),
}

/// Custom JSON extractor that provides user-friendly error messages
pub struct AppJson<T>(pub T);

impl<S, T> FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(
        req: Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(AppJson(value)),
            Err(rejection) => Err(transform_json_error(rejection)),
        }
    }
}

fn transform_json_error(rejection: JsonRejection) -> ApiError {
    let message = match &rejection {
        JsonRejection::JsonDataError(err) => {
            let serde_err = err.body_text();
            transform_serde_error(&serde_err)
        }
        JsonRejection::JsonSyntaxError(err) => {
            format!("Invalid JSON syntax: {}", err.body_text())
        }
        JsonRejection::MissingJsonContentType(_) => {
            "Missing Content-Type header. Expected application/json".to_string()
        }
        JsonRejection::BytesRejection(err) => {
            format!("Failed to read request body: {}", err.body_text())
        }
        other => {
            // Extract meaningful error from the rejection
            let body = other.body_text();
            if body.is_empty()
                || body == "Unprocessable Entity"
                || body == "Bad Request"
            {
                "Invalid entity data. Check that all required fields are provided.".to_string()
            } else {
                transform_serde_error(&body)
            }
        }
    };
    ApiError::RequestValidation(message)
}

fn transform_serde_error(err: &str) -> String {
    // Handle typetag "unknown variant" errors for entity types
    if err.contains("unknown variant") {
        if let Some(variant) = extract_unknown_variant(err) {
            return format!(
                "Unknown entity type: \"{}\". Check that the \"type\" field matches a registered entity type.",
                variant
            );
        }
    }

    // Handle missing field errors
    if err.contains("missing field") {
        if let Some(field) = extract_missing_field(err) {
            return format!(
                "Missing required field: \"{}\". Please include this field in your entity JSON.",
                field
            );
        }
    }

    // Handle invalid type errors
    if err.contains("invalid type") {
        return format!("Type mismatch in JSON: {}", simplify_type_error(err));
    }

    // Return a simplified version of the original error
    simplify_error_message(err)
}

fn extract_unknown_variant(err: &str) -> Option<&str> {
    // Pattern: "unknown variant `SomeName`"
    let start = err.find("unknown variant `")? + "unknown variant `".len();
    let rest = &err[start..];
    let end = rest.find('`')?;
    Some(&rest[..end])
}

fn extract_missing_field(err: &str) -> Option<&str> {
    // Pattern: "missing field `field_name`"
    let start = err.find("missing field `")? + "missing field `".len();
    let rest = &err[start..];
    let end = rest.find('`')?;
    Some(&rest[..end])
}

fn simplify_type_error(err: &str) -> String {
    // Try to extract the key part of the type error
    if let Some(start) = err.find("invalid type:") {
        let relevant = &err[start..];
        if let Some(end) = relevant.find(" at line") {
            return relevant[..end].to_string();
        }
        return relevant.to_string();
    }
    err.to_string()
}

fn simplify_error_message(err: &str) -> String {
    // Remove line/column information for cleaner messages
    if let Some(idx) = err.find(" at line") {
        return err[..idx].to_string();
    }
    err.to_string()
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Serialization(_) => StatusCode::BAD_REQUEST,
            ApiError::RequestValidation(_) => StatusCode::BAD_REQUEST,
            ApiError::Audit(e) => match e {
                ents_admin::AuditError::EntityNotFound(_) => {
                    StatusCode::NOT_FOUND
                }
                ents_admin::AuditError::UnexpectedEntityType(_, _) => {
                    StatusCode::BAD_REQUEST
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            error: self.to_string(),
        });

        (status, body).into_response()
    }
}
