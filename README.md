# Zed Claude Proxy

Расширение для **Zed**, которое автоматизирует запуск **Claude Code** в Docker-контейнере с VPN/прокси для регионов, где API недоступен напрямую.

Пользователь открывает проект в Zed и одной командой получает интерактивный shell внутри контейнера, где уже настроены прокси и установлен `claude-code`. Расширение само проверяет, поднят ли стек для **текущего** workspace, при необходимости создаёт и запускает его, без ручного редактирования `docker-compose.yml` в репозитории проекта.

## Стек

| Слой | Технология |
|------|------------|
| IDE | [Zed](https://zed.dev) |
| Расширение | Zed Extension: `extension.toml` + Rust → **WebAssembly** (`zed_extension_api`) |
| Оркестрация | **Docker Compose** v2 (`docker compose`) |
| Прокси | [gost](https://github.com/ginuerzh/gost) (`ginuerzh/gost`) — SOCKS/HTTP relay к внешнему VPN |
| Runtime Claude | Node.js slim + `@anthropic-ai/claude-code` (образ по аналогии с текущим `Dockerfile`) |
| Данные на хосте | Временные `.env` и compose-файлы в каталоге расширения (`~/.local/share/...`) |

**Системные зависимости на машине пользователя:** Docker Desktop / Docker Engine, `docker compose`, доступ к Docker socket.

## Подход

### Принципы

1. **Изоляция по проекту** — один workspace = один compose-проект. Несколько проектов могут работать параллельно.
2. **Не трогаем репозиторий пользователя** — все нужные файлы генерируются расширениям и хранятся в служебной папке
3. **Идемпотентность** — повторный вызов команды не поднимает дубликаты, а подключается к уже работающему стеку.

### Идентификация проекта

```
project_root    = корень workspace в Zed
project_id      = стабильный slug/hash от project_root (например, sha256[:8])
compose_project = "zed-claude-" + project_id
```

### Интеграция с Zed

- **Команды:** `Claude: Ensure Container`, `Claude: Open Shell` (MVP — одна объединённая команда).
- **Реализация:** Rust extension → `process` / shell-команды; вывод и ошибки — в notification или panel.
- **Настройки** (`settings.json` / extension settings): `claude_proxy`, пути к шаблонам, имя сервиса (`claude-hypognn`).
