//! Moverse sin mouse: historial atrás/adelante (Alt + ← →), canal vecino
//! (Alt + ↑ ↓), siguiente no leído (Alt + Shift + ↑ ↓) y rotar workspaces
//! (Ctrl + Tab). Solo lógica; los ids son los de las conversaciones.

/// Cuántas visitas se recuerdan hacia atrás.
const TOPE: usize = 50;

/// Historial de conversaciones visitadas, como el del navegador.
#[derive(Debug, Default)]
pub(crate) struct Historial {
    atras: Vec<String>,
    actual: Option<String>,
    adelante: Vec<String>,
}

impl Historial {
    /// Registra que se abrió `id`. Abrir algo nuevo borra el "adelante".
    pub(crate) fn visitar(&mut self, id: &str) {
        if self.actual.as_deref() == Some(id) {
            return;
        }
        if let Some(anterior) = self.actual.replace(id.to_string()) {
            self.atras.push(anterior);
            if self.atras.len() > TOPE {
                self.atras.remove(0);
            }
        }
        self.adelante.clear();
    }

    /// Vuelve a la anterior y la devuelve, si hay.
    pub(crate) fn atras(&mut self) -> Option<&str> {
        let anterior = self.atras.pop()?;
        if let Some(actual) = self.actual.replace(anterior) {
            self.adelante.push(actual);
        }
        self.actual.as_deref()
    }

    /// Lo visitado antes de la actual, la más reciente primero (menú 🕘).
    pub(crate) fn recientes(&self) -> Vec<&str> {
        self.atras.iter().rev().map(String::as_str).collect()
    }

    /// Rehace un `atras` y devuelve a dónde se llegó, si hay.
    pub(crate) fn adelante(&mut self) -> Option<&str> {
        let siguiente = self.adelante.pop()?;
        if let Some(actual) = self.actual.replace(siguiente) {
            self.atras.push(actual);
        }
        self.actual.as_deref()
    }
}

/// La conversación a `paso` lugares de `actual` en el orden de la lateral, sin
/// dar la vuelta. Si `actual` no está, Alt + ↓ va a la primera y Alt + ↑ a la última.
pub(crate) fn vecino<'a>(orden: &[&'a str], actual: Option<&str>, paso: i32) -> Option<&'a str> {
    vecino_sin_leer(orden, |_| true, actual, paso)
}

/// Como [`vecino`], pero salta a la siguiente con no leídos en esa dirección.
pub(crate) fn vecino_sin_leer<'a>(
    orden: &[&'a str],
    no_leido: impl Fn(&str) -> bool,
    actual: Option<&str>,
    paso: i32,
) -> Option<&'a str> {
    let pos = actual.and_then(|a| orden.iter().position(|id| *id == a));
    let candidatos: Box<dyn Iterator<Item = &&str>> = match (pos, paso >= 0) {
        (Some(p), true) => Box::new(orden[p + 1..].iter()),
        (Some(p), false) => Box::new(orden[..p].iter().rev()),
        (None, true) => Box::new(orden.iter()),
        (None, false) => Box::new(orden.iter().rev()),
    };
    candidatos.copied().find(|id| no_leido(id))
}

/// Índice del workspace a `paso` lugares de `i`, dando la vuelta entre `n`.
pub(crate) fn rotar(i: usize, n: usize, paso: i32) -> usize {
    if n == 0 {
        return 0;
    }
    (i as i64 + paso as i64).rem_euclid(n as i64) as usize
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn recientes_sin_repetir_la_actual() {
        let mut h = visitado(&["a", "b", "c"]);
        assert_eq!(h.recientes(), ["b", "a"]);
        h.atras();
        assert_eq!(h.recientes(), ["a"]);
    }

    fn visitado(ids: &[&str]) -> Historial {
        let mut h = Historial::default();
        for id in ids {
            h.visitar(id);
        }
        h
    }

    #[test]
    fn atras_y_adelante() {
        let mut h = visitado(&["a", "b", "c"]);
        assert_eq!(h.atras(), Some("b"));
        assert_eq!(h.atras(), Some("a"));
        assert_eq!(h.atras(), None);
        assert_eq!(h.adelante(), Some("b"));
        assert_eq!(h.adelante(), Some("c"));
        assert_eq!(h.adelante(), None);
    }

    #[test]
    fn visitar_despues_de_volver_borra_el_adelante() {
        let mut h = visitado(&["a", "b", "c"]);
        h.atras();
        h.visitar("d");
        assert_eq!(h.adelante(), None);
        assert_eq!(h.atras(), Some("b"));
        assert_eq!(h.atras(), Some("a"));
    }

    #[test]
    fn la_misma_dos_veces_no_se_apila() {
        let mut h = visitado(&["a", "b", "b"]);
        assert_eq!(h.atras(), Some("a"));
        assert_eq!(h.atras(), None);
    }

    #[test]
    fn recuerda_hasta_el_tope() {
        let ids: Vec<String> = (0..TOPE + 10).map(|i| i.to_string()).collect();
        let mut h = Historial::default();
        for id in &ids {
            h.visitar(id);
        }
        let mut pasos = 0;
        while h.atras().is_some() {
            pasos += 1;
        }
        assert_eq!(pasos, TOPE);
    }

    #[test]
    fn vecino_sin_dar_la_vuelta() {
        let orden = ["a", "b", "c"];
        assert_eq!(vecino(&orden, Some("b"), 1), Some("c"));
        assert_eq!(vecino(&orden, Some("b"), -1), Some("a"));
        assert_eq!(vecino(&orden, Some("c"), 1), None);
        assert_eq!(vecino(&orden, Some("a"), -1), None);
    }

    #[test]
    fn vecino_sin_actual_entra_por_el_extremo() {
        let orden = ["a", "b", "c"];
        assert_eq!(vecino(&orden, None, 1), Some("a"));
        assert_eq!(vecino(&orden, Some("zz"), -1), Some("c"));
        assert_eq!(vecino(&[], None, 1), None);
    }

    #[test]
    fn siguiente_sin_leer_en_cada_direccion() {
        let orden = ["a", "b", "c", "d", "e"];
        let sin_leer = |id: &str| id == "b" || id == "e";
        assert_eq!(vecino_sin_leer(&orden, sin_leer, Some("c"), 1), Some("e"));
        assert_eq!(vecino_sin_leer(&orden, sin_leer, Some("c"), -1), Some("b"));
        assert_eq!(vecino_sin_leer(&orden, sin_leer, Some("e"), 1), None);
        assert_eq!(vecino_sin_leer(&orden, sin_leer, None, 1), Some("b"));
    }

    #[test]
    fn sin_no_leidos_no_se_mueve() {
        let orden = ["a", "b"];
        assert_eq!(vecino_sin_leer(&orden, |_| false, Some("a"), 1), None);
    }

    #[test]
    fn rotar_da_la_vuelta() {
        assert_eq!(rotar(0, 3, 1), 1);
        assert_eq!(rotar(2, 3, 1), 0);
        assert_eq!(rotar(0, 3, -1), 2);
        assert_eq!(rotar(0, 1, 1), 0);
        assert_eq!(rotar(0, 0, 1), 0);
    }
}
