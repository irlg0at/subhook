FROM rust:alpine as build

RUN apk add openssl-dev musl-dev

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

FROM scratch

COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group

COPY --from=build --chown=subhook:subhook ./target/x86_64-unknown-linux-musl/release/subhook /app/subhook

USER subhook:subhook

ENTRYPOINT ["./app/subhook"]
