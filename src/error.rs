use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;


#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Resource not found {0}")]
    Notfound(String),
    #[error("Conflic {0}")]
    Conflict(String)
}


#[derive(Serialize)]
struct ApplicationErrorResponse {
    message: String
}

impl IntoResponse for ApplicationError {
    fn into_response(self) -> axum::response::Response {
        let (status, data) = match &self {
            ApplicationError::DatabaseError(e) => {
                eprintln!("InternalServerError: Database error {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApplicationErrorResponse {
                        message: format!("A database error occurred!")
                    }
                )

            },
            ApplicationError::Notfound(message) => {
                eprintln!("Notfound: {} notfound", &message);
                (
                    StatusCode::NOT_FOUND,
                    ApplicationErrorResponse {
                        message: format!("{}", &message)
                    }
                )
            },
            ApplicationError::Conflict(message) => {
                eprintln!("Conflic: {}", &message);
                (
                    StatusCode::CONFLICT,
                    ApplicationErrorResponse {
                        message: format!("{}", &message)
                    }
                )
            }
        };
        let body = Json(data);
        return (status, body).into_response();
    }
}