# Games: `make` alone lists the targets. Targets are in alphabetical order; each has its .PHONY line beside it.
# Every target works on the whole workspace; pick one crate with cargo's -p (cargo test -p bounty-core).

# On NixOS, cargo runs inside nix-shell so a game finds the libraries it loads at run time (GL, X11). Elsewhere, cargo
# runs directly.
ifneq ($(wildcard /etc/NIXOS),)
  CARGO = nix-shell --run "cargo $(1)"
  NEW_VENV = nix-shell --run "python3 -m venv --clear $(1)"
else
  CARGO = cargo $(1)
  NEW_VENV = python3 -m venv --clear $(1)
endif

# Tools (rumdl, prek, shfmt, shellcheck) live in .venv, installed from the hash-pinned requirements.txt.
VENV = .venv
BIN = $(VENV)/bin

.DEFAULT_GOAL := help

.PHONY: ascii-check
ascii-check: ## Check that code and config are ASCII (see tools/check_ascii.sh)
	git ls-files -z | grep -zE '(\.(rs|toml|py|nix|sh)|Makefile|\.editorconfig)$$' | xargs -0 tools/check_ascii.sh

.PHONY: balance
balance: ## Bounty: play the fixed battles and write the report to target/bounty-balance.json
	cargo run --quiet --release -p bounty-tools -- balance run --suite bounty/balance/suite.json \
		--out target/bounty-balance.json --revision "$$(git describe --always --dirty)" $(BALANCE_ARGS)

.PHONY: balance-check
balance-check: balance ## Bounty: compare the fresh report with the recorded baseline; any battle that moved fails
	cargo run --quiet --release -p bounty-tools -- balance compare --exact \
		--baseline bounty/balance/baselines/prototype-0001.json --candidate target/bounty-balance.json

.PHONY: build
build: ## Build optimised, without running
	$(call CARGO,build --release --workspace)

.PHONY: check
check: ## Type-check, fast
	$(call CARGO,check --workspace --all-targets)

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
	$(call CARGO,clippy --workspace --all-targets -- -D warnings)

.PHONY: shell
shell: tools ## Format shell scripts (shfmt)
	$(BIN)/shfmt -i 2 -ci -w tools

.PHONY: shell-check
shell-check: tools ## Check shell scripts (shfmt, shellcheck)
	$(BIN)/shfmt -i 2 -ci -d tools
	$(BIN)/shellcheck tools/*.sh

.PHONY: test
test: ## Run the tests
	$(call CARGO,test --workspace)

.PHONY: tools
tools: $(VENV)/.installed ## Install the tools into .venv

$(VENV)/.installed: requirements.txt
	$(call NEW_VENV,$(VENV))
	$(BIN)/pip install --quiet --disable-pip-version-check --require-hashes -r requirements.txt
	touch $@
