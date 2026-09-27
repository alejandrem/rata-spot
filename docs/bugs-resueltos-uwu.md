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

## 14. Pausa fantasma: 1 tap pausaba y se despausaba solo

**Síntoma:** un click a `space` pausaba solo mientras lo sostenías; al
soltar volvía a sonar. Sostenerla = flicker aleatorio.

**Causa técnica:** el handler hacía `match key.code` ignorando `key.kind`,
y crossterm 0.27 en Windows emite Press + Repeat + Release por tecla (sin
filtrar nada). 1 tap = Press(toggle pausa) + Release(toggle play) = neto
cero. Sostener = N+2 toggles = estado final aleatorio. De paso `n`/`p`
saltaban 2 rolas por tap sin que nadie lo notara.

**Fix:** ignorar `Release` en todo; `Repeat` solo en flechas/`j/k` (con el
throttle de 120ms); acciones (`space/n/p/enter/l`) solo en `Press`.

**Para bebé:** cada que aplaudías una vez, un duende aplaudía otra vez por
ti y se cancelaba el aplauso jajaja. Ahora al duende se le dice "shhh" en
los Release y solo cuenta tu aplauso owo.

---

## 15. Barra congelada con GSMTC clavado en ~40ms

**Síntoma:** sonando, la barra clavada en 0:00; solo se movía al pausar.

**Causa técnica:** verificado en vivo (`diag_gsmtc`): `playing=true`,
duración OK (202s), pero `Position` clavado en 39.525ms en 6 segundos.
Spotify a veces NUNCA actualiza el timeline. El interpolador anterior lo
reseteaba con el mismo valor congelado en cada tick.

**Fix:** modo *free-run* en `ProgressSmoother::update`: si sonando la
posición nueva difiere <500ms de la guardada, NO se resetea el reloj y se
sigue interpolando con `elapsed()`. Solo saltos reales (>500ms: ráfagas,
seek, cambio de rola) resincronizan. Funciona igual para Spotify sano, a
ráfagas o congelado. Tests: `free_run`, `pausa_congela`, `salto_resincroniza`.

**Para bebé:** el velocímetro estaba pegado en 0 porque el cable del sensor
no mandaba nada. Ahora cuando el cable se calla, la rata calcula sola con
su relojito, y si el cable vuelve a hablar le hace caso uwu.

---

## 16. Enter (buscar/abrir) pausaba la rola que sonaba

**Síntoma:** flujo `/` → escribo → Enter → la música actual se cortaba;
solo volvía a sonar la nueva después.

**Causa técnica:** `search()`, `open_playlist()` y `play_uri()` usaban
`Page.navigate`, que RECARGA el documento y destruye el elemento de audio.
Cada Enter mataba el stream antes de empezar el nuevo.

**Fix:** `spa_navigate()` en `cdp/tabs.rs`: `history.pushState` +
`PopStateEvent` para que React Router cambie de vista SIN destruir el
reproductor (igual que clickear un link a mano). Si la ruta no cambia en
~2s, fallback a `Page.navigate`. Verificado con test
`busqueda_no_corta_musica` (misma rola antes/después de buscar).

**Para bebé:** cada Enter era como cambiar de casa tirando la anterior con
todo y estéreo adentro. Ahora es como caminar a otro cuarto: la música te
sigue sonando hasta que pones la nueva jajaja.

---

## 17. Click lotería: sonó ZAPATA pidiendo Pika Pika

**Síntoma:** el test pedía Pika Pika y la verificación encontraba ZAPATA
sonando. El click "exitoso" había prendido otra cosa.

**Causa técnica:** en el DOM hay 100+ botones "Reproducir X" y el código
clickeaba el primero visible (`document.querySelector`). Lotería total.

**Fix:** click POR TÍTULO: se busca el botón cuyo `aria-label` contenga el
título esperado (con `scrollIntoView` si está virtualizado). Si no existe,
se reporta en vez de clickear al azar. Probado con inventario DOM
(`diag_play_buttons`) que mostró al culpable.

