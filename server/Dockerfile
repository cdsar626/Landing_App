# Stage 1: Build Frontend
FROM oven/bun:1 as frontend-builder
WORKDIR /app
COPY package.json bun.lock ./
# If you have patches or other config files, copy them too.
RUN bun install --frozen-lockfile
COPY . .
RUN bun run build

# Stage 2: Build Backend
FROM rust:1.81 as backend-builder
WORKDIR /usr/src/app
COPY server/Cargo.toml ./Cargo.toml
# Create a dummy main.rs to build dependencies (caching optimization)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
# Now copy the actual source
COPY server/src ./src
# Touch main.rs to force rebuild
RUN touch src/main.rs
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
WORKDIR /app
# Install OpenSSL as it is likely required by reqwest/sqlx
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /usr/src/app/target/release/server /app/server
COPY --from=frontend-builder /app/dist /app/dist

# Expose port
EXPOSE 3000

# Environment variables
ENV STATIC_DIR=/app/dist
ENV PORT=3000
# DATABASE_URL and BREVO_API_KEY must be provided at runtime

CMD ["./server"]
