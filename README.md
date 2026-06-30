# Rust Backend Project

English | [中文](./README_zh-CN.md)

## Project Introduction

This is a backend service project developed using Rust language, built with Axum framework, providing RESTful API to support <https://wycode.cn>.

### Key Features

- RESTful API service
- Database operations
- Email sending functionality
- OpenAPI Specification
- Asynchronous processing
- Caddy Access Log Ingestion

## Technology Stack

- **Web Framework**: Axum 0.7
- **Database**: SQLite (via sqlx)
- **Async Runtime**: Tokio
- **Logging**: Tracing & Tracing Subscriber
- **Serialization**: Serde
- **Email**: Lettre
- **Environment Variables**: Dotenv
- **File Identification**: file-id (for cross-platform file tracking)

## Project Structure

```
rust_backend/
├── src/
│   ├── controller/      # API Controller Layer
│   │   ├── blog.rs      # Blog related interfaces
│   │   ├── clipboard.rs # Clipboard interfaces
│   │   ├── comment.rs   # Comment interfaces
│   │   ├── config.rs    # Config interfaces
│   │   ├── email.rs     # Email sending interfaces
│   │   ├── state.rs     # Status check interfaces
│   │   ├── wechat.rs    # WeChat related interfaces
│   │   ├── yml.rs       # YML file service
│   │   └── mod.rs
│   ├── dao/             # Data Access Layer
│   │   ├── app.rs       # App data operations
│   │   ├── blog.rs      # Blog data operations
│   │   ├── clipboard.rs # Clipboard data operations
│   │   ├── comment.rs   # Comment data operations
│   │   ├── database.rs  # Database connection management
│   │   └── mod.rs
│   ├── util/            # Utilities
│   │   ├── email.rs
│   │   ├── uuid.rs
│   │   └── mod.rs
│   ├── task/            # Background Tasks
│   │   ├── caddy.rs     # Caddy log ingestion task
│   │   ├── visit.rs     # Visit record cleanup task
│   │   └── mod.rs
│   ├── after_startup.rs # Post-startup tasks
│   ├── openapi.yml      # OpenAPI definition
│   └── main.rs          # Application entry
├── data/                # Data Directory
│   ├── migrations/      # Database migration files
│   │   └── 20251217100000_init_tables.sql  # Initial table structure
│   └── db/              # SQLite database files
│       └── sqlite.db
├── .gitignore
├── Cargo.lock
└── Cargo.toml
```

## Development Environment Setup

### Prerequisites

- Rust 1.65+ (install via `rustup`)
- SQLite 3

### Install Dependencies

```bash
cargo build
```

### Environment Variables Configuration

Create a `.env` file and configure environment variables. Search for `env::var` related code and configure according to actual situations.

## Database

This project uses SQLite as the database, with asynchronous database operations via the sqlx library.

### Database File

- Database file path: `./data/db/sqlite.db`

### Database Migrations

Database migration files are stored in the `./data/migrations/` directory, using timestamp naming format.

- Initial migration file: `20251217100000_init_tables.sql`

## Background Tasks

### Caddy Log Ingestion

The project automatically ingests Caddy's JSON access logs into the SQLite database.

- **Log Directory**: `data/caddy-access-logs/`
- **File Naming**: `<domain>.access.log` (e.g., `wycode.cn.access.log`)
- **Frequency**: Polling every 5 seconds.
- **Retention**: Automatically cleans up logs older than 30 days.
- **State Maintenance**: Records offset and file ID for resume and rotate support.

### Blog Visit Cleanup

Automatically cleans up blog visit records older than 30 days in the database.

- **Frequency**: Every 24 hours.
- **Scope**: Records in `blog_visits` table where `timestamp` is older than 30 days.

## Run Commands

### Run in Development Mode

```bash
cargo run
```

The application will start at http://127.0.0.1:8080

### Build for Production

```bash
cargo build --release
```

### Run Tests

```bash
cargo test
```

### Code Format Check

```bash
cargo fmt
```

### Code Quality Check

```bash
cargo clippy
```

## API Documentation

This project provides an OpenAPI specification file.
You can access the OpenAPI definition file at `/api/v1/openapi.yml`.

## Related Links

- [Rust Official Website](https://www.rust-lang.org/)
- [Axum Documentation](https://docs.rs/axum/latest/axum/)
- [Tracing Documentation](https://docs.rs/tracing/latest/tracing/)
- [SQLx Documentation](https://docs.rs/sqlx/latest/sqlx/)
- [Serde Documentation](https://serde.rs/)
- [Lettre Documentation](https://docs.rs/lettre/latest/lettre/)
