# KaosNet Makefile
# Usage: make help

.PHONY: help build test run clean docker seed reset console

# Colors
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RESET := \033[0m

help: ## Show this help
	@echo "$(CYAN)KaosNet - Game Server Framework$(RESET)"
	@echo ""
	@echo "$(YELLOW)Usage:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(RESET) %s\n", $$1, $$2}'

# ============================================================================
# Development
# ============================================================================

build: ## Build kaosnet library
	cargo build -p kaosnet --features "console,lua"

build-release: ## Build release version
	cargo build -p kaosnet --features "console,lua" --release

test: ## Run all tests
	cargo test -p kaosnet

test-verbose: ## Run tests with output
	cargo test -p kaosnet -- --nocapture

check: ## Run clippy lints
	cargo clippy -p kaosnet --features "console,lua" -- -D warnings

fmt: ## Format code
	cargo fmt -p kaosnet

# ============================================================================
# Running
# ============================================================================

run-demo: ## Run kaos.io demo server
	cargo run -p kaos-io --bin kaos-io-server

run-asteroids: ## Run asteroids demo server
	cargo run -p kaos-asteroids --bin asteroids-server

run-client: ## Run asteroids client (pass NAME=YourName)
	cargo run -p kaos-asteroids --bin asteroids-client $(NAME)

run-console: ## Start console UI dev server
	cd ../console-ui && npm run dev

# ============================================================================
# Docker
# ============================================================================

docker-build: ## Build Docker image
	docker build -t kaosnet:latest .

docker-up: ## Start all services with Docker Compose
	docker compose up -d

docker-down: ## Stop all Docker services
	docker compose down

docker-logs: ## Show Docker logs
	docker compose logs -f

docker-ps: ## Show running containers
	docker compose ps

docker-reset: ## Reset Docker (remove volumes)
	docker compose down -v
	docker compose up -d

# ============================================================================
# Database & Sample Data
# ============================================================================

seed: ## Seed database with sample data
	@echo "$(CYAN)Seeding sample data...$(RESET)"
	@./scripts/seed.sh

reset: ## Reset database and reseed
	@echo "$(YELLOW)Resetting database...$(RESET)"
	@./scripts/reset.sh
	@echo "$(GREEN)Database reset complete$(RESET)"

migrate: ## Run database migrations
	@echo "$(CYAN)Running migrations...$(RESET)"
	psql -h localhost -U kaos -d kaosnet -f src/storage/postgres_schema.sql

# ============================================================================
# Console API
# ============================================================================

console-login: ## Get auth token (admin/admin)
	@curl -s -X POST http://localhost:7350/api/auth/login \
		-H "Content-Type: application/json" \
		-d '{"username":"admin","password":"admin"}' | jq .

console-status: ## Get server status
	@curl -s http://localhost:7350/api/status | jq .

console-players: ## List players
	@curl -s http://localhost:7350/api/players | jq .

console-leaderboards: ## List leaderboards
	@curl -s http://localhost:7350/api/leaderboards | jq .

# ============================================================================
# Cleanup
# ============================================================================

clean: ## Clean build artifacts
	cargo clean -p kaosnet

clean-all: ## Clean all build artifacts
	cargo clean

# ============================================================================
# Documentation
# ============================================================================

docs: ## Generate documentation
	cargo doc -p kaosnet --features "console,lua" --no-deps --open

# ============================================================================
# Benchmarks
# ============================================================================

bench: ## Run benchmarks
	cargo bench -p kaosnet
