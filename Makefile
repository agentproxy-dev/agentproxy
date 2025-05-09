# Image configuration
DOCKER_REGISTRY ?= ghcr.io
DOCKER_REPO ?= agentgateway
IMAGE_NAME ?= agentgateway
VERSION ?= $(shell git describe --tags --always --dirty)
IMAGE_TAG ?= $(VERSION)
IMAGE_FULL_NAME ?= $(DOCKER_REGISTRY)/$(DOCKER_REPO)/$(IMAGE_NAME):$(IMAGE_TAG)
DOCKER_BUILDER ?= docker
DOCKER_BUILD_ARGS ?=
KIND_CLUSTER_NAME ?= agentgateway

# docker
.PHONY: docker
docker:
	$(DOCKER_BUILDER) build $(DOCKER_BUILD_ARGS) -t $(IMAGE_FULL_NAME) .

.PHONY: docker-ext
docker-ext:
	$(DOCKER_BUILDER) build $(DOCKER_BUILD_ARGS) -t $(IMAGE_FULL_NAME)-ext -f Dockerfile.ext .

CARGO_BUILD_ARGS ?=
# build
.PHONY: build
build:
	cargo build --release --features ui $(CARGO_BUILD_ARGS)

# lint
.PHONY: lint
lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

# test
.PHONY: test
test:
	cargo test --all-targets

# clean
.PHONY: clean
clean:
	cargo clean

objects := $(wildcard examples/*/config.json)

.PHONY: validate
validate: $(objects)

%/config.json:
	cargo run -- --mode=validate -f $*/config.json

# Code generation for xds apis
.PHONY: generate-apis
generate-apis:
	protoc --proto_path=./crates/agentgateway/proto/ \
		--go_out=./go/api/a2a \
		--go_opt=paths=source_relative \
		--go_opt=Mcommon.proto=github.com/agentgateway/go/api/common \
		./crates/agentgateway/proto/a2a/target.proto
	protoc --proto_path=./crates/agentgateway/proto/ \
		--go_out=./go/api/mcp \
		--go_opt=paths=source_relative \
		--go_opt=Mcommon.proto=github.com/agentgateway/go/api/common \
		./crates/agentgateway/proto/mcp/target.proto
	protoc --proto_path=./crates/agentgateway/proto/ \
		--go_out=./go/api \
		--go_opt=paths=source_relative \
		./crates/agentgateway/proto/listener.proto
	