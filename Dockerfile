FROM rust:1.99.1-trixie AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin buildkit-operator \
    && cp target/release/buildkit-operator /usr/local/bin/buildkit-operator

FROM gcr.io/distroless/cc-debian13:nonroot AS runtime
COPY --from=builder /usr/local/bin/buildkit-operator /usr/local/bin/buildkit-operator
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/buildkit-operator"]
