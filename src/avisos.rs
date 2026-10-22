//! Cuándo un mensaje nuevo merece notificación de escritorio, con las reglas por
//! defecto de Slack: DMs y menciones; nada si es tuyo, si lo estás viendo o si
//! pausaste las notificaciones. Mandarla al sistema es aparte.

/// Lo que hace falta saber de un mensaje recién llegado.
#[derive(Debug, Clone, Default)]
pub(crate) struct Contexto<'a> {
    pub(crate) autor: &'a str,
    /// `#canal` o el nombre del grupo; vacío en una DM.
    pub(crate) donde: &'a str,
    pub(crate) texto: &'a str,
    pub(crate) propio: bool,
    pub(crate) es_dm: bool,
    pub(crate) me_menciona: bool,
    /// El canal está abierto y la ventana tiene el foco.
    pub(crate) a_la_vista: bool,
    pub(crate) silenciado: bool,
    /// El canal está en "Todos los mensajes": avisa aunque no te mencionen.
    pub(crate) todo: bool,
    pub(crate) pausado: bool,
    /// Preferencia "Include a preview of the message".
    pub(crate) vista_previa: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Aviso {
    pub(crate) titulo: String,
    pub(crate) cuerpo: String,
}

/// Largo máximo del cuerpo, en caracteres.
const LARGO: usize = 140;

pub(crate) fn debe_avisar(ctx: &Contexto) -> Option<Aviso> {
    if ctx.propio || ctx.a_la_vista || ctx.pausado {
        return None;
    }
    if !(ctx.me_menciona || (ctx.es_dm || ctx.todo) && !ctx.silenciado) {
        return None;
    }
    let titulo = if ctx.donde.is_empty() {
        ctx.autor.to_string()
    } else {
        format!("{} en {}", ctx.autor, ctx.donde)
    };
    let cuerpo = if !ctx.vista_previa {
        "Mensaje nuevo".to_string()
    } else if ctx.texto.chars().count() > LARGO {
        ctx.texto.chars().take(LARGO).chain(['…']).collect()
    } else {
        ctx.texto.to_string()
    };
    Some(Aviso { titulo, cuerpo })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn dm(texto: &str) -> Contexto<'_> {
        Contexto {
            autor: "Ana",
            texto,
            es_dm: true,
            vista_previa: true,
            ..Default::default()
        }
    }

    fn canal(texto: &str) -> Contexto<'_> {
        Contexto {
            autor: "Ana",
            donde: "#tech",
            texto,
            vista_previa: true,
            ..Default::default()
        }
    }

    #[test]
    fn dm_avisa_con_el_autor_de_titulo() {
        let aviso = debe_avisar(&dm("hola")).unwrap();
        assert_eq!(aviso.titulo, "Ana");
        assert_eq!(aviso.cuerpo, "hola");
    }

    #[test]
    fn mencion_en_canal_avisa_con_el_canal() {
        let mut ctx = canal("oye <@U0>");
        ctx.me_menciona = true;
        assert_eq!(debe_avisar(&ctx).unwrap().titulo, "Ana en #tech");
    }

    #[test]
    fn canal_sin_mencion_no_avisa() {
        assert_eq!(debe_avisar(&canal("hola")), None);
    }

    #[test]
    fn lo_propio_y_lo_que_estas_viendo_no_avisan() {
        let mut propio = dm("hola");
        propio.propio = true;
        assert_eq!(debe_avisar(&propio), None);
        let mut visto = dm("hola");
        visto.a_la_vista = true;
        assert_eq!(debe_avisar(&visto), None);
    }

    #[test]
    fn pausado_nunca_avisa() {
        let mut ctx = dm("hola");
        ctx.pausado = true;
        assert_eq!(debe_avisar(&ctx), None);
    }

    #[test]
    fn canal_con_todos_los_mensajes_avisa_siempre() {
        let mut ctx = canal("hola");
        ctx.todo = true;
        assert!(debe_avisar(&ctx).is_some());
        ctx.silenciado = true;
        assert!(debe_avisar(&ctx).is_none());
    }

    #[test]
    fn silenciado_solo_con_mencion() {
        let mut ctx = canal("hola");
        ctx.silenciado = true;
        assert_eq!(debe_avisar(&ctx), None);
        ctx.me_menciona = true;
        assert!(debe_avisar(&ctx).is_some());
    }

    #[test]
    fn sin_vista_previa_no_muestra_el_texto() {
        let mut ctx = dm("secreto");
        ctx.vista_previa = false;
        assert_eq!(debe_avisar(&ctx).unwrap().cuerpo, "Mensaje nuevo");
    }

    #[test]
    fn el_cuerpo_se_recorta_en_caracteres() {
        let largo = "ñ".repeat(LARGO + 20);
        let cuerpo = debe_avisar(&dm(&largo)).unwrap().cuerpo;
        assert_eq!(cuerpo.chars().count(), LARGO + 1);
        assert!(cuerpo.ends_with('…'));
    }
}
