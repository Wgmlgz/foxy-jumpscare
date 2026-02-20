# Foxy Jumpscare

![foxy](./public/media/foxy.gif)

A tiny Tauri desktop app with a vanilla frontend that plays a fullscreen jump-scare GIF with synchronized audio. In default `once` mode it exits after playback; `random` and `daemon` modes are also supported. The window is transparent, always-on-top, and ignores mouse input.

Tested on both macos and windows.

## Runtime modes (CLI)

Default mode is `once` (single jumpscare, then app exits).

- Delay before jumpscare:

```bash
./app --delay-ms 8000
```

- Random mode:

```bash
./app --random --random-min-ms 10000 --random-max-ms 45000 --random-count 5
```

- Daemon mode (HTTP trigger):

```bash
./app --daemon --daemon-bind 127.0.0.1:15151
```

Trigger in daemon mode:

```bash
curl http://127.0.0.1:15151/foxy
```

Daemon health check:

```bash
curl http://127.0.0.1:15151/health
```

## One-liner run

These commands download the latest portable binary from the latest published release and run it directly (no installer).
Set `FOXY_REPO=owner/repo` first if you want to target a fork.

macOS / Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/Wgmlgz/foxy-jumpscare/main/scripts/foxy.sh | bash
```

or if you lucky

```bash
curl -fsSL https://amogos.pro/foxy.sh | bash
```

Windows (PowerShell):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/Wgmlgz/foxy-jumpscare/main/scripts/foxy.ps1 | iex"
```

or if you lucky

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://amogos.pro/foxy.ps1 | iex"
```
