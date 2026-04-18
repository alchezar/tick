SHELL := /bin/bash
LOAD_ENV := set -eo pipefail; set -a; source ./.env; set +a;

.PHONY: help
# Environment
.PHONY: env-up env-down migrate migrate-reset restart
# Code quality
.PHONY: check ci validate fmt lint openapi prepare
# Testing
.PHONY: test test_one test_in test_not
# Development
.PHONY: build release dev run clean

# Help -------------------------------------------------------------------------

help: ## Show available targets
	@grep -E '^[a-zA-Z0-9_.-]+:.*?## ' Makefile                                \
	| sort                                                                     \
	| awk 'BEGIN {FS = ":.*?## "}; {printf "  make %-10s %s\n", $$1, $$2}'

# Environment ------------------------------------------------------------------

env-up: ## Start environment
	@echo "[*] Starting environment..."
	@docker compose up -d

env-down: ## Stop environment
	@echo "[*] Stopping environment..."
	@docker compose down -v

migrate: ## Apply pending migrations (non-destructive)
	@echo "[*] Applying pending migrations..."
	@$(LOAD_ENV) cargo sqlx migrate run --source crates/db/migrations

migrate-reset: ## Drop and recreate database, then run all migrations (DESTRUCTIVE)
	@echo "[!] Resetting database - all data will be lost."
	@$(LOAD_ENV) cargo sqlx database reset -y --source crates/db/migrations


restart: env-down env-up migrate ## Restart environment and run migrations

# Code Quality -----------------------------------------------------------------

ci: fmt lint build ## CI pipeline (GitHub Actions)

validate: check test ## Validate full local pipeline (requires running environment)

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
	@missing=0;                                                                \
	if [ ! -d crates/api/src ]; then exit 0; fi;                               \
	for f in $$(find crates/api/src -name 'models.rs' -type f | sort); do      \
		for name in $$(grep -A5 'derive.*ToSchema' "$$f"                       \
			| grep -oE 'pub (struct|enum) [A-Za-z0-9_]+'                       \
			| awk '{print $$3}'); do                                           \
			if ! grep -q "$$name" crates/api/src/openapi.rs; then              \
				echo "  $${f#crates/api/src/}: $$name";                        \
				missing=$$((missing + 1));                                     \
			fi;                                                                \
		done;                                                                  \
	done;                                                                      \
	if [ "$$missing" -gt 0 ]; then                                             \
		echo "";                                                               \
		echo "[!] $$missing type(s) with ToSchema not found in openapi.rs";    \
		exit 1;                                                                \
	fi

prepare: ## Generate SQLx offline query metadata for CI builds (bash/zsh)
	@echo "[*] Generating SQLx offline query metadata..."
	@test -f .env || (echo "Error: .env file not found" && exit 1)
	@$(LOAD_ENV)                                                               \
		CARGO_TERM_COLOR=always cargo                                          \
		sqlx prepare --workspace -- --all-features --tests 2>&1                \
		| grep -v 'query data written'

# Testing ----------------------------------------------------------------------

test: ## Run nextest (use ARGS="..." for extra arguments)
	@echo "[*] Running tests..."
	@$(LOAD_ENV) cargo nextest run --all-features --no-fail-fast $(ARGS)

test_one: ## Run single test: `make test_one <test_name>`
	@$(MAKE) test ARGS="$(filter-out $@,$(MAKECMDGOALS))"

test_in: ## Run tests in module: `make test_in <module_name>`
	@$(MAKE) test ARGS="--test $(filter-out $@,$(MAKECMDGOALS))"

test_not: ## Exclude tests: `make test_not <test1> <test2> ...`
	@args="$(filter-out $@,$(MAKECMDGOALS))";                                  \
	expr=$$(printf '%s\n' $$args                                               \
		| sed 's/.*/not test(&)/'                                              \
		| paste -sd' and ' -);                                                 \
	$(MAKE) test ARGS="-E '$$expr'"

# Development ------------------------------------------------------------------

build: ## Build all workspace crates
	@echo "[*] Building workspace..."
	@cargo build --workspace

release: ## Build release binary and copy to project root
	@echo "[*] Building CLI (release)..."
	@cargo build --bin tt --release
	@cp target/release/tt .
	@echo "[+] Built: ./tt"

dev: ## Run CLI in debug mode with .env loaded
	@echo "[*] Running CLI (debug)..."
	@set -a && . ./.env && set +a && cargo run --bin tt

run: ## Run CLI in release mode
	@echo "[*] Running CLI (release)..."
	@cargo run --bin tt --release

clean: ## Clean build artifacts
	@echo "[*] Cleaning build artifacts..."
	@cargo clean

# Prevent "No rule to make target" error for arguments
#   %: — catch-all target, matches any unknown target (e.g. arguments like "test_name")
#   @: — no-op command, does nothing silently
%:
	@:
