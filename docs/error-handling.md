# Catatan Belajar: Elegant Error Handling di Axum + SQLx

Ringkasan pembelajaran soal bikin error handling yang rapi (gak numpuk `match`/`unwrap()` di tiap handler) di branch `feat/error-handling`, plus kenapa `.await` itu wajib dan gimana efeknya kalau lupa.

## 1. Kenapa Gak Cuma `.unwrap()` di Semua Tempat?

Kalau tiap query pakai `.unwrap()`/`.expect()`, satu error DB (row not found, koneksi putus, dll) bikin handler **panic** → axum balikin response 500 generik tanpa detail, dan proses request itu crash di tengah jalan. Gak ada cara ngasih tau client "kenapa" gagalnya (404? 409? 400?).

Solusinya: pakai `Result<T, E>` sebagai return type handler, dengan `E` adalah tipe error custom milik aplikasi sendiri.

## 2. Bikin Tipe Error Terpusat dengan `thiserror`

```rust
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Resource not found {0}")]
    Notfound(String),
    #[error("Conflic {0}")]
    Conflict(String),
}
```

- `#[derive(thiserror::Error)]` — otomatis implement trait `std::error::Error` + `Display` (pesan errornya diambil dari `#[error("...")]`).
- `#[from] sqlx::Error` — ini kuncinya. Dengan atribut ini, `sqlx::Error` bisa otomatis ter-convert jadi `ApplicationError::DatabaseError(...)` lewat `?` operator, tanpa perlu `.map_err(...)` manual di tiap query.

## 3. Satu Tempat untuk Mapping Error → HTTP Response

Alih-alih tiap handler nentuin sendiri status code & body error, cukup implement `IntoResponse` **sekali** untuk `ApplicationError`:

```rust
#[derive(Serialize)]
struct ApplicationErrorResponse {
    message: String,
}

impl IntoResponse for ApplicationError {
    fn into_response(self) -> axum::response::Response {
        let (status, data) = match &self {
            ApplicationError::DatabaseError(e) => {
                eprintln!("InternalServerError: Database error {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, ApplicationErrorResponse {
                    message: format!("A database error occurred!"),
                })
            }
            ApplicationError::Notfound(message) => {
                (StatusCode::NOT_FOUND, ApplicationErrorResponse { message: message.clone() })
            }
            ApplicationError::Conflict(message) => {
                (StatusCode::CONFLICT, ApplicationErrorResponse { message: message.clone() })
            }
        };
        (status, Json(data)).into_response()
    }
}
```

Poin penting soal keamanan: untuk `DatabaseError`, pesan detail dari `sqlx::Error` (`e`) di-log ke server (`eprintln!`) tapi **tidak** dikirim ke client — client cuma dikasih pesan generik "A database error occurred!". Ini nyegah bocornya detail internal (nama tabel, struktur query, dll) ke luar.

## 4. Handler Jadi Bersih Berkat `?` Operator

Dengan `ApplicationError` yang implement `From<sqlx::Error>` (lewat `#[from]`) dan `IntoResponse`, handler jadi flat, gak perlu nested `match`:

```rust
pub async fn handle_post_user(
    State(app_state): State<ApplicationState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<SuccessResponse<CreateUserResponse>>, ApplicationError> {

    let is_exist = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users AS u WHERE u.email = $1)", &payload.email
    )
    .fetch_one(&app_state.database)
    .await?;   // <- error DB otomatis jadi ApplicationError::DatabaseError

    if is_exist.unwrap_or(false) {
        return Err(ApplicationError::Conflict(format!(
            "email with {} alredy registered, use another email", &payload.email
        )));
    }

    let user = sqlx::query_as!(
        CreateUserResponse,
        "INSERT INTO users(name, email, password, is_active)
            VALUES($1, $2, $3, $4) RETURNING id, name, email, is_active, created_at, updated_at",
        payload.name, payload.email, payload.password, payload.is_active
    )
    .fetch_one(&app_state.database)
    .await?;

    Ok(Json(SuccessResponse {
        message: String::from("Successfully create new user"),
        payload: user,
    }))
}
```

Kalau ada error di titik manapun (`?`), function langsung berhenti dan return `Err(ApplicationError::...)` — Axum otomatis panggil `into_response()` yang sudah kita definisikan sekali di langkah 3, tanpa perlu handler ini tau soal `StatusCode` atau `Json` error sama sekali.

## 5. Kenapa `.await` Wajib Setelah Query

`sqlx::query_scalar!(...)`/`sqlx::query_as!(...)` cuma bikin *query builder* — belum benar-benar jalan ke database. Baru setelah dirantai dengan `.fetch_one()/.fetch_optional()/.fetch_all()` dan **di-`.await`**, query itu benar-benar dieksekusi dan hasilnya jadi `Result<T, sqlx::Error>`.

Lupa `.await` bikin error type mismatch:
```
mismatched types
expected opaque type `impl Future<Output = Result<..., sqlx::Error>>`
          found enum `Result<_, _>`
```

Karena tanpa `.await`, yang kamu pegang itu `Future` (representasi kerjaan yang belum dijalankan), bukan hasil akhirnya. Rust gak auto-`await` — ini bagian dari model `async`/`await` eksplisit, beda dari bahasa lain yang auto-resolve promise.

Kalau ditulis pakai `?` sekaligus:
```rust
let is_exist = sqlx::query_scalar!(...)
    .fetch_one(&app_state.database)
    .await?;
//              ^^^^^ jalankan query, tunggu hasilnya
//                    ^ kalau Err, convert ke ApplicationError & return lebih awal
```
Dua simbol ini punya peran beda: `.await` nunggu future selesai jadi `Result`, `?` yang membongkar `Result` itu (return `Err` lebih awal, atau lanjut dengan value `Ok`-nya).

## 6. Cek "Ada/Tidak Ada" dengan `let-else`

Untuk `fetch_optional()` (hasil `Option<T>`), cara paling ringkas nge-handle "kalau gak ketemu, return 404" adalah `let-else`:

```rust
let user = sqlx::query_as!(CreateUserResponse, "SELECT ... WHERE id = $1", &id)
    .fetch_optional(&app_state.database)
    .await?;

let Some(user) = user else {
    return Err(ApplicationError::Notfound(format!("user with id {} notfound", &id)));
};

// di sini `user` sudah pasti `CreateUserResponse`, bukan `Option` lagi
```

## Ringkasan Pola

| Tanpa error handling rapi | Dengan `thiserror` + `IntoResponse` |
|---|---|
| `.unwrap()` di mana-mana → panic kalau gagal | `?` operator, error di-propagate rapi |
| Tiap handler nentuin status code sendiri | Satu `impl IntoResponse` terpusat |
| Pesan error DB bisa kebocor ke client | Detail di-log di server, client dapat pesan generik |
| Susah tau kenapa gagal (404? 409? 500?) | Tiap varian error punya makna & status code sendiri |
