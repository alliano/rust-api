use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{AppendHeaders, IntoResponse};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use axum::{Json, Router};
use axum::routing::{get, post};
use tokio::sync::Mutex;


#[tokio::main]
async fn main() {    
    dotenv::dotenv().ok();

    let database_url: String = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must provided in .env");

    let database_pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to create database PgPool");


    let app_state = AppState {
        users: Arc::new(Mutex::new(Vec::new())),
        db_pool: database_pool
    };

    // membuat router group
    let user_group_router = Router::new()
        .route("/", get(get_users))
        .route("/", post(handle_post_user))
        .route("/profile", get(get_user_profile))
        .route("/{id}", get(handle_get_user_by_id))
        .route("/pages", get(handle_get_user_with_pagging))
        .route("/hobbie", post(handle_post_hobbie))
        .with_state(app_state);

    let app = Router::new()
        .nest("/v1/user", user_group_router);
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_users(State(app_state): State<AppState>) -> Json<Vec<User>> {
    let users = app_state.users.lock().await;
    Json(users.clone())
}


async fn get_user_profile() -> String {
    String::from("{name: Kim jeha, age: 23}")
}



// membuat http response
/*
 * axum::Json<T> ini sudah mengimplementasikan IntoResponse
 * Jadi semua data yang akan di response kan ke user harus implement IntoResponse 
 */
async fn  handle_get_user_by_id(Path(id): Path<u32>, State(app_state): State<AppState>)-> (StatusCode, Json<GetUserResponse>){
    
    let fetch_date: chrono::NaiveDate = sqlx::query_scalar("SELECT CURRENT_DATE")
        .fetch_one(&app_state.db_pool)
        .await
        .expect("Faild to fetch version");
    
    return (StatusCode::OK, Json(GetUserResponse { 
        id: id, 
        name: String::from("Abdillah Kim"),
        email: String::from("kim@gmail.com"),
        featch_date: fetch_date
    }));
    
}



async fn handle_get_user_with_pagging(Query(pagination): Query<Paggination>) -> String {
    format!("get user in page: {}, and page_size: {}", pagination.page, pagination.page_size)
}


async fn handle_post_user(State(app_state): State<AppState>, Json(payload): Json<CreateUserRequest>) -> (StatusCode, Json<CreateUserRespose>) {
    let mut users = app_state.users.lock().await;
    users.push(User { id: payload.id, name: payload.name.clone(), email: payload.email.clone(), password: payload.password.clone() });
     
    return (StatusCode::CREATED, Json(CreateUserRespose {
        id: payload.id,
        name: payload.name,
        email: payload.email,
        password: payload.password
    }));
}


async fn handle_post_hobbie(Json(hobbie): Json<CreateHobbie>) -> impl IntoResponse {
    return (
        StatusCode::CREATED,
        AppendHeaders([
            ("X-POWERED-BY", "AXUM"),
            ("API-VERSION", "V1")
        ]),
        Json(CreateHobbie {
            id: hobbie.id,
            name: hobbie.name
        })
    );
}


#[derive(Deserialize, Serialize)]
struct CreateHobbie {
    id: u32,
    name: String
}

#[derive(Deserialize)]
struct CreateUserRequest {
    id: u32,
    name: String,
    email: String,
    password: String
}

#[derive(Serialize)]
struct CreateUserRespose {
    id: u32,
    name: String,
    email: String,
    password: String
}

#[derive(Serialize)]
struct GetUserResponse {
    id: u32,
    name: String,
    email: String,
    featch_date: chrono::NaiveDate
}



#[derive(Deserialize)]
struct  Paggination {
    page: u32,
    page_size: u32
}


#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: u32,
    name: String,
    email: String,
    password: String
}

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<Vec<User>>>,
    db_pool: sqlx::PgPool
}