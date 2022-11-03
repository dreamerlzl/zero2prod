FROM rust as builder

WORKDIR /app

run apt update && apt install -y lld clang

COPY . .

run cargo build --release

FROM debian:bullseye-slim as runner

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zero2prod zero2prod

ENTRYPOINT ["./zero2prod"]
