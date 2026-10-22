# slack-rs: cómo está hecho y por qué es más ligero

Este documento describe el clon tal como está en el código hoy (no como lo cuenta el
README), cómo fluye la información de Slack a la pantalla, y qué tan ligero es, con
números medidos en esta máquina. Lo que no se midió está marcado como **inferido**.

Las referencias van como `archivo:línea` sobre el commit `5f21165`.

## 1. Qué hace hoy

Verificado leyendo el código, no el README.

### Interfaz

- **Distribución de Slack**: barra superior con buscador y estado del tiempo real,
  rail de workspaces a la izquierda, lateral morada, canal al centro y panel de hilo a
  la derecha (`vista.rs:50`, `vista.rs:132`). Ventana inicial de 1280×820
  (`main.rs:22`).
- **Rail**: un ícono por workspace (imagen del equipo vía `team.info`, o su inicial si
  no tiene). Con varios workspaces, el activo lleva anillo blanco y los demás una
  insignia roja con menciones o un punto blanco si hay no leídos; tooltip con el
  nombre y `Ctrl+N` (`vista.rs:87`). Debajo, botones *Inicio* y *MD* (`vista.rs:200`).
- **Lateral**: secciones *Canales* y *Mensajes directos*, plegables
  (`vista.rs:239`). Canales públicos con `#`, privados con 🔒, DMs con el avatar del
  otro, grupos (mpim) con el número de miembros y sus nombres visibles en lugar de
  `mpdm-ana--beto-1` (`main.rs:622`). No leídos en negrita, insignia con menciones
  (`vista.rs:280`). Los canales se ordenan por nombre (`main.rs:309`).
- **Canal**: encabezado con nombre, lista de mensajes anclada abajo
  (`vista.rs:428`), compositor.
  - Avatar (foto de 72 px del CDN o inicial sobre color fijo por usuario).
  - Mensajes del mismo autor agrupados si hay menos de 5 min entre ellos
    (`vista.rs:772`); en los agrupados la hora aparece al pasar el mouse.
  - Separadores de día: *Hoy*, *Ayer*, o `martes, 3 de junio [de 2025]`
    (`vista.rs:785`).
  - Texto con formato mrkdwn (ver §2.6), menciones resaltadas, enlaces clicables.
  - Archivos como tarjeta con nombre y tipo (`vista.rs:835`); no se descargan ni se
    previsualizan.
  - Adjuntos de bots: si el mensaje no trae `text`, se muestra el `fallback` del
    primer adjunto (`slack.rs:163`).
  - Reacciones como píldoras con conteo; la tuya resaltada en azul; clic alterna la
    tuya (`vista.rs:621`).
  - Enlace "N respuestas" con hasta 4 avatares de quienes respondieron
    (`vista.rs:646`). Las respuestas de hilo no se mezclan en el canal, salvo las
    publicadas también en el canal (`thread_broadcast`, `slack.rs:149`).
- **Barra al pasar el mouse** sobre un mensaje: 👍 ✅ 👀 y ↩ responder en hilo
  (`vista.rs:671`, `main.rs:27`).
- **Hilo**: panel de 400 px con el mensaje raíz, separador "N respuestas", las
  respuestas y su propio compositor (`vista.rs:449`).
- **Compositor** (`vista.rs:697`): botones B / I / S / `</>` que envuelven la selección
  con `*`, `_`, `~`, `` ` `` o dejan el cursor entre dos marcas (`main.rs:675`).
  Intro envía, Mayús + Intro agrega línea (`vista.rs:752`). El botón de enviar se
  desactiva si el texto está vacío.
- **Atajos**: Ctrl + 1…9 cambia de workspace, también con el foco en el compositor
  (`main.rs:189`, `vista.rs:761`).
- **Enlaces**: se abren con `xdg-open`, solo si empiezan con `https://`, `http://` o
  `mailto:` (`main.rs:399`).
- **Título de la ventana**: `canal - equipo - Slack` (`main.rs:279`).
- **Errores**: franja rosa arriba del canal, con botón para cerrarla
  (`vista.rs:868`). Sin credenciales, el centro explica qué variables faltan
  (`vista.rs:365`).

### Comportamiento con Slack

- **Marcar como leído** (`conversations.mark`) al cargar el historial del canal
  abierto, al llegar un mensaje al canal abierto, y al volver a un workspace si llegó
  algo mientras estaba en segundo plano (`main.rs:256`). Un workspace en segundo plano
  nunca marca nada como leído (`main.rs:497`).
- **No leídos y menciones** al arrancar vía `client.counts`; después se actualizan
  localmente con el tiempo real: cualquier mensaje ajeno en otro canal lo marca no
  leído, y suma mención si es DM o te menciona con `<@tu_id>` (`main.rs:486-506`).
