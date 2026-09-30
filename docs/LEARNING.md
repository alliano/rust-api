# Catatan Belajar: Dasar-Dasar Axum (branch `master`)

Ringkasan konsep dasar yang dipelajari dari `src/main.rs` di branch `master` — sebelum masuk ke topik lanjutan seperti CRUD (`feat/crud`) atau error handling (`feat/error-handling`).

## 0. Instalasi Dependency

Semua dependency di project ini ditambahkan pakai `cargo add`, bukan diketik manual di `Cargo.toml`:

```bash
# Web framework
cargo add axum

# Async runtime (wajib untuk axum) — aktifkan semua fitur
cargo add tokio --features full

# Serialisasi/deserialisasi JSON
cargo add serde --features derive

# Load variable dari file .env
cargo add dotenv

# Tipe tanggal/waktu + dukungan serde
cargo add chrono --features serde

# Driver Postgres async + query macro
cargo add sqlx --features runtime-tokio,postgres,chrono,macros
```

Penjelasan flag `--features`:
- `tokio --features full` — Tokio modular, defaultnya minim fitur. `full` mengaktifkan semua (networking, macro `#[tokio::main]`, dll) — cukup buat belajar, nanti bisa dipersempit (`rt-multi-thread`, `macros`, `net`, dst) kalau mau optimasi ukuran binary.
- `serde --features derive` — biar bisa pakai `#[derive(Serialize, Deserialize)]` di struct sendiri.
- `sqlx --features runtime-tokio,postgres,chrono,macros`:
  - `runtime-tokio` — SQLx butuh tau async runtime apa yang dipakai (di sini Tokio).
  - `postgres` — driver khusus Postgres (SQLx juga support MySQL/SQLite lewat feature lain).
  - `chrono` — biar tipe kolom `TIMESTAMPTZ`/`DATE` bisa otomatis dipetakan ke `chrono::DateTime`/`NaiveDate`.
  - `macros` — biar bisa pakai `sqlx::query!`/`query_as!`/`query_scalar!` (query yang dicek ke database saat kompilasi).

### `sqlx-cli` — Tools untuk Migration

`sqlx-cli` itu **binary terpisah** (bukan library/dependency biasa), dipakai buat generate & menjalankan file migration SQL:

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

Kalau mau tetap dicatat sebagai project dependency (opsional, biar konsisten versi antar developer), bisa juga ditambahkan ke `Cargo.toml` lewat:

```bash
cargo add sqlx-cli --no-default-features --features postgres,rustls
```

Perintah `sqlx-cli` yang biasa dipakai:

```bash
# Bikin file migration baru (kosong, isi manual SQL-nya)
sqlx migrate add create_users_table

# Jalankan semua migration yang belum di-apply ke database
sqlx migrate run

# Cek status migration
sqlx migrate info
```

> Catatan: `sqlx::query_as!`/`query_scalar!` (macro, pakai tanda `!`) butuh **koneksi database aktif saat kompilasi** (`DATABASE_URL` di `.env` harus valid & tabel-nya sudah ada) — beda dari `sqlx::query_as`/`query_scalar` (fungsi biasa, tanpa `!`) yang gak dicek ke database saat compile time.

## 1. `#[tokio::main]` dan `async fn main`

```rust
#[tokio::main]
async fn main() {
    // ...
}
```

- Rust `main()` biasa itu **sync**, padahal semua yang berhubungan dengan I/O (network, database) di Axum/SQLx bersifat **async**.
- `#[tokio::main]` adalah macro yang men-generate runtime Tokio dan membungkus `main()` supaya body-nya (yang `async`) bisa dijalankan. Tanpa ini, `async fn main()` gak akan bisa dieksekusi langsung oleh Rust.
- Di dalam `main`, kita bisa `.await` hal-hal async seperti koneksi database dan `axum::serve(...)`.

## 2. State (`AppState`)

```rust
#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<Vec<User>>>,
    db_pool: sqlx::PgPool,
}
```

State adalah data yang **dibagikan** ke semua handler — di sini isinya list user in-memory (`users`) dan connection pool database (`db_pool`).

