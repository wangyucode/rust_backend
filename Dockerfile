# ======================== 构建阶段 ========================
# 使用官方的 rust 镜像作为构建环境
FROM rust:1.92 AS builder

# 设置环境变量以优化构建
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    CARGO_TERM_COLOR=always \
    RUSTFLAGS="-C target-cpu=native"

# 设置工作目录
WORKDIR /app

# 安装系统依赖（编译 Rust 项目所需）
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    libsqlite3-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# 复制项目文件
COPY Cargo.deps.toml Cargo.toml

# 创建一个临时的 src/main.rs 文件（仅用于构建依赖）
RUN mkdir -p src
RUN echo 'fn main() { println!("Dummy main function"); }' > src/main.rs

RUN cargo build --release

RUN rm -rf src Cargo.lock Cargo.toml

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# 构建项目（--release 确保编译优化）
RUN cargo build --release

# ======================== 运行阶段 ========================
# 使用 Debian trixie-slim 镜像作为运行环境（更小的镜像体积，适合生产环境）
FROM debian:trixie-slim

# 设置工作目录
WORKDIR /app

# 安装运行时依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    libsqlite3-0 \
    tzdata \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 设置时区（根据你的需求调整，这里用上海时区）
ENV TZ=Asia/Shanghai
ENV RUST_BACKTRACE=full

# 创建非 root 用户（安全最佳实践）
RUN groupadd -g 1000 rust && useradd -u 1000 -g rust -m -s /bin/bash rust
USER rust

# 从构建阶段复制编译好的二进制文件
COPY --from=builder --chown=rust:rust /app/target/release/rust_backend ./

# 复制必要的静态资源
COPY --chown=rust:rust swagger ./swagger

# 暴露端口（根据你的 actix-web 服务端口调整，默认 8080）
EXPOSE 8080

# 添加健康检查（使用应用现有的state端点）
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO- http://localhost:8080/api/v1/ || exit 1

# 设置启动命令
CMD ["./rust_backend"]