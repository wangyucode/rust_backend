# ======================== 构建阶段 ========================
# 使用官方的 rust alpine 镜像作为构建环境
FROM rust:1.92-alpine3.20 AS builder

# 设置工作目录
WORKDIR /app

# 安装 Alpine 系统依赖（编译 Rust 项目所需）
# musl-dev: 提供 musl libc 开发库，适配 Alpine
# openssl-dev: 满足 reqwest/sqlx 等库的 TLS 需求
# sqlite-dev: sqlx sqlite 驱动依赖
# pkgconfig: 用于检测系统库
RUN apk add --no-cache musl-dev openssl-dev sqlite-dev pkgconfig

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
# 使用纯净的 alpine 镜像作为运行环境
FROM alpine:3.20

# 安装运行时依赖
# openssl: reqwest/sqlx 运行时需要
# sqlite-libs: sqlite 运行时库
# ca-certificates: HTTPS 请求需要的 CA 证书
# tzdata: 解决 chrono 时区问题
RUN apk add --no-cache openssl sqlite-libs ca-certificates tzdata

# 设置时区（根据你的需求调整，这里用上海时区）
ENV TZ=Asia/Shanghai

# 创建非 root 用户（安全最佳实践）
RUN addgroup -S rustapp && adduser -S rustapp -G rustapp
USER rustapp

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/rust_backend ./

# 暴露端口（根据你的 actix-web 服务端口调整，默认 8080）
EXPOSE 8080

# 设置启动命令
CMD ["./rust_backend"]