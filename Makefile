SHELL := /bin/bash
.DEFAULT_GOAL := help

RUST_CRATES := auctioning auctioning-magicblock shuttle-auctioning
DATABASE_URL ?= postgres://postgres:postgres@127.0.0.1:5432/auctioning

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# ---------------------------------------------------------------- quality gate
.PHONY: check
check: fmt-check clippy test ## fmt + clippy + tests (what CI runs)

.PHONY: fmt
fmt: ## Format all Rust code
	cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Verify formatting
	cargo fmt --all -- --check

.PHONY: clippy
clippy: ## Lint (same flags as CI)
	cargo clippy --workspace --exclude leptos-auctioning --all-targets -- \
		-D warnings -A dead_code -A deprecated -A unexpected_cfgs -A ambiguous_glob_reexports

.PHONY: test
test: ## Unit + pure-logic tests for every native crate (no DB; integration_rp self-skips)
	@for c in $(RUST_CRATES); do env -u DATABASE_URL cargo test -p $$c --no-fail-fast || exit 1; done

.PHONY: test-db
test-db: ## Postgres smoke + integration tests (needs DATABASE_URL, see db-up)
	DATABASE_URL=$(DATABASE_URL) cargo test -p shuttle-auctioning --test integration_rp --no-fail-fast
	DATABASE_URL=$(DATABASE_URL) cargo test -p shuttle-auctioning --features sqlx-test --test smoke_db --no-fail-fast

.PHONY: wasm-check
wasm-check: ## Type-check the Leptos dApp for wasm32
	cargo check -p leptos-auctioning --target wasm32-unknown-unknown

.PHONY: deny
deny: ## Supply-chain check (cargo-deny)
	cargo deny check advisories bans sources licenses

.PHONY: audit
audit: ## RustSec + npm advisories
	cargo audit
	cd marketing && npm audit --audit-level=high

# ---------------------------------------------------------------- local run
.PHONY: db-up
db-up: ## Start local Postgres 16 (docker compose)
	docker compose up -d postgres
	@until docker compose exec -T postgres pg_isready -U postgres >/dev/null 2>&1; do sleep 1; done
	@echo "postgres ready at $(DATABASE_URL)"

.PHONY: db-down
db-down: ## Stop local Postgres
	docker compose down

.PHONY: db-reset
db-reset: ## Drop volumes and restart Postgres (destructive)
	docker compose down -v && $(MAKE) db-up

.PHONY: api
api: ## Run the backend without Shuttle on :8000 (reads .env)
	set -a && source .env && set +a && export DATABASE_URL=$${DATABASE_URL:-$(DATABASE_URL)} && \
	RUST_LOG=$${RUST_LOG:-info,sqlx=warn} cargo run -p shuttle-auctioning --bin auctioning-api-runner

.PHONY: app
app: ## Leptos dApp dev server on :3000
	cd app/leptos-auctioning && trunk serve

.PHONY: web
web: ## Marketing site dev server
	cd marketing && npm run dev

.PHONY: web-build
web-build: ## Production build of the marketing site
	cd marketing && npm ci && npm run build

.PHONY: seed
seed: ## Dry-run the outbid.lol seeder against the sample snapshot
	python3 tools/seeder/outbid_seed.py --snapshot tools/seeder/seed.sample.json

.PHONY: gen-test-includes
gen-test-includes: ## Regenerate backend/.../tests/inc (legacy; see CHANGELOG)
	backend/shuttle-auctioning/scripts/gen-integration-includes.sh

# ---------------------------------------------------------------- program
.PHONY: program-build
program-build: ## anchor build (needs anchor-cli + keys/auctioning-keypair.json)
	mkdir -p programs/auctioning/target/deploy
	cp keys/auctioning-keypair.json programs/auctioning/target/deploy/auctioning-keypair.json
	cd programs/auctioning && anchor build

.PHONY: program-test
program-test: ## Pure-logic + PDA contract tests (no validator)
	cargo test -p auctioning

# ---------------------------------------------------------------- housekeeping
.PHONY: clean
clean: ## Remove build output
	cargo clean
	rm -rf app/leptos-auctioning/dist marketing/.next marketing/out
