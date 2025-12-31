# Stage 1: Build Backend
FROM rust:alpine AS builder
WORKDIR /app

# Install build dependencies
# musl-dev: for C compilation (needed by sqlite, ring, etc)
# pkgconfig & openssl-dev: for native-tls
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

# Create a dummy project to cache dependencies
RUN mkdir src
RUN echo 'fn main() { println!("Dummy main function"); }' > src/main.rs
COPY Cargo.toml Cargo.lock ./

# Build dependencies only
RUN cargo build --release

# Copy actual source code
COPY src ./src

# Touch main.rs to force rebuild of the application
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Stage 2: Runtime
FROM alpine:latest
WORKDIR /app

# Install necessary runtime dependencies
# ca-certificates: for HTTPS
# tzdata: for timezone
# openssl: for native-tls (dynamic linking)
# libgcc: often required for Rust binaries
RUN apk add --no-cache ca-certificates tzdata openssl libgcc

# Set timezone
ENV TZ=Asia/Shanghai
ENV RUST_BACKTRACE=full

# Copy backend binary
COPY --from=builder /app/target/release/rust_backend .

# Copy static assets
COPY swagger ./swagger

# Expose port
EXPOSE 8080

# Run the application
CMD ["./rust_backend"]
