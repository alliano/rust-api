use axum::{Json, extract::{Path, State}, http::StatusCode, routing::{delete, get, post, put}};
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
        .route("/", get(handle_get_users))
        .route("/{id}", get(handle_get_user_by_id))
        .route("/{id}", put(handle_put_user))
        .route("/{id}", delete(handle_delete_user_by_id))
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
async fn handle_post_user(State(app_state): State<ApplicationState> ,axum::Json(payload): Json<CreateUserRequest>) -> Result<Json<CreateUserResponse>, (StatusCode, Json<ErrorResponse>)>{
    let user = sqlx::query_as!(
        CreateUserResponse,
        "INSERT INTO users(name, email, password, is_active)
            VALUES($1, $2, $3, $4)
             RETURNING id, name, email, is_active, created_at, updated_at",
        payload.name, payload.email, payload.password, payload.is_active
    ).fetch_one(&app_state.database)
    .await;

    match user {
        Ok(u) => Ok(Json(CreateUserResponse { id: u.id, name: u.name, email: u.email, is_active: u.is_active, created_at: u.created_at, updated_at: u.updated_at })),
        Err(e) => Err(
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { message: e.to_string() }))
        )
    }
}


async fn handle_get_user_by_id(State(app_state): State<ApplicationState>, Path(id): Path<i64>) -> Result<Json<CreateUserResponse>, (StatusCode, Json<ErrorResponse>)>{
    let user = sqlx::query_as!(
        CreateUserResponse,
        "SELECT u.id AS id, u.name AS name, u.email AS email, u.is_active AS is_active, u.created_at AS created_at, u.updated_at AS updated_at FROM users AS u WHERE u.id = $1",
        &id
    )
    .fetch_one(&app_state.database)
    .await;

    match user {
       Ok(u) => Ok(Json(CreateUserResponse {
        id: u.id,
        name: u.name,
        email: u.email,
        is_active: u.is_active,
        created_at: u.created_at,
        updated_at: u.updated_at
       })),
       Err(_) => Err(
        (StatusCode::NOT_FOUND, Json(ErrorResponse {
            message: format!("user with id {} notfound", &id)
        }))
       )
    }
}


async fn handle_get_users(State(app_state): State<ApplicationState>) -> Result<Json<Vec<CreateUserResponse>>, (StatusCode, Json<ErrorResponse>)> {
    
    let users = sqlx::query_as!(
        CreateUserResponse,
        "SELECT u.id AS id, u.name AS name, u.email AS email, u.is_active AS is_active, u.created_at AS created_at, updated_at AS updated_at FROM users AS u ORDER BY u.id",
    ).fetch_all(&app_state.database)
    .await;

    match users {
        Ok(users) => {
            let response: Vec<CreateUserResponse> = users.into_iter().map(|user| CreateUserResponse {
                id: user.id,
                name: user.name,
                email: user.email,
                is_active: user.is_active,
                created_at: user.created_at,
                updated_at: user.updated_at
            }).collect();
           Ok(Json(response))
        },
        Err(_) => Err(
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                message: String::from("Faild to get all users")
            }))
        )
    }

}


async fn handle_put_user(Path(id): Path<i64>, State(app_state): State<ApplicationState>, Json(payload): Json<UpdateUserRequest>) -> Result<Json<CreateUserResponse>, (StatusCode, Json<ErrorResponse>)> {

    // periksa apakah user dengan id tersebut ada di database apa enga
    let user = sqlx::query_as!(
        CreateUserResponse,
        "SELECT u.id AS id, u.name AS name, u.email AS email, u.is_active AS is_active, u.created_at AS created_at, u.updated_at AS updated_at FROM users AS u WHERE u.id = $1",
        &id
    ).fetch_one(&app_state.database)
    .await;

    match user {
        Ok(_) => {
            let updated = sqlx::query_as!(
                CreateUserResponse,
                "UPDATE users SET name = $1, email = $2, password = $3, is_active = $4 WHERE id = $5 RETURNING id, name, email, is_active, created_at, updated_at",
                payload.name, payload.email, payload.password, payload.is_active, &id
            ).fetch_one(&app_state.database)
            .await;

            match updated {
                Ok(u) => Ok(Json(CreateUserResponse{
                    id: u.id,
                    name: u.name,
                    email: u.email,
                    is_active: u.is_active,
                    created_at: u.created_at,
                    updated_at: u.updated_at
                })),
                Err(_) => Err(
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                        message: format!("Faild update user with id {}", &id)
                    }))
                )
            }

        },
        Err(_) => Err(
            (StatusCode::NOT_FOUND, Json(ErrorResponse { message: format!("user with id {} notfound", &id) }))
        )
    }

}


async fn handle_delete_user_by_id(Path(id): Path<i64>, State(app_state): State<ApplicationState>) -> Result<Json<CreateUserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let is_exist = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM users AS u WHERE u.id = $1)", &&id).fetch_one(&app_state.database)
        .await;
    match is_exist {
        Ok(Some(true)) => {
            match sqlx::query_as!(CreateUserResponse, "DELETE FROM users AS u WHERE u.id = $1 RETURNING id, name, email, is_active, created_at, updated_at", &id).fetch_one(&app_state.database).await {
                Ok(deleted_user) => Ok(Json(deleted_user)),
                Err(_) => Err(
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { message: format!("failde delete user with id {}", &id) }))
                )
            }
        },
        Ok(Some(false)) => {
            Err(
                (StatusCode::NOT_FOUND, Json(ErrorResponse { message: format!("user with id {} notfound", &id) }))
            )
        },
        Ok(None) => {
            Err(
                (StatusCode::NOT_FOUND, Json(ErrorResponse { message: format!("user with id {} notfound", &id) }))
            )
        },
        Err(_) => {
            Err(
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { message: format!("Faild Delete user") }))
            )
        }
    }

}
#[derive(Serialize)]
struct ErrorResponse {
    message: String
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

 #[derive(Deserialize)]
 struct UpdateUserRequest {
    name: Option<String>,
    email: String,
    password: String,
    is_active: bool
 }