FROM rust:1.83-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src src
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/app/target cargo install --path .

FROM debian:stable-slim
ARG APP_NAME=factorio-tg-bridge

COPY --from=builder /usr/local/cargo/bin/$APP_NAME /usr/local/bin/$APP_NAME

CMD ["factorio-tg-bridge"]