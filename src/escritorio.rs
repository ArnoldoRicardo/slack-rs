//! Notificaciones de escritorio por D-Bus (`org.freedesktop.Notifications`), sin
//! lanzar procesos. Cuándo avisar lo decide `avisos`; aquí solo se muestran.

use futures::channel::oneshot;

use crate::avisos::Aviso;

/// Muestra el aviso y espera a que se cierre; `true` si le dieron clic.
pub async fn mostrar(aviso: Aviso) -> bool {
    let (listo, clic) = oneshot::channel();
    // `wait_for_action` bloquea hasta que el aviso se cierra: va en su propio hilo.
    std::thread::spawn(move || {
        let mostrada = notify_rust::Notification::new()
            .appname("Slack")
            .summary(&aviso.titulo)
            .body(&escapar(&aviso.cuerpo))
            .action("default", "Abrir")
            .show();
        let mut abrir = false;
        if let Ok(n) = mostrada {
            n.wait_for_action(|accion| abrir = accion == "default");
        }
        let _ = listo.send(abrir);
    });
    clic.await.unwrap_or(false)
}

/// El cuerpo admite un poco de marcado: `<`, `>` y `&` del mensaje van escapados.
fn escapar(texto: &str) -> String {
    texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Muestra un aviso de verdad (necesita un servidor de notificaciones, p. ej.
    /// dunst): `cargo test -- --ignored aviso_real`.
    #[tokio::test]
    #[ignore]
    async fn aviso_real() {
        let aviso = Aviso {
            titulo: "Prueba de slack-rs".into(),
            cuerpo: "Si ves esto, las notificaciones funcionan <3".into(),
        };
        // Se cierra solo por tiempo; sin clic devuelve false.
        let _ = mostrar(aviso).await;
    }

    #[test]
    fn escapa_el_marcado() {
        assert_eq!(escapar("a <b> & c"), "a &lt;b&gt; &amp; c");
    }
}