- **Enviar** con `chat.postMessage`, al canal o al hilo (`main.rs:510`). Con tiempo
  real conectado no se vuelve a pedir el historial: el mensaje llega por el websocket
  (`main.rs:380-391`).
- **Reacciones optimistas**: se aplican en pantalla al instante y luego se manda
  `reactions.add`/`reactions.remove` (`main.rs:532`). `aplicar_reaccion` es
  idempotente para que el eco del websocket no cuente doble (`slack.rs:175`).
- **Tiempo real**: mensajes nuevos, editados, borrados, respuestas de hilo (sube el
  contador y agrega al hilo abierto) y reacciones (`main.rs:425-508`).
- **Varios workspaces** a la vez (ver §2.5).

### Lo que falta o está a medias

| Qué | Estado en el código |
|---|---|
| Buscador de arriba | Escribir filtra la lateral; Intro busca mensajes con `search.messages` y muestra los resultados en el centro (clic en el canal lo abre). No salta al mensaje exacto ni tiene filtros. |
| Botón *MD* del rail | Panel de DMs y grupos ordenados por actividad (`latest` de `client.counts` y tiempo real), con hora y vista previa del último mensaje cargado. **Falta verificar** que `client.counts` traiga `latest` en un workspace real. |
| `▾` junto al equipo y al canal | Decorativos, sin acción (`vista.rs:229`, `vista.rs:419`). |
| Historial | 100 mensajes al abrir; al subir cerca del tope pide los 100 anteriores (`latest` + `has_more`) y dice "Este es el principio" cuando ya no hay. Hilos: 200 respuestas. |
| Imágenes y archivos | Las imágenes muestran su miniatura de 360 px (`thumb_360`, bajada con la sesión solo desde files.slack.com); clic abre el archivo en Slack. Lo demás sigue como tarjeta con nombre y tipo; no se descarga. |
| Emoji | Estándar (crate `emojis`) y propios del workspace (`emoji.list`, alias resueltos; las imágenes se bajan una vez, al aparecer). Los propios se ven como imagen en reacciones, selector y mensajes que son solo emoji (en grande, como Slack); dentro de un renglón de texto quedan como `:nombre:` porque `rich_text` de iced no admite imágenes. Selector con búsqueda (botón 😀 en la barra de acciones y en el compositor). |
| Formato | Anida estilos (`*_x_*`, `*<@U…>*`), citas `>`, bloques ```` ``` ```` en caja y listas `•` / `1.` (`formato::bloques`). Si el mensaje solo trae bloques `rich_text`, se pintan de ahí (`formato::del_mensaje`); los bloques de bots (`section`, etc.) siguen usando `text`. Las listas anidadas se pintan planas. |
| Editar / borrar mensajes propios | ✏️ edita en el lugar (Intro guarda, Esc cancela; ↑ con el compositor vacío edita el último propio) con `chat.update`; 🗑️ pide confirmación y usa `chat.delete`. Dejarlo vacío al editar pide borrarlo. Ambos se aplican al instante. |
| Autocompletado | `@` personas (las ya conocidas, no el directorio completo), `#` canales y `:` emoji (desde 2 letras); ↑ ↓, Intro/Tab y Esc. En el compositor se ve `@Nombre` y al enviar se traduce a `<@U…>`. |
| Ctrl+K | Ventana "Saltar a…" con coincidencia por prefijo, palabra o subcadena, sin acentos; ↑ ↓ Intro Esc. |
| Notificaciones, presencia, "escribiendo…", subir archivos | No existen. |
| Secciones personalizadas, destacados, canales silenciados | Se leen de `users.channelSections.list` y `users.prefs.get` (no documentados; forma **por verificar** contra respuestas reales). Si fallan, quedan las dos secciones fijas. Los silenciados van atenuados y no cuentan para el rail salvo menciones. No pagina secciones con muchos canales. |
| Tema | Solo Aubergine claro, fijo. |
| Caché | Nada en disco: cada arranque vuelve a pedir canales, usuarios y avatares. Al abrir un canal siempre se pide el historial otra vez (mientras llega se ve el que ya estaba en memoria). |
| Envío fallido | Si `chat.postMessage` falla, el texto vuelve al compositor (si no se empezó a escribir otra cosa) y queda el error. |
| Reconexión | Al volver el websocket después de un corte se piden los mensajes posteriores al último de cada canal cargado (`oldest`, hasta 200), `client.counts` y el hilo abierto, y se fusionan sin repetir. La primera conexión no cuenta como corte. Sigue sin backoff (5 s fijos). |
| Modo demo | `#desarrollo`, su hilo y los DMs tienen mensajes, con secciones, un canal silenciado, un emoji propio y una imagen generados en memoria. Los canales sin mensajes se quedan en "Cargando…" porque no hay cliente. Reaccionar y enviar no van a la red. |
| Fuentes | Lato y Noto Color Emoji se piden por nombre al sistema (`estilo.rs:37-38`), no van dentro del binario. Sin Lato instalada, iced usa otra fuente. |

