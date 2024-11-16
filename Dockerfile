FROM rust:alpine AS build

RUN apk add --update musl-dev sqlite-static pkgconf git 

ENV SQLITE3_STATIC=1 SQLITE3_LIB_DIR=/usr/lib/
COPY ./src ./src
COPY ./Cargo.lock .
COPY ./Cargo.toml .

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid 10001 \
    "subhook"

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM rust:alpine

COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group
RUN mkdir -p /data/ && chown subhook:subhook /data/
COPY --from=build --chown=subhook:subhook ./target/x86_64-unknown-linux-musl/release/subhook /app/subhook

USER subhook:subhook

ENTRYPOINT ["/app/subhook"]
