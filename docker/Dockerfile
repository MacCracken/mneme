# Stage 1: Build
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
COPY benches/ benches/

RUN cargo build --release --workspace

# Stage 2: Runtime
FROM ghcr.io/maccracken/agnosticos:latest

USER root

RUN groupadd --system mneme && \
    useradd --system --gid mneme --uid 1004 --shell /usr/sbin/nologin --create-home mneme

COPY --from=builder /build/target/release/mneme-api /usr/local/bin/
COPY --from=builder /build/target/release/mneme-mcp /usr/local/bin/

RUN mkdir -p /var/lib/mneme/data /var/lib/mneme/index /etc/mneme && \
    chown -R mneme:mneme /var/lib/mneme

COPY deploy/mneme.service /etc/systemd/system/

USER mneme

ENV MNEME_VAULT_DIR=/var/lib/mneme/data
ENV MNEME_BIND=0.0.0.0:3838
ENV RUST_LOG=info

EXPOSE 3838

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3838/health || exit 1

ENTRYPOINT ["tini", "--"]
CMD ["mneme-api"]
