# MVP Roadmap: Zed Claude Proxy

Документ описывает план создания минимально жизнеспособной версии: **плагин для Zed** + **Docker-стек** (прокси + Claude Code), без ручной настройки в репозитории пользователя.

## Цель MVP

Пользователь открывает workspace в Zed, настраивает прокси **только в настройках расширения**, затем:

1. **Поднимает контейнер** одной командой (или убеждается, что стек уже работает для этого проекта).
2. **Открывает Claude Code в терминале** Zed — интерактивная сессия внутри контейнера с уже настроенным `HTTP(S)_PROXY` и установленным `claude-code`.

Успех MVP: два сценария подряд на чистой машине с Docker — «первый запуск» и «повторный запуск того же проекта» без дубликатов контейнеров и без правок файлов в git-репозитории проекта.

---

## Scope

### В scope (MVP)

| Область | Содержание |
|--------|------------|
| Расширение Zed | `extension.toml`, Rust → WASM (`zed_extension_api`), 2 команды |
| Настройки | Прокси (`CLAUDE_PROXY`), пути/шаблоны — **только через settings расширения** |
| Оркестрация | Генерация `docker-compose.yml` с подставленными значениями из конфига в `state_dir`, `docker compose up` (без отдельного `.env`) |
| Изоляция | Один workspace = один compose-проект (`zed-claude-<project_id>`) |
| Docker | gost (relay) + сервис с Node + `@anthropic-ai/claude-code` |
| Терминал | `docker compose exec` → `claude` (или shell с предзагруженным env) |

---

## Архитектура (MVP)

```mermaid
flowchart TB
  subgraph Zed["Zed IDE"]
    Ext["Extension WASM"]
    Settings["Extension Settings\n(claude_proxy, ...)"]
    Cmd1["Claude: Start Container"]
    Cmd2["Claude: Open in Terminal"]
    Term["Integrated Terminal"]
  end

  subgraph Host["Host (~/.local/share/zed-claude-proxy/...)"]
    Gen["Generated:\ncompose.yml (rendered),\nDockerfile context"]
  end

  subgraph Docker["Docker Compose (per project_id)"]
    Gost["claude-gost\nSOCKS/HTTP relay"]
    App["claude-app\nNode + claude-code\n/project → /app"]
  end

  Settings --> Ext
  Cmd1 --> Ext
  Cmd2 --> Ext
  Ext --> Gen
  Ext -->|"docker compose -p zed-claude-{id}"| Docker
  Cmd2 --> Term
  Term -->|"exec -it"| App
  Gost --> App
  Ext -.->|"CLAUDE_PROXY"| Gost
```

### Идентификация проекта

```
project_root    = корень workspace в Zed
project_id      = стабильный slug (например, sha256(project_root)[:8])
compose_project   = "zed-claude-" + project_id
state_dir       = ~/.local/share/zed/extensions/work/zed-claude-proxy/projects/{project_id}/
```

Все артефакты (`docker-compose.yml`, `Dockerfile`, опционально `claude_config/`) — только в `state_dir`, не в репозитории пользователя. Отдельный `.env` не используем: settings и runtime-поля (путь workspace, uid/gid) вшиваются в compose при рендере шаблона.

---

## Команды плагина (MVP)

### 1. `Claude: Start Container`

**Поведение:**

1. Прочитать `workspace_root` и настройки расширения.
2. Вычислить `project_id`, подготовить `state_dir`.
3. Срендерить `docker-compose.yml` из шаблона (подстановка settings + `workspace_root`, uid/gid) и записать в `state_dir` вместе с `Dockerfile`.
4. Выполнить `docker compose -f ... -p <compose_project> up -d --build` (или `up -d`, если образ уже собран).
5. Дождаться healthy у gost (или таймаут с понятной ошибкой).
6. Показать notification: успех / ошибка с stderr.

**Идемпотентность:** повторный вызов не создаёт второй стек для того же `project_id`; при уже running — «already running» и exit 0.

### 2. `Claude: Open in Terminal`

**Поведение:**

1. Убедиться, что контейнер приложения запущен (`docker compose ps` / inspect).
2. Если не запущен — предложить запустить первую команду (notification + optional auto-start — **опционально для MVP**, достаточно явного сообщения).
3. Открыть терминал Zed с командой вида:

   ```bash
   docker compose -f "<state_dir>/docker-compose.yml" -p "zed-claude-<id>" exec -it claude-app claude
   ```

   Альтернатива для отладки: `exec -it claude-app bash`, затем пользователь вводит `claude` вручную — для MVP предпочтительнее сразу `claude`.

**Требование:** рабочая директория терминала на хосте — `project_root` (удобство для путей; exec всё равно внутри контейнера в `/app`).

