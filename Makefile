# Games: `make` alone lists the targets. Targets are in alphabetical order; each has its .PHONY line beside it.
# Every target works on the whole workspace; pick one crate with cargo's -p (cargo test -p bounty-core).

# Tools (rumdl, prek, shfmt, shellcheck) live in .venv, installed from the hash-pinned requirements.txt.
VENV = .venv
BIN = $(VENV)/bin

.DEFAULT_GOAL := help

.PHONY: ascii-check
ascii-check: ## Check that code and config are ASCII (see tools/check_ascii.sh)
	git ls-files -z | grep -zE '(\.(rs|toml|py|nix|sh)|Makefile|\.editorconfig)$$' | xargs -0 tools/check_ascii.sh

.PHONY: build
build: ## Build optimised, without running
	cargo build --release --workspace

.PHONY: check
check: ## Type-check, fast
	cargo check --workspace --all-targets

.PHONY: clean
clean: ## Remove build output and the tools
	cargo clean
	rm -rf $(VENV)

.PHONY: docs
docs: tools ## Rewrap and fix Markdown docs (120 columns)
	$(BIN)/rumdl fmt .

.PHONY: docs-check
docs-check: tools ## Check Markdown docs
	$(BIN)/rumdl check .

.PHONY: fmt
fmt: docs shell ## Format everything: Rust, Markdown and shell
	cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Check Rust formatting
	cargo fmt --all --check

.PHONY: help
help: ## List the targets
	@grep -hE '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN { FS = ":.*## " } { printf "  %-14s %s\n", $$1, $$2 }'

.PHONY: hooks
hooks: tools ## Run all git hooks on every file
	$(BIN)/prek run --all-files

.PHONY: hooks-install
hooks-install: tools ## Install the git hooks
	$(BIN)/prek install --hook-type pre-commit --hook-type commit-msg --hook-type pre-push

.PHONY: lint
lint: ## Clippy on code and tests, warnings are errors
	cargo clippy --workspace --all-targets -- -D warnings

.PHONY: shell
shell: tools ## Format shell scripts (shfmt)
	$(BIN)/shfmt -i 2 -ci -w tools

.PHONY: shell-check
shell-check: tools ## Check shell scripts (shfmt, shellcheck)
	$(BIN)/shfmt -i 2 -ci -d tools
	$(BIN)/shellcheck tools/*.sh

.PHONY: test
test: ## Run the tests
	cargo test --workspace

.PHONY: tools
tools: $(VENV)/.installed ## Install the tools into .venv

$(VENV)/.installed: requirements.txt
	python3 -m venv --clear $(VENV)
	$(BIN)/pip install --quiet --disable-pip-version-check --require-hashes -r requirements.txt
	touch $@
