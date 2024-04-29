FROM rust:1.77.2-bookworm as builder

COPY . .

RUN cargo install --path . --root /usr/local --locked

FROM debian:bookworm-slim
RUN apt-get update &&  \
    DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates

COPY --from=builder /usr/local/bin/ecs-run /usr/local/bin/ecs-run

ENTRYPOINT [ "/usr/local/bin/ecs-run" ]
