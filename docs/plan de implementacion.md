================================================================================
  RATA-SPOT 🐀  —  PLAN DE IMPLEMENTACION COMPLETO
  Modo: Brave minimizado + RAM minima + Windows
  Objetivo RAM: ~123MB total (Brave ~120MB + Rust ~3MB)
================================================================================


--------------------------------------------------------------------------------
FASE 1 — SETUP DEL ENTORNO RUST
--------------------------------------------------------------------------------

PASO 1.1 — Instalar Rust

  Abrir PowerShell como administrador y correr:

    winget install Rustlang.Rustup
    rustup toolchain install stable-x86_64-pc-windows-msvc
    rustup default stable-x86_64-pc-windows-msvc

  Verificar:
    rustc --version
    cargo --version

  IMPORTANTE: usar el toolchain MSVC (no GNU). windows-rs lo requiere.


PASO 1.2 — Crear el proyecto

    cargo new rata-spot --bin
    cd rata-spot


PASO 1.3 — Cargo.toml

  Abrir Cargo.toml y dejarlo asi:

    [package]
    name = "rata-spot"
    version = "0.1.0"
    edition = "2021"

    [dependencies]
    ratatui   = "0.27"
    crossterm = "0.27"
    tokio     = { version = "1", features = ["full"] }
    anyhow    = "1"

    [target.'cfg(windows)'.dependencies]
    windows = { version = "0.58", features = [
      "Media_Control",
      "Media_Playback",
      "Foundation",
    ]}

    [profile.release]
    opt-level     = "z"      # optimizar por tamano binario
    lto           = true     # link-time optimization agresivo
    codegen-units = 1        # un solo codegen para mejor LTO
    panic         = "abort"  # elimina stack unwinding (~500KB menos)
    strip         = true     # borra simbolos de debug del binario


--------------------------------------------------------------------------------
FASE 2 — LANZADOR DE BRAVE ULTRA-OPTIMIZADO
--------------------------------------------------------------------------------

  El objetivo es bajar Brave de ~200MB a ~120MB desactivando todo lo que no
  se necesita para reproducir audio.