**Para bebé:** gritabas "¡pon Pika Pika!" en un cuarto con 100 botones y el
robot apretaba el primero que veía. Ahora lee las etiquetas y aprieta el
que dice Pika Pika owo.

---

## 18. Click untrusted no arranca audio (ni el trusted de mouse, a veces)

**Síntoma:** click entregado OK al botón correcto... y silencio. Ni sesión
GSMTC aparecía.

**Causa técnica:** `.click()` por JS es evento *untrusted* (sin user
activation) y Spotify no inicia streams nuevos así. El click por
coordenadas (`Input.dispatchMouseEvent`) funcionó unas veces y otras no.

**Fix:** tecla **Espacio real** por CDP (`Input.dispatchKeyEvent`
keyDown+keyUp) = exactamente lo que haría tu dedo, con user activation.
`play_uri` ahora: si el player dice Pausar con sesión → listo; si dice
Pausar sin sesión (wedged) → recarga; si no → Space + espera registro
GSMTC (3s antes de reintentar, para no pausar lo recién prendido).
Probado en vivo: `SUENA: Pika Pika [0:00 / 2:28]`.

**Para bebé:** era como tocar la puerta con guantes de fantasma: la puerta
oía el toc-toc pero no abría porque no sentía mano de verdad. Ahora
mandamos una manita de verdad (tecla Espacio) y sí abren jajaja.

---

## 19. Un video de Facebook secuestraba la sesión (y los tests)

**Síntoma:** el test pedía Pika Pika, todo "en verde"... pero lo que sonaba
era un video de Facebook ("Claude Code en Español"). `get_brave_session`
agarraba la PRIMERA sesión Brave sonando, fuera la que fuera.

**Causa técnica:** GSMTC no dice de qué pestaña viene cada sesión. Con tu
Brave normal + el nuestro conviviendo, cualquier video con audio compite.

**Fix triple:** `brave_sessions()` (todas) + `pick_session(hint)` ( boot
prefiere la que coincida con tu historial) + switch anti-oscilación en el
tick (~1s): si la actual está pausada y OTRA flippeó a sonando, cambiarse;
con la actual sonando jamás se cambia (imposible oscilar). Tests estrictos
por título en vez de "cualquier sesión".

**Para bebé:** el control remoto agarraba la primera tele prendida, aunque
fuera la del vecino viendo Facebook. Ahora pregunta "¿cuál se prendió
al último?" y si la tuya estaba pausada, cambia a la nueva. Y nunca brinca
como loco entre teles owo.

---

## 20. Player wedged: UI en "Pausar" sin audio ni `<audio>`

**Síntoma:** la página juraba que sonaba (botones en "Pausar") pero no
había ni elementos `<audio>` en el DOM ni sesión GSMTC. Clicks al vacío.

**Causa técnica:** verificado con `diag_media`: reproductor en estado
imposible (intento de play anterior atorado a medias). Probable vendor:
navegaciones rápidas de los tests + recaptcha Enterprise al acecho.

**Fix:** `player_stuck()` lo detecta (Pausar + 0 media + 0 sesiones) y
`play_uri` recarga la pestaña antes de clickear. Solo cuando nada suena
(jamás interrumpe tu música).

**Para bebé:** el estéreo decía "SONANDO" con foquitos prendidos pero sin
bocinas conectadas jajaja. Se detecta el estéreo fantasma, se desconecta
y se vuelve a conectar, y ya suena de verdad.

---

## 21. `/json/list` a veces flap ea vacío + tabs que se evaporan

**Síntoma:** a mitad de un flujo, "no hay pestana Spotify" aunque existía
un segundo antes (pestañas cerradas a mano, renderer crasheado, lista
flappeando).

**Causa técnica:** el WS se resolvía UNA vez al inicio del flujo y se
reusaba; si la pestaña moría o la lista flappeaba, todo lo demás fallaba
en cascada.

**Fix:** WS fresco por intento en los 4 loops (play, tracks, playlist,
open), `ensure_spotify_tab()` al inicio de cada flujo (recrea la pestaña
si falta, sin duplicar) y reintentos con timeout en todo (8s fetch, 5s
WS, 12/30s lecturas).

