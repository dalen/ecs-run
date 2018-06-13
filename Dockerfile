FROM rust:1.26.2 as builder

COPY . .

RUN cargo install --root /usr/local

FROM debian:stretch-slim as runner

RUN apt-get update && \
  DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates && \
  apt-get clean && \
  rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/ecs-run /usr/local/bin/ecs-run

ENTRYPOINT [ "/usr/local/bin/ecs-run" ]
