# Trueno-Viz Makefile
# Lean-Scientific Development Workflow with PMAT Integration
#
# Quality Philosophy:
#   - Toyota Way: Jidoka (stop the line), Kaizen (continuous improvement)
#   - Scientific Rigor: Evidence-based, reproducible, peer-reviewed
#   - Zero Tolerance: 95% coverage, 80% mutation score, A- TDG grade

SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help

# ============================================================================
# Configuration
# ============================================================================

CARGO := cargo
PMAT := pmat
RUSTFLAGS := -D warnings

# Quality thresholds (enforced)
MIN_COVERAGE := 95
MIN_MUTATION_SCORE := 80
MIN_TDG_GRADE := A-
MAX_COMPLEXITY := 15
MAX_DEAD_CODE_PCT := 5.0
MAX_UNWRAP_CALLS := 50

# Paths
SRC_DIR := src
TEST_DIR := tests
DOCS_DIR := docs
PMAT_DIR := .pmat
BASELINE_FILE := $(PMAT_DIR)/tdg-baseline.json

# ============================================================================
# Help
# ============================================================================

.PHONY: help
help: ## Show this help
	@echo "Trueno-Viz Development Commands"
	@echo "================================"
	@echo ""
	@echo "Build & Test:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '(build|test|check)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'
	@echo ""
	@echo "Quality Gates (PMAT):"
	@grep -E '^pmat-[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'
	@echo ""
	@echo "Development:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -vE '(build|test|check|pmat-)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'

# ============================================================================
# Build Commands
# ============================================================================

.PHONY: build
build: ## Build the project (debug)
	$(CARGO) build

.PHONY: build-release
build-release: ## Build the project (release, optimized)
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO) build --release

.PHONY: build-wasm
build-wasm: ## Build for WebAssembly target (wasm-pack)
	@command -v wasm-pack >/dev/null 2>&1 || { echo "Installing wasm-pack..."; cargo install wasm-pack; }
	wasm-pack build --target web --features wasm --out-dir pkg
	@cp pkg/README.md pkg/README.md.bak 2>/dev/null || true
	@if [ -f pkg/package.json.template ]; then \
		jq -s '.[0] * .[1]' pkg/package.json pkg/package.json.template > pkg/package.json.tmp && \
		mv pkg/package.json.tmp pkg/package.json; \
	fi
	@mv pkg/README.md.bak pkg/README.md 2>/dev/null || true
	@echo "WASM package built in pkg/"

.PHONY: build-wasm-nodejs
build-wasm-nodejs: ## Build WASM for Node.js
	wasm-pack build --target nodejs --features wasm --out-dir pkg-node

.PHONY: build-wasm-bundler
build-wasm-bundler: ## Build WASM for bundlers (webpack, etc)
	wasm-pack build --target bundler --features wasm --out-dir pkg-bundler

# ============================================================================
# Test Commands
# ============================================================================

.PHONY: test
test: ## Run all tests
	$(CARGO) test --all-features

.PHONY: test-fast
test-fast: ## Run tests without slow integration tests
	$(CARGO) test --lib

.PHONY: test-doc
test-doc: ## Run documentation tests
	$(CARGO) test --doc

.PHONY: test-coverage
test-coverage: ## Run tests with coverage report
	$(CARGO) tarpaulin --out Html --output-dir target/coverage \
		--fail-under $(MIN_COVERAGE) \
		--skip-clean \
		--timeout 300

# ============================================================================
# Lint & Format
# ============================================================================

.PHONY: fmt
fmt: ## Format code
	$(CARGO) fmt

.PHONY: fmt-check
fmt-check: ## Check code formatting
	$(CARGO) fmt -- --check

.PHONY: clippy
clippy: ## Run Clippy lints
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: lint
lint: fmt-check clippy ## Run all lints

# ============================================================================
# PMAT Quality Gates
# ============================================================================

.PHONY: pmat-init
pmat-init: ## Initialize PMAT configuration
	@mkdir -p $(PMAT_DIR)
	@echo "Initializing PMAT quality gates..."
	$(PMAT) diagnose --full || true

.PHONY: pmat-baseline
pmat-baseline: pmat-init ## Create TDG baseline (run once, then on major releases)
	$(PMAT) tdg baseline create --output $(BASELINE_FILE) --path .
	@echo "Baseline created at $(BASELINE_FILE)"

.PHONY: pmat-quality
pmat-quality: ## Run O(1) quality gates (<30ms)
	$(PMAT) quality-gate --fail-on-violation

