# Linux setup (Discord Quest Completer)

Адаптация под Linux: нативный Tauri-клиент + маленький dummy-runner (`src-linux`, winit), который Discord видит по **имени процесса**.

## Зависимости

1. **Rust** — https://rustup.rs  
2. **Node.js 20+** и **pnpm**  
3. **Системные пакеты Tauri** (Fedora / Nobara):

```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel librsvg2-devel openssl-devel gcc
```

Debian / Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev patchelf
```

Также нужен установленный **Discord Desktop** (не только браузер).

## Запуск (готовый билд)

После сборки portable-папка уже есть:

```bash
./run-linux.sh
# или:
./dist-linux/discord-quest-completer
```

Пакеты:
- `src-tauri/target/release/bundle/rpm/Discord Quest Completer-25.4.18-1.x86_64.rpm`
- `src-tauri/target/release/bundle/deb/Discord Quest Completer_25.4.18_amd64.deb`

## Сборка с нуля

```bash
pnpm install
pnpm linux:dev      # соберёт runner и запустит tauri dev
# или релиз:
pnpm linux:build
```

Эквивалент вручную:

```bash
pnpm sync:runner   # build:runner:linux + copy в src-tauri/resources
pnpm tauri dev
```

## Что проверено на этой машине

- Системные deps Tauri установлены
- Release-бинарник собирается; окно **Discord Quest Completer** открывается
- Dummy `overwatch.exe` появляется в процессе как `overwatch.exe` и убивается через `pkill -x`
- AppImage-бандл отключён (linuxdeploy на Nobara падает); используются `deb` + `rpm`

## Как это работает на Linux

1. Приложение копирует шаблон `src-linux` в `games/<app-id>/…` под именем detectable-исполняемого файла (часто `Something.exe`).
2. Запускает этот файл — в списке процессов появляется нужное имя.
3. Discord Desktop матчит имя с `/applications/detectable` и крутит квест.

Если для игры нет `linux`-entry, UI подставляет `win32`-имя: Discord на Linux всё равно часто ищет именно его.

## Важно

- Каталог приложения должен быть **доступен на запись** (создаётся `games/`).
- Пока идёт таймер квеста, dummy-окно и Discord должны оставаться запущенными.
- RPC / Rich Presence — отдельно и **не** засчитывает quest gameplay.
