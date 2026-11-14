# Bounty: `make` alone lists the targets. Targets are in alphabetical order; each has its .PHONY line beside it.
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

# The suites of Bounty: the report each writes under target/ and the baseline it is held to.
BALANCE_SUITES = suite:bounty-balance:prototype-0001 reference-suite:bounty-balance-reference:reference-0001 \
	modern-suite:bounty-balance-modern:modern-0001
# The expeditions whose routes are played: the folder under content/, the rules it is played under (a folder
# under content/, or - for the rules the expedition names) and the baseline.
BALANCE_EXPEDITIONS = demo:-:expedition-demo-0001 meadow:reference:expedition-meadow-reference-0001 \
	meadow:modern:expedition-meadow-modern-0001

.PHONY: balance
balance: ## Play the fixed battles and expedition routes; reports in target/bounty-*.json
	@for entry in $(BALANCE_SUITES); do \
		suite=$${entry%%:*}; rest=$${entry#*:}; report=$${rest%%:*}; \
		cargo run --quiet --release -p bounty-tools -- balance run --suite balance/$$suite.json \
			--out target/$$report.json --revision "$$(git describe --always --dirty)" $(BALANCE_ARGS) || exit 1; \
	done
	@for entry in $(BALANCE_EXPEDITIONS); do \
		name=$${entry%%:*}; rest=$${entry#*:}; rules=$${rest%%:*}; baseline=$${rest#*:}; \
		under=""; [ "$$rules" = - ] || under="--rules content/$$rules/rules.json"; \
		cargo run --quiet --release -p bounty-tools -- expedition run \
			--expedition content/$$name/expedition.json $$under --out target/bounty-$$baseline.json \
			--revision "$$(git describe --always --dirty)" || exit 1; \
	done

.PHONY: balance-check
balance-check: balance ## Compare the fresh reports with the recorded baselines; anything that moved fails
	@for entry in $(BALANCE_SUITES); do \
		rest=$${entry#*:}; report=$${rest%%:*}; baseline=$${rest#*:}; \
		cargo run --quiet --release -p bounty-tools -- balance compare --exact \
			--baseline balance/baselines/$$baseline.json --candidate target/$$report.json || exit 1; \
	done
	@for entry in $(BALANCE_EXPEDITIONS); do \
		baseline=$${entry##*:}; \
		cargo run --quiet --release -p bounty-tools -- expedition compare \
			--baseline balance/baselines/$$baseline.json \
			--candidate target/bounty-$$baseline.json || exit 1; \
	done

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
