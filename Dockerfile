FROM rust:1.66.1 as builder

COPY . .

RUN cargo install --path . --root /usr/local --locked

FROM debian:bullseye-slim
RUN apt-get update &&  \
    DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates

COPY --from=builder /usr/local/bin/ecs-run /usr/local/bin/ecs-run

ENTRYPOINT [ "/usr/local/bin/ecs-run" ]
