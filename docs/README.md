# Documentación

| Documento | Qué tiene |
|---|---|
| [`slack-ui.md`](slack-ui.md) | La interfaz del Slack oficial, zona por zona: medidas, colores, tipografía, cada menú y vista, las preferencias y todos los atajos. Es la referencia visual del clon. |
| [`slack-servidor.md`](slack-servidor.md) | Lo que hace el cliente oficial contra el servidor: la secuencia de arranque y los métodos de la API, el caché de borde, el protocolo del websocket, el almacenamiento local, cuánto cuesta en memoria y CPU, y los comportamientos de UX que dependen del servidor. |
| [`funciones.md`](funciones.md) | Inventario de funciones: cada una con su estado en el clon (✅ 🟡 ❌) y una prioridad sugerida. |
| [`clon-rust.md`](clon-rust.md) | Cómo está hecho slack-rs: qué hace hoy, la arquitectura módulo por módulo, el flujo de datos, por qué es más rápido (medido contra el oficial) y sus límites. |
| [`otros-clientes.md`](otros-clientes.md) | Otros clientes de Slack en GitHub y cómo hace sludge (Rust/GTK) las notificaciones y el caché local, con lo que conviene copiar y lo que no. Base para la fase de notificaciones. |
| [`datos/slack-css-tokens.json`](datos/slack-css-tokens.json) | Los ~480 tokens de color `--dt_color-*` del cliente oficial, tal como estaban en el tema relevado. |

Las capturas del relevamiento van en `docs/capturas/`, que está en `.gitignore`
porque muestran datos reales del workspace (nombres, fotos, correos, mensajes).
Solo existen en la máquina donde se hizo el relevamiento.

Todo se relevó el 2026-10-21 con Chrome 150 contra `app.slack.com`. Slack cambia su
UI seguido y por experimentos por usuario (`experiments.getByUser`), así que algunos
menús pueden verse distintos en otra cuenta o en otra fecha.
