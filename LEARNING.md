# Catatan Belajar: CRUD REST API dengan Axum + SQLx

Ringkasan hal-hal yang dipelajari dan error yang dihadapi saat membangun endpoint CRUD `users` di `src/main.rs`.

## 1. Struktur Routing (Axum)

Router disusun berlapis: sub-router per resource (`user_router_group`) di-`nest` ke router utama.

```rust
let user_router_group = axum::Router::new()
    .route("/", post(handle_post_user))
    .route("/", get(handle_get_users))
    .route("/{id}", get(handle_get_user_by_id))
    .route("/{id}", put(handle_put_user))
    .route("/{id}", delete(handle_delete_user_by_id))
    .with_state(app_state);

let main_router = axum::Router::new()
    .nest("/api/v1/user", user_router_group);
```

Path parameter (`{id}`) diambil pakai extractor `Path<T>`, misalnya `Path(id): Path<i64>`.

## 2. Tipe Data untuk Path Parameter Harus Cocok dengan Postgres

**Error yang muncul:**
```
the trait bound `u64: sqlx::Encode<'_, Postgres>` is not satisfied
```

**Penyebab:** Postgres tidak punya tipe integer *unsigned*. SQLx cuma implement `Encode`/`Decode` untuk tipe signed (`i16`, `i32`, `i64`, dst). Kolom `id` di tabel bertipe `BIGINT`, jadi harus dipetakan ke `i64` di Rust, bukan `u32`/`u64`.

**Fix:** ganti `Path<u64>` → `Path<i64>`.

## 3. `query_as!` vs `query_scalar!`

- `sqlx::query_as!(Struct, ...)` → dipakai kalau hasil query dipetakan ke **struct** (punya beberapa kolom).
- `sqlx::query_scalar!(...)` → dipakai kalau hasil query cuma **satu kolom/nilai tunggal**, misalnya `SELECT COUNT(*)` atau `SELECT EXISTS(...)`.

**Error kalau salah pakai:**
```
expected struct, variant or union type, found builtin type `i64`
```
Ini muncul karena `query_as!` butuh target berupa struct, bukan tipe primitif seperti `i64`.

## 4. `query_as!`/`query_scalar!` Belum Jalan Sebelum `.fetch_*().await`

Manggil `sqlx::query_as!(...)` doang **belum** menjalankan query — itu baru bikin query builder (`sqlx::query::Map<...>`). Harus dirantai dengan salah satu:

- `.fetch_one(&pool).await` → 1 row, error kalau tidak ketemu / lebih dari 1
- `.fetch_optional(&pool).await` → 0 atau 1 row (`Option<T>`)
- `.fetch_all(&pool).await` → banyak row (`Vec<T>`)

Baru setelah itu hasilnya jadi `Result<T, sqlx::Error>` yang bisa di-`match`.

**Error kalau lupa:**
```
mismatched types
expected struct `sqlx::query::Map<...>`
     found enum `Result<_, _>`
