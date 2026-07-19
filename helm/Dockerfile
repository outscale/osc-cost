# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause
# syntax=docker/dockerfile:1
FROM rust:1.96-trixie@sha256:1f0dbad1df66647807e6952d1db85d0b2bda7606cb2139d82517e4f009967376 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian13@sha256:8b5d1db6d2253036a53cb8362d3e3fa82a7caf84c247772c46a023166c64e977
COPY --from=build-env /app/target/release/osc-cost-exporter /
EXPOSE 8080
ENTRYPOINT ["/osc-cost-exporter", "--bind", "0.0.0.0:8080"]
