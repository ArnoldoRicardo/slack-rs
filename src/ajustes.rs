//! Preferencias de la app (`~/.config/slack-rs/prefs.json`, junto a las credenciales):
//! las de la ventana de Preferencias y los atajos que se recuerdan. Lo que no esté en
//! el archivo toma su valor por defecto, así los archivos viejos siguen sirviendo.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::slack::Resultado;

/// Formato de la hora de los mensajes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum Reloj {
    /// La preferencia `time24` de Slack.
    #[default]
    Slack,
    Doce,
    Veinticuatro,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Ajustes {
    pub(crate) oscuro: bool,
    /// Tamaño de toda la interfaz (Ctrl + + / − / 0).
    pub(crate) escala: f32,
    /// Notificaciones de escritorio pausadas (🌙).
    pub(crate) pausado: bool,
    /// Paleta primaria elegida aquí; gana sobre la del workspace. `"clasico"` es el
    /// Aubergine de siempre; `None`, el tema de cada workspace.
    pub(crate) tema: Option<String>,
    /// Densidad compacta: sin avatares, nombre y hora en la línea del mensaje.
    pub(crate) compacta: bool,
    pub(crate) reloj: Reloj,
    /// Intro hace salto de línea y Ctrl + Intro envía.
    pub(crate) ctrl_intro: bool,
    /// Mostrar el texto del mensaje en las notificaciones.
    pub(crate) vista_previa: bool,
}

impl Default for Ajustes {
    fn default() -> Self {
        Self {
            oscuro: false,
            escala: 1.0,
            pausado: false,
            tema: None,
            compacta: false,
            reloj: Reloj::Slack,
            ctrl_intro: false,
            vista_previa: true,
        }
    }
}

impl Ajustes {
    fn ruta(credenciales: &Path) -> PathBuf {
        credenciales.with_file_name("prefs.json")
    }

    pub(crate) fn leer(credenciales: &Path) -> Self {
        std::fs::read_to_string(Self::ruta(credenciales))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub(crate) fn guardar(&self, credenciales: &Path) -> Resultado<()> {
        let ruta = Self::ruta(credenciales);
        if let Some(dir) = ruta.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let json = serde_json::to_string_pretty(self).expect("ajustes serializables");
        std::fs::write(&ruta, json).map_err(|e| format!("{}: {e}", ruta.display()))
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn carpeta(nombre: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("slack-rs-ajustes-{}-{nombre}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn por_defecto() {
        let a = Ajustes::default();
        assert_eq!(a.escala, 1.0);
        assert_eq!(a.reloj, Reloj::Slack);
        assert!(a.vista_previa, "como Slack: el aviso muestra el texto");
        assert!(!a.oscuro && !a.compacta && !a.ctrl_intro && !a.pausado);
        assert_eq!(a.tema, None);
    }

    #[test]
    fn un_archivo_viejo_se_completa_con_lo_nuevo() {
        let dir = carpeta("viejo");
        std::fs::write(dir.join("prefs.json"), r#"{"oscuro":true,"escala":1.2}"#).unwrap();
        let a = Ajustes::leer(&dir.join("credenciales.json"));
        assert!(a.oscuro);
        assert_eq!(a.escala, 1.2);
        assert!(a.vista_previa);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ida_y_vuelta() {
        let dir = carpeta("ida");
        let cred = dir.join("credenciales.json");
        assert_eq!(Ajustes::leer(&cred), Ajustes::default(), "sin archivo");
        let a = Ajustes {
            tema: Some("ocean".into()),
            compacta: true,
            reloj: Reloj::Doce,
            ctrl_intro: true,
            vista_previa: false,
            ..Ajustes::default()
        };
        a.guardar(&cred).unwrap();
        assert_eq!(Ajustes::leer(&cred), a);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
