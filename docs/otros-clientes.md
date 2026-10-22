# Otros clientes de Slack y qué aprender de sludge

Relevado el 2026-10-22 para decidir cómo hacer las **notificaciones de escritorio** y
un **caché local**. Ninguna de las dos está hecha todavía: las notificaciones son la
Fase 7 del plan de la sesión que hace el clon visual, y las reglas ya están en
`src/avisos.rs` (falta mandarlas al sistema). Este documento junta lo investigado
para retomarlo cuando esa sesión termine sus fases.

## 1. Qué hay en GitHub

No apareció ningún cliente hecho con **iced** (ni egui), ni ninguno que busque ser un
clon visual del oficial con varios workspaces en el rail.

| Proyecto | Lenguaje / UI | Autenticación | Notas |
|---|---|---|---|
| [sludge](https://github.com/emtek/sludge) | Rust, GTK4 + libadwaita, solo Linux | xoxc/xoxd + RTM | El más parecido. v0.2.2 del 2026-04-08, 55 descargas en crates.io. GPL-3.0. Ver §2. |
| [Make Slack Great Again](https://github.com/punarinta/make-slack-great-again) | C++ / Qt6, multiplataforma | — | Teams y correo experimentales. |
| [slk](https://github.com/gammons/slk) | Go, TUI (bubbletea) | — | Multi-workspace; dice ~60 MB de RSS. |
| [strack](https://github.com/waf/strack) | Rust + GTK | — | Su README dice que no funciona. Abandonado. |
| [wey](https://github.com/yue/wey) | Node.js + Yue | — | Archivado; sigue en el fork lounge-lizard. |

Con la misma autenticación pero sin interfaz gráfica: `slackcli`, `slackctl` (CLI),
`slack-token-extractor` y `slack-mcp-server` (para agentes de IA), y el crate
`slacko`, que tiene un modo "stealth" con xoxc + xoxd y es el que usa sludge.

## 2. sludge por dentro

Leído del crate publicado en crates.io; no se compiló ni se ejecutó. Las referencias van
como `archivo:línea` sobre sludge 0.2.2. **Es GPL-3.0**: se pueden tomar ideas, pero
copiar código obligaría a publicar slack-rs bajo GPL.

### Interfaz

App GNOME estándar, nada parecido a Slack: `HeaderBar` con logo, título y el avatar
propio con punto de presencia (`src/ui/app.rs:92`); debajo, lateral de canales |
mensajes + compositor, y panel de hilo. Un solo juego de credenciales, así que **un
solo workspace**.

Lo distinto está en la integración con el escritorio:

- **Proveedor de búsqueda de GNOME Shell** (`src/search_provider.rs`, interfaz D-Bus
  `org.gnome.Shell.SearchProvider2`, declarado en
  `assets/dev.sludge.app.search-provider.ini`): buscar mensajes desde Actividades sin
  abrir la app. `@texto` busca personas, `#texto` canales y lo demás va al índice de
  texto completo. Al elegir un resultado lanza `sludge` con argumentos y la instancia
  única de GTK lo abre.
- **"Notify when online"** (`src/ui/channel_sidebar.rs:633`): lista de personas
  vigiladas; avisa cuando su presencia pasa a `active`.
- Botón de **Google Meet** (`src/ui/app.rs:1055`).

### Notificaciones

`src/ui/app.rs:2257`. Por cada mensaje de un canal que no es el abierto:

1. Arma el título `"{autor} in #{canal}"` (para DMs, el nombre de la persona) y el
   cuerpo con `format_message_plain` (`src/slack/helpers.rs:75`), que resuelve
   `<@U…>`, `<!subteam^…>`, `<#C…|nombre>`, enlaces y `:emoji:`.
2. Lanza `canberra-gtk-play -i message-new-instant` para el sonido.
3. Lanza `notify-send --app-name=Sludge --expire-time=15000 --action=default=Open
   --wait titulo cuerpo` y lee su stdout con un tope de 30 s (después mata el
   proceso).
4. Si stdout dice `default` (clic en la notificación), manda `canal:ts[:hilo_ts]`
   por un `mpsc` al hilo de GTK. La acción `app.navigate` (`src/ui/app.rs:1757`) abre
   el canal, se desplaza al mensaje (o abre el hilo) y le pone la clase CSS
   `notification-highlight` un momento.

Lo que **no** hay que copiar:

- **No filtra**: avisa de todo mensaje de cualquier canal no abierto, sin mirar
  silenciados, menciones, DND ni si la ventana tiene el foco. Nuestro
  `avisos::debe_avisar` ya aplica las reglas del oficial (DMs y menciones; nada si es
  propio, está a la vista, silenciado sin mención o pausado).
- **Panic con UTF-8**: recorta con `&plain_text[..200]` (bytes). Un acento o emoji
  en el byte 200 tumba la app. `avisos.rs` recorta por caracteres y tiene test.
- Uno o dos procesos por aviso, y cada uno queda vivo hasta 30 s esperando el clic.

### Caché local

`src/db.rs`. **SurrealDB** embebido (motor SurrealKV) en
`~/.local/share/sludge/db`, namespace `slack`, base `main`. Esquema
(`src/db.rs:68`):

- `cache`: blobs JSON por clave: canales, usuarios, emoji propios, último canal,
  estados recientes y `messages_{canal}` con los **últimos 100 mensajes** del canal.
- `message`: una fila por mensaje (`channel`, `ts`, `thread_ts`, `user`, `text`,
  `data` = JSON completo), índice único `(channel, ts)` y **índice de texto completo
  BM25** con analizador `snowball(english)`.
- `channel_meta`: `oldest_ts`, `newest_ts` y `backfill_checked_at` por canal.
- `credentials`, `settings`: xoxc y xoxd **en texto plano** (`src/db.rs:119`).

Cómo lo usa:

- **Arranque** (`src/ui/app.rs:1305`): carga canales, usuarios, último canal,
  actividad y emoji del caché antes de tocar la red, así la lateral aparece al
  instante.
- **Abrir canal** (`src/ui/app.rs:1675`): pinta lo cacheado y después lo reemplaza
  con `conversation_history(canal, 25)`.
- **Mensaje entrante**: `append_message` (`src/db.rs:420`) lee el blob entero, agrega,
  recorta a 100, lo reescribe y **reindexa los 100** (`save_messages` llama a
  `index_messages`). Caro para cada mensaje.
- **Backfill** (`src/ui/app.rs:2858`): en segundo plano baja **30 días** (el
  comentario en la línea 1586 dice "1 año", pero el código usa 30) de todos los
  canales, de a 200 por página, con backoff exponencial hasta 5 min; repasa cada 3
  días.
- **Búsqueda local** (`src/db.rs:665`): `text @0@ $query`, 200 resultados, ordenados
  por `score` BM25 combinado con recencia `exp(-días/10)` (vida media ~7 días).

## 3. Qué haría en slack-rs

### Notificaciones (Fase 7)

1. Decidir con `avisos::debe_avisar` (ya hecho). Falta llenar el `Contexto` desde el
   evento del RTM: `a_la_vista` necesita saber si la ventana tiene el foco (evento de
   ventana de iced), `silenciado` sale de las preferencias que ya se leen para la
   lateral, `pausado` de DND (`dnd.info` / evento `dnd_updated`).
2. Mandar por **D-Bus** a `org.freedesktop.Notifications` con el crate `notify-rust`
   (o `zbus` directo), no lanzando `notify-send`: sin procesos, y el clic llega como
   señal `ActionInvoked`.
3. El clic se convierte en un mensaje de iced que lleva la clave del workspace:
   `Global::Espacio(clave, …)` para abrir canal y mensaje o hilo, aunque el aviso sea
   de un workspace que no es el visible. Desde la Fase 3, `Espacio::clave` es un
   `u64` estable y `Slack::al_activo(msg)` manda al visible.
4. Texto plano del cuerpo: reutilizar `formato::texto_plano` (`formato.rs:266`, hoy
   privado) o `formato::segmentos`, que ya resuelven menciones y emoji, en lugar de
   escribir otro conversor como `format_message_plain`.
5. Sonido opcional: `canberra-gtk-play` o el hint `sound-name` de la notificación.

### Caché

- **SQLite** (`rusqlite` con `bundled`) en vez de SurrealDB: mucho más chico, sin
  runtime propio. Una fila por mensaje para no reescribir blobs.
- Primero lo que hace rápido el arranque: canales, usuarios, emoji propios y los
  últimos N mensajes por canal, por workspace. Pintar del caché y luego refrescar,
  como sludge.
- Búsqueda local con FTS5 y backfill solo si se quiere buscar sin red; hoy ya se usa
  `search.messages` de Slack.
- No duplicar los tokens en la base del caché: hoy viven en
  `~/.config/slack-rs/credenciales.json` (`login.rs:31`), también en texto plano;
  si se mueven, mejor al llavero del sistema (Secret Service).
- Coherente con [`funciones.md`](funciones.md): "si hace falta caché, conviene uno
  chico y propio" (ver [`slack-servidor.md`](slack-servidor.md) §5).

### Ideas sueltas que valen la pena

- Proveedor de búsqueda de GNOME Shell sobre el caché local.
- "Avisarme cuando se conecte" (presencia vigilada).
- Resaltar un momento el mensaje al llegar desde un aviso.
