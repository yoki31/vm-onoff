FROM rust:1-slim-bullseye AS builder

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  ca-certificates \
  libssl-dev \
  libcrypto++-dev \
  pkg-config \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY . .
RUN ls -la \
  && cargo build --locked --release \
  && cp target/release/vm-onoff /usr/local/bin

FROM debian:bullseye-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  ca-certificates \
  libssl-dev \
  libcrypto++-dev \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/vm-onoff /usr/local/bin/vm-onoff

RUN ["ldd", "/usr/local/bin/vm-onoff"]

CMD ["vm-onoff"]
