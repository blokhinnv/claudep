# Разработка расширения

Инструкции по локальной сборке и установке **Zed Claude Proxy** как dev extension.

## Что установить (кратко)

| Для чего | macOS | Обязательно сейчас (фаза 0) |
|----------|-------|-----------------------------|
| Сборка WASM | Rust через **rustup** + target `wasm32-wasip2` | да |
| Линкер / компилятор C | Xcode Command Line Tools | да (обычно уже есть) |
| Проверка в IDE | [Zed](https://zed.dev) **1.4.x** (или 0.205+) | да |
| Контейнеры Claude | [Docker Desktop](https://www.docker.com/products/docker-desktop/) | нет (фаза 2+) |

## Установка на macOS

### 1. Xcode Command Line Tools

Нужны для сборки Rust и нативных зависимостей. Если ещё не ставили:

```bash
xcode-select --install
```

Проверка:

```bash
xcode-select -p
# ожидается: /Library/Developer/CommandLineTools или путь к Xcode.app
```

### 2. Rust (только через rustup)

**Не ставьте Rust через Homebrew** (`brew install rust`) — Zed при dev extension ожидает toolchain от rustup.

Установка (официальный скрипт):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Выберите вариант по умолчанию (`1`), затем перезагрузите shell или выполните:

```bash
source "$HOME/.cargo/env"
```

Проверка:

```bash
rustc --version
cargo --version
which cargo
# ожидается путь вида /Users/<you>/.cargo/bin/cargo
```

### 3. WASM target для Zed

```bash
rustup target add wasm32-wasip2
rustup target list --installed | grep wasm32-wasip2
```

### 4. Zed

Скачайте с [zed.dev](https://zed.dev) или установите, если ещё нет.

**Рекомендуется:** Zed **1.4.x** (например, 1.4.2) — актуальная stable-линейка с нумерацией `1.x`. Старые релизы `0.205.x` и новее тоже подходят, если в них доступен extension API 0.7.

Проверка версии: **Zed → About Zed** (или Command Palette → `about`).

Для **Zed 1.4.2** менять код расширения не нужно: в репозитории уже `zed_extension_api = 0.7.0`, это максимум для stable-сборок Zed.

### 5. Docker Desktop (позже, для полного MVP)

Для фазы 0 **не обязателен**. Понадобится, когда будете поднимать контейнеры (фаза 2+):

- [Docker Desktop for Mac](https://www.docker.com/products/docker-desktop/) (Apple Silicon или Intel — свой установщик)
- После установки: `docker compose version` в терминале

На Apple Silicon убедитесь, что Docker запущен (иконка в menu bar).

### Типичные проблемы на macOS

| Симптом | Решение |
|---------|---------|
| `cargo: command not found` | `source "$HOME/.cargo/env"` или перезапустите терминал; добавьте в `~/.zshrc`: `. "$HOME/.cargo/env"` |
| Dev extension в Zed не собирается | Убедитесь, что `which cargo` указывает на `~/.cargo/bin/cargo`, не на Homebrew |
| Ошибки линковки при `cargo build` | Установите Command Line Tools (`xcode-select --install`) |
| `rustup target add wasm32-wasip2` падает | Обновите rustup: `rustup update stable` |
| «unsupported» / «incompatible» extension API | На stable нельзя собирать с API **0.8+** (ещё не на crates.io). Оставьте `zed_extension_api = "0.7.0"` |
| `curl failed` / `Timeout was reached` при `cargo build` | См. [Проблемы с crates.io](#проблемы-с-cratesio) ниже |

### Проблемы с crates.io

Ошибка вида `download of fu/tu/futures failed` / `unable to update registry crates-io` — **не баг проекта**, а медленный или обрывающийся доступ к [crates.io](https://crates.io). Cargo обрывает загрузку, если скорость падает ниже ~10 байт/с дольше 30 секунд.

**Что сделать по порядку:**

1. **Повторить** при нормальном интернете (или через VPN, если crates.io режется провайдером):

   ```bash
   make fetch    # только скачать зависимости
   make build
   ```

2. В репозитории уже есть [`.cargo/config.toml`](../.cargo/config.toml) с `net.retry = 10` — больше повторов при обрыве.

3. **Проверить прокси** — если в shell заданы `HTTP_PROXY` / `HTTPS_PROXY`, они должны быть рабочими, иначе отключите на время сборки:

   ```bash
   unset HTTP_PROXY HTTPS_PROXY ALL_PROXY
   make build
   ```

4. **Зеркало индекса** (если crates.io стабильно недоступен). Добавьте в `~/.cargo/config.toml` (или допишите в `.cargo/config.toml` проекта):

   ```toml
   [source.crates-io]
   replace-with = "rsproxy-sparse"

   [source.rsproxy-sparse]
   registry = "sparse+https://rsproxy.cn/index/"
   ```

   Затем снова `make fetch && make build`. Зеркало [rsproxy](https://rsproxy.cn/) ориентировано на Китай; для других регионов чаще помогает VPN к официальному индексу.

5. **Диагностика сети:**

   ```bash
   curl -I https://index.crates.io/
   cargo --version
   ```

Если после `make fetch` зависимости лежат в кэше (`~/.cargo/registry`), повторный `make build` может пройти даже при слабом канале.

## Требования

| Компонент | Версия / примечание |
|-----------|---------------------|
| [Zed](https://zed.dev) | **1.4.x** (рекомендуется) или **0.205+** с поддержкой API 0.7 |
| Rust | Установлен через [rustup](https://rustup.rs/) (**не** Homebrew — иначе dev extension может не собраться) |
| WASM target | `wasm32-wasip2` |
| Docker | Для фаз 2+ (поднятие контейнеров) |

## Совместимость Zed и extension API

Zed проверяет не «версию приложения 1.4.2» напрямую, а **версию API**, вшитую в WASM (`zed:api-version`). В [`extension/Cargo.toml`](../extension/Cargo.toml) зафиксировано **`zed_extension_api = "0.7.0"`**.

### Zed 1.4.x (в т.ч. 1.4.2)

| Канал Zed | Поддерживаемый `zed_extension_api` | Наш проект |
|-----------|-----------------------------------|------------|
| **Stable / Preview** | `0.0.1` … **`0.7.0`** | **0.7.0** — подходит |
| Dev / Nightly | до `0.8.0` | 0.8 пока **не** на [crates.io](https://crates.io/crates/zed_extension_api); для stable не требуется |

Источник: [Zed v1.4.2 — `wasm_api_version_range`](https://github.com/zed-industries/zed/blob/v1.4.2/crates/extension_host/src/wasm_host/wit.rs) (stable ограничен `0.7.0`).

### Старая нумерация Zed (0.x)

| Zed (0.x) | `zed_extension_api` |
|-----------|---------------------|
| `0.205.x` | `0.0.1` … `0.7.0` |
| `0.192.x` | `0.0.1` … `0.6.0` (для 0.7 нужен более новый Zed) |

**Итог для Zed 1.4.2:** код и зависимости менять не нужно; достаточно собрать WASM и установить dev extension. Target **`wasm32-wasip2`** без изменений.

## Makefile

**Основные команды для терминала** (сборка, Docker, тесты и т.п.) добавляются в корневой [`Makefile`](../Makefile), а не размазываются по README. Сейчас:

| Команда | Назначение |
|---------|------------|
| `make` / `make build` | Release-сборка WASM расширения |
| `make build-dev` | Debug-сборка (как у Zed при Install Dev Extension) |
| `make check-zed-env` | Проверка cargo/rustup/target для Zed |
| `make fetch` | Скачать зависимости без компиляции (удобно при плохой сети) |
| `make setup` | Установить Rust target `wasm32-wasip2` |
| `make clean` | Очистить `extension/target/` |
| `make help` | Список целей |

При добавлении новой операции (например, `docker compose` для фазы 2) — новая цель в `Makefile` и строка в этой таблице.

## Toolchain

Один раз (или через `make setup`):

```bash
rustup target add wasm32-wasip2
```

## Сборка

Из **корня** репозитория:

```bash
make build
```

`make build` сам вызывает `make setup` (добавляет target, если его ещё нет).

Артефакт: `extension/target/wasm32-wasip2/release/zed_claude_proxy.wasm`.

Эквивалент вручную:

```bash
cd extension && cargo build --release --target wasm32-wasip2
```

## Установка в Zed (dev extension)

**Важно:** `make build` в терминале **не заменяет** установку в Zed. При **Install Dev Extension** Zed **сам** запускает `cargo build --target wasm32-wasip2` (в **debug**, без `--release`). Нужны `rustup`, `cargo` и target в **PATH процесса Zed**.

### Перед установкой

```bash
make check-zed-env    # cargo, rustup, wasm32-wasip2
make build-dev        # опционально: та же сборка, что сделает Zed
```

### Шаги в Zed

1. Command Palette → `zed: extensions`.
2. **Install Dev Extension**.
3. Выберите каталог **`extension/`** (внутри должны быть `extension.toml` и `Cargo.toml`), **не** корень репозитория.

### Если ошибка «failed to compile Rust extension»

| Причина | Что сделать |
|---------|-------------|
| Zed запущен из Dock / Spotlight | В PATH нет `~/.cargo/bin`. **Закройте Zed**, в терминале: `source "$HOME/.cargo/env"` → `/Applications/Zed.app/Contents/MacOS/cli --foreground` (или `zed --foreground` после `cli: install cli binary`) → Install Dev Extension |
| Rust через Homebrew | Только [rustup](https://rustup.rs/): `brew uninstall rust` при необходимости |
| Нет `wasm32-wasip2` | `rustup target add wasm32-wasip2` |
| Таймаут crates.io при сборке в Zed | Сначала `make fetch` при нормальной сети, затем Install Dev Extension снова |
| Не та папка | Только `.../zed-claude-proxy/extension` |

Подробный stderr сборки:

1. Command Palette → **`zed: open log`**
2. Или запуск **`zed --foreground`** из терминала, где `which cargo` показывает `~/.cargo/bin/cargo`

Чтобы GUI-Zed всегда видел Rust, добавьте в **`~/.zprofile`** (не только `~/.zshrc`):

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Перелогиньтесь или перезапустите Mac, затем снова откройте Zed.

### PATH для macOS (кратко)

```bash
# В терминале, где всё работает:
source "$HOME/.cargo/env"
which cargo rustup
make build-dev

# Запуск Zed с тем же PATH (см. ниже, если `zed: command not found`)
/Applications/Zed.app/Contents/MacOS/cli --foreground
```

### Команда `zed` не найдена в терминале

CLI ставится отдельно от приложения:

1. Откройте **Zed** (из Applications).
2. Command Palette (`Cmd+Shift+P`) → **`cli: install cli binary`**.
3. Новое окно терминала → проверка: `which zed` (обычно `/usr/local/bin/zed`).

Без установки CLI можно вызывать бинарник напрямую:

```bash
/Applications/Zed.app/Contents/MacOS/cli --foreground
/Applications/Zed.app/Contents/MacOS/cli .
```

Если Zed в `~/Applications`:

```bash
~/Applications/Zed.app/Contents/MacOS/cli --foreground
```

Без прав на `/usr/local/bin` — symlink в домашний каталог:

```bash
mkdir -p ~/.local/bin
ln -sf /Applications/Zed.app/Contents/MacOS/cli ~/.local/bin/zed
# в ~/.zprofile:
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
```

Логи без терминала: в Zed → **`zed: open log`** (Command Palette).

Если опубликованная версия с тем же id уже установлена, она будет заменена; в списке появится пометка «Overridden by dev extension».

## Проверка

- В списке extensions должно быть **Zed Claude Proxy** (`zed-claude-proxy`).
- При ошибках: Command Palette → `zed: open log`.
- Подробные логи: запустите Zed из терминала: `zed --foreground`.

## Отладка Rust-кода

`println!` / `dbg!` из WASM попадают в stdout процесса Zed — видны при запуске с `--foreground`.

## Дальнейшие фазы

См. [mvp-roadmap.md](mvp-roadmap.md): settings, Docker-шаблоны и команды `Claude: Start Container` / `Claude: Open in Terminal` — в следующих фазах.
