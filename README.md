# claudep

**claudep** (*claude* + *p*roxy) — CLI для [Claude Code](https://docs.anthropic.com/en/docs/claude-code) в изолированном Docker-стеке с relay (gost) до вашего upstream-прокси, когда API недоступен напрямую из региона.

Обычная команда **`claude`** запускает CLI на хосте. **`claudep`** поднимает per-project стек (gost + app), монтирует текущий каталог в контейнер и направляет трафик через relay — без `docker-compose.yml` в git.

Работает из **корня любого проекта** в терминале.

> Неофициальный инструмент, не связан с Anthropic.

---

## Зачем

| Проблема | Решение claudep |
|----------|-----------------|
| `claude` на хосте не ходит в API без VPN/прокси | Relay в контейнере **gost** → ваш `CLAUDEP_UPSTREAM` |
| Не хочется класть `docker-compose.yml` в репозиторий | Артефакты только в `~/.local/share/claudep/state/` |
| Несколько проектов параллельно | Один `project_id` на каталог → отдельный compose-проект |
| Повторный заход в тот же проект | Идемпотентный `claudep` + быстрый `claudep attach` |

---

## Быстрый старт

### Установка (один раз)

```bash
curl -fsSL https://raw.githubusercontent.com/blokhinnv/claudep/main/install.sh | sh
```

Установщик:

1. Кладёт бинарь **`claudep`** в `~/.local/bin` (или путь из `$CLAUDEP_INSTALL_DIR`).
2. Скачивает **шаблоны Docker** в `~/.local/share/claudep/templates/` (Dockerfile).
3. Дописывает в shell-профиль (`~/.zshrc`, `~/.bashrc`) — спрашивает `CLAUDEP_UPSTREAM` с дефолтом `socks5://127.0.0.1:1080`:

   ```bash
   export CLAUDEP_HOME="${CLAUDEP_HOME:-$HOME/.local/share/claudep}"
   export CLAUDEP_TEMPLATES="$CLAUDEP_HOME/templates"
   export CLAUDEP_UPSTREAM="${CLAUDEP_UPSTREAM:-socks5://127.0.0.1:1080}"
   ```

4. Проверяет наличие `docker` и `docker compose` (предупреждение, если нет).

Перезагрузите shell или `source ~/.zshrc`.

### Установка из исходников (dev)

```bash
git clone https://github.com/blokhinnv/claudep.git
cd claudep
make install-local
export CLAUDEP_UPSTREAM=socks5://127.0.0.1:1080
claudep sync   # положит Dockerfile в ~/.local/share/claudep/templates/
```

### В каталоге проекта

```bash
cd ~/Developer/my-app

# Поднять стек для cwd, если ещё не запущен (gost + app)
claudep

# Интерактивный claude (или shell с --shell) внутри контейнера
claudep attach
```

Первый **`claudep`** для этой папки: генерирует compose в state, собирает образ, стартует контейнеры.  
Повторный **`claudep`** для той же папки: `compose up -d` без дубликатов; сообщение «stack already running».

---

## Команды

| Команда | Назначение |
|---------|------------|
| **`claudep`** | Убедиться, что стек для **текущего** `cwd` запущен (идемпотентно) |
| `claudep attach` | `docker compose exec -it app claude` |
| `claudep attach --shell` | `docker compose exec -it app bash` |
| `claudep down` | Остановить стек этого проекта |
| `claudep remove` | Остановить стек и удалить `state/` проекта |
| `claudep remove --image` | То же + удалить локально собранный образ app |
| `claudep status` | Контейнеры / путь к state / upstream |
| `claudep doctor` | Docker, переменные, шаблоны |
| `claudep sync` | Обновить шаблоны с GitHub release (fallback: embedded Dockerfile) |

Глобальный флаг: `--project-dir /path/to/project`.

---

## Как это устроено

### Принципы

1. **Изоляция по проекту** — несколько проектов = несколько compose.
2. **Репозиторий пользователя не меняется** — только `~/.local/share/claudep/state/<project_id>/`.
3. **Идемпотентность** — **`claudep`** без субкоманды безопасно вызывать многократно.

### Идентификация проекта

```
project_root    = абсолютный путь к cwd (или --project-dir)
project_id      = hex(sha256(project_root))[:12]
compose_project = "claudep-" + project_id
state_dir       = $CLAUDEP_HOME/state/<project_id>/
```

В `state_dir`: `docker-compose.yml`, `Dockerfile`, `.render-manifest.json` — **не** в git.

### Стек Docker (на проект)

| Сервис | Роль |
|--------|------|
| **gost** | Relay: `-F=$CLAUDEP_UPSTREAM`, локальный HTTP на `:1080` |
| **app** | Node slim + `@anthropic-ai/claude-code`, mount `project_root` → `/app` |

Переменные в app-контейнере: `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` → gost; `NO_PROXY=localhost,127.0.0.1`.

---

## Конфигурация

### Переменные окружения

| Переменная | По умолчанию | Описание |
|------------|--------------|----------|
| `CLAUDEP_HOME` | `~/.local/share/claudep` | Корень данных и state |
| `CLAUDEP_TEMPLATES` | `$CLAUDEP_HOME/templates` | Шаблон Dockerfile |
| `CLAUDEP_UPSTREAM` | *(нет)* | Upstream для gost (`socks5://…`, `http://…`, `relay+wss://…`) |
| `CLAUDEP_GOST_IMAGE` | `ginuerzh/gost:2.12.0` | Образ gost |
| `CLAUDEP_NODE_IMAGE` | `node:22-slim` | Базовый образ app-сервиса |
| `CLAUDEP_INSTALL_DIR` | `~/.local/bin` | Куда положить `claudep` при install |

---

## Разработка

```bash
make test              # unit tests
make build             # release binary
make render-fixture    # sample compose in /tmp/claudep-fixture-state/
make templates-tar     # dist/templates.tar.gz for releases
make check             # fmt + clippy + test
```

Ручная проверка с Docker:

```bash
export CLAUDEP_UPSTREAM=socks5://127.0.0.1:1080   # ваш upstream
cd /tmp && mkdir -p claudep-smoke && cd claudep-smoke
cargo run -- claudep
cargo run -- claudep          # повтор — «already running»
cargo run -- status
cargo run -- down
```

---

## Troubleshooting

| Симптом | Решение |
|---------|---------|
| `CLAUDEP_UPSTREAM is required` | `export CLAUDEP_UPSTREAM=...` в shell или в `~/.zshrc` |
| `Docker daemon is not available` | Запустите Docker Desktop / `docker info` |
| `docker compose v2 is required` | Обновите Docker; нужен plugin `compose`, не standalone `docker-compose` |
| Первый запуск долгий | `docker build` качает Node и ставит claude-code — это нормально |
| `templates Dockerfile: missing` | `claudep sync` или переустановите через `install.sh` |

---

## Требования

- **Docker** Engine или Docker Desktop
- **`docker compose`** v2
- Доступный **upstream** (`CLAUDEP_UPSTREAM`) с хоста, куда gost сможет подключиться
- macOS или Linux (Windows — через WSL2 + Docker внутри WSL)

---

## Лицензия

MIT — см. [LICENSE](LICENSE).