```

## 5. Handling "Data Tidak Ditemukan"

- `.fetch_one()` + `.unwrap()` → kalau row tidak ada, langsung **panic** (crash), response ke client jadi 500 tanpa pesan yang jelas.
- `.fetch_optional()` → aman, hasilnya `Option<T>`, bisa di-`match` untuk membedakan `Some(data)` (200 OK) vs `None` (404 Not Found).

## 6. Return Type Handler: `impl IntoResponse` vs `Result<T, E>`

Axum handler boleh return apa saja yang implement trait `IntoResponse`. Dua pola yang dipakai:

**Pola A — `impl IntoResponse` + manual `.into_response()`:**
```rust
async fn handler(...) -> impl IntoResponse {
    match hasil {
        Some(data) => (StatusCode::OK, Json(data)).into_response(),
        None => (StatusCode::NOT_FOUND, Json(ErrorResponse { .. })).into_response(),
    }
}
```
Aturan penting: `.into_response()` harus membungkus **seluruh tuple** `(StatusCode, Json<...>)`, bukan cuma bagian `Json`-nya saja — kalau tidak, tipe tiap cabang `match` tidak seragam (`(StatusCode, Response<Body>)` vs `Response<Body>`).

**Pola B — `Result<T, E>` (lebih rapi kalau tipe sukses/error tetap):**
```rust
async fn handler(...) -> Result<Json<Vec<CreateUserResponse>>, (StatusCode, Json<ErrorResponse>)> {
    match hasil {
        Ok(data) => Ok(Json(data)),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { .. }))),
    }
}
```
`Result<T, E>` otomatis implement `IntoResponse` selama `T: IntoResponse` dan `E: IntoResponse` — Axum yang convert ke response HTTP.

Perhatikan urutan tipe generic: `Json<Vec<T>>` (satu response array) itu beda dari `Vec<Json<T>>` (Axum tidak implement `IntoResponse` untuk bentuk ini).

## 7. Aturan Titik Koma (`;`) di Akhir Block

Baris terakhir dalam sebuah block `{ ... }` **tanpa** `;` menjadi nilai return block tersebut. Kalau dikasih `;`, block itu jadi statement biasa dan return `()` (unit) sebagai gantinya.

```rust
Ok(users) => {
    let response = ...;
    Ok(Json(response))   // benar — tanpa `;`, ini jadi nilai return match arm
    // Ok(Json(response));  <- salah, bikin arm return () bukan Result
}
```

## 8. WHERE Clause Dinamis (`ILIKE`) — Tetap Pakai Bind Parameter

Jangan format string langsung ke SQL (rawan SQL injection). Format dulu di Rust, baru bind sebagai parameter:

```rust
let pattern = format!("%{}%", params.name);
sqlx::query_scalar!(
    r#"SELECT COUNT(*) AS "total!: i64" FROM users AS u WHERE u.name ILIKE $1"#,
    pattern
)
```

## 9. Warning `field is never read`

Kalau struct punya field yang di-*select* dari DB tapi tidak pernah dipakai/di-return (misalnya `password` di struct `User`/response), compiler kasih warning dead code. Solusi paling bersih: jangan select kolom yang memang tidak dipakai di response tersebut, atau pisahkan struct khusus untuk kebutuhan yang benar-benar butuh field itu (misal proses login).

## 10. Cek "Exists" Sebelum Delete — Hindari Race Condition

Pola awal: query `SELECT EXISTS(...)` dulu, baru `DELETE` terpisah. Ini 2 round-trip ke database dan ada celah waktu antara cek dan hapus (row bisa saja sudah dihapus request lain di antaranya).

Alternatif yang lebih ringkas & aman — langsung `DELETE ... RETURNING` dengan `fetch_optional`, tanpa query cek terpisah:

```rust
let deleted = sqlx::query_as!(
    CreateUserResponse,
    "DELETE FROM users WHERE id = $1 RETURNING id, name, email, is_active, created_at, updated_at",
    id
)
.fetch_optional(&app_state.database)
.await;

match deleted {
    Ok(Some(user)) => Ok(Json(user)),
    Ok(None) => Err((StatusCode::NOT_FOUND, Json(ErrorResponse { .. }))),
    Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { .. }))),
}
```

## Ringkasan Endpoint yang Sudah Dibuat

| Method | Path              | Handler                     | Keterangan               |
|--------|--------------------|------------------------------|---------------------------|
| POST   | `/api/v1/user`      | `handle_post_user`           | Create user               |
| GET    | `/api/v1/user`      | `handle_get_users`           | List semua user           |
| GET    | `/api/v1/user/{id}` | `handle_get_user_by_id`      | Get user by id (404 jika tidak ada) |
| PUT    | `/api/v1/user/{id}` | `handle_put_user`            | Update user by id         |
| DELETE | `/api/v1/user/{id}` | `handle_delete_user_by_id`   | Delete user by id         |
