//! Atajos de teclado de Slack (Ayuda → Keyboard shortcuts) traducidos a acciones.
//! Tipos propios para no atarse a los de iced: el cableado convierte el evento.
//! Ctrl + K y Ctrl + 1…9 se resuelven aparte, en `main.rs`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tecla {
    Arriba,
    Abajo,
    Izquierda,
    Derecha,
    Esc,
    Tab,
    Caracter(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Mods {
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Accion {
    Atras,
    Adelante,
    CanalAnterior,
    CanalSiguiente,
    NoLeidoAnterior,
    NoLeidoSiguiente,
    WorkspaceSiguiente,
    WorkspaceAnterior,
    MarcarLeido,
    MarcarTodoLeido,
    PanelDerecho,
    TextoMas,
    TextoMenos,
    TextoNormal,
    /// Ctrl + /: la lista de atajos.
    VerAtajos,
    /// Ctrl + Shift + Y.
    ModoOscuro,
    /// Ctrl + , (la ventana de Preferencias).
    Preferencias,
    /// Pestaña del rail: 1 Inicio, 2 DMs, 3 Actividad, 4 Archivos, 5 Pendientes.
    Vista(u8),
    EditarUltimo,
}

/// Formato del compositor, de los botones o de un atajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Formato {
    Negrita,
    Cursiva,
    Tachado,
    Codigo,
    BloqueCodigo,
    Numerada,
    Vinetas,
    Cita,
}

/// Si Intro (con Ctrl o Shift) manda el mensaje o hace un salto de línea.
pub(crate) fn intro_envia(ctrl: bool, shift: bool, ctrl_intro: bool) -> bool {
    if ctrl_intro { ctrl } else { !shift }
}

/// Atajo de formato con el compositor enfocado, como los de Slack.
pub(crate) fn formato(tecla: Tecla, mods: Mods) -> Option<Formato> {
    let Tecla::Caracter(c) = tecla else {
        return None;
    };
    let Mods { ctrl, alt, shift } = mods;
    if !ctrl {
        return None;
    }
    Some(match (c.to_ascii_lowercase(), alt, shift) {
        ('b', false, false) => Formato::Negrita,
        ('i', false, false) => Formato::Cursiva,
        ('x', false, true) => Formato::Tachado,
        ('c', false, true) => Formato::Codigo,
        ('c', true, true) => Formato::BloqueCodigo,
        ('7', false, true) => Formato::Numerada,
        ('8', false, true) => Formato::Vinetas,
        ('9', false, true) => Formato::Cita,
        _ => return None,
    })
}

/// La acción de una tecla, si tiene. ↑ solo edita tu último mensaje con el
/// compositor vacío; con texto, la flecha es del editor.
pub(crate) fn accion(tecla: Tecla, mods: Mods, compositor_vacio: bool) -> Option<Accion> {
    let Mods { ctrl, alt, shift } = mods;
    Some(match (tecla, ctrl, alt, shift) {
        (Tecla::Izquierda, false, true, false) => Accion::Atras,
        (Tecla::Derecha, false, true, false) => Accion::Adelante,
        (Tecla::Arriba, false, true, false) => Accion::CanalAnterior,
        (Tecla::Abajo, false, true, false) => Accion::CanalSiguiente,
        (Tecla::Arriba, false, true, true) => Accion::NoLeidoAnterior,
        (Tecla::Abajo, false, true, true) => Accion::NoLeidoSiguiente,
        (Tecla::Arriba, _, false, false) if compositor_vacio => Accion::EditarUltimo,
        (Tecla::Tab, true, false, false) => Accion::WorkspaceSiguiente,
        (Tecla::Tab, true, false, true) => Accion::WorkspaceAnterior,
        (Tecla::Esc, false, false, false) => Accion::MarcarLeido,
        (Tecla::Esc, false, false, true) => Accion::MarcarTodoLeido,
        (Tecla::Caracter('.'), true, false, false) => Accion::PanelDerecho,
        (Tecla::Caracter('+' | '='), true, false, _) => Accion::TextoMas,
        (Tecla::Caracter('-'), true, false, false) => Accion::TextoMenos,
        (Tecla::Caracter('0'), true, false, false) => Accion::TextoNormal,
        (Tecla::Caracter('/'), true, false, false) => Accion::VerAtajos,
        (Tecla::Caracter(','), true, false, false) => Accion::Preferencias,
        (Tecla::Caracter('y' | 'Y'), true, false, true) => Accion::ModoOscuro,
        (Tecla::Caracter(c @ '1'..='5'), true, false, true) => Accion::Vista(c as u8 - b'0'),
        _ => return None,
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use Accion::*;
    use Tecla::*;

    const fn m(ctrl: bool, alt: bool, shift: bool) -> Mods {
        Mods { ctrl, alt, shift }
    }

    const NADA: Mods = m(false, false, false);
    const CTRL: Mods = m(true, false, false);
    const ALT: Mods = m(false, true, false);
    const SHIFT: Mods = m(false, false, true);
    const ALT_SHIFT: Mods = m(false, true, true);
    const CTRL_SHIFT: Mods = m(true, false, true);

    fn a(tecla: Tecla, mods: Mods) -> Option<Accion> {
        accion(tecla, mods, false)
    }

    #[test]
    fn historial_con_alt_y_flechas() {
        assert_eq!(a(Izquierda, ALT), Some(Atras));
        assert_eq!(a(Derecha, ALT), Some(Adelante));
    }

    #[test]
    fn canales_con_alt_y_no_leidos_con_alt_shift() {
        assert_eq!(a(Arriba, ALT), Some(CanalAnterior));
        assert_eq!(a(Abajo, ALT), Some(CanalSiguiente));
        assert_eq!(a(Arriba, ALT_SHIFT), Some(NoLeidoAnterior));
        assert_eq!(a(Abajo, ALT_SHIFT), Some(NoLeidoSiguiente));
    }

    #[test]
    fn workspaces_con_ctrl_tab() {
        assert_eq!(a(Tab, CTRL), Some(WorkspaceSiguiente));
        assert_eq!(a(Tab, CTRL_SHIFT), Some(WorkspaceAnterior));
        assert_eq!(a(Tab, NADA), None);
    }

    #[test]
    fn esc_marca_leido_y_shift_esc_todo() {
        assert_eq!(a(Esc, NADA), Some(MarcarLeido));
        assert_eq!(a(Esc, SHIFT), Some(MarcarTodoLeido));
    }

    #[test]
    fn panel_y_tamano_de_texto() {
        assert_eq!(a(Caracter('.'), CTRL), Some(PanelDerecho));
        assert_eq!(a(Caracter('+'), CTRL), Some(TextoMas));
        assert_eq!(a(Caracter('='), CTRL), Some(TextoMas));
        assert_eq!(a(Caracter('-'), CTRL), Some(TextoMenos));
    }

    #[test]
    fn texto_normal_atajos_y_modo_oscuro() {
        assert_eq!(a(Caracter('0'), CTRL), Some(TextoNormal));
        assert_eq!(a(Caracter('/'), CTRL), Some(VerAtajos));
        assert_eq!(a(Caracter('y'), CTRL_SHIFT), Some(ModoOscuro));
        assert_eq!(a(Caracter('Y'), CTRL_SHIFT), Some(ModoOscuro));
        assert_eq!(a(Caracter('y'), CTRL), None);
    }

    #[test]
    fn vistas_con_ctrl_shift_numero() {
        for n in 1..=5 {
            let c = char::from(b'0' + n);
            assert_eq!(a(Caracter(c), CTRL_SHIFT), Some(Vista(n)));
        }
        assert_eq!(a(Caracter('6'), CTRL_SHIFT), None);
    }

    #[test]
    fn ctrl_numero_no_es_de_aqui() {
        assert_eq!(a(Caracter('1'), CTRL), None);
        assert_eq!(a(Caracter('k'), CTRL), None);
    }

    #[test]
    fn flecha_arriba_edita_solo_con_el_compositor_vacio() {
        assert_eq!(accion(Arriba, NADA, true), Some(EditarUltimo));
        assert_eq!(accion(Arriba, CTRL, true), Some(EditarUltimo));
        assert_eq!(accion(Arriba, NADA, false), None);
        assert_eq!(accion(Arriba, CTRL, false), None);
    }

    #[test]
    fn atajos_de_formato_del_compositor() {
        use Formato::*;
        let f = |t, m| formato(t, m);
        assert_eq!(f(Caracter('b'), CTRL), Some(Negrita));
        assert_eq!(f(Caracter('i'), CTRL), Some(Cursiva));
        assert_eq!(f(Caracter('x'), CTRL_SHIFT), Some(Tachado));
        assert_eq!(f(Caracter('c'), CTRL_SHIFT), Some(Codigo));
        assert_eq!(f(Caracter('c'), m(true, true, true)), Some(BloqueCodigo));
        assert_eq!(f(Caracter('7'), CTRL_SHIFT), Some(Numerada));
        assert_eq!(f(Caracter('8'), CTRL_SHIFT), Some(Vinetas));
        assert_eq!(f(Caracter('9'), CTRL_SHIFT), Some(Cita));
        assert_eq!(f(Caracter('b'), NADA), None);
        assert_eq!(f(Caracter('x'), CTRL), None, "Ctrl + X es cortar");
    }

    #[test]
    fn ctrl_coma_abre_preferencias() {
        assert_eq!(a(Caracter(','), CTRL), Some(Preferencias));
    }

    #[test]
    fn intro_envia_segun_la_preferencia() {
        // Como siempre: Intro envía, Shift + Intro es salto de línea.
        assert!(intro_envia(false, false, false));
        assert!(!intro_envia(false, true, false));
        // Con "Ctrl + Intro envía": Intro es salto de línea.
        assert!(!intro_envia(false, false, true));
        assert!(intro_envia(true, false, true));
    }

    #[test]
    fn letras_sueltas_no_son_atajo() {
        assert_eq!(a(Caracter('e'), NADA), None);
        assert_eq!(a(Abajo, NADA), None);
    }
}
