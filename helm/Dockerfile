# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause
# syntax=docker/dockerfile:1
FROM lukemathwalker/cargo-chef:latest-rust-1 AS cargo-chef
LABEL maintainer="Outscale SAS"
WORKDIR /app

FROM cargo-chef AS planner
LABEL maintaner="Outscale SAS"
WORKDIR /app
COPY Cargo.* /app
COPY src/ /app/src/
RUN cargo chef prepare --recipe-path recipe.json

FROM cargo-chef AS builder 
LABEL maintainer="Outscale SAS"
WORKDIR /app
COPY --from=planner /app/recipe.json /app/
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.* /app
COPY src/ /app/src
RUN cargo build --release --bin osc-cost

FROM debian:bullseye-slim
LABEL maintainer="Outscale SAS"
ENV USER_ID=65535
ENV GROUP_ID=65535
ENV USER_NAME=osc-cost
ENV GROUP_NAME=osc-cost
RUN --mount=type=cache,target=/var/cache/apt apt-get update && \
   apt-get install -y jq=1.6-2.1 netcat=1.10-46 curl ca-certificates=20210119 --no-install-recommends &&\
   apt-get clean --dry-run && \
   rm -rf /var/lib/apt/lists/*
RUN addgroup --gid $GROUP_ID $GROUP_NAME && \
    adduser --shell /sbin/nologin  --gecos "${USER_NAME},0,0,0" --disabled-password --uid $USER_ID --ingroup $GROUP_NAME ${USER_NAME} && \
    mkdir /home/osc-cost/.osc && \
    chown -R osc-cost:osc-cost /home/osc-cost/.osc
ENV PATH="$PATH:/home/osc-cost/.local/bin"
WORKDIR /app
COPY --chown=osc-cost:osc-cost ./helm/entrypoint.sh /app
COPY --from=builder  /app/target/release/osc-cost /usr/local/bin
RUN chown -R osc-cost:osc-cost /usr/local/bin/osc-cost
ENV OSCCOST_EXTRA_PARAMS=''
HEALTHCHECK CMD curl --fail http://localhost:8080/metrics || exit 1
EXPOSE 8080
USER osc-cost
ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["8080", "/home/osc-cost/.osc/config.json", "/usr/local/bin/osc-cost", "--format=prometheus"]