---

## Настройки расширения (единственный источник конфигурации)

Параметры из `docker/.env.example` — в **extension settings** (Zed `settings.json` → секция расширения). На диск пишется один файл — **`docker-compose.yml`** с уже подставленными значениями (отдельный `.env` не нужен: меньше артефактов, compose самодостаточен для `docker compose -f ...`).

| Ключ | Обязательный | Описание | Пример |
|------|--------------|----------|--------|
| `claude_proxy` | да | Строка для gost `-F=` (relay/http/socks5/ss) | `relay+wss://user:pass@host:port` |
| `claude_service_name` | нет | Имя сервиса в compose (default: `claude-app`) | `claude-app` |
| `gost_image` | нет | Образ gost | `ginuerzh/gost:2.12.0` |
| `node_image` | нет | Базовый образ Node | `node:22-slim` |

**Runtime** (не в settings, подставляются при каждом рендере):

| Поле | Источник |
|------|----------|
| `project_dir` | `workspace_root` |
| `user_id` / `group_id` | uid/gid процесса пользователя на хосте |

Рендер: шаблон `docker-compose.yml.tpl` + контекст → валидный YAML. Строки с `@`, `:`, `+` в `claude_proxy` экранировать для YAML (кавычки / block scalar).

Документация для пользователя: «настроил прокси один раз в Zed → команды работают».

---

## Адаптация Docker-шаблонов

Текущие файлы в `docker/` — **референс**. Для MVP упростить и привязать к модели «один проект — один compose-project».

### Что убрать / упростить

| В примере | Решение для MVP |
|-----------|-----------------|
| `env_file: .env` | Убрать — значения из settings/runtime вшиваются в сгенерированный compose |
| Фиксированные `container_name` | Убрать — мешают параллельным проектам |
| `venv-data`, `uv` в Dockerfile | Убрать, если Claude Code не требует Python venv |
| `./claude_config/`, `./.claude.json` bind | Отложить или положить в `state_dir` при необходимости auth |
| `restart: unless-stopped` | Оставить для gost/app или `no` для app — зафиксировать в roadmap-итерации |
| `mirror.gcr.io` | Сделать настраиваемым prefix или использовать публичные образы по умолчанию |
| Имя `claude-hypognn` | Переименовать в нейтральное `claude-app` |

### Целевой compose (после рендера, без `${...}` из .env)

```yaml
services:
  claude-gost:
    image: ginuerzh/gost:2.12.0
    command:
      - "-L=auto://0.0.0.0:1080"
      - "-F=relay+wss://user:pass@host:port"   # claude_proxy из settings
    expose: ["1080"]
    healthcheck: ...

  claude-app:
    build:
      context: /path/to/state_dir
      dockerfile: Dockerfile
    stdin_open: true
    tty: true
    user: "501:20"                            # user_id:group_id с хоста
    environment:
      HTTP_PROXY: http://claude-gost:1080
      HTTPS_PROXY: http://claude-gost:1080
      ALL_PROXY: http://claude-gost:1080
      NO_PROXY: localhost,127.0.0.1
      HOME: /var/home
    volumes:
      - /Users/me/my-project:/app              # project_dir = workspace_root
    depends_on:
      claude-gost:
        condition: service_healthy
```

Шаблон в репозитории — с плейсхолдерами (`{{ claude_proxy }}`, `{{ project_dir }}`, …); в `state_dir` попадает уже готовый файл.

### Целевой Dockerfile (MVP)

- База: Node slim
- Пакеты: `git`, `curl`, `ca-certificates` (минимум для claude-code)
- `npm install -g @anthropic-ai/claude-code`
- `WORKDIR /app`
- `CMD ["tail", "-f", "/dev/null"]` — контейнер живёт, сессия через `exec`

Шаблоны хранить в репозитории расширения (`extension/templates/`), копировать в `state_dir` при первом запуске.

---

## Фазы разработки

### Фаза 0 — Каркас репозитория (0.5–1 дн.)

- [x] Структура crate: `extension/` (Rust + `extension.toml`)
- [x] CI: `cargo build --target wasm32-wasip2`
- [x] `docs/` + обновить корневой `README.md` (ссылка на roadmap, quick start)
- [x] Зафиксировать минимальную версию Zed / `zed_extension_api`
- [x] Инструкция как собирать и устанавливать расширение

**Результат:** пустое расширение ставится в Zed, видно в списке extensions.

---

### Фаза 1 — Settings и генерация артефактов (1–2 дн.)

- [ ] Описать settings в `extension.toml` / schema
- [ ] Модуль `config`: чтение settings + `workspace_root` → `ProjectContext`
- [ ] Модуль `templates`: embed `Dockerfile`, `docker-compose.yml.tpl`; рендер в итоговый `docker-compose.yml`
- [ ] Модуль `state`: `ensure_state_dir`, запись `docker-compose.yml` + `Dockerfile`, перегенерация при смене settings/workspace
- [ ] Инструкция как проверить что все что нужно собирается

**Результат:** по команде (или тесту) в `state_dir` появляется валидный `docker-compose.yml` с литеральными значениями (без `.env`).

---

### Фаза 2 — Docker без Zed UI

- [ ] Упростить шаблоны по таблице выше
- [ ] Ручная проверка: `docker compose -p test up -d` из сгенерированного `state_dir`
- [ ] Проверка: `exec` → `claude --version` через прокси
- [ ] Документировать системные зависимости: Docker, Compose v2

**Результат:** стек поднимается вручную по сгенерированным файлам; Claude Code отвечает внутри контейнера.

---

### Фаза 3 — Команда «Start Container»

- [ ] Вызов `docker compose` из WASM (`zed_extension_api::process` или аналог)
- [ ] `-p zed-claude-{project_id}`, `-f {state_dir}/docker-compose.yml`
- [ ] Парсинг exit code; notifications при ошибках
- [ ] Идемпотентность: detect running stack
- [ ] (Опционально) progress: «building…», «waiting for healthy…»

**Результат:** команда 1 в Zed поднимает стек для текущего workspace.

---

### Фаза 4 — Команда «Open in Terminal»

- [ ] Проверка, что сервис `claude-app` running
- [ ] `workspace.open_terminal` (или API Zed для терминала) с compose exec
- [ ] Сообщение, если контейнер не запущен

**Результат:** команда 2 открывает терминал с `claude` внутри контейнера.

---

### Фаза 5 — Полировка MVP (1–2 дн.)

- [ ] README: установка extension, первый запуск, пример `claude_proxy`
- [ ] Обработка типичных ошибок: Docker не запущен, нет `claude_proxy`, нет прав на socket
- [ ] Тег/version `0.1.0-mvp`

**Результат:** можно отдать тестировщику без устных инструкций.

---

## Критерии приёмки MVP

1. **Настройки:** прокси задаётся только в settings расширения; в репозитории проекта нет сгенерированного `docker-compose.yml` от нас (только в `state_dir` на хосте).
2. **Два проекта:** два workspace → два `compose_project`, оба могут быть `up` одновременно.
3. **Команда 1:** поднимает gost + app, повторный вызов не плодит контейнеры.
4. **Команда 2:** открывает терминал; `claude` стартует в `/app` (= корень workspace).
5. **Прокси:** запросы Claude Code идут через gost (проверка: лог gost / отключение прокси → ожидаемая ошибка региона).
6. **Ошибки:** понятное сообщение, если Docker недоступен или `claude_proxy` пустой.


## Структура репозитория (целевая)

```text
zed-claude-proxy/
├── extension/
│   ├── Cargo.toml
│   ├── extension.toml
│   ├── src/
│   │   ├── lib.rs              # register commands
│   │   ├── config.rs
│   │   ├── project_id.rs
│   │   ├── state.rs
│   │   ├── docker.rs           # compose invocations
│   │   └── commands/
│   │       ├── start_container.rs
│   │       └── open_terminal.rs
│   └── templates/
│       ├── Dockerfile
│       └── docker-compose.yml.tpl
├── docker/                     # референс (можно синхронизировать с templates)
├── docs/
│   └── mvp-roadmap.md          # этот файл
└── README.md
```

---

## Риски и митигация

| Риск | Митигация |
|------|-----------|
| Ограничения WASM (нет произвольного FS на хосте) | Все файлы только в разрешённых путях; вызов `docker` через API процессов Zed |
| Auth Claude (`~/.claude`) | MVP: volume из `state_dir/claude_config`; позже — настройка пути |
| Долгий `docker build` | Кэш образа; notification «first run may take minutes» |
| UID/GID на macOS Docker | `user: "uid:gid"` в сгенерированном compose; при проблемах — doc troubleshooting |
| Спецсимволы в `claude_proxy` | Корректное YAML-экранирование при рендере шаблона |
| Имена сервисов в примере vs код | Единый default `claude-app` в шаблоне и settings |

---

## После MVP (краткий backlog)

1. Объединённая команда «Ensure + Open» (как в раннем README) — опционально третья команда.
2. `Claude: Stop Container` / `Claude: Logs`.
3. Синхронизация credentials с хостом.
4. Публикация в Zed Extension Registry.
5. Поддержка Linux без Docker Desktop.


## Связанные документы

- [README.md](../README.md) — обзор продукта и стека
- [docker/](../docker/) — референсные шаблоны до переноса в `extension/templates/`