## 2. Arquitectura

```
src/                        (en 5f21165 → ahora)
  main.rs       685 → 229    la app (Slack, Global), atajos globales, subscription
  espacio.rs      — → 2358   estado de un workspace (Espacio, Msg) y update; casi todas las pruebas
  vista.rs      913 → 1541   todas las vistas (view)
  slack.rs      481 → 1324   cliente HTTP de la Web API y tipos de datos
  formato.rs    330 → 847    mrkdwn y rich text → bloques y segmentos; emoji
  estilo.rs     221 → 250    colores, fuentes y estilos de widgets
  demo.rs       188 → 303    datos de ejemplo
  rtm.rs        127 → 196    websocket de tiempo real como Subscription
  saltar.rs       — → 104    coincidencias de Ctrl+K
  sugerencias.rs  — → 247    selector de emoji y autocompletado del compositor
```

### 2.1 El ciclo de iced (estilo Elm)

`main.rs:16-24` arma la app:

```rust
iced::application(Slack::new, Slack::update, Slack::view)
    .title(Slack::titulo)
    .subscription(Slack::subscription)
    .theme(|_: &Slack| estilo::tema())
    .default_font(estilo::LATO)
```

- **Estado**: `Slack { espacios: Vec<Espacio>, activo }` (`main.rs:51`). Cada
  `Espacio` (`main.rs:67`) es un workspace completo: cliente HTTP, sesión, equipo,
  canales, contadores, usuarios, avatares ya decodificados como `image::Handle`,
  mensajes por canal, canal seleccionado, editor, hilo abierto, mensaje bajo el mouse,
  filtro, vista, secciones plegadas, si el tiempo real está conectado y si es el
  visible.
- **Mensajes** en dos niveles:
  - `Global` (`main.rs:57`): `Espacio(índice, Msg)`, `Activar(i)`, `Tecla`, `Captura`.
  - `Msg` (`main.rs:94`): todo lo de un workspace (respuestas de la API, clics,
    edición, eventos RTM).

  La vista de un workspace produce `Msg` y se envuelve con
  `.map(move |m| Global::Espacio(i, m))` (`vista.rs:54-55`); las tareas igual
  (`main.rs:184`). Así cada respuesta HTTP vuelve al workspace que la pidió aunque
  mientras tanto cambies de workspace.
- **update**: `Slack::update` (`main.rs:180`) enruta; `Espacio::update`
  (`main.rs:291`) hace el trabajo. Nunca bloquea: lo que va a la red se devuelve como
  `Task::perform(futuro, Msg::…)` y su resultado regresa como otro mensaje.
- **view**: función pura del estado (`vista.rs:50`). Solo se pinta el workspace
  activo.
- **subscription** (`main.rs:168`): un websocket RTM por cada workspace con cliente,
  más el teclado global. iced identifica cada suscripción por su valor
  (`Credenciales` deriva `Hash`) y la mantiene viva mientras siga devolviéndose; no se
  reinicia en cada `update`.

### 2.2 Autenticación

Se usan los mismos tokens de sesión que app.slack.com, no un token de app:

- `SLACK_TOKEN`: uno o varios `xoxc-…`, separados por comas o espacios. Se quitan
  duplicados y se rechaza cualquiera que no empiece con `xoxc-`
  (`slack.rs:25-51`).
- `SLACK_COOKIE`: el valor de la cookie `d` (`xoxd-…`), con o sin el prefijo `d=`
  (`slack.rs:36`). Una sola para todos los workspaces del navegador.

La app no lee `.env`: las variables tienen que estar exportadas en el entorno.

Cada llamada (`Cliente::llamar`, `slack.rs:262`) es un `POST` a
`https://slack.com/api/<método>` con el formulario `token=xoxc-…&…` y el encabezado
`Cookie: d=xoxd-…`. Si la respuesta no trae `"ok": true`, el `error` de Slack se
vuelve el texto del error. User-Agent: `slack-rs/0.1` (`slack.rs:252`).

Las imágenes (avatares, ícono del equipo) se bajan con `GET` simple y **sin** la
cookie, porque el CDN no la pide (`slack.rs:458`).

### 2.3 Endpoints de la Web API

Todos en `src/slack.rs`. "Línea" es donde se nombra el método.

