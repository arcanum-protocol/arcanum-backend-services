FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef

WORKDIR /app
RUN apt-get update && apt-get --assume-yes install musl-dev libssl-dev

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

FROM debian:latest as price-oracle

RUN apt-get update && \
    apt-get --assume-yes install \
        make \
        libpq5 \
        libpq-dev \
        -qqy \
        --no-install-recommends
RUN apt-get update && apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
WORKDIR /app
COPY --from=builder /app/target/release/price_oracle app

CMD ["./app"]

FROM debian:latest as trader

RUN apt-get update && \
    apt-get --assume-yes install \
        make \
        libpq5 \
        libpq-dev \
        -qqy \
        --no-install-recommends
RUN apt-get update && apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
WORKDIR /app
COPY --from=builder /app/target/release/trader app

CMD ["./app"]
