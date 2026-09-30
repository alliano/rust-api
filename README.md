# rust_api

Proyek latihan REST API menggunakan [Axum](https://github.com/tokio-rs/axum) dan [SQLx](https://github.com/launchbadge/sqlx) dengan Postgres. Branch `master` ini adalah versi dasar/awal proyek — router sederhana dengan state di memori, sebelum dipecah ke fitur-fitur yang lebih lengkap.

## Tech Stack

- **Axum** — web framework
- **SQLx** — async Postgres driver + compile-time query check
- **Tokio** — async runtime
- **Postgres** (via Docker Compose)
- **Serde** — (de)serialisasi JSON
- **Chrono** — tipe tanggal/waktu

## Struktur Belajar per Branch

Proyek ini dipakai sebagai catatan belajar Axum, di mana tiap topik dipisah jadi branch sendiri:

| Branch | Topik |
|---|---|
| `master` | Setup dasar: router, state, koneksi Postgres |
| `feat/crud` | CRUD lengkap (create, read, update, delete) untuk resource `users` |
| `feat/error-handling` | Error handling terpusat pakai `thiserror` + `IntoResponse` |

Tiap branch fitur punya catatan pembelajaran sendiri (`LEARNING.md` atau folder `docs/`) yang menjelaskan pola-pola dan error yang ditemui selama proses belajar.

## Menjalankan Proyek

### 1. Jalankan database (Docker Compose)

```bash
docker-compose up -d
```

Ini bakal menjalankan Postgres di `localhost:5432` dengan kredensial default (lihat `docker-compose.yml`).

### 2. Konfigurasi environment

Buat file `.env` di root project:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/rust_api
```

### 3. Jalankan aplikasi

```bash
cargo run
```

Server akan listen di `0.0.0.0:8080`.

## Endpoint yang Tersedia (branch `master`)

Base path: `/v1/user`

| Method | Path | Keterangan |
|---|---|---|
| GET | `/v1/user` | List semua user (dari state di memori) |
| POST | `/v1/user` | Tambah user baru (disimpan di memori) |
| GET | `/v1/user/profile` | Contoh endpoint statis |
| GET | `/v1/user/{id}` | Ambil user by id (masih dummy, belum query beneran ke DB) |
| GET | `/v1/user/pages` | Contoh pagination via query string |
| POST | `/v1/user/hobbie` | Contoh endpoint dengan custom response header |

> Catatan: di versi ini, data user masih disimpan di `Vec<User>` dalam memori (`AppState.users`), bukan di database — koneksi Postgres cuma dipakai untuk contoh query (`SELECT CURRENT_DATE`). Persistensi ke database baru diimplementasikan penuh di branch `feat/crud`.

## Struktur Kode

Semua kode masih dalam satu file `src/main.rs`:
- `AppState` — state bersama (in-memory store + connection pool)
- Handler functions (`get_users`, `handle_post_user`, dst)
- DTO/struct request-response (`CreateUserRequest`, `CreateUserRespose`, dst)
