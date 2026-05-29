#!/usr/bin/env bash
# Проверка окружения для Install Dev Extension в Zed (Zed вызывает cargo/rustup сам).
set -euo pipefail

ok=true

check() {
  if "$@"; then
    echo "OK   $*"
  else
    echo "FAIL $*"
    ok=false
  fi
}

echo "=== Zed dev extension: проверка окружения ==="
echo

if command -v rustup >/dev/null; then
  echo "OK   rustup: $(command -v rustup)"
  rustup show active-toolchain 2>/dev/null || true
else
  echo "FAIL rustup не найден в PATH"
  echo "     Установите: https://rustup.rs/"
  ok=false
fi

if command -v cargo >/dev/null; then
  echo "OK   cargo: $(command -v cargo)"
  cargo --version
else
  echo "FAIL cargo не найден в PATH"
  ok=false
fi

if command -v rustc >/dev/null; then
  echo "OK   rustc: $(command -v rustc)"
else
  echo "FAIL rustc не найден"
  ok=false
fi

echo
if rustup target list --installed 2>/dev/null | grep -q 'wasm32-wasip2'; then
  echo "OK   target wasm32-wasip2 установлен"
else
  echo "FAIL target wasm32-wasip2 не установлен"
  echo "     Выполните: rustup target add wasm32-wasip2"
  ok=false
fi

echo
echo "PATH (первые компоненты):"
echo "$PATH" | tr ':' '\n' | head -8

if [[ "${PATH:-}" != *".cargo/bin"* ]]; then
  echo
  echo "WARN ~/.cargo/bin нет в PATH — Zed из Dock может не найти cargo."
  echo "     Добавьте в ~/.zprofile:"
  echo '       export PATH="$HOME/.cargo/bin:$PATH"'
  echo "     Или запускайте Zed из терминала: zed"
fi

if command -v brew >/dev/null && brew list rust 2>/dev/null | grep -q rust; then
  echo
  echo "WARN Обнаружен rust из Homebrew — dev extension может не работать."
  echo "     Рекомендуется только rustup: brew uninstall rust"
fi

echo
if $ok; then
  echo "=== Итог: окружение выглядит нормально ==="
  echo "Если Install Dev Extension всё равно падает:"
  echo "  1. Закройте Zed полностью"
  echo "  2. В этом же терминале:"
  echo "       /Applications/Zed.app/Contents/MacOS/cli --foreground"
  echo "     (или zed --foreground после cli: install cli binary в Zed)"
  echo "  3. Install Dev Extension → папка extension/ (с extension.toml)"
  echo "  4. zed: open log — полный stderr cargo"
  exit 0
else
  echo "=== Итог: исправьте ошибки выше ==="
  exit 1
fi
