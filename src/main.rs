use axum::{Json, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main(){
    dotenv::dotenv().ok();

    let database_url: String = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must provided");

    let database_pool: sqlx::PgPool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Faild Create Database PgPool");



    let app_state = ApplicationState {
        database: database_pool
    };


    let user_router_group = axum::Router::new()
        .route("/", post(handle_post_user))
        .with_state(app_state);

    let main_router = axum::Router::new()
        .nest("/api/v1/user", user_router_group);

    let listerner = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    axum::serve(listerner, main_router)
        .await
        .unwrap();
}




/**
 * User handlers
 */
async fn handle_post_user(State(app_state): State<ApplicationState> ,axum::Json(payload): Json<CreateUserRequest>) -> (StatusCode, Json<CreateUserResponse>){
    
    
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users(name, email, password, is_active)
            VALUES($1, $2, $3, $4)
             RETURNING id, name, email, is_active, created_at, updated_at",
        payload.name, payload.email, payload.password, payload.is_active
    ).fetch_one(&app_state.database)
    .await
    .expect("Failed Insert New User");
    
    return (StatusCode::CREATED, Json(CreateUserResponse{
        id: user.id,
        name: user.name,
        email: user.email,
        is_active: user.is_active,
        created_at: user.created_at,
        updated_at: user.updated_at
    }));
}



/*
 * Application State 
 */


 #[derive(Clone)]
 struct ApplicationState {
    database: sqlx::PgPool
 }



/*
 * USER SPEC SCHEMA AND DTO
 */


 #[derive(Deserialize)]
 struct CreateUserRequest {
    name: String,
    email: String,
    password: String,
    is_active: bool,
 }

 #[derive(Serialize)]
 struct CreateUserResponse {
    id: i64,
    name: Option<String>,
    email: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
 }


#[derive(Clone)]
struct User {
    id: i64,
    name: Option<String>,
    email: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}