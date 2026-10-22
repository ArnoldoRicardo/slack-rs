# slack-rs

Cliente de escritorio de Slack en Rust con [iced](https://iced.rs): un clon visual y
funcional del oficial que usa una fracción de su memoria y CPU (ver
[`docs/clon-rust.md`](docs/clon-rust.md)).

> **Estado: detenido.** El proyecto llegó hasta donde está descrito abajo y no se sigue
> desarrollando. Se publica como referencia para quien busque cómo hablar con Slack
> desde Rust, cómo está hecho el cliente oficial por dentro o cómo armar una app de
> escritorio grande con iced.

> **Aviso.** No es un producto de Slack ni está afiliado a Salesforce. Se autentica con
> los **tokens de sesión del navegador** (lo mismo que usa app.slack.com) y llama a
> métodos internos no documentados. Es una vía no oficial: Slack puede cambiarla en
> cualquier momento y puede ir contra los términos de tu workspace. Úsalo bajo tu
> responsabilidad y nunca compartas tus tokens.

## Qué hace

- **Varios workspaces a la vez**, con su propio tema, insignias en el rail, Ctrl + 1…9 y
  Ctrl + Tab. Inicio de sesión desde la app (abre Chrome con un perfil temporal) y
  "Cerrar sesión" por workspace.
- **Lateral como la oficial**: secciones del usuario (Destacados, propias), filtro
  "Activas (30 días)" / No leídas / Menciones / Todas, orden A–Z o reciente,
  silenciados, borrador por conversación (✎), presencia en las DMs y accesos a Hilos,
  Borradores, Guardados y Directorio.
- **Canal**: encabezado con ☆, miembros, 🔔 (todos / menciones / silenciar), 🔍 y ⋮;
  pestañas Mensajes, Archivos, Fijados y marcadores; ventana de detalles.
- **Mensajes**: rich text, mrkdwn, menciones (la tuya en amarillo), enlaces cortos,
  imágenes y galerías, tarjetas de bots, reacciones con "quién reaccionó", hilos,
  "se unieron" agrupado, menú ⋮ (copiar enlace o texto, marcar no leído, guardar,
  fijar), editar y borrar los propios. Hora en 12 o 24 h.
- **Compositor**: formato (negrita, cursiva, tachado, código, listas, cita, bloque de
  código) con botones y atajos, autocompletado de `@`, `#` y `:emoji:`, selector de
  emoji, subir archivos (+ o arrastrar), programar el envío, "Enviar también a #canal"
  en hilos, borradores.
- **Tiempo real**: mensajes, ediciones, reacciones, "escribiendo…" en los dos sentidos,
  presencia, leído en otro dispositivo, reconexión con recuperación de lo perdido.
- **Notificaciones de escritorio** por D-Bus (DMs y menciones; clic abre la
  conversación) y 🌙 para pausarlas.
- **Vistas**: Actividad, Hilos, Borradores y programados, Guardados, Archivos,
  Directorio de personas y perfil con hora local.
- **Búsqueda** de mensajes, Ctrl + K para saltar, historial ← → y menú de recientes.
- **Apariencia**: el tema de cada workspace (las 20 paletas de Slack) o uno fijo, modo
  claro u oscuro (Ctrl + Shift + Y), densidad compacta, zoom (Ctrl + + / − / 0).
  Preferencias en Ctrl + , y lista de atajos en Ctrl + /.

Lo que falta está en [`docs/funciones.md`](docs/funciones.md).

## Limitaciones conocidas

- **Workspaces Enterprise con sesiones atadas al navegador.** En al menos una
  organización Enterprise, Slack invalidó la sesión (`invalid_auth`) en cuanto su token
  se usó fuera del navegador donde se inició, aun con la misma IP y las mismas cookies.
  En esos workspaces este enfoque no sirve; en workspaces normales funciona sin
  problemas. El clon detecta la sesión rechazada, deja de reintentar y ofrece volver a
  iniciar sesión.
- Sin huddles, canvas, listas, workflows ni comandos `/` de apps.
- Los emoji propios dentro del texto salen como `:nombre:` (el texto con estilo de iced
  no admite imágenes en línea).
- Linux: las notificaciones usan D-Bus y el selector de archivos, GTK 3. Los emoji se
  ven en color con una fuente de emoji instalada (`fonts-noto-color-emoji`).

## Uso

```sh
SLACK_DEMO=1 cargo run                                  # demo con datos de ejemplo, sin red
SLACK_DEMO=1 SLACK_CAPTURA=/ruta/captura.png cargo run  # guarda la ventana y sale
cargo run --release                                     # tu Slack real
```

Sin sesiones guardadas, la app muestra **Iniciar sesión en Slack**: abre Chrome (o
Chromium) con un perfil temporal, entras como siempre y la ventana se cierra sola cuando
Slack acepta la sesión. El **+** del rail suma otro workspace. Las sesiones se guardan
en `~/.config/slack-rs/credenciales.json` (permisos 0600) y las preferencias en
`~/.config/slack-rs/prefs.json`.

También se pueden pasar credenciales por variables de entorno (van antes que las
guardadas): `SLACK_TOKEN` (`xoxc-…`, varios separados por comas) y `SLACK_COOKIE` (la
cookie `d`, `xoxd-…`). Ambas dan acceso completo a tu cuenta: no las pongas en el
historial del shell ni en el repo (`.env*` está en `.gitignore`).

## Estructura

| Módulo | Qué hace |
|---|---|
| `main.rs` | La app: workspaces, rail, atajos globales, preferencias, inicio de sesión. |
| `espacio.rs` | Estado de un workspace y su `update` (la mayor parte de la lógica). |
| `vista.rs`, `estilo.rs`, `paleta.rs` | Vistas, estilos y temas. |
| `slack.rs` | Cliente de la Web API, incluidos los métodos internos del cliente web. |
| `rtm.rs` | Websocket de tiempo real como `Subscription` de iced. |
| `login.rs` | Inicio de sesión con Chrome por el protocolo de depuración. |
| `formato.rs`, `textos.rs` | mrkdwn y rich text; textos derivados de la UI. |
| `atajos.rs`, `navegacion.rs`, `lateral.rs`, `borradores.rs`, `avisos.rs`, `ajustes.rs`, `saltar.rs`, `sugerencias.rs` | Lógica pura con sus pruebas. |
| `escritorio.rs` | Notificaciones por D-Bus. |
| `demo.rs` | Datos de ejemplo. |

`cargo test` corre unas 285 pruebas: estado de cada workspace, la Web API contra un
servidor falso (wiremock, sin red) y las funciones puras. `cargo test -- --ignored`
agrega las que necesitan Chrome o un servidor de notificaciones.

## Documentación

En [`docs/`](docs/README.md): cómo está hecho el clon y por qué es más ligero, la
referencia de la UI del Slack oficial, lo que hace el cliente oficial contra el servidor
(incluidos los métodos internos de cada vista) y el inventario de funciones.

## Licencia

[MIT](LICENSE).
