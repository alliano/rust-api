use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SuccessResponse<T> {
    pub message: String,
    pub payload: T
}

#[derive(Clone)]
pub struct ApplicationState {
    pub database: sqlx::PgPool
}


#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: Option<String>,
    pub email: String,
    pub password: String,
    pub is_active: bool
}


#[derive(Serialize)]
pub struct CreateUserResponse {
    pub id: i64,
    pub name: Option<String>,
    pub email: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub is_active: Option<bool>
}