use axum::routing::{patch, post, get};
use crate::{dto::ApplicationState, handles::{handle_get_user_by_id, handle_patch_user, handle_post_user}};

mod error;
mod dto;
mod handles;

#[tokio::main]
async fn main(){
    dotenv::dotenv().ok();


    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL shoudl provided");

    let database_pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Faild connect to database");

    let app_state = ApplicationState {
        database: database_pool
    };

    let group_user_router = axum::Router::new()
        .route("/", post(handle_post_user))
        .route("/{id}", patch(handle_patch_user))
        .route("/{id}", get(handle_get_user_by_id))
        .with_state(app_state);

    let router = axum::Router::new()
        .nest("/api/v1/user", group_user_router);
   
   let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
    .await
    .unwrap();

    axum::serve(listener, router).await.unwrap();

}


