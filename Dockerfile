FROM rust:alpine as build
RUN apk update && apk --no-cache --update add build-base openssl openssl-dev perl
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --target x86_64-unknown-linux-musl \
    && cp target/x86_64-unknown-linux-musl/release/ytfeed ./ytfeed

FROM scratch
COPY --from=build /app/ytfeed .
ENTRYPOINT [ "/ytfeed" ]
