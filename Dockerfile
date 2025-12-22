# ======================== 构建阶段 ========================
# 使用 Debian-based Rust 镜像，避免 musl 动态链接问题
FROM rust:1.92-slim-bullseye AS builder

# 设置工作目录
WORKDIR /app

# 安装构建依赖（Debian 包管理）
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# 先复制 Cargo.toml 和 Cargo.lock，利用 Docker 缓存
COPY Cargo.toml Cargo.lock ./

# 创建一个空的 src/main.rs 来预编译依赖（缓存优化）
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# 构建依赖（--release 确保编译优化）
RUN cargo build --release

# 删除空的 main.rs，复制真实的源码
RUN rm -rf src
COPY src ./src

# 重新构建项目（此时依赖已缓存，仅编译源码）
RUN cargo build --release

# ======================== 运行阶段 ========================
# 使用更小的 Debian 运行时镜像
FROM debian:bullseye-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# 设置时区
ENV TZ=Asia/Shanghai
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# 创建非 root 用户（安全最佳实践）
RUN groupadd -r rustapp && useradd -r -g rustapp rustapp
USER rustapp

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/rust_backend ./

# 暴露端口（根据你的 actix-web 服务端口调整，默认 8080）
EXPOSE 8080

# 设置启动命令
CMD ["./rust_backend"]