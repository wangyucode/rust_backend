# Rust Backend Project

## 项目介绍 (Project Introduction)

这是一个基于 Rust 语言开发的后端服务项目，使用 Axum 框架构建，提供支持 <https://wycode.cn> 的相关 RESTful API 服务。

This is a backend service project developed using Rust language, built with Axum framework, providing RESTful API to support <https://wycode.cn>.

### 主要功能 (Key Features)

- RESTful API 服务 (RESTful API service)
- 数据库操作 (Database operations)
- 邮件发送功能 (Email sending functionality)
- OpenAPI 规范 (OpenAPI Specification)
- 异步处理 (Asynchronous processing)
- Caddy 访问日志导入 (Caddy Access Log Ingestion)
- AI SQL纠正 (AI-powered SQL correction)

## 技术栈 (Technology Stack)

- **Web 框架**: Axum 0.7
- **数据库**: SQLite (通过 sqlx)
- **异步运行时**: Tokio
- **日志**: Tracing & Tracing Subscriber
- **序列化**: Serde
- **邮件发送**: Lettre
- **环境变量**: Dotenv
- **文件标识**: file-id (用于跨平台文件追踪)

## 项目结构 (Project Structure)

```
rust_backend/
├── src/
│   ├── controller/      # API 控制器层
│   │   ├── blog.rs      # 博客相关接口
│   │   ├── clipboard.rs # 剪贴板接口
│   │   ├── comment.rs   # 评论接口
│   │   ├── config.rs    # 配置接口
│   │   ├── coze.rs      # Coze 代理接口
│   │   ├── sql_ai.rs    # AI SQL纠正接口
│   │   ├── email.rs     # 邮件发送接口
│   │   ├── state.rs     # 状态检查接口
│   │   ├── wechat.rs    # 微信相关接口
│   │   ├── yml.rs       # YML文件服务
│   │   └── mod.rs
│   ├── dao/             # 数据访问层
│   │   ├── app.rs       # 应用数据操作
│   │   ├── blog.rs      # 博客数据操作
│   │   ├── clipboard.rs # 剪贴板数据操作
│   │   ├── comment.rs   # 评论数据操作
│   │   ├── database.rs  # 数据库连接管理
│   │   └── mod.rs
│   ├── util/            # 工具类
│   │   ├── email.rs
│   │   ├── uuid.rs
│   │   └── mod.rs
│   ├── task/            # 后台任务
│   │   ├── caddy.rs     # Caddy 日志导入任务
│   │   ├── visit.rs     # 访问记录清理任务
│   │   └── mod.rs
│   ├── after_startup.rs # 启动后任务
│   ├── openapi.yml      # OpenAPI 定义
│   └── main.rs          # 应用入口
├── db/                  # 数据库相关
│   ├── migrations/      # 数据库迁移文件
│   │   └── 20251217100000_init_tables.sql  # 初始化表结构
│   └── sqlite.db        # SQLite 数据库文件
├── .gitignore
├── Cargo.lock
└── Cargo.toml
```

## 开发环境搭建 (Development Environment Setup)

### 前置要求 (Prerequisites)

- Rust 1.65+ (使用 `rustup` 安装)
- SQLite 3

### 安装依赖 (Install Dependencies)

```bash
cargo build
```

### 环境变量配置 (Environment Variables Configuration)

创建 `.env` 文件并配置环境变量。搜索 `env::var` 相关代码，根据实际情况配置。

Create a `.env` file and configure environment variables. Search for `env::var` related code and configure according to actual situations.

## 数据库 (Database)

本项目使用 SQLite 作为数据库，通过 sqlx 库进行异步数据库操作。

This project uses SQLite as the database, with asynchronous database operations via the sqlx library.

### 数据库文件 (Database File)

- 数据库文件路径: `./db/sqlite.db`
- Database file path: `./db/sqlite.db`

### 数据库迁移 (Database Migrations)

数据库迁移文件存放在 `./db/migrations/` 目录下，使用时间戳命名格式。

Database migration files are stored in the `./db/migrations/` directory, using timestamp naming format.

- 初始化迁移文件: `20251217100000_init_tables.sql`
- Initial migration file: `20251217100000_init_tables.sql`

## AI SQL纠正 (AI SQL Correction)

使用 OpenAI 兼容的大模型 API 自动纠正有语法或逻辑错误的 SQL 语句。

Uses an OpenAI-compatible LLM API to automatically correct SQL statements with syntax or logic errors.

### API 端点 (API Endpoint)

```
POST /api/v1/sql/correct
```

**请求体 (Request Body):**
```json
{
  "sql": "SELCT * FORM users WHER id = 1",
  "error": "near \"SELCT\": syntax error"
}
```

**响应 (Response):**
```json
{
  "success": true,
  "message": "success",
  "payload": {
    "original_sql": "SELCT * FORM users WHER id = 1",
    "corrected_sql": "SELECT * FROM users WHERE id = 1",
    "explanation": "修正了拼写错误: SELCT→SELECT, FORM→FROM, WHER→WHERE"
  }
}
```

### 环境变量 (Environment Variables)

| 变量名 | 必填 | 默认值 | 说明 |
|--------|------|--------|------|
| `OPENAI_API_KEY` | ✅ | - | OpenAI 兼容 API 的密钥 |
| `OPENAI_API_BASE` | ❌ | `https://api.openai.com/v1` | API 基础 URL（可替换为其他兼容服务） |
| `OPENAI_MODEL` | ❌ | `gpt-4o` | 使用的模型名称 |

### 工作原理 (How It Works)

1. 自动读取当前 SQLite 数据库的完整 schema（表结构）
2. 将 schema + 用户的 SQL + 可选的错误信息一起发送给大模型
3. 大模型分析并返回纠正后的 SQL 和修改说明
4. 支持任何 OpenAI 兼容的 API（如 OpenAI、DeepSeek、通义千问等）

## 后台任务 (Background Tasks)

### Caddy 日志导入 (Caddy Log Ingestion)

项目会自动将 Caddy 的 JSON 访问日志导入到 SQLite 数据库中。

- **日志目录**: `db/caddy-access-logs/`
- **文件命名**: `<domain>.access.log` (例如 `wycode.cn.access.log`)
- **执行频率**: 每 5 秒轮询一次。
- **数据保留**: 自动清理超过 30 天的日志记录。
- **状态维护**: 自动记录文件读取位置（Offset）和文件唯一标识（File-ID），支持断点续传和日志轮转（Rotate）。

The project automatically ingests Caddy's JSON access logs into the SQLite database.

- **Log Directory**: `db/caddy-access-logs/`
- **File Naming**: `<domain>.access.log` (e.g., `wycode.cn.access.log`)
- **Frequency**: Polling every 5 seconds.
- **Retention**: Automatically cleans up logs older than 30 days.
- **State Maintenance**: Records offset and file ID for resume and rotate support.

### 访问记录清理 (Blog Visit Cleanup)

自动清理数据库中超过 30 天的博客访问记录。

- **执行频率**: 每 24 小时执行一次。
- **清理范围**: `blog_visits` 表中 `timestamp` 超过 30 天的数据。

Automatically cleans up blog visit records older than 30 days in the database.

- **Frequency**: Every 24 hours.
- **Scope**: Records in `blog_visits` table where `timestamp` is older than 30 days.

## 运行命令 (Run Commands)

### 开发模式运行 (Run in Development Mode)

```bash
cargo run
```

应用将在 http://127.0.0.1:8080 启动

The application will start at http://127.0.0.1:8080

### 构建生产版本 (Build for Production)

```bash
cargo build --release
```

### 运行测试 (Run Tests)

```bash
cargo test
```

### 代码格式检查 (Code Format Check)

```bash
cargo fmt
```

### 代码质量检查 (Code Quality Check)

```bash
cargo clippy
```

## API 文档 (API Documentation)

本项目提供 OpenAPI 规范文件。
可以通过访问 `/api/v1/openapi.yml` 获取 OpenAPI 定义文件。

This project provides an OpenAPI specification file.
You can access the OpenAPI definition file at `/api/v1/openapi.yml`.

## 相关链接 (Related Links)

- [Rust 官方网站](https://www.rust-lang.org/)
- [Axum 文档](https://docs.rs/axum/latest/axum/)
- [Tracing 文档](https://docs.rs/tracing/latest/tracing/)
- [SQLx 文档](https://docs.rs/sqlx/latest/sqlx/)
- [Serde 文档](https://serde.rs/)
- [Lettre 文档](https://docs.rs/lettre/latest/lettre/)
