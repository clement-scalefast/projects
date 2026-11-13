# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS build

# Bundles the fonts and stylesheet that the app serves. Keep the version in step
# with the `topcoat` dependency in Cargo.toml.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install topcoat-cli --version 0.12.1 --locked

WORKDIR /app
COPY . .

# The bundle must come from the same build as the binary, so build both in one
# step and copy them out of the cached target directory.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    topcoat asset bundle --release \
    && mkdir /out \
    && cp target/release/projects /out/projects \
    && cp -r target/release/assets /out/assets

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /app app

WORKDIR /app
# The app loads the bundle from the `assets` directory next to the executable.
COPY --from=build --chown=app:app /out/ ./

USER app
ENV HOST=0.0.0.0 \
    PORT=3000
EXPOSE 3000

CMD ["/app/projects"]
