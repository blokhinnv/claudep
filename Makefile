# Основные команды разработки — добавляйте новые цели сюда (сборка, Docker, тесты и т.д.).
# См. docs/development.md

EXTENSION_DIR := extension
WASM_TARGET   := wasm32-wasip2
CARGO         := cargo
RELEASE_WASM  := $(EXTENSION_DIR)/target/$(WASM_TARGET)/release/zed_claude_proxy.wasm

.PHONY: build build-dev setup fetch clean check-zed-env help

.DEFAULT_GOAL := build

help:
	@echo "Targets:"
	@echo "  make build         — release-сборка (для CI / ручной проверки)"
	@echo "  make build-dev     — debug-сборка (как при Install Dev Extension в Zed)"
	@echo "  make check-zed-env — проверить cargo/rustup/wasm32-wasip2 для Zed"
	@echo "  make fetch         — только скачать зависимости (без компиляции)"
	@echo "  make setup         — rustup target add $(WASM_TARGET)"
	@echo "  make clean         — cargo clean в $(EXTENSION_DIR)/"

setup:
	rustup target add $(WASM_TARGET)

fetch: setup
	cd $(EXTENSION_DIR) && $(CARGO) fetch --target $(WASM_TARGET)

# Zed при Install Dev Extension вызывает: cargo build --target wasm32-wasip2 (без --release)
build-dev: setup
	cd $(EXTENSION_DIR) && $(CARGO) build --target $(WASM_TARGET)
	@echo "Built (debug): $(EXTENSION_DIR)/target/$(WASM_TARGET)/debug/zed_claude_proxy.wasm"

build: setup
	cd $(EXTENSION_DIR) && $(CARGO) build --release --target $(WASM_TARGET)
	@echo "Built: $(RELEASE_WASM)"

check-zed-env:
	@bash scripts/check-zed-env.sh

clean:
	cd $(EXTENSION_DIR) && $(CARGO) clean
