.DEFAULT_GOAL := help
SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables --no-builtin-rules

# ---- Configuration (override with `make VAR=value`) ----
CARGO      ?= cargo
KUBECTL    ?= kubectl
BIN        ?= buildkit-operator
CRD_FILE   ?= deploy/crd.json
KUBE_CONTEXT ?=
KUBECTL_CMD = $(KUBECTL) $(if $(KUBE_CONTEXT),--context $(KUBE_CONTEXT))

.PHONY: help build release check lint fmt fmt-check test dev \
        gen-crd apply-crd delete-crd diff-crd ci clean

help: ## Show available commands
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_-]+:.*## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

##@ Development

run: ## Run the operator locally
	$(CARGO) run --bin $(BIN)

build: ## Build debug binaries
	$(CARGO) build

release: ## Build optimized release binaries
	$(CARGO) build --release --locked

clean: ## Remove build artifacts
	$(CARGO) clean

##@ Quality

check: ## Type-check without building binaries
	$(CARGO) check --all-targets

lint: ## Run clippy, failing on warnings
	$(CARGO) clippy --all-targets -- -D warnings

fmt: ## Format source code
	$(CARGO) fmt --all

fmt-check: ## Verify formatting (no changes)
	$(CARGO) fmt --all -- --check

test: ## Run unit tests
	$(CARGO) test --all-targets

ci: fmt-check lint test ## Run everything CI runs

##@ CRD

# Real file target: only regenerates when sources change.
$(CRD_FILE): $(shell find src -name '*.rs' 2>/dev/null) Cargo.toml Cargo.lock
	@mkdir -p $(dir $@)
	$(CARGO) run --quiet --bin crdgen > $@

gen-crd: $(CRD_FILE) ## Generate the CRD JSON

apply-crd: $(CRD_FILE) ## Generate and apply the CRD to the cluster
	$(KUBECTL_CMD) apply -f $(CRD_FILE)

diff-crd: $(CRD_FILE) ## Show what applying the CRD would change
	-$(KUBECTL_CMD) diff -f $(CRD_FILE)

delete-crd: ## Delete the CRD from the cluster (destroys all CRs!)
	$(KUBECTL_CMD) delete -f $(CRD_FILE)
