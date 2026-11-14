# SPDX-License-Identifier: AGPL-3.0-or-later

FROM rust:1.99-slim-bookworm AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --release -p weft-cli -p weft-validation

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --user-group weft \
    && install -d -o weft -g weft /var/lib/weft /etc/weft

COPY --from=builder /src/target/release/weft /usr/local/bin/weft
COPY --from=builder /src/target/release/weft-decode /usr/local/bin/weft-decode
COPY deploy/config.toml /etc/weft/config.toml

USER weft
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/weft"]
CMD ["serve", "--config", "/etc/weft/config.toml"]