.PHONY: pmat-score
pmat-score: ## Full Rust project quality assessment (211 pts scale)
	$(PMAT) rust-project-score --full

.PHONY: pmat-score-json
pmat-score-json: ## Rust project score as JSON (for CI)
	$(PMAT) rust-project-score --full --format json --output $(PMAT_DIR)/score.json
	@cat $(PMAT_DIR)/score.json | jq '.total_earned, .total_possible'

.PHONY: pmat-tdg
pmat-tdg: ## Analyze technical debt (A+ to F grade)
	$(PMAT) analyze satd --path . --format table

.PHONY: pmat-tdg-check
pmat-tdg-check: ## Check for TDG regression against baseline
	@if [ -f $(BASELINE_FILE) ]; then \
		$(PMAT) tdg check-regression \
			--baseline $(BASELINE_FILE) \
			--max-score-drop 5.0 \
			--fail-on-regression; \
	else \
		echo "No baseline found. Run 'make pmat-baseline' first."; \
		exit 1; \
	fi

.PHONY: pmat-complexity
pmat-complexity: ## Analyze cyclomatic/cognitive complexity hotspots
	$(PMAT) analyze complexity --top-files 10

.PHONY: pmat-dead-code
pmat-dead-code: ## Detect dead/unreachable code
	$(PMAT) analyze dead-code --max-percentage $(MAX_DEAD_CODE_PCT)

.PHONY: pmat-defects
pmat-defects: ## Scan for known defect patterns (unwrap, expect, etc.)
	$(PMAT) analyze defects --path .

.PHONY: pmat-mutate
pmat-mutate: ## Run mutation testing (test quality validation)
	$(PMAT) mutate --target $(SRC_DIR)/ --threshold $(MIN_MUTATION_SCORE)

.PHONY: pmat-mutate-failures
pmat-mutate-failures: ## Show only surviving mutants (test gaps)
	$(PMAT) mutate --target $(SRC_DIR)/ --failures-only

.PHONY: pmat-context
pmat-context: ## Generate AI-ready deep context for Claude
	$(PMAT) context --output deep_context.md --format llm-optimized

.PHONY: pmat-dag
pmat-dag: ## Generate dependency graph (Mermaid)
	$(PMAT) analyze dag --format mermaid --output $(DOCS_DIR)/dependency-graph.md

.PHONY: pmat-churn
pmat-churn: ## Analyze code churn (change frequency)
	$(PMAT) analyze churn --days 30

.PHONY: pmat-validate-docs
pmat-validate-docs: ## Validate README accuracy (hallucination detection)
	$(PMAT) validate-readme --targets README.md CONTRIBUTING.md

.PHONY: pmat-repo-health
pmat-repo-health: ## Quick repository health score (0-110)
	$(PMAT) repo-score

.PHONY: pmat-metrics
pmat-metrics: ## Show quality metrics trend
	$(PMAT) show-metrics --trend

# ============================================================================
# Composite Quality Commands
# ============================================================================

.PHONY: check
check: lint test ## Basic CI check (lint + test)

.PHONY: quality
quality: lint pmat-quality pmat-tdg-check test ## Full quality gate (pre-commit)
	@echo "All quality gates passed!"

.PHONY: quality-full
quality-full: quality pmat-mutate pmat-dead-code pmat-complexity ## Comprehensive quality (pre-merge)
	@echo "Comprehensive quality validation complete!"

.PHONY: self-check
self-check: ## Author's pre-submission checklist
	@echo "=== Self-Check: Pre-Submission Validation ==="
	@echo ""
	@echo "1. Formatting..."
	@$(CARGO) fmt -- --check || (echo "FAIL: Run 'make fmt'" && exit 1)
	@echo "   PASS"
	@echo ""
	@echo "2. Clippy lints..."
	@$(CARGO) clippy --all-targets --all-features -- -D warnings || (echo "FAIL: Fix Clippy warnings" && exit 1)
	@echo "   PASS"
	@echo ""
	@echo "3. Tests..."
	@$(CARGO) test --quiet || (echo "FAIL: Tests failed" && exit 1)
	@echo "   PASS"
	@echo ""
	@echo "4. Quality gates..."
	@$(PMAT) quality-gate --fail-on-violation || (echo "FAIL: Quality gate violation" && exit 1)
	@echo "   PASS"
	@echo ""
	@echo "5. Technical debt..."
	@$(PMAT) analyze satd --path . --format table
	@echo ""
	@echo "=== Self-Check Complete: Ready for Review ==="