Poin penting:
- **`#[derive(Clone)]` wajib** — Axum meng-clone state untuk tiap request yang masuk (di-share antar task async secara concurrent), makanya state harus murah untuk di-clone.
- **`Arc<Mutex<T>>`** dipakai supaya data bisa **dibagikan** (`Arc` = shared ownership, aman dipakai lintas thread) sekaligus **dimodifikasi** dengan aman dari banyak request bersamaan (`Mutex` = lock, cegah race condition saat baca/tulis `Vec<User>`).
- `sqlx::PgPool` sendiri sudah aman di-clone (internal-nya connection pool yang di-share), gak perlu dibungkus `Arc`/`Mutex` lagi.
- State di-attach ke router lewat `.with_state(app_state)`, lalu diakses di handler lewat extractor `State<AppState>`.

## 3. Router & Routing Group

```rust
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
```

- `.route(path, method(handler))` — daftarin satu endpoint. `get(...)`/`post(...)` menentukan HTTP method-nya.
- Path yang sama (`"/"`) bisa dipakai untuk method berbeda (`GET` vs `POST`) — Axum membedakan berdasarkan kombinasi path + method.
- `{id}` adalah **path parameter**, diambil di handler lewat extractor `Path<T>`.
- `.nest("/v1/user", user_group_router)` — "menempelkan" grup router ke prefix tertentu. Jadi route `"/"` di dalam grup otomatis jadi `/v1/user`, `"/profile"` jadi `/v1/user/profile`, dst. Ini cara mengorganisir endpoint per-resource biar gak semua route didaftarkan flat di satu tempat.
- `.with_state(...)` menentukan tipe state `S` dari `Router<S>` — wajib dipanggil sebelum router bisa dipakai di `axum::serve()` (yang butuh `Router<()>`, state sudah "diserap").

## 4. Extractor: Mengambil Data dari Request

Axum menyediakan berbagai **extractor** sebagai parameter fungsi handler — urutannya bebas, tapi tiap tipe extractor "tahu" cara mengambil bagian tertentu dari request:

| Extractor | Ambil dari | Contoh |
|---|---|---|
| `State<T>` | State yang di-attach ke router | `State(app_state): State<AppState>` |
| `Path<T>` | Path parameter (`{id}`) | `Path(id): Path<u32>` |
| `Query<T>` | Query string (`?page=1&page_size=10`) | `Query(pagination): Query<Paggination>` |
| `Json<T>` | Body request berformat JSON | `Json(payload): Json<CreateUserRequest>` |

Contoh gabungan beberapa extractor sekaligus:
```rust
async fn handle_post_user(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> (StatusCode, Json<CreateUserRespose>) {
    // ...
}
```

Catatan: extractor yang "menghabiskan" body request (seperti `Json<T>`) harus jadi **parameter terakhir** — aturan Axum karena body cuma bisa dibaca sekali.

## 5. Response: `IntoResponse`

```rust
async fn handle_post_hobbie(Json(hobbie): Json<CreateHobbie>) -> impl IntoResponse {
    return (
        StatusCode::CREATED,
        AppendHeaders([
            ("X-POWERED-BY", "AXUM"),
            ("API-VERSION", "V1"),
        ]),
        Json(CreateHobbie { id: hobbie.id, name: hobbie.name }),
    );
}
```

- `IntoResponse` adalah trait yang menandakan "sesuatu bisa diubah jadi HTTP response". Banyak tipe sudah implement ini secara built-in: `String`, `StatusCode`, `Json<T>`, dan **tuple** dari tipe-tipe itu (kombinasi status + headers + body).
- Kalau return type handler ditulis eksplisit seperti `(StatusCode, Json<T>)`, semua cabang (misal di dalam `match`) harus punya tipe itu persis.
- Kalau butuh return bentuk berbeda-beda tergantung kondisi (200 dengan data vs 404 dengan pesan error), pakai `impl IntoResponse` sebagai return type, dan pastikan tiap cabang dipanggil `.into_response()` supaya semua "diseragamkan" jadi satu tipe (`Response`).
- Urutan elemen dalam tuple response fleksibel: `(StatusCode, Json<T>)`, atau `(StatusCode, HeaderMap/AppendHeaders, Json<T>)` seperti contoh di atas — Axum yang urus penggabungannya jadi response HTTP yang valid.

