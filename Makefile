.PHONY: build test fmt clippy check install-local render-fixture templates-tar clean help

CARGO ?= cargo

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