**Para bebé:** antes anotabas el número de tu amigo una vez y si cambiaba
de número ya no le atinabas nunca. Ahora preguntas el número fresh cada
vez que marcas, y si no existe lo vuelves a invitar owo.

---

## 22. Tests ruidosos: la suite te despertaba con Pika Pika

**Síntoma:** `cargo test` a secas sonaba música, navegaba tu Brave y te
asustaba a media noche. Además medía lo que TÚ mirabas (stories) en vez
de Spotify.

**Fix:** los tests que suenan/navegan llevan `#[ignore]`: `cargo test` =
silenciosos y deterministas; `cargo test -- --include-ignored` = vivos
con manos fuera 1 minuto. Los de lectura/diag siguen corriendo siempre.

**Para bebé:** la alarma de pruebas gritaba a las 3am con tu canción.
Ahora tiene modo silencioso por default y modo fiesta solo si lo pides
jajaja.

---

## 23. Selectores regados + search solo-DOM (blindaje anti-rediseño)

**Síntoma:** cada rediseño de Spotify Web era cacería en 5 archivos, y el
search dependía 100% del DOM.

**Causa técnica:** los JS vivían duplicados en `playback/library/search/
tracks/track_page` y el search solo sabía leer anchors del DOM.

**Fix doble:** 1) `src/cdp/selectors.rs`: TODOS los selectores y JS en un
solo lugar, con cadena de fallback (`data-testid` → `role`+`href` →
`aria-label` ES/EN, jamás clases hash) + `health_summary()` que corre en
`boot.rs` y pinta `DOM 6/6 ok` o qué falló. Tests obligan a que el JS
mencione las consts. 2) `src/cdp/api.rs`: el search intenta JSON/red
primero (`fetch` dentro de la página con tu login + Web API `/v1/search`,
tracks primero, cap 24 igual que el DOM) y cae al DOM si falla
(`RATA_SPOT_API=0` lo apaga). Verificado en vivo: `diag_selectores` da
6/6 y `diag_api` dice la verdad (`token web status 403` → DOM; la red
real hoy es `pathfinder/v2` + `clienttoken.spotify.com`, no el endpoint
viejo). Cero regresiones: lo peor es un HTTP fallido de ms.

**Para bebé:** antes tenías tus juguetes regados por toda la casa y si
Spotify movía un mueble llorabas buscándolos. Ahora viven en UNA caja
con etiquetas (selectors) y la rata primero pregunta por teléfono (API)
antes de ir a buscar a mano (DOM). Y si el teléfono no contesta, va a
mano como siempre owo.

---

## 24. Puerto 9222 fijo + `--remote-allow-origins=*` (P0/P1)

**Síntoma:** el puerto fijo colisiona con otras herramientas y es un
objetivo famoso; el `*` anulaba la defensa de Origin de Chromium
(cualquier web visitada podía manejar el Brave por CDP: cookies,
navegación, JS en tus sesiones).

**Causa técnica:** flags estáticos en `launcher/config.rs` y
`transport.rs` con `CDP_PORT = 9222` clavado.

**Fix:** nuevo `src/launcher/ports.rs`: al lanzar se aparta un puerto
libre de verdad (bind a `127.0.0.1:0`), se publica para el transporte y
los flags salen como `--remote-debugging-port={p}` +
`--remote-allow-origins=http://127.0.0.1:{p}` (acotado, jamás `*`: ni el
HTTP crudo ni tungstenite mandan `Origin`, así que pasa el check igual).
El transporte prueba `[nuestro puerto, 9222 legacy]` para no romper al
Brave ya abierto con flags viejos. `RATA_SPOT_PORT` fija el puerto a
mano. Tests: candidatos sin duplicar, flags sin wildcard, puerto
enlazable. Riesgo conocido: Brave ya abierto SIN depuración sigue
degradando a GSMTC (eso es P2, pendiente).

**Para bebé:** antes la puerta de tu casa era siempre la misma (9222) y
con letrero de "pasen todos" (`*`). Ahora cada arranque usa una puerta
distinta que solo tú conoces, y el letrero dice "solo la rata" owo. Si
llegas y la casa ya estaba abierta de antes, tocas la puerta vieja por
si acaso.

