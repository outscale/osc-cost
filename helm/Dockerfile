# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause
# syntax=docker/dockerfile:1
FROM rust:1.96-trixie@sha256:fb328f0f58becb23ba1719940a2c94ece8b0b48afa837d05b79ef64bc1e18f6e AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian13@sha256:8b5d1db6d2253036a53cb8362d3e3fa82a7caf84c247772c46a023166c64e977
COPY --from=build-env /app/target/release/osc-cost-exporter /
EXPOSE 8080
ENTRYPOINT ["/osc-cost-exporter", "--bind", "0.0.0.0:8080"]
