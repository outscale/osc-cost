# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause
# syntax=docker/dockerfile:1
FROM rust:1.92-trixie AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian13
COPY --from=build-env /app/target/release/osc-cost-exporter /
EXPOSE 8080
ENTRYPOINT ["/osc-cost-exporter", "--bind", "0.0.0.0:8080"]