---

## 25. Brave abierto pero sordo: hijo inútil + "reinicia todo" (P2)

**Síntoma:** con Brave abierto a mano (sin CDP), el boot spawneaba un
hijo que solo delegaba `--new-window` y moría, `space` degradaba y el
mensaje te mandaba a cerrar todo y reiniciar la app.

**Causa técnica:** `boot()` solo distinguía "hay sesión GSMTC / no la
hay"; nunca preguntaba si había PUERTO CDP (`/json/list`) ni si
`brave.exe` seguía vivo.

**Fix:** boot en 3 estados con `cdp::debug_alive()` (un GET a
`/json/list`) + `is_brave_running()` (tasklist): con Brave sordo se
entra a sala de espera (println visibles pre-TUI, sin spawnear nada),
y al detectar el cierre se auto-lanza en frío —ahí sí aplican los
flags— y el boot continúa solo (con 2s de respiro al SO para no delegar
al cadáver). Si aparece CDP solo, se sigue sin lanzar. Sin timeout
(filosofía del proyecto); Ctrl+C cancela sin limpiar nada porque nada
se lanzó.

**Para bebé:** antes tocabas a una puerta sorda y te mandaban a tu casa
a reiniciar la vida. Ahora la rata espera sentada en la banqueta,
cuando sales y cierras, ella solita abre la puerta buena y te avisa
owo.

---

## 26. Modo aislado P3: perfil propio sin delegación [REVERTIDO]

**Idea:** `RATA_SPOT_ISOLATED=1` lanzaba con su propio
`--user-data-dir` (arranque siempre en frío, sin delegación).

**Por qué se revirtió:** el requisito real es abrir Brave CON las
cuentas ya logueadas; el perfil aislado nace vacío y pide re-login.
Brave con tus cuentas = perfil normal, punto. Quedan vigentes P0/P1
(puerto efímero + origins acotados) y P2 (sala de espera).

**Lección de test:** el test vivo `aislado_lanza_y_limpia` ponía asserts
ANTES del cleanup; al tronar un assert (DevToolsActivePort ausente con
puerto explícito: Chromium solo lo escribe siempre con puerto 0), el
`cleanup()` se saltaba y la ventana quedaba huérfana. Regla nueva: en
tests vivos, el cleanup va con guardia (o antes de los asserts que
pueden tronar), nunca al final feliz. El huérfano se cazó por
`CommandLine` con filtro `rata-spot-brave-profile` (el Brave normal ni
se tocó, la música siguió sonando).

---

## 27. Los álbumes no abrían (track-list vs playlist-tracklist)

**Síntoma:** Enter en un álbum → `abriendo playlist...` eterno (TUI
congelada hasta 16s: 10s esperando contenedor + 6s reintentando) y
nunca mostraba canciones. Los caracteres raros del reporte eran
copiado de terminal con líneas envueltas, no bug de render (la vista
`ui/tracks.rs` dibuja bien).

**Causa técnica:** la página de álbum usa
`section[data-testid="album-page"]` con grid
`data-testid="track-list"`, pero `open_playlist()` y `TRACKS_JS` solo
aceptaban `playlist-tracklist` → espera imposible + `{error:
'no-tracklist'}`. Las filas son idénticas adentro (`internal-track-link`,
`a[href*="/artist/"]`, duración `m:ss`, botón `Reproducir ...`).

**Fix:** `selectors.rs`: const `TRACKLIST_TESTIDS =
["playlist-tracklist", "track-list"]` y los 4 snippets (exists, scroll
top/down, snapshot) aceptan ambos; test de sincronía lo obliga.
Mensajes neutros (`abriendo...`, `la pagina no cargo sus canciones`).
Test vivo `album_tracks_se_leen` (ignorado): probado en vivo, 43/43
rolas con número, título, artista y duración.

**Para bebé:** la rata solo conocía la puerta de las playlists y se
quedaba tocando la pared de los álbumes. Ahora toca las dos puertas owo.

---

*Fin de la bitácora — buena suerte rata 🐀 uwu*


