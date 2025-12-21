# 第一阶段：构建阶段
FROM rust:latest as builder

# 设置工作目录
WORKDIR /app

# 复制Cargo.toml和Cargo.lock文件
COPY Cargo.toml Cargo.lock ./

# 创建一个空的main.rs文件，以便能够构建依赖
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs

# 构建依赖（这将缓存依赖，以便后续构建更快）
RUN cargo build --release

# 删除临时的main.rs文件
RUN rm src/main.rs

# 复制实际的源代码
COPY src ./src
COPY swagger ./swagger

# 重新构建应用
RUN cargo build --release

# 第二阶段：运行阶段
FROM alpine:3.19

# 安装必要的依赖
RUN apk add --no-cache sqlite-libs openssl ca-certificates

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/rust_backend ./

# 复制swagger目录
COPY --from=builder /app/swagger ./swagger

# 暴露应用使用的端口（根据实际情况修改）
EXPOSE 8080

# 设置环境变量（如果需要）
ENV DATABASE_FILE=./db/sqlite.db

# 运行应用
CMD ["./rust_backend"]