# bugs-resueltos-uwu 🐀

Bitácora de cada bug que nos salió en rata-spot, cómo se resolvió,
explicado técnico y con ejemplo para bebé owo.

---

## 1. Se abrían 2 pestañas de Spotify en vez de 1

**Síntoma:** `cargo run` → ventana nueva de Brave con DOS pestañas `/intl-es/`.

**Causa técnica:** en `src/launcher.rs`, al construir el `Command`, quedó
`cmd.arg(SPOTIFY_URL)` escrito DOS veces (líneas 148-149). El proceso final
era `brave.exe ... --new-window <URL> <URL>` y Chromium abre una pestaña
por cada URL. Se confirmó con `Get-CimInstance Win32_Process`: el
`CommandLine` del proceso Brave traía la URL duplicada literal.

**Fix:** dejar UN solo `cmd.arg(SPOTIFY_URL)` + comentario
`// UNA sola URL: duplicarla abria 2 pestanas`.

**Para bebé:** es como gritarle dos veces "¡tráeme un pan!" al mandadero:
llega con dos panes. Le gritamos una sola vez y listo uwu.

---

## 2. `IAsyncOperation` no se puede `.await` (windows 0.58)

**Síntoma:** `cargo check` fallaba con 6 errores `is not a future` en
`src/gsmtc.rs` (`RequestAsync`, `TryGetMediaPropertiesAsync`, `TryPlayAsync`…).

**Causa técnica:** en windows-rs 0.58, `IAsyncOperation<T>` NO implementa el
trait `Future` (verificado en el fuente del crate: solo expone `.get()` y
`.GetResults()` bloqueantes). El `RequestAsync()?.await?` de los tutoriales
no compila en esta versión.

**Fix:** usar `.get()` bloqueante dentro de nuestras `async fn` (son llamadas
COM locales de milisegundos; aceptable en un loop de 250ms/1s). Si algún día
congela la TUI, mover a `spawn_blocking`.

**Para bebé:** queríamos pedir la cena con paloma mensajera mágica (`.await`)
pero esta versión solo tiene teléfono de disco (`.get()`): marcas, esperas
pegado a la bocina y te contestan. Funciona igual owo.

---

## 3. Faltaba el feature `Foundation_Collections`

**Síntoma:** `GetSessions()` no existía al compilar.

**Causa técnica:** en el fuente de windows 0.58,
`GetSessions()` está bajo `#[cfg(feature = "Foundation_Collections")]`.
Nuestro `Cargo.toml` solo habilitaba `Media_Control`, `Media_Playback` y
`Foundation`, así que el método ni se compilaba.

**Fix:** agregar `"Foundation_Collections"` a los features de `windows`.

**Para bebé:** es como comprar un control remoto sin pilas: el botón existe
pero no jala hasta ponerle las pilas (el feature) jajaja.

---

## 4. `cargo run` con Brave abierto (sin Spotify) se quedaba en "Conectando..."

**Síntoma:** con Brave abierto pero sin Spotify sonando, la TUI decía
`Conectando con Spotify...` para siempre y nunca abría nada.

**Causa técnica:** `ensure_brave_running()` solo miraba si `brave.exe`
existía vía `tasklist`. Si existía, asumía "ya está todo" y no lanzaba la
pestaña Spotify. Pero el plan (Fase 2.2) pedía checar la **sesión GSMTC**,
no el proceso: sin reproducción no hay sesión GSMTC que controlar.

**Fix:** al arrancar se pregunta `gsmtc::get_brave_session()`:
si hay sesión → se reutiliza; si no → se lanza Brave con la pestaña.

**Para bebé:** veías el refri (Brave) abierto y decías "ya hay comida",
pero adentro no había nada (sin sesión). Ahora primero abres el refri y
MIRAS si hay comida de verdad owo.

---

## 5. Perfil aislado = Brave nuevo sin tus cuentas

**Síntoma:** con `--user-data-dir` propio se abría un Brave "recién nacido":
sin login de Spotify, sin nada.

**Causa técnica:** `--user-data-dir` crea un perfil Chromium totalmente
separado (cookies, logins y todo aparte). Bueno para aislar, malo porque
obliga a loguearse de nuevo.

**Fix (tras varias vueltas bebé ↔ técnico):** quitar `--user-data-dir` y
usar `--new-window` con tu perfil normal: ventana nueva e independiente,
pero del MISMO navegador logueado. Nota honesta: si Brave ya estaba
abierto, Chromium ignora los flags de ahorro en esa ventana.

**Para bebé:** el perfil aislado era mudarte a otra casa donde no tienes
tus juguetes. `--new-window` es abrir otro cuarto EN TU MISMA casa:
nuevo pero con tus juguetes adentro uwu.

---

## 6. `--single-process` mataba la ventana de Spotify al instante

**Síntoma:** la ventana Brave "se cerraba sola" sin verse nada.

**Causa técnica:** `--single-process` deshabilita el sandboxing de Chromium
y rompe Widevine (el DRM que descifra el audio de Spotify). El renderer
moría al cargar Spotify. El plan ya lo advertía como opcional.

**Fix:** desactivarlo y quedarse con `--process-per-site` (el plan lo
permite). Se pierden ~60MB de ahorro pero Spotify sí abre.

**Para bebé:** quisimos que UN solo monito hiciera todo el trabajo para
ahorrar comida, pero el monito se cansó y se desmayó con la caja fuerte
(Widevine). Mejor varios monitos, comen más pero sí abren la caja jajaja.

---

## 7. Con `q` no se cerraba la pestaña de Brave (ni liberaba RAM)

**Síntoma:** `cargo run` → Brave → `q` → la TUI cerraba pero la pestaña
Spotify quedaba abierta comiendo RAM.

**Causa técnica:** cuando Brave ya estaba abierto, nuestro `Child` de
`Command::spawn` es un proceso delegador: le pasa `--new-window` a la
instancia viva y MUERE al instante. El `child.kill()` del cleanup mataba
un cadáver. La pestaña real vive en otro proceso.

**Fix:** rastrear la ventana por `HWND`: foto de ventanas Brave
(`Chrome_WidgetWin_1` visibles vía `EnumWindows`) antes y después de
lanzar; lo nuevo es NUESTRA ventana. Al salir se le manda `WM_CLOSE`
con `PostMessageW`. Seguridades: si ya la cerraste se ignora; si
navegaste a otro sitio (título sin "spotify") no se toca. Arranque en
frío (hijo vivo = proceso nuestro) sigue con `kill`. Requirió features
`Win32_Foundation` + `Win32_UI_WindowsAndMessaging` (y aprender que en
0.58 `HWND` es puntero `*mut c_void`, no `isize`: conversión con `as`).

**Para bebé:** nuestro control remoto (kill) apagaba una tele que YA estaba
apagada (el delegador muerto), mientras la tele de verdad seguía prendida.
Ahora anotamos QUÉ tele prendimos (foto antes/después) y apagamos justo
esa, sin tocar tus otras teles owo.

---

## 8. Los `println!` de diagnóstico quedaban tapados

**Síntoma:** nunca veías si reutilizó sesión o lanzó Brave nuevo.

**Causa técnica:** la TUI usa pantalla alternativa (`EnterAlternateScreen`):
todo lo impreso antes queda oculto debajo.

**Fix:** el diagnóstico viaja en `AppState.status` y se pinta en el CENTRO
de la TUI (`Brave ventana NUEVA visible PID ... | ... | dale play...`).

**Para bebé:** gritabas el diagnóstico y luego te tapabas los oídos con la
TUI jajaja. Ahora lo escribimos EN la TUI para leerlo con los ojos uwu.

---

## 9. `space` no iniciaba música en página fresca (EL bug importante)

**Síntoma:** si nada sonaba, `space` no hacía NADA. Solo funcionaba con
música ya reproduciéndose.

**Causa técnica:** GSMTC solo controla media EXISTENTE. En página fresca
(sin cola de reproducción) `TryPlayAsync` no tiene nada que resumir.
La TUI necesitaba hablarle a la PÁGINA, no al sistema.

**Fix (sin API Spotify, puro DOM como se pidió):** módulo `src/cdp.rs` con
Chrome DevTools Protocol: Brave se lanza con `--remote-debugging-port=9222`
+ `--remote-allow-origins=*`; `[space]` sin sesión hace `Runtime.evaluate`
con click al botón Play probando selectores ES/EN
(`control-button-playpause`, `play-button`, `Reproducir`, `Play`). Deps
mínimas nuevas: `serde_json`, `tokio-tungstenite`, `futures-util`.
La sesión GSMTC aparece sola y el loop reactivo la toma.

**Para bebé:** le pedías al vecino (GSMTC) que te prenda tu estéreo, pero el
vecino solo sabe pausar/cambiar lo que YA suena. Ahora la TUI estira su
propia manita (CDP) y aprieta el botón Play de la página. Sin pedirle nada
a Spotify por API owo.

---

## 10. El servidor CDP cerraba HTTP/1.0 sin responder (0 bytes)

**Síntoma:** el test del primer play fallaba con `respuesta CDP sin body`.

**Causa técnica:** verificado en el cable con socket crudo en PowerShell:
`GET /json/list HTTP/1.0` → el servidor DevTools cierra la conexión con
CERO bytes. Exige HTTP/1.1.

**Fix:** request `HTTP/1.1` con `Host: 127.0.0.1:9222` + `Connection: close`.

**Para bebé:** tocabas la puerta hablando en idioma viejito (1.0) y el
señor CDP te la azotaba sin decir nada. Hablando moderno (1.1) sí te abre
jajaja.

---

## 11. `read_to_end` colgaba el test 60s+ (sin timeout)

**Síntoma:** `cargo test space_inicia_musica` se trabó más de 60 segundos
y hubo que cancelarlo.

**Causa técnica:** `read_to_end` espera EOF; el servidor CDP a veces NO
cierra la conexión (keep-alive) → espera eterna. Ni `connect` ni la
lectura tenían timeout.

**Fix:** lectura por `Content-Length` (headers primero, luego bytes exactos)
+ fallback `dechunk()` si viene `Transfer-Encoding: chunked` + timeout
global de 8s en `spotify_ws_url` (y 5s encada llamada WS). Ahora falla
rápido y con mensaje claro en vez de colgarse.

**Para bebé:** esperabas a que el señor CDP colgara el teléfono para
empezar a hablar, pero el señor se quedaba en línea respirando para
siempre xd. Ahora cuentas sus palabras (Content-Length) y si tarda mucho
cuelgas tú (timeout) owo.

---

## 12. Sesión muerta si cerrabas la ventana a mano

**Síntoma:** cerrabas la ventana Brave y la TUI quedaba colgada mostrando
la última rola sin reconectar.

**Causa técnica:** `session: Option<Session>` nunca volvía a `None`; los
fallos de `get_track()` se tragaban en silencio eternamente.

**Fix:** racha de fallos: 12 ticks seguidos (~3s, más largo que los huecos
normales de cambio de canción) → `session = None`, `connected = false`,
status `sesion perdida, reintentando...` y el loop reactivo reconecta solo.

**Para bebé:** se te murió el perrito (ventana) pero seguías hablándole
como si viviera. Ahora cuentas 12 "¿sigues ahí?" sin respuesta y aceptas
que hay que buscar otro perrito jajaja (perdón, bebé).

---

## 13. Extras que no fueron bugs pero cuentan

- **Última canción persistente:** `ProgressSmoother` interpola
  `last_position + elapsed()` para barra suave (Spotify manda el progreso
  a saltos) y guarda la última rola en
  `%LOCALAPPDATA%/rata-spot-last-track.txt` en UTF-8 (emojis intactos) para
  mostrarla al arrancar aunque diga "Conectando...".
- **Anuncios:** `--disable-extensions` NO apaga el bloqueo de Brave porque
  Shields/uBlock integrado es nativo, no extensión. Sigue saltando anuncios.
- **Playlists en TUI:** GSMTC jamás verá tus playlists (solo lo que suena).
  Quedó definido que se hará leyendo el DOM vía CDP, sin API Spotify.
- **`HWND` en windows 0.58** es `*mut c_void`, no `isize`: las conversiones
  van con `as` en ambos sentidos.

---

*Fin de la bitácora — buena suerte rata 🐀 uwu*