.PHONY: verify-reproducibility
verify-reproducibility: ## Reviewer's reproducibility audit
	@echo "=== Reproducibility Audit ==="
	@echo ""
	@echo "1. Clean build..."
	@$(CARGO) clean
	@$(CARGO) build || (echo "FAIL: Build failed" && exit 1)
	@echo "   PASS"
	@echo ""
	@echo "2. Test run 1/3..."
	@$(CARGO) test --quiet || (echo "FAIL: Tests failed (run 1)" && exit 1)
	@echo "   PASS"
	@echo ""
	@echo "3. Test run 2/3..."
	@$(CARGO) test --quiet || (echo "FAIL: Tests failed (run 2)" && exit 1)
	@echo "   PASS"
	@echo ""
	@echo "4. Test run 3/3..."
	@$(CARGO) test --quiet || (echo "FAIL: Tests failed (run 3)" && exit 1)
	@echo "   PASS"
	@echo ""
	@echo "=== Reproducibility Audit: PASS ==="
	@echo "Record: 'Reproduced locally on $$(date +%Y-%m-%d)'"

# ============================================================================
# CI/CD Pipeline Targets
# ============================================================================

.PHONY: ci
ci: lint test-coverage pmat-quality pmat-score-json ## Full CI pipeline
	@echo "CI pipeline complete"

.PHONY: ci-quick
ci-quick: lint test-fast pmat-quality ## Quick CI (PRs)
	@echo "Quick CI complete"

.PHONY: ci-nightly
ci-nightly: quality-full pmat-mutate test-coverage ## Nightly comprehensive CI
	@echo "Nightly CI complete"

# ============================================================================
# Development Utilities
# ============================================================================

.PHONY: dev
dev: ## Start development (watch mode)
	$(CARGO) watch -x check -x test

.PHONY: doc
doc: ## Generate documentation
	$(CARGO) doc --no-deps --open

.PHONY: readme
readme: ## Regenerate README.md from template (reproducible docs)
	@echo "Regenerating README.md..."
	@$(CARGO) build --example readme_demo --quiet
	@./scripts/generate-readme.sh > README.md
	@echo "README.md updated"

.PHONY: bench
bench: ## Run benchmarks
	$(CARGO) bench

.PHONY: clean
clean: ## Clean build artifacts
	$(CARGO) clean
	rm -rf target/coverage
	rm -f deep_context.md

.PHONY: clean-all
clean-all: clean ## Clean everything including PMAT cache
	rm -rf $(PMAT_DIR)
	$(PMAT) cache clear || true

# ============================================================================
# Release
# ============================================================================

.PHONY: release-check
release-check: quality-full pmat-score ## Pre-release validation
	@echo ""
	@echo "=== Release Readiness Check ==="
	@$(PMAT) rust-project-score --full
	@echo ""
	@echo "Review the score above. Target: 180+/211 (85%+)"
	@echo "If ready, run: make release-tag VERSION=x.y.z"

.PHONY: release-tag
release-tag: ## Tag a release (requires VERSION=x.y.z)
ifndef VERSION
	$(error VERSION is required. Usage: make release-tag VERSION=0.1.0)
endif
	@echo "Tagging release v$(VERSION)..."
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	@echo "Run 'git push origin v$(VERSION)' to publish"

# ============================================================================
# MCP Server (for Claude Code integration)
# ============================================================================

.PHONY: mcp-server
mcp-server: ## Start PMAT MCP server for Claude Code
	$(PMAT) mcp

.PHONY: mcp-demo
mcp-demo: ## Interactive PMAT demo (web-based)
	$(PMAT) demo --url .

# ============================================================================
# Hooks Management
# ============================================================================

.PHONY: hooks-install
hooks-install: ## Install Git pre-commit hooks
	$(PMAT) hooks install
	@echo "Pre-commit hooks installed"

.PHONY: hooks-status
hooks-status: ## Check hook installation status
	$(PMAT) hooks status

# ============================================================================
# Roadmap Integration
# ============================================================================

.PHONY: roadmap
roadmap: ## Show project roadmap with quality gates
	$(PMAT) roadmap status || echo "No roadmap configured. Use 'pmat roadmap create'"

.PHONY: roadmap-create
roadmap-create: ## Create initial roadmap
	$(PMAT) roadmap create --output $(PMAT_DIR)/roadmap.toml

# ============================================================================
# Phony declarations for safety
# ============================================================================

.PHONY: all
all: build test quality
