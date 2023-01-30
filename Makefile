TARGET=target/x86_64-unknown-linux-musl/release/osc-cost
REGISTRY ?= outscale
IMAGE_NAME ?= osc-cost
TAG ?= DEV
IMG ?= $(REGISTRY)/$(IMAGE_NAME):$(TAG)

all: help

.PHONY: help
help:
	@echo "help:"
	@echo "- make build : build stand-alone binary of osc-cost"
	@echo "- make test : run all tests"

build: $(TARGET)

target/x86_64-unknown-linux-musl/release/osc-cost: src/*.rs
	cargo build --target x86_64-unknown-linux-musl --release

.PHONY: test
test: cargo-test format-test integration-test reuse-test
	@echo all tests OK

.PHONY: cargo-test
	cargo test

.PHONY: format-test
format-test:
	cargo fmt --check
	cargo clippy

.PHONY: integration-test
integration-test: $(TARGET)
	./int-tests/run.sh

.PHONY: reuse-test
reuse-test:
	docker run --rm --volume $(PWD):/data fsfe/reuse:0.11.1 lint

.PHONY: docker-build
docker-build: # Build docker image with the manager 
	DOCKER_BUILDKIT=1 docker build -f helm/Dockerfile -t ${IMG} .

.PHONY: docker-push
docker-push: ## Push docker image with the manager.
	docker push ${IMG}