| Método | Función | Línea | Cuándo se llama |
|---|---|---|---|
| `auth.test` | `Cliente::sesion` | `slack.rs:288` | Al arrancar, por workspace (`main.rs:233`). Da tu `user_id` y el nombre del equipo. |
| `team.info` | `Cliente::equipo` | `slack.rs:296` | Al arrancar (`main.rs:234`). Nombre e ícono del equipo. Si falla, se ignora. |
| `users.conversations` | `Cliente::canales` | `slack.rs:318` | Al arrancar (`main.rs:235`). Tipos `public_channel,private_channel,mpim,im`, sin archivados, 200 por página, sigue el cursor hasta el final. |
| `client.counts` | `Cliente::contadores` | `slack.rs:358` | Al arrancar (`main.rs:236`). No documentado; es el del cliente web. No leídos y menciones por canal. Si falla, se ignora. |
| `users.info` | `Cliente::usuarios` | `slack.rs:418` | Cada vez que aparece un ID que no se ha pedido: tú, el otro lado de cada DM, autores, quienes respondieron hilos y mencionados (`main.rs:575`, `main.rs:660`). Hasta 8 en paralelo; los que fallan se omiten. Cada ID se pide una sola vez por ejecución (`usuarios_pedidos`). |
| `conversations.history` | `Cliente::historial` | `slack.rs:381` | Al abrir un canal (`main.rs:331`), y después de enviar si no hay tiempo real. `limit=100`. |
| `conversations.replies` | `Cliente::respuestas` | `slack.rs:396` | Al abrir un hilo (`main.rs:349`), y después de responder si no hay tiempo real. `limit=200`. |
| `conversations.mark` | `Cliente::marcar_leido` | `slack.rs:404` | Ver "Marcar como leído" arriba (`main.rs:549`). |
| `chat.postMessage` | `Cliente::enviar` | `slack.rs:431` | Al enviar; con `thread_ts` si es en hilo. |
| `reactions.add` / `reactions.remove` | `Cliente::reaccionar` | `slack.rs:444`, `slack.rs:446` | Al alternar una reacción. |
| `rtm.connect` | `Cliente::url_tiempo_real` | `slack.rs:479` | Cada vez que se (re)conecta el websocket (`rtm.rs:59`). |
| *(GET al CDN)* | `Cliente::imagen` | `slack.rs:458` | Ícono del equipo (`main.rs:302`) y avatar de 72 px de cada usuario resuelto (`main.rs:318`). |

Nada más: no hay `client.boot`, `users.list`, `emoji.list`, `search.*`, presencia ni
telemetría.

**Agregados después de `5f21165`** (todos en `src/slack.rs`, probados con wiremock):

| Método | Función | Cuándo se llama |
|---|---|---|
| `search.messages` | `Cliente::buscar` | Intro en el buscador de arriba. |
| `users.channelSections.list` | `Cliente::secciones` | Al arrancar. No documentado; si falla, quedan las dos secciones fijas. |
| `users.prefs.get` | `Cliente::silenciados` | Al arrancar: `muted_channels` y `all_notifications_prefs`. |
| `emoji.list` | `Cliente::emoji` | Al arrancar; las imágenes se bajan solo de los emoji que aparecen. |
| `chat.update` / `chat.delete` | `Cliente::editar`, `Cliente::borrar` | Al guardar una edición o confirmar un borrado. |
| `conversations.history` con `latest` / `oldest` | `Cliente::historial`, `Cliente::posteriores` | Al subir en el historial / al reconectar. |
| *(GET a files.slack.com con la sesión)* | `Cliente::archivo` | Miniaturas de imágenes. Se niega a mandar la sesión a otro host. |

### 2.4 Tiempo real (`src/rtm.rs`)

