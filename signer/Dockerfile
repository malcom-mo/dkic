FROM rust:bookworm AS builder
WORKDIR /dkic-signer
COPY Cargo.toml ./
COPY src ./src
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libc6 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/dkic-signer /usr/local/bin/dkic-signer
ENTRYPOINT ["dkic-signer"]
CMD ["sign"]