PASO 2.1 — Crear src/launcher.rs

  Este modulo se encarga de:
    - Encontrar la ruta de instalacion de Brave
    - Lanzarlo con los flags correctos si no esta abierto
    - Mover la ventana fuera de pantalla (no headless, pero invisible)

  FLAGS A USAR (en orden de impacto en RAM):

    --disable-extensions
        Mata todas las extensiones. Ahorro: ~30-50MB

    --disable-background-networking
        Desactiva peticiones de red en segundo plano (sync, etc)

    --disable-background-timer-throttling
        Evita timers innecesarios en background

    --disable-backgrounding-occluded-windows
        No procesa ventanas que estan detras de otras

    --disable-sync
        Desactiva sincronizacion con cuenta Brave/Google

    --disable-translate
        Quita el modulo de traduccion automatica

    --disable-plugins
        Sin plugins (Flash ya no existe pero limpia procesos)

    --disable-default-apps
        No carga apps por defecto de Chrome/Brave

    --process-per-site
        En vez de un proceso por tab, uno por sitio. Ahorro: ~40MB

    --single-process
        TODO en un solo proceso. Ahorro: ~60MB adicionales.
        NOTA: esto deshabilita el sandboxing. Para rata-spot que
        solo abre una URL es aceptable. Si te preocupa la seguridad,
        omite este flag y usa solo --process-per-site.

    --window-position=-32000,-32000
        Mueve la ventana completamente fuera del area visible.
        La ventana EXISTE pero no la ves. No es headless.

    --window-size=1280,720
        Tamano fijo pequeno. Sin esto algunos layouts crashean.

    --autoplay-policy=no-user-gesture-required
        Permite que el audio inicie sin que el usuario haga click.
        CRITICO para que Spotify suene solo al abrir.

    --no-first-run
        Salta el wizard de primera vez.

    --disable-gpu (OPCIONAL)
        Desactiva aceleracion GPU. Ahorro: ~20MB adicionales.
        ADVERTENCIA: puede causar que Spotify Web baje calidad de
        reproduccion en algunos sistemas. Probar primero sin este flag.

  URL OBJETIVO: https://open.spotify.com
  (o https://music.apple.com si prefieres Apple Music)


PASO 2.2 — Logica del lanzador en Rust

  La funcion principal hace esto:
    1. Checa si ya hay una sesion GSMTC activa de Brave
    2. Si ya hay sesion -> no hace nada
    3. Si no hay sesion -> busca brave.exe en rutas comunes y lo lanza

  Rutas comunes de Brave en Windows:
    C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe
    C:\Program Files (x86)\BraveSoftware\Brave-Browser\Application\brave.exe
    %LOCALAPPDATA%\BraveSoftware\Brave-Browser\Application\brave.exe

  Guardar el handle del proceso lanzado en una variable global (Arc<Mutex>)
  para poder matarlo cuando el usuario salga de rata-spot.

  Usar stdout(Stdio::null()) y stderr(Stdio::null()) para que Brave no
  imprima nada en la terminal donde corre rata-spot.

  NO uses un delay fijo de 3-4 segundos despues de lanzar Brave.
  Spotify Web puede tardar mas dependiendo del internet. En lugar de eso,
  deja que el loop reactivo de la Fase 3.2 se encargue de esperar:
  lanza Brave y regresa inmediatamente; la TUI mostrara "Conectando..."
  mientras get_brave_session() sigue reintentando hasta que haya sesion.


--------------------------------------------------------------------------------
FASE 3 — BACKEND GSMTC (el control remoto real)
--------------------------------------------------------------------------------

  GSMTC = Global System Media Transport Controls
  Es la API nativa de Windows que controla la barra de media (la que
  aparece cuando subes el volumen y muestra la cancion actual).
  Cualquier app que reproduzca audio se registra aqui automaticamente.
  Brave lo hace solo, sin configuracion extra.

PASO 3.1 — Crear src/gsmtc.rs

  Structs necesarios:

    pub struct TrackInfo {
        pub title:    String,
        pub artist:   String,
        pub album:    String,
        pub playing:  bool,
        pub progress: f64,   // 0.0 a 1.0
    }

  Funciones a implementar:

    get_brave_session() -> Result<Session>
      Pide al SessionManager todas las sesiones activas.
      Itera buscando la que tenga "brave" en el SourceAppUserModelId.
      Si no la encuentra, retorna error (Brave no esta listo todavia).

    get_track(session) -> TrackInfo
      Llama a TryGetMediaPropertiesAsync() para titulo/artista/album.
      Llama a GetPlaybackInfo() para saber si esta playing o paused.
      Llama a GetTimelineProperties() para obtener Position y EndTime
      y calcular el ratio de progreso (position / end_time).

      ADVERTENCIA IMPORTANTE — progreso a saltos:
      Spotify Web no actualiza TimelineProperties cada milisegundo;
      manda rafagas cada varios segundos para ahorrar recursos. Esto
      hace que la barra de Ratatui se mueva a saltos en lugar de fluir.

      SOLUCION — contador interno en el estado de Rust:
      En tu AppState guarda:
        last_gsmtc_position: Duration   (la posicion que llego de GSMTC)
        last_gsmtc_timestamp: Instant   (cuando llego ese dato)

      Para calcular la posicion actual en cada tick del render:
        posicion_real = last_gsmtc_position + last_gsmtc_timestamp.elapsed()

      Cuando llega un update nuevo de GSMTC, actualiza ambos valores.
      Cuando la cancion esta en pausa, NO sumes el elapsed.
      Con esto la barra se mueve suavemente aunque GSMTC tarde en avisar.

    play(session)   -> llama TryPlayAsync()
    pause(session)  -> llama TryPauseAsync()
    next(session)   -> llama TrySkipNextAsync()
    prev(session)   -> llama TrySkipPreviousAsync()

    toggle(session)
      Checa el estado actual con GetPlaybackInfo().
      Si esta playing -> pause. Si esta paused -> play.


PASO 3.2 — Manejo de errores GSMTC (loop reactivo)

  GSMTC puede fallar si:
    - Brave todavia no cargo Spotify (sesion no registrada aun)
    - El usuario pauso manualmente y no hay metadata
    - Windows no tiene el servicio GSMTC activo (raro pero pasa)
    - La conexion es lenta y Spotify tarda mas de lo esperado en cargar

  Estrategia — NO usar delay fijo, usar loop reactivo:

    loop {
      match get_brave_session() {
        Ok(session) => break session,
        Err(_) => {
          // mostrar "Conectando con Spotify..." en la TUI
          // y reintentar cada 1 segundo indefinidamente
          tokio::time::sleep(Duration::from_secs(1)).await;
        }
      }
    }

  Una vez conectado, si get_track() falla en medio de una sesion
  (por ejemplo la cancion termino y hay un momento sin metadata),
  mostrar el ultimo TrackInfo conocido en lugar de pantalla en blanco,
  y seguir reintentando silenciosamente en background.

  No crashear el programa bajo ninguna circunstancia de GSMTC.


--------------------------------------------------------------------------------
FASE 4 — INTERFAZ TUI CON RATATUI
--------------------------------------------------------------------------------

  Layout de 4 zonas verticales:

    +------------------------------------------+
    |  [HEADER] titulo — artista               |  3 lineas
    +------------------------------------------+
    |  [PROGRESO] =====>................. 1:23  |  3 lineas
    +------------------------------------------+
    |                                          |
    |  [CENTRO] espacio libre / estado         |  flexible
    |                                          |
    +------------------------------------------+
    |  [space] play/pause  [n] next  [q] quit  |  1 linea
    +------------------------------------------+


PASO 4.1 — src/ui.rs

  Importar de ratatui:
    Layout, Constraint, Paragraph, Block, Borders, Gauge,
    Style, Color, Line, Span, Alignment

  Funcion render(frame, state):

    HEADER:
      Block con bordes y titulo " 🐀 rata-spot "
      Paragraph con dos spans: titulo en bold blanco, artista en gris
      Si state.playing -> mostrar "▶" antes del titulo
      Si no -> mostrar "⏸"

    BARRA DE PROGRESO:
      Gauge con ratio = state.progress (f64 entre 0.0 y 1.0)
      Color verde cuando playing, gris cuando paused
      Label con el tiempo: "1:23 / 3:45"
      Calcular el tiempo desde los valores de TimelineProperties

    CENTRO:
      Si hay error de sesion -> "Conectando con Spotify..."
      Si todo ok -> dejar vacio o mostrar album

    FOOTER:
      Paragraph con los keybindings en gris oscuro
      Sin bordes, pegado al fondo


PASO 4.2 — src/main.rs — Event loop

  Estructura del loop principal:

    1. setup_terminal() — modo raw, pantalla alternativa
    2. ensure_brave_running() — lanzar Brave si no esta
    3. get_brave_session() con reintentos
    4. LOOP:
         a. terminal.draw(|f| render(f, &state))
         b. event::poll(Duration::from_millis(250))
         c. si hay evento de teclado:
              SPACE -> toggle(&session)
              n     -> next(&session)
              p     -> prev(&session)
              q     -> break
         d. state.track = get_track(&session) [puede fallar, manejar]
    5. restore_terminal()
    6. cleanup() — matar Brave si rata-spot lo lanzo

  El poll de 250ms es el balance entre responsividad y CPU.
  No bajar de 100ms (CPU innecesaria) ni subir de 1000ms (se siente lento).


--------------------------------------------------------------------------------
FASE 5 — OPTIMIZACIONES FINALES DE RAM
--------------------------------------------------------------------------------

PASO 5.1 — Compilar en release

    cargo build --release

  El binario queda en target/release/rata-spot.exe
  Tamano esperado: ~1.5-2MB
  RAM en runtime: ~2-4MB


PASO 5.2 — Verificar RAM real con Task Manager

  Abrir Task Manager -> Details
  Buscar brave.exe (habra varios procesos si no usas --single-process)
  Sumar todos los "Private Bytes" de los procesos de Brave
  El objetivo es que el total sea < 130MB


PASO 5.3 — Script de cleanup al salir

  Guardar el PID del proceso Brave que rata-spot lanzo.
  Al salir con 'q', matar solo ese proceso.
  Si Brave ya estaba abierto antes -> NO matarlo.
  Usar un flag booleano: brave_was_already_running


PASO 5.4 — Opcional: reducir mas RAM con --disable-gpu

  Si el audio sigue funcionando bien con --disable-gpu, agregarlo.
  Ahorro adicional: ~15-25MB.

  ADVERTENCIA IMPORTANTE — Widevine DRM:
  Spotify Web usa aceleracion por hardware para descifrar el audio
  mediante Widevine (el DRM de Google). Sin GPU, este proceso cae
  sobre la CPU y puede volverse inestable: audio cortado, trabado
  o que directamente no suene.

  COMO PROBARLO CORRECTAMENTE:
    1. Agrega --disable-gpu a los flags
    2. Abre rata-spot y deja que Spotify cargue completo
    3. Reproduce 2-3 canciones completas escuchando si hay cortes
    4. Si hay cualquier problema de audio -> quita el flag
    5. Los ~20MB que ahorras NO valen un audio arruinado

  Si tu maquina tiene GPU dedicada (no integrada) es mas probable
  que falle con este flag. En GPUs integradas suele funcionar mejor.


--------------------------------------------------------------------------------
ORDEN DE IMPLEMENTACION RECOMENDADO
--------------------------------------------------------------------------------

  Semana 1:
    [ ] Fase 1 completa — entorno Rust funcionando, cargo build OK

  Semana 2:
    [ ] Fase 2 — Brave se lanza con los flags y suena Spotify
    [ ] Verificar en Task Manager que la RAM bajo vs Brave normal

  Semana 3:
    [ ] Fase 3 — GSMTC conectado, play/pause/next/prev funcionando
    [ ] Probar con println! antes de tener TUI

  Semana 4:
    [ ] Fase 4 — TUI basica con Ratatui, layout funcionando
    [ ] Conectar TUI con GSMTC

  Semana 5:
    [ ] Fase 5 — compilar release, medir RAM real, ajustar flags
    [ ] Cleanup al salir


--------------------------------------------------------------------------------
RESUMEN DE RAM ESPERADA
--------------------------------------------------------------------------------

  Componente                   RAM
  --------------------------   ----------
  rata-spot (Rust, release)    ~3 MB
  Brave (flags optimizados)    ~120 MB
  --------------------------   ----------
  TOTAL                        ~123 MB

  Comparacion:
    Spotify app nativa Windows  ~200-300 MB
    Brave sin optimizar         ~200 MB
    rata-spot + Brave optim.    ~123 MB  <- nuestro objetivo


================================================================================
  FIN DEL PLAN — buena suerte rata 🐀
================================================================================