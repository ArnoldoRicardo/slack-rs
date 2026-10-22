//! Un borrador por conversación, como Slack: lo que escribes en #a sigue ahí al
//! volver de #b. Los hilos tienen el suyo, con la clave `canal/ts`.

use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct Borradores(HashMap<String, String>);

/// Clave del borrador de un hilo.
pub(crate) fn clave_hilo(canal: &str, ts: &str) -> String {
    format!("{canal}/{ts}")
}

impl Borradores {
    /// Guarda el texto al salir de una conversación; vacío o solo espacios lo borra.
    pub(crate) fn guardar(&mut self, clave: &str, texto: &str) {
        if texto.trim().is_empty() {
            self.0.remove(clave);
        } else {
            self.0.insert(clave.to_string(), texto.to_string());
        }
    }

    /// Saca el borrador para ponerlo en el editor; queda vacío hasta el próximo `guardar`.
    pub(crate) fn tomar(&mut self, clave: &str) -> String {
        self.0.remove(clave).unwrap_or_default()
    }

    /// Si la conversación tiene borrador, para el ✎ de la lateral.
    pub(crate) fn tiene(&self, clave: &str) -> bool {
        self.0.contains_key(clave)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn ida_y_vuelta_entre_dos_canales() {
        let mut b = Borradores::default();
        b.guardar("C1", "hola a");
        b.guardar("C2", "hola b");
        assert_eq!(b.tomar("C1"), "hola a");
        assert_eq!(b.tomar("C2"), "hola b");
    }

    #[test]
    fn tomar_lo_saca() {
        let mut b = Borradores::default();
        b.guardar("C1", "x");
        assert!(b.tiene("C1"));
        assert_eq!(b.tomar("C1"), "x");
        assert!(!b.tiene("C1"));
        assert_eq!(b.tomar("C1"), "");
    }

    #[test]
    fn vacio_o_espacios_borra() {
        let mut b = Borradores::default();
        b.guardar("C1", "algo");
        b.guardar("C1", "  \n ");
        assert!(!b.tiene("C1"));
    }

    #[test]
    fn conserva_el_texto_tal_cual() {
        let mut b = Borradores::default();
        b.guardar("C1", "  línea 1\nlínea 2 ");
        assert_eq!(b.tomar("C1"), "  línea 1\nlínea 2 ");
    }

    #[test]
    fn el_hilo_no_pisa_al_canal() {
        let mut b = Borradores::default();
        b.guardar("C1", "en el canal");
        b.guardar(&clave_hilo("C1", "1726.0001"), "en el hilo");
        assert_eq!(b.tomar("C1"), "en el canal");
        assert_eq!(b.tomar("C1/1726.0001"), "en el hilo");
    }
}
