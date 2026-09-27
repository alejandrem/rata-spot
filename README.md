<div align="center">

# 🐀 rata-spot

### Spotify liviano: Brave minimizado + TUI en tu terminal

<img src="https://img.shields.io/badge/rust-1.98+-orange?style=for-the-badge&logo=rust" alt="Rust">
<img src="https://img.shields.io/badge/windows-10%2F11-blue?style=for-the-badge&logo=windows" alt="Windows">
<img src="https://img.shields.io/badge/ratatui-0.27-green?style=for-the-badge" alt="Ratatui">
<img src="https://img.shields.io/badge/spotify_sin_api-sin_API-1DB954?style=for-the-badge" alt="Sin API">
<img src="https://img.shields.io/badge/license-MIT-purple?style=for-the-badge" alt="MIT">

*Sin API de Spotify. Sin Electron. Sin 300MB de RAM. Solo tu música.* owo

</div>

---

## ✨ ¿Qué es esto?

**rata-spot** controla tu Spotify desde la terminal. Abre Brave en segundo
plano (con tu sesión ya logueada, bloqueando anuncios con Shields), lee lo
que suena por **GSMTC** (la API nativa de Windows) y maneja la página por
**CDP** (clicks y lecturas del DOM real, cero APIs).

```
┌ 📚 biblioteca (58) ─────┬ 🐀 rata-spot ─────────────────────────┐
│ 🎵 Pa' Mi San Juditas   │ ⏸ MONEY EDITION — Eden Muñoz         │
│ 🎵 😾                   ├ progreso ────────────────────────────┤
│ 🎤 Beethoven            │ ████████>············· 1:24 / 3:45    │
│ 💿 Ramdom               │ Conectando con Spotify...            │
│                         │ [spc] play [n] sig ...  [q] salir    │
└─────────────────────────┴──────────────────────────────────────┘
```

## 🚀 Uso rápido

> **Requisitos:** Windows 10/11 · [Brave](https://brave.com) instalado ·
> [Rust](https://rustup.rs) toolchain `stable-x86_64-pc-windows-msvc` ·
> cuenta de Spotify (login único en el perfil rata-spot la primera vez;
> tu Brave personal ni se toca).

```powershell
cargo run
```

| Tecla | Hace |
|---|---|
| `Espacio` | play / pausa (prende la música sola si no suena nada) |
| `n` / `p` | siguiente / anterior |
| `j` `k` `↑` `↓` | moverte por listas (con delay anti-dedo-pegado) |
| `→` / `Enter` | abrir playlist · tocar rola · ver dashboard |
| `←` / `Esc` | volver |
| `/` | enfocarla barra de búsqueda (siempre visible) |
| `q` | pausar + cerrar lo nuestro + bye 🐀 |

## 🔍 Buscador + dashboard

La barra vive **siempre** en el centro (`🔍 presiona / para buscar`):
escribes, `Enter`, y salen rolas/artistas/playlists del DOM de verdad.
`Enter` en un resultado lo toca **por título** (nada de lotería) y el centro
cambia al **dashboard individual**: artista, álbum, año, duración,
reproducciones y **letra con scroll** (`j/k`).

## 🧠 Cómo funciona (sin magia)

<div align="center">

| Capa | Qué hace |
|---|---|
| `launcher/` | Brave con flags de dieta + ventana nueva + cierre quirúrgico por HWND |
| `gsmtc/` | Lee/controla lo que suena (título, progreso, play/pause/next/prev) |
| `cdp/` | Habla con la página: clicks confiables, biblioteca, tracklists, search |
| `ui/` + `app/` | TUI Ratatui: reproductor, sidebar, buscador, dashboard |

</div>

<details>
<summary><b>🧪 Tests (click para ver)</b></summary>

```powershell
cargo test                          # silenciosos y deterministas
cargo test -- --include-ignored     # vivos (suenan de verdad, manos fuera 1 min)
```

Incluyen pruebas vivas: primer play, playlists, tracklist, búsqueda,
no-corte de audio y dashboard — más diagnósticos (`diag_*`) que imprimen
el estado real del DOM y GSMTC.

</details>

<details>
<summary><b>⚙️ Variables de entorno</b></summary>

| Variable | Efecto |
|---|---|
| `RATA_SPOT_HIDDEN=1` | Brave oculto fuera de pantalla (debug: visible) |
| `RATA_SPOT_NOGPU=0` | Quita `--disable-gpu` (va por defecto; solo si Widevine corta el audio) |
| `RATA_SPOT_API=0` | Buscador solo por DOM (apaga el intento JSON/red) |
| `RATA_SPOT_PORT=9333` | Fija el puerto CDP (default: efímero + fallback 9222) |

</details>

## 📉 RAM (objetivo Fase 5)

| Componente | RAM |
|---|---|
| `rata-spot.exe` (release, 0.72 MB) | ~17 MB |
| Brave con flags | ~120 MB *(medir en Task Manager → Details → Private Bytes)* |
| **Total** | **~137 MB** vs 200-300 MB de la app nativa |

## 🗺️ Mapa del repo

```
src/
  main.rs        # punto de entrada delgado
  app/           # boot, loop, teclas, sync, biblioteca en fondo, terminal
  cdp/           # transport, client, tabs, selectors, api, playback, library, search, tracks
  gsmtc/         # session, track, smoother (free-run), history
  launcher/      # config (flags), process, window (HWND)
  ui/            # player, sidebar, search, tracks, detail, state
docs/
  bugs-resueltos-uwu.md   # bitácora de bugs con ejemplos para bebé 🍼
```

## 📜 Licencia

MIT — haz lo que quieras, pero si suena chido acuérdate de la rata 🐀.
Ver [LICENSE](LICENSE).

---

<div align="center">

Hecho con Rust, Brave y mucho perreo intenso 🐀🔥

</div>
