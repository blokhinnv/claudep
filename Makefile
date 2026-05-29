.PHONY: build test fmt clippy check install-local render-fixture templates-tar \
	clean help release release-bump release-verify release-push release-dry-run

CARGO ?= cargo
VERSION ?=

build:
	$(CARGO) build --release

test:
	$(CARGO) test

fmt:
	$(CARGO) fmt --all

clippy:
	$(CARGO) clippy --all-targets -- -D warnings

check: fmt clippy test

install-local:
	$(CARGO) install --path . --force

render-fixture:
	$(CARGO) run --bin render-fixture

templates-tar:
	mkdir -p dist
	tar -czf dist/templates.tar.gz -C templates .

clean:
	$(CARGO) clean
	rm -rf dist

# --- Release (CI builds binaries on tag push; see .github/workflows/release.yml) ---

cargo-version = $(shell grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

release-bump:
ifndef VERSION
	$(error Set VERSION, e.g. make release-bump VERSION=0.1.1)
endif
	@perl -pi -e 's/^version = .*/version = "$(VERSION)"/' Cargo.toml
	$(CARGO) check --quiet
	@echo "Cargo.toml version is now $(VERSION)"

release-verify:
ifndef VERSION
	$(error Set VERSION, e.g. make release VERSION=0.1.1)
endif
	@test "$(cargo-version)" = "$(VERSION)" || \
		(echo "error: Cargo.toml has $(cargo-version), expected $(VERSION)" >&2; \
		 echo "hint: make release-bump VERSION=$(VERSION)" >&2; exit 1)

release-dry-run: release-verify
	@echo "Would run: make check"
	@echo "Would commit (if needed): Cargo.toml Cargo.lock with message 'Release v$(VERSION)'"
	@echo "Would push: origin HEAD"
	@echo "Would tag:  v$(VERSION)"
	@echo "Would push: origin v$(VERSION)"
	@echo "Then GitHub Actions release.yml builds binaries and publishes the release."

release-push: release-verify
	git push origin HEAD
	@if git rev-parse "v$(VERSION)" >/dev/null 2>&1; then \
		echo "error: tag v$(VERSION) already exists locally" >&2; exit 1; \
	fi
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	git push origin "v$(VERSION)"

# Full release: tests, optional version commit, push master, tag, push tag.
# Usage: make release-bump VERSION=0.1.1   # if bumping
#        git add … && git commit …          # other release changes
#        make release VERSION=0.1.1
release: check release-verify
	@git diff --quiet Cargo.toml Cargo.lock 2>/dev/null || \
		(git add Cargo.toml Cargo.lock && git commit -m "Release v$(VERSION)")
	@$(MAKE) release-push VERSION=$(VERSION)
	@echo "Release v$(VERSION) pushed. Watch: gh run list --workflow=release.yml"

help:
	@echo "Targets:"
	@echo "  build           Release build"
	@echo "  test            Run unit tests"
	@echo "  fmt             cargo fmt --check"
	@echo "  clippy          Run clippy"
	@echo "  check           fmt + clippy + test"
	@echo "  install-local   cargo install --path ."
	@echo "  render-fixture  Write sample compose to /tmp/claudep-fixture-state/"
	@echo "  templates-tar   Build dist/templates.tar.gz"
	@echo "  clean           Remove build artifacts"
	@echo ""
	@echo "Release (binaries built by GitHub Actions on tag push):"
	@echo "  release-bump VERSION=x.y.z   Set version in Cargo.toml"
	@echo "  release-dry-run VERSION=x.y.z  Show release steps without executing"
	@echo "  release VERSION=x.y.z        check + commit lockfile + push + tag"