## 6. `StatusCode`

```rust
use axum::http::StatusCode;

return (StatusCode::CREATED, Json(response));
```

`StatusCode` adalah enum berisi semua kode status HTTP standar (`OK` = 200, `CREATED` = 201, `NOT_FOUND` = 404, `INTERNAL_SERVER_ERROR` = 500, dst). Dipasangkan dengan body response dalam tuple supaya client tahu hasil request-nya (bukan cuma selalu 200 buat semua kasus).

## 7. `Serialize` & `Deserialize` (Serde)

```rust
#[derive(Deserialize)]
struct CreateUserRequest {
    id: u32,
    name: String,
    email: String,
    password: String,
}

#[derive(Serialize)]
struct CreateUserRespose {
    id: u32,
    name: String,
    email: String,
    password: String,
}
```

- **`Deserialize`** — dipakai untuk struct yang jadi **input** (body request masuk). Axum pakai ini di extractor `Json<T>` untuk mengubah JSON mentah dari client jadi struct Rust.
- **`Serialize`** — dipakai untuk struct yang jadi **output** (response keluar). Dipakai `Json<T>` di return type untuk mengubah struct Rust jadi JSON yang dikirim ke client.
- Struct yang dipakai dua arah (request dan response, kayak `User`) bisa derive keduanya sekaligus: `#[derive(Serialize, Deserialize, Clone)]`.
- Ini semua "otomatis" berkat crate **`serde`** — Axum sendiri gak ngurusin parsing JSON, dia cuma manggil `serde_json` di belakang layar lewat `Json<T>` extractor/response, dan `serde` yang tau cara convert struct ↔ JSON berdasarkan derive macro-nya.

## 8. SQLx: Query ke Postgres

```rust
let fetch_date: chrono::NaiveDate = sqlx::query_scalar("SELECT CURRENT_DATE")
    .fetch_one(&app_state.db_pool)
    .await
    .expect("Faild to fetch version");
```

- `sqlx::query_scalar(...)` — query yang hasilnya cuma **satu kolom/nilai** (beda dengan `query_as!` yang hasilnya struct dengan banyak kolom, dipelajari lebih lanjut di `feat/crud`).
- `.fetch_one(&pool)` — jalankan query, ambil **tepat satu row**. Ada juga `.fetch_optional()` (0 atau 1 row) dan `.fetch_all()` (banyak row).
- **Wajib `.await`** — query builder di atas belum benar-benar jalan sebelum di-`.await`, karena semua operasi I/O di Rust bersifat lazy sampai di-`await` (lihat juga `docs/error-handling.md` di branch `feat/error-handling` untuk penjelasan lebih detail soal ini).
- `.expect(...)` di sini masih cara "kasar" buat handle error (panic kalau gagal) — versi lebih rapi pakai `Result<T, E>` custom dipelajari di branch `feat/error-handling`.

## Ringkasan Peta Belajar

| Konsep | Contoh di kode | Topik lanjutan |
|---|---|---|
| Instalasi dependency | `cargo add`, `cargo install sqlx-cli` | — |
| Async runtime | `#[tokio::main]` | — |
| Shared state | `AppState` + `Arc<Mutex<T>>` | `feat/crud` (state cuma `PgPool`, data pindah ke DB) |
| Routing & grouping | `Router::new().route(...).nest(...)` | `feat/crud` (route CRUD lengkap) |
| Extractor | `State`, `Path`, `Query`, `Json` | — |
| Response | `IntoResponse`, tuple response | `feat/crud` (pola `Result<T, E>` sebagai response) |
| Query database | `sqlx::query_scalar` + `.fetch_one().await` | `feat/crud`, `feat/error-handling` |
| Migration | `sqlx migrate add` / `sqlx migrate run` | `feat/crud` |
| Error handling | `.expect()` (panic kalau gagal) | `feat/error-handling` (custom error + `?` operator) |
