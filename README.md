# Discord Quest Completer (Linux)

**Автор:** [kavtunov](https://github.com/kavtunov)

Приложение для Linux, которое помогает выполнять **Discord Quests** без установки полных игр.  
Создаёт маленький dummy-процесс с нужным именем exe — Discord Desktop детектит его как игру и крутит таймер квеста.

> Нужен **Discord Desktop** (не браузерная версия).

---

## Возможности

- Поиск игр из списка Discord Detectable
- Запуск dummy-игры по кнопке **Play**
- Поддержка Linux (нативный runner на `winit`)
- Fallback на `win32`-имена exe, если у игры нет linux-entry (Discord на Linux часто ищет именно их)
- Сборка в `.deb` / `.rpm`

---

## Как это работает

1. При **Play** приложение копирует шаблон `src-linux` в `games/<id>/…` под именем detectable-файла (например `endfield.exe`).
2. Запускает его — в системе появляется процесс с этим именем.
3. Discord видит процесс → квест идёт.

**Stop** / закрытие dummy-окна / `pkill -x <имя.exe>` — останавливает «игру».

RPC / Test Rich Presence **не** засчитывает квесты — только **Play**.

---

## Быстрый старт

### Зависимости

- [Rust](https://rustup.rs)
- Node.js 20+ и [pnpm](https://pnpm.io)
- Discord Desktop

**Fedora / Nobara:**

```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel librsvg2-devel openssl-devel gcc
```

**Debian / Ubuntu:**

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev patchelf
```

### Сборка и запуск

```bash
pnpm install
pnpm linux:build          # runner + production build
./run-linux.sh            # запуск portable-сборки
```

Для разработки:

```bash
pnpm linux:dev
```

Портативная папка после билда: `dist-linux/`  
Пакеты: `src-tauri/target/release/bundle/{deb,rpm}/`

---

## Использование

1. Открой Discord Desktop и прими квест.
2. Запусти это приложение.
3. Вкладка **Home** → найди игру квеста в поиске → добавь.
4. Нажми **Play** и жди таймер Discord (~15 мин для gameplay-квестов).
5. Забери награду в Discord → **Stop**.

Каталог приложения должен быть **доступен на запись** (создаётся `games/`).

---

## Структура (portable)

```text
dist-linux/
├── discord-quest-completer   # главное приложение
├── data/
│   └── src-linux             # шаблон dummy-runner
└── games/
    └── <quest-app-id>/
        └── <game.exe>        # создаётся при Play
```

---

## Стек

- Rust + Tauri 2
- Vue 3 + Vite
- Dummy runner: `src-linux` (winit)

---

## Лицензия

См. [LICENSE](./LICENSE).

---

## Дисклеймер

Инструмент для личного использования. Учитывай Terms of Service Discord и издателей игр.  
Автор не несёт ответственности за блокировки или другие последствия. Проект не связан с Discord.
