SHELL := /bin/bash

.PHONY: help
# Environment
.PHONY: env-up env-down migrate restart
# Code quality
.PHONY: check ci local fmt lint openapi prepare
# Testing
.PHONY: test test_one test_in test_not
# Development
.PHONY: build dev run clean

#===============================================================================
# Help
#===============================================================================

help: ## Show available targets
	@grep -E '^[a-zA-Z0-9_.-]+:.*?## ' Makefile | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  make %-10s %s\n", $$1, $$2}'

#===============================================================================
# Environment
#===============================================================================

env-up: ## Start environment
	@echo "[*] Starting environment..."
	@docker compose up -d

env-down: ## Stop environment
	@echo "[*] Stopping environment..."
	@docker compose down -v

migrate: ## Reset database and run migrations
	@echo "[*] Resetting database and applying migrations..."
	. ./.env && cargo sqlx database reset -y --source crates/db/migrations


restart: env-down env-up migrate ## Restart environment and run migrations

#===============================================================================
# Code Quality
#===============================================================================

ci: fmt lint build ## CI pipeline (GitHub Actions)

local: check test ## Full local pipeline (requires running environment)

check: fmt prepare lint openapi ## Full code quality check (requires .env and db)

fmt: ## Check and fix formatting if needed
	@echo "[*] Checking formatting..."
	@cargo fmt --all -- --check \
		|| (echo "[*] Formatting code..." && cargo fmt --all)

lint: ## Run clippy in strict mode
	@echo "[*] Running clippy..."
	@cargo clippy --workspace --all-targets --all-features -- -D warnings

openapi: ## Check all ToSchema types are registered in openapi.rs
	@echo "[*] Checking OpenAPI schema completeness..."
	@missing=0; \
	for f in $$(find crates/api/src -name 'models.rs' -type f | sort); do \
		for name in $$(grep -A5 'derive.*ToSchema' "$$f" \
			| grep -oE 'pub (struct|enum) [A-Za-z0-9_]+' \
			| awk '{print $$3}'); do \
			if ! grep -q "$$name" crates/api/src/openapi.rs; then \
				echo "  $${f#crates/api/src/}: $$name"; \
				missing=$$((missing + 1)); \
			fi; \
		done; \
	done; \
	if [ "$$missing" -gt 0 ]; then \
		echo ""; \
		echo "[!] $$missing type(s) with ToSchema not found in openapi.rs"; \
		exit 1; \
	fi

prepare: ## Generate SQLx offline query metadata for CI builds (requires bash/zsh)
	@echo "[*] Generating SQLx offline query metadata..."
	@test -f .env || (echo "Error: .env file not found" && exit 1)
	@set -eo pipefail; \
		set -a; source ./.env; set +a; \
		cargo sqlx prepare --workspace -- --all-features --tests 2>&1 \
		| sed '/^[^ ]/s/^/    /'

#===============================================================================
# Testing
#===============================================================================

test: ## Run nextest (use ARGS="..." for extra arguments)
	@echo "[*] Running tests..."
	@DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5433/postgres \
		cargo nextest run --all-features --no-fail-fast $(ARGS)

test_one: ## Run single test: `make test_one <test_name>`
	@$(MAKE) test ARGS="$(filter-out $@,$(MAKECMDGOALS))"

test_in: ## Run tests in module: `make test_in <module_name>`
	@$(MAKE) test ARGS="--test $(filter-out $@,$(MAKECMDGOALS))"

test_not: ## Exclude tests: `make test_not <test1> <test2> ...`
	@expr=$$(echo "$(filter-out $@,$(MAKECMDGOALS))" \
		| awk '{for(i=1;i<=NF;i++){printf "not test(%s)%s", $$i, (i<NF?" and ":"")}}'); \
	$(MAKE) test ARGS="-E '$$expr'"

#===============================================================================
# Development
#===============================================================================

build: ## Build all workspace crates
	@echo "[*] Building workspace..."
	@cargo build --workspace

dev: ## Run CLI in debug mode with .env loaded
	@echo "[*] Running CLI (debug)..."
	@set -a && . ./.env && set +a && cargo run --bin cli

run: ## Run CLI in release mode with .env loaded
	@echo "[*] Running CLI (release)..."
	@set -a && . ./.env && set +a && cargo run --bin cli --release

clean: ## Clean build artifacts
	@echo "[*] Cleaning build artifacts..."
	@cargo clean

# Prevent "No rule to make target" error for arguments
#   %: — catch-all target, matches any unknown target (e.g. arguments like "test_name")
#   @: — no-op command, does nothing silently
%:
	@:
