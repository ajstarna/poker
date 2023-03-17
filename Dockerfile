# build environment
FROM node:13.12.0-alpine as react
WORKDIR /app
ENV PATH /app/node_modules/.bin:$PATH
COPY react-ui/package.json ./
COPY react-ui/package-lock.json ./
RUN npm ci --silent
RUN npm install react-scripts@3.4.1 -g --silent
COPY ./react-ui ./
RUN npm run build

FROM rust:1.67.0 AS chef
# cargo-chef lets us build the the dependencies as a docker layer
# We only pay the installation cost once, 
# it will be cached from the second build onwards
RUN cargo install cargo-chef 
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin poker

# We do not need the Rust toolchain to run the binary!
FROM debian:buster-slim AS runtime
WORKDIR app
COPY --from=builder /app/target/release/poker .
# need the front end files
COPY --from=react /app/build ./site
ENTRYPOINT ["./poker", "--ip", "0.0.0.0"]
