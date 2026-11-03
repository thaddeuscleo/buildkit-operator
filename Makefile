.DEFAULT_GOAL := help
.PHONY: help check test fmt apply-crd

help: ## Show available commands
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_-]+:.*## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

check: ## Check Rust code without building binaries
	cargo check

test: ## Run unit tests
	cargo test

fmt: ## Format Rust source code
	cargo fmt

apply-crd: gen-crd ## Generate and apply CRD to Kubernetes
	kubectl apply -f deploy/crd.json
