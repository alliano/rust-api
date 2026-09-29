use axum::{Json, extract::{Path, State}};


use crate::{dto::{ApplicationState, CreateUserRequest, CreateUserResponse, SuccessResponse, UpdateUserRequest}, error::ApplicationError};

// handle user
pub async fn handle_post_user(State(app_state): State<ApplicationState>, Json(payload): Json<CreateUserRequest>) -> Result<Json<SuccessResponse<CreateUserResponse>>, ApplicationError> {
    let is_exist = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM users AS u WHERE u.email = $1)", &payload.email)
        .fetch_one(&app_state.database)
        .await?;

    if is_exist.unwrap() {
        return Err(ApplicationError::Conflict(format!("email with {} alredy registered, user another email", &payload.email)));
    }

    let user = sqlx::query_as!(
        CreateUserResponse,
        "INSERT INTO users(name, email, password, is_active) 
            VALUES($1, $2, $3, $4) RETURNING id, name, email, is_active, created_at, updated_at",
            payload.name, payload.email, payload.password, payload.is_active
    ).fetch_one(&app_state.database)
    .await?;

    Ok(Json(SuccessResponse { message: String::from("Successfully create new user"), payload: user }))
}

pub async fn handle_patch_user(State(app_state): State<ApplicationState>, Path(id): Path<i64>, Json(payload): Json<UpdateUserRequest>) -> Result<Json<SuccessResponse<CreateUserResponse>>, ApplicationError>{
    let is_exist = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users AS u WHERE id = $1)", &id
    )
    .fetch_one(&app_state.database)
    .await?;

    if !is_exist.unwrap() {
        return Err(ApplicationError::Notfound(format!("user with id {} notfound", &id)));
    }


    if let Some(email) = &payload.email {
        let is_email_exist = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users AS u WHERE u.email = $1)", payload.email
        )
        .fetch_one(&app_state.database)
        .await?;

        if is_email_exist.unwrap_or(false){
            return Err(ApplicationError::Conflict(format!("email {} alredy used, please use another email", email)));
        }        
    }

    let updated_user = sqlx::query_as!(
        CreateUserResponse,
        "UPDATE users 
            SET name = COALESCE($1, name),
                email = COALESCE($2, email),
                password = COALESCE($3, password),
                is_active = COALESCE($4, is_active)
        WHERE id = $5 RETURNING id, name, email, is_active, created_at, updated_at",
        payload.name, payload.email, payload.password, payload.is_active, &id
    )
    .fetch_one(&app_state.database)
    .await?;

    Ok(Json(SuccessResponse { message: String::from("Success fully update user"), payload: updated_user }))
}

pub async fn handle_get_user_by_id(State(app_state): State<ApplicationState>, Path(id): Path<i64>) -> Result<Json<SuccessResponse<CreateUserResponse>>, ApplicationError> {
    let is_user_exist = sqlx::query_as!(
        CreateUserResponse,
        "SELECT 
            u.id AS id, u.name AS name, u.email AS email, u.is_active AS is_active, u.created_at AS created_at, u.updated_at AS updated_at
        FROM users AS u WHERE u.id = $1",
        &id
    )
    .fetch_optional(&app_state.database)
    .await?;

   let user = match is_user_exist {
       Some(user) => user,
       None => return Err(ApplicationError::Notfound(format!("user with id {} notfound", &id)))
   };

   Ok(Json(
    SuccessResponse { message: String::from("successfully get user by id"), payload: user }
   ))
}   