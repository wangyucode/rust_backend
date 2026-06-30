# Development Guidance

This file provides quick reference for development workflows and standards in this project. For comprehensive details, please refer to [README.md](./README.md).

## Critical Workflows

- **API Updates**: When adding or modifying API endpoints, you MUST update:
  - [openapi.yml](./src/openapi.yml): Keep the OpenAPI specification in sync with implementation.
  - [README.md](./README.md) & [README_zh-CN.md](./README_zh-CN.md): Update the project structure if new files are added.
- **Versioning**: For any new release or significant change, increment the version in [Cargo.toml](./Cargo.toml).

## Build & Test Commands

- **Build**: `cargo build`
- **Run**: `cargo run`
- **Test**: `cargo test`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`

## Coding Standards

- Use Axum for web routing and handlers.
- Use SQLx for asynchronous database operations.
- Ensure all new public modules are exported in [mod.rs](./src/controller/mod.rs) or [mod.rs](./src/dao/mod.rs).
- Follow Rust 2024 edition idioms.