- `suscripcion(cred)` (`rtm.rs:40`) envuelve `escuchar` en
  `Subscription::run_with`. `escuchar` (`rtm.rs:44`) es un `iced::stream::channel`
  con un ciclo infinito:
  1. `sesion()` (`rtm.rs:58`): pide `rtm.connect`, abre el websocket que devuelve con
     `tokio-tungstenite` (TLS con rustls) y le pone la cookie `d` en el encabezado
     (`rtm.rs:61-65`).
  2. Dentro, un `tokio::select!` entre un latido cada 30 s que manda
     `{"type":"ping","id":n}` (`rtm.rs:71-79`) y la lectura de mensajes.
  3. Cada texto JSON pasa por `interpretar` (`rtm.rs:98`):
     - `hello` → `Conectado` (la barra superior muestra "● En vivo").
     - `message` → `Nuevo`; con `subtype` `message_changed` → `Editado` (reemplaza el
       mensaje entero); `message_deleted` → `Borrado`.
     - `reaction_added` / `reaction_removed` → `Reaccion`.
     - Lo demás se descarta.
  4. Si el servidor cierra, llega `Close`, o hay error, se emite
     `Desconectado(motivo)` (se imprime en stderr y la barra muestra "Sin conexión en
     vivo"), se esperan **5 s fijos** (`rtm.rs:53`) y se vuelve al paso 1.
- Limitaciones: no hay backoff; no se revisa que lleguen los `pong`, así que una
  conexión medio muerta solo se detecta cuando falla una escritura; al reconectar no
  se vuelve a pedir historial ni `client.counts`, así que lo que llegó durante el
  corte no aparece hasta reabrir el canal. Cada suscripción crea su propio
  `reqwest::Client` (`rtm.rs:45`).
- Los mensajes nuevos solo se agregan a canales que ya están en memoria
  (`main.rs:491`); los demás se piden completos al abrirlos.

### 2.5 Varios workspaces

- Un `Espacio` por token, en el orden de `SLACK_TOKEN` (`main.rs:132-144`). Cada uno
  tiene su `Cliente`, sus cuatro llamadas de arranque en paralelo y su websocket.
- Solo uno es `visible`. Cambiar con clic en el rail (`Global::Activar`) o
  Ctrl + número (`main.rs:217`). Al volver a uno, su canal abierto se marca leído si
  tenía pendientes (`main.rs:256`).
- La insignia del rail suma las menciones de todos los canales del workspace
  (`main.rs:273`).

### 2.6 mrkdwn (`src/formato.rs`)

Dos pasadas, sin expresiones regulares ni árbol:

1. `segmentos` (`formato.rs:34`) corta el texto en `<…>`. Cada referencia se resuelve
   en `referencia` (`formato.rs:100`):
   - `<@U…>` → `@Nombre visible` (del mapa de usuarios; si no está, la etiqueta o el
     ID).
   - `<#C…|nombre>` → `#nombre`.
   - `<!here>`, `<!channel>`, `<!subteam^…|@grupo>` → texto de mención.
   - Lo demás es enlace: `<url|texto>` o `<url>`.
2. El texto entre referencias pasa por `marcas` (`formato.rs:139`):
   - Primero decodifica `&lt;` `&gt;` `&amp;`.
   - ```` ```bloque``` ```` → código.
   - `*`, `_`, `~`, `` ` `` abren estilo solo si antes no hay letra o número, adentro
     no está vacío, no empieza ni termina con espacio, no cruza líneas, y después no
     sigue letra o número. Así `x*y*z` y `2*3` quedan como texto.
   - Fuera del código, `con_emoji` (`formato.rs:206`) cambia `:código:` por el emoji
     Unicode y descarta los tonos de piel `::skin-tone-N:`.
3. Al pintar, `partir_emoji` (`formato.rs:71`) separa los tramos de emoji para
   dibujarlos con Noto Color Emoji (si no, el sistema puede tomarlos de una fuente en
   blanco y negro). `vista.rs:580` convierte cada segmento en un `span` de
   `rich_text`.

`menciones` (`formato.rs:51`) extrae los IDs `<@U…>` de un texto para pedirlos a
`users.info` antes de pintar.

Pruebas: 5 en `formato.rs:236-330` en ese commit. Después se agregaron pruebas de estado (`espacio.rs`), HTTP con un servidor falso (`slack.rs`, wiremock), de `rtm::interpretar` y de las funciones puras de `saltar.rs` y `sugerencias.rs`: 99 en total, todas pasan.

### 2.7 Tema (`src/estilo.rs`)

- Colores como constantes (`estilo.rs:6-35`): barra `#350d36`, lateral `#3f0e40`,
  activo `#1164a3`, texto `#1d1c1d`, rojo de menciones `#e01e5a`, verde de enviar
  `#007a5a`, seis colores para avatares sin foto.
- Fuentes por nombre: `Lato` en normal, negrita, negra y cursiva; `Noto Color Emoji`
  (`estilo.rs:37-50`).
- `tema()` (`estilo.rs:52`) registra un `Theme::custom("Aubergine")` con esa
  paleta.
- Lo demás son funciones de estilo por widget que se pasan a `.style(…)` en
  `vista.rs`: `item_lateral`, `boton_rail`, `boton_plano`, `reaccion`, `enviar`,
  `buscador`, `editor`, `scroll_lateral`, `anillo`, y ayudantes `fondo`,
  `redondeado`, `tarjeta`.

### 2.8 Modo demo y modo captura

- `SLACK_DEMO=1` (`main.rs:129`): no lee credenciales ni toca la red. `demo.rs` arma
  dos workspaces en memoria: *Acme MX* (5 canales, 2 DMs, 1 grupo, `#desarrollo` con
  6 mensajes y un hilo de 3 respuestas abierto, no leídos en `#despliegues` y en un
  DM) y *Proyecto Luna* (3 canales, 3 menciones). Sin `Cliente` no hay websocket, así
  que la barra dice "Sin conexión en vivo".
- `SLACK_CAPTURA=ruta.png` **solo funciona junto con `SLACK_DEMO`**: la tarea solo se
  crea en esa rama (`main.rs:130`). Espera 2 s fijos, pide la última ventana, toma un
  screenshot, lo guarda como PNG con el crate `image` y sale (`main.rs:202-212`,
  `main.rs:649`).

## 3. Flujo de datos

```
ARRANQUE (por cada workspace)

 SLACK_TOKEN, SLACK_COOKIE ──► Credenciales::desde_entorno (slack.rs:25)
                                      │
               Espacio::new (main.rs:230): 4 Task::perform en paralelo
     ┌───────────────┬────────────────┼────────────────────┐
     ▼               ▼                ▼                    ▼
 auth.test       team.info    users.conversations     client.counts
     │               │         (todas las páginas)         │
     ▼               ▼                ▼                    ▼
 Msg::Sesion    Msg::Equipo      Msg::Canales        Msg::Contadores
     │               │                │                    └─► negritas e insignias
     │               ▼                │
     │         GET ícono (CDN)        │
     ▼                                ▼
 users.info (yo)          users.info (otro lado de cada DM)
     └───────────────┬────────────────┘
                     ▼
              Msg::Usuarios ──► GET avatar 72 px por usuario ──► Msg::Avatar

 subscription() ──► rtm.connect ──► wss:// + cookie d ──► "hello" ──► "● En vivo"


CLIC EN UN CANAL

 Msg::Seleccionar(id) ──► conversations.history (100) ──► Msg::Historial
                                                            ├──► users.info de IDs nuevos
                                                            └──► conversations.mark


EVENTO EN TIEMPO REAL

 websocket ──► interpretar (rtm.rs:98) ──► rtm::Evento
           ──► Global::Espacio(i, Msg::Rtm) ──► Espacio::evento_rtm (main.rs:425)
                 ├─ Nuevo:            agrega al canal / hilo, o sube no leídos y menciones
                 ├─ Editado / Borrado: reemplaza / quita en canal y en hilo
                 └─ Reaccion:         aplicar_reaccion (idempotente)


PINTAR (después de cualquier mensaje)

 update() muta el estado ──► iced llama view() ──► árbol de widgets
                         ──► layout ──► wgpu (Vulkan/GL) dibuja el cuadro
```

## 4. Por qué es más rápido y más ligero

### 4.1 Lo medido

Máquina: Intel Core i7-1185G7 (8 hilos), 15 GiB de RAM, gráficos Intel Xe (Mesa),
Debian 12 con X.Org 21.1.7, rustc 1.94.0. Binario de `cargo build --release` con el
perfil por defecto (el `Cargo.toml` no tiene `[profile.release]`: sin LTO, sin
`strip`, sin `opt-level` especial).

Todas las corridas son en **modo demo**: sin red, sin websocket, con pocos mensajes y
sin avatares descargados. Con un workspace real los números de memoria van a ser
mayores; eso no se midió.

| Qué | Resultado | Cómo |
|---|---|---|
| Compilación release en frío | **6 min 25 s** de reloj, 2 562 s de CPU de usuario (≈ 6.8 núcleos en promedio), 409 crates compilados. Al principio corrió a la par de un `cargo test`, así que puede estar un poco inflado | `time cargo build --release` con `CARGO_TARGET_DIR` vacío (dependencias ya descargadas) |
| Binario | **40.1 MB** (40 134 632 B) sin quitar símbolos; **32.1 MB** con `strip`; 12.0 MB comprimido con `gzip -9` | `ls -l`, `strip`, `gzip` |
| Bibliotecas enlazadas | Solo `libc`, `libm`, `libgcc_s`. X11, Wayland, Vulkan y Mesa se cargan en tiempo de ejecución | `ldd`, `/proc/<pid>/maps` |
| Dependencias | 10 directas; **395** crates distintos en el árbol normal (409 contando dependencias de compilación); 573 paquetes en `Cargo.lock` (incluye los de otras plataformas) | `cargo tree --prefix none -e normal \| sort -u` |
| Código propio | **2 945** líneas de Rust en 7 archivos; 2 676 sin líneas en blanco ni comentarios; 95 son pruebas | `wc -l`, `grep` |
| Ventana visible | **124–141 ms** desde `exec` (5 corridas: 141, 126, 124, 125, 133) | `xdotool search --onlyvisible --pid` cada ~5 ms |
| Modo captura completo | **2.074–2.100 s** de reloj, de los cuales 2.000 s son la espera fija de `main.rs:653`. Arrancar, pintar, capturar, guardar el PNG y salir cabe en **≤ 100 ms** | `SLACK_DEMO=1 SLACK_CAPTURA=…`, 5 corridas |
| CPU total de una corrida de captura | 0.13–0.15 s (usuario + sistema) | `getrusage(RUSAGE_CHILDREN)` |
| Memoria en reposo (5 s después de abrir) | RSS **≈ 101 MiB** (103 064–103 440 kB), de los cuales **≈ 21 MiB son anónimos** (montículo propio + buffers) y ≈ 79 MiB son archivos mapeados (controladores Mesa/Vulkan, bibliotecas, fuentes) | `/proc/<pid>/status`: `VmRSS`, `RssAnon`, `RssFile` |
| PSS (memoria compartida repartida) | **≈ 73 MiB** (74 497 / 74 934 kB) | `/proc/<pid>/smaps_rollup` |
| Pico de memoria | 105.3–106.6 MiB en modo captura (incluye el buffer RGBA de la captura) | `ru_maxrss` |
| CPU en reposo | **0.00–0.01 s en 20 s** | `/proc/<pid>/stat`, entre el segundo 3 y el 23 |
| Procesos / hilos | 1 proceso, 26–27 hilos | `/proc/<pid>/task` |

Nota sobre "ventana visible": es cuando X11 reporta la ventana mapeada, no
exactamente el primer cuadro pintado. El modo captura acota lo otro: a los 2 s el
cuadro ya está y todo el trabajo extra cabe en menos de 100 ms.

Visto en `/proc/<pid>/maps`: wgpu carga `libvulkan_intel` y también Mesa EGL
(`iris_dri`) al enumerar adaptadores. Qué backend termina usando no se verificó.

### 4.2 Por qué (estructural)

Cada punto dice si viene del código, de lo medido arriba, o si es inferido.

1. **No hay motor web.** *(código + medido)* La interfaz es un árbol de widgets de
   iced construido en Rust (`vista.rs`); iced hace el layout y lo dibuja con wgpu
   sobre Vulkan/GL (con `tiny-skia` por software de respaldo; ambos están en el
   árbol de dependencias). No hay DOM, CSS, motor de JavaScript ni recolector de
   basura. Por eso lo propio de la app en memoria son ~21 MiB anónimos.
   *(inferido)* El cliente oficial es Electron (Chromium + Node) con una app React;
   carga, analiza y compila su bundle de JS en cada arranque y mantiene un heap de V8.
2. **Un solo proceso.** *(medido)* Un proceso con 26–27 hilos.
   *(inferido, conocido de Chromium/Electron)* Electron reparte el trabajo en
   proceso principal, renderer, GPU y utilitarios, cada uno con su propio costo base.
3. **Nativo y compilado de antemano.** *(medido)* La ventana aparece en ~125 ms y no
   hay fase de "cargando la app": lo que se ve en el primer cuadro ya es la interfaz.
4. **En reposo no hace nada.** *(código + medido)* iced solo vuelve a llamar `view` y
   a pintar cuando llega un mensaje (entrada, respuesta de red, evento RTM). El único
   temporizador es el ping de 30 s del websocket (`rtm.rs:71`), que en demo ni
   existe. Medido: 0.00–0.01 s de CPU en 20 s.
5. **Pide solo lo que pinta.** *(código)*
   - Al arrancar, 4 llamadas por workspace, en paralelo (`main.rs:232-237`).
   - Historial solo del canal que abres, 100 mensajes (`slack.rs:382`).
   - Usuarios uno por uno y solo los que aparecen en pantalla, sin repetir
     (`main.rs:575-587`), en vez de bajar el directorio completo.
   - Avatares de 72 px, solo de esos usuarios.
   - 12 métodos de la API en total (§2.3). Sin telemetría, sin `client.boot`, sin
     lista de emoji, sin presencia.
6. **Datos tipados y recortados.** *(código)* Los mensajes se deserializan con serde
   a structs con solo los campos que se usan (`slack.rs:98`); el resto del JSON se
   descarta. Matiz honesto: `llamar` primero arma un `serde_json::Value` completo de
   la respuesta y luego lo convierte (`slack.rs:269`, `slack.rs:284`), así que el
   JSON entero sí vive en memoria un momento.
7. **Menos ida y vuelta percibida.** *(código)* Reacciones optimistas
   (`main.rs:532`); al enviar con tiempo real no se vuelve a pedir el historial
   (`main.rs:380-391`).

### 4.3 Lo que juega en contra

- **El binario no es chico**: 40 MB (32 MB sin símbolos). Lo pesan wgpu/naga, el
  stack TLS (rustls + aws-lc) y cosmic-text. No se ha probado LTO, `strip`,
  `opt-level = "z"` ni `panic = "abort"`.
- **79 MiB del RSS son archivos mapeados**, sobre todo controladores de GPU. Se
  comparten con otros procesos que usen Mesa/Vulkan (por eso el PSS baja a ~73 MiB),
  pero cuentan en el RSS que muestra un monitor de sistema.
- **Sin caché en disco**: cada arranque vuelve a bajar canales, usuarios y avatares,
  y cada vez que abres un canal se vuelve a pedir su historial.
- **`users.info` uno por uno**: en un canal con muchos autores nuevos son muchas
  llamadas (8 en paralelo), y puede pegar con el límite de tasa de Slack. No hay
  manejo especial de `ratelimited`: los que fallan simplemente no se resuelven.
- **Ningún número de esta sección es con un workspace real**: no se usaron
  credenciales.

## 5. Comparación con Slack web

Slack web: `app.slack.com` en Chrome 150, en la misma máquina, el 2026-10-21, medido
por CDP sobre un workspace real (unas 25 conversaciones en la lateral, 40 miembros en
#general) con el caché del service worker caliente. El detalle y el método están en
[`slack-servidor.md`](slack-servidor.md) §2, §3 y §6. La app de escritorio
(Electron) no se midió.

| Métrica | slack-rs (demo) | Slack web (workspace real) |
|---|---|---|
| Arranque | ventana visible en ~125 ms; el cuadro completo cabe en ≤ 100 ms más | first paint 888 ms, first contentful paint **2,388 ms**, `load` 3,979 ms |
| Memoria del proceso que pinta | RSS ~101 MiB, **PSS ~73 MiB** (anónima ~21 MiB) | renderer RSS 617 MB, **PSS 488 MB** (heap de JS 166 MB usado / 214 MB reservado) |
| Memoria adicional | — (1 proceso) | proceso GPU PSS 161 MB y proceso principal PSS 261 MB, compartidos con el resto de Chrome |
| CPU en reposo | 0.00–0.01 s en 20 s | **≈ 2.9 s por minuto** (renderer 1.89 + GPU 0.64 + principal 0.33) |
| Tráfico en reposo | ping cada 30 s | ping/pong cada 10 s, sin HTTP en 60 s |
| Peticiones al arrancar | 4 de API + `users.info` por ID + avatares, por workspace | **209**: ~45 métodos de API, 25 de `edgeapi`, 44 scripts, 69 imágenes, 15 de telemetría; 731 KB por la red |
| Código que carga | binario de 40 MB (32 MB sin símbolos), ya compilado | **45.6 MB de JS** ya descomprimido, que hay que interpretar y compilar en cada arranque |
| Abrir un canal | `conversations.history` (+ `users.info` de autores nuevos) | 12–15 peticiones, 11–13 KB |
| Disco | nada | 158 MB (157 MB del caché del service worker) |
| Procesos | 1 | 1 renderer para la pestaña más los de Chrome (20 en total con extensiones) |

Cómo leerla, con honestidad:

- **No es una comparación pareja.** El clon está en modo demo, con pocos mensajes y sin
  red. Con un workspace real su memoria va a crecer por los mensajes, usuarios y
  avatares en memoria (*inferido*: pocos MB, porque guarda structs recortados y
  avatares de 72 px, no el JSON ni el DOM). Aun así la brecha es de órdenes de
  magnitud en CPU en reposo y de ~6–7× en la memoria del proceso que pinta.
- **El oficial hace mucho más**: presencia, borradores sincronizados, búsqueda,
  huddles, IA, workflows. Parte del costo es eso, y parte no: experimentos (42 KB por
  arranque), telemetría y ofertas. Ver [`funciones.md`](funciones.md).
- La CPU en reposo del oficial se midió con una sesión CDP conectada. *(inferido)*
  Agrega poco, porque en esos 60 s solo hubo pings.

En resumen, el clon es más rápido por cuatro razones:

1. No interpreta 45 MB de JS ni mantiene un heap de 166 MB.
2. No tiene DOM: el oficial tiene 5,543 nodos y 2,815 listeners.
3. En reposo no pinta ni corre nada.
4. Pide 4 cosas al arrancar, en vez de ~70 llamadas a la API y al caché de borde
   (más 139 de scripts, imágenes y telemetría).

## 6. Limitaciones conocidas y riesgos

### Tokens de sesión no oficiales

- `xoxc` + cookie `d` **son tu sesión del navegador**: quien los tenga puede leer y
  escribir como tú en esos workspaces. No pasan por una app aprobada ni tienen
  permisos acotados.
- Viven en variables de entorno: cualquier proceso de tu usuario puede leerlas en
  `/proc/<pid>/environ`, y es fácil que acaben en el historial del shell. `.env` y
  `.env.*` están en `.gitignore`, pero la app no los lee: hay que exportarlas.
- Es una vía no oficial: Slack puede cambiarla o cortarla sin aviso, y está en zona
  gris con sus términos. `client.counts` no está documentado; `rtm.connect` es la API
  vieja de tiempo real. Cualquiera de los dos puede desaparecer.
- El User-Agent `slack-rs/0.1` (`slack.rs:252`) identifica las peticiones como de un
  cliente que no es el oficial. *(inferido)* Un administrador o Slack mismo lo puede
  ver en los registros de acceso.
- *(no verificado)* Cerrar sesión en el navegador probablemente invalida el token y
  la cookie, y la app empezará a fallar con `invalid_auth`.

### Funcionales

Ver la tabla de §1. Las más notorias en uso diario: sin búsqueda de mensajes, sin
imágenes, sin emoji propios, historial de 100 mensajes, sin notificaciones, y huecos
después de una reconexión del websocket.
