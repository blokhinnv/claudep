# Zed Claude Proxy

Расширение для **Zed**, которое автоматизирует запуск **Claude Code** в Docker-контейнере с VPN/прокси для регионов, где API недоступен напрямую.

Пользователь открывает проект в Zed и одной командой получает интерактивный shell внутри контейнера, где уже настроены прокси и установлен `claude-code`. Расширение само проверяет, поднят ли стек для **текущего** workspace, при необходимости создаёт и запускает его, без ручного редактирования `docker-compose.yml` в репозитории проекта.

**Статус:** фаза 0 — каркас расширения (сборка WASM, dev install). Команды Docker — в разработке (см. [roadmap](docs/mvp-roadmap.md)).

## Документация

- [MVP Roadmap](docs/mvp-roadmap.md) — план фаз и критерии приёмки
- [Разработка](docs/development.md) — сборка, установка dev extension, совместимость

## Quick start (фаза 0)

**macOS:** пошаговая установка (Xcode CLT, rustup, Zed, Docker позже) — [docs/development.md#установка-на-macos](docs/development.md#установка-на-macos).

1. Установите [Rust через rustup](https://rustup.rs/).

2. Соберите расширение из корня репозитория:

   ```bash
   make build
   ```

   (установит WASM target и соберёт release; см. [`Makefile`](Makefile))

3. В Zed: `zed: extensions` → **Install Dev Extension** → выберите каталог `extension/` этого репозитория.

Подробнее: [docs/development.md](docs/development.md).

## Стек

| Слой | Технология |
|------|------------|
| IDE | [Zed](https://zed.dev) **1.4.x** (протестировано с 1.4.2; также 0.205+) |
| Расширение | `extension.toml` + Rust → **WebAssembly** (`wasm32-wasip2`, `zed_extension_api` **0.7.0**) |
| Оркестрация | **Docker Compose** v2 (`docker compose`) |
| Прокси | [gost](https://github.com/ginuerzh/gost) — SOCKS/HTTP relay |
| Runtime Claude | Node.js slim + `@anthropic-ai/claude-code` |
| Данные на хосте | Сгенерированные compose/Dockerfile в `state_dir` расширения |

**Системные зависимости (для полного MVP):** Docker Engine / Docker Desktop, `docker compose`, доступ к Docker socket.

## Планируемые команды (MVP)

- **Claude: Start Container** — поднять gost + app для текущего workspace
- **Claude: Open in Terminal** — `docker compose exec` → `claude` в контейнере

## Подход

### Принципы

1. **Изоляция по проекту** — один workspace = один compose-проект. Несколько проектов могут работать параллельно.
2. **Не трогаем репозиторий пользователя** — артефакты только в служебной папке на хосте.
3. **Идемпотентность** — повторный вызов не поднимает дубликаты.

### Идентификация проекта

```
project_root    = корень workspace в Zed
project_id      = стабильный slug/hash от project_root (например, sha256[:8])
compose_project = "zed-claude-" + project_id
```

### Настройки

Прокси и образы задаются **только в settings расширения** (`claude_proxy` и др.) — см. [roadmap](docs/mvp-roadmap.md).

## Референс Docker

Каталог [`docker/`](docker/) — пример compose/Dockerfile до переноса в `extension/templates/` (фаза 1+).

## Лицензия

MIT — см. [LICENSE](LICENSE).
