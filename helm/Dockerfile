# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause
# syntax=docker/dockerfile:1
ARG DEBIAN_CODENAME="trixie"

FROM lukemathwalker/cargo-chef:latest-rust-1.91-${DEBIAN_CODENAME} AS cargo-chef
WORKDIR /app

FROM cargo-chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM cargo-chef AS builder 
WORKDIR /app
COPY --from=planner /app/recipe.json /app/
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin osc-cost-exporter

FROM debian:${DEBIAN_CODENAME}-slim
LABEL maintainer="Outscale SAS"
ENV USER_ID=65535
ENV GROUP_ID=65535
ENV USER_NAME=osc-cost
ENV GROUP_NAME=osc-cost
RUN groupadd --gid $GROUP_ID $GROUP_NAME && \
    useradd --shell /sbin/nologin --uid $USER_ID --gid $GROUP_NAME --create-home ${USER_NAME} && \
    mkdir /home/osc-cost/.osc && \
    chown -R osc-cost:osc-cost /home/osc-cost/.osc
ENV PATH="$PATH:/home/osc-cost/.local/bin"
WORKDIR /app
COPY --from=builder  /app/target/release/osc-cost-exporter /usr/local/bin
EXPOSE 8080
USER osc-cost
ENTRYPOINT ["/usr/local/bin/osc-cost-exporter", "--bind", "0.0.0.0:8080"]
