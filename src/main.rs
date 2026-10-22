mod ajustes;
mod atajos;
mod avisos;
mod borradores;
mod demo;
mod escritorio;
mod espacio;
mod estilo;
mod formato;
mod lateral;
mod login;
mod navegacion;
mod paleta;
mod rtm;
mod saltar;
mod slack;
mod sugerencias;
mod textos;
mod vista;

use std::path::PathBuf;

use iced::{Subscription, Task, keyboard};

use ajustes::Ajustes;
use espacio::{Espacio, Msg};
use slack::{Credenciales, Resultado};

fn main() -> iced::Result {
    iced::application(Slack::new, Slack::update, Slack::view)
        .title(Slack::titulo)
        .subscription(Slack::subscription)
        .theme(|_: &Slack| estilo::tema())
        .default_font(estilo::LATO)
        .window_size((1280.0, 820.0))
        .scale_factor(|s: &Slack| s.ajustes.escala)
        .run()
}

/// Toda la app: un [`Espacio`] por workspace, como el rail de la app de escritorio.
/// Sin ninguno se ve la bienvenida para iniciar sesión.
struct Slack {
    espacios: Vec<Espacio>,
    activo: usize,
    /// Donde se guardan las sesiones iniciadas desde la app.
    archivo: PathBuf,
    inicio: Inicio,
    /// Por qué falló el último inicio de sesión.
    aviso: Option<String>,
    /// Preferencias (modo oscuro, tamaño, tema…); se guardan en `prefs.json`.
    ajustes: Ajustes,
    /// Ventana con la lista de atajos (Ctrl + /).
    ver_atajos: bool,
    /// Menú de tu avatar al pie del rail.
    menu_cuenta: bool,
    /// Ventana de Preferencias (Ctrl + ,).
    ver_preferencias: bool,
    /// Próxima clave libre para un workspace.
    siguiente: u64,
}

const ESCALA_MIN: f32 = 0.7;
const ESCALA_MAX: f32 = 1.6;

/// Lo que se puede cambiar en la ventana de Preferencias.
#[derive(Debug, Clone)]
pub(crate) enum Cambio {
    Oscuro(bool),
    Tema(Option<String>),
    Compacta(bool),
    Reloj(ajustes::Reloj),
    CtrlIntro(bool),
    VistaPrevia(bool),
}

/// Inicio de sesión con el navegador desde el botón + del rail o la bienvenida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Inicio {
    Libre,
    /// Hay una ventana de Chrome abierta esperando a que el usuario entre.
    Esperando,
}

#[derive(Debug, Clone)]
enum Global {
    /// Mensaje para el workspace con esa clave; se descarta si ya no está.
    Espacio(u64, Msg),
    Activar(usize),
    /// Cierra la sesión del workspace con esa clave y la borra de las guardadas.
    CerrarSesion(u64),
    VerAtajos,
    MenuCuenta,
    /// Abrir o cerrar la ventana de Preferencias (Ctrl + ,).
    VerPreferencias,
    /// Un cambio hecho en Preferencias.
    Ajustar(Cambio),
    /// 🌙: pausar o reanudar las notificaciones de escritorio.
    Pausar,
    /// La ventana ganó o perdió el foco.
    Foco(bool),
    /// Archivo arrastrado a la ventana: se adjunta al compositor del canal.
    Soltado(PathBuf),
    ModoOscuro,
    Tecla(keyboard::Event),
    Captura(iced::window::Screenshot),
    IniciarSesion,
    SesionIniciada(Resultado<Credenciales>),
    CerrarAviso,
}

impl Slack {
    fn new() -> (Self, Task<Global>) {
        let archivo = login::archivo();
        if std::env::var_os("SLACK_DEMO").is_some() {
            let (mut slack, _) = Self::con(vec![], archivo);
            slack.espacios = demo::espacios();
            for e in &mut slack.espacios {
                e.clave = slack.siguiente;
                slack.siguiente += 1;
            }
            slack.espacios[0].visible = true;
            return (slack, capturar_si_se_pide());
        }
        let guardadas = login::cargar_de(&archivo);
        let (slack, tarea) = match login::iniciales(|var| std::env::var(var).ok(), guardadas) {
            Ok(todas) => Self::con(todas, archivo),
            Err(e) => {
                let (mut slack, tarea) = Self::con(vec![], archivo);
                slack.espacios.push(Espacio::con_error(e));
                slack.espacios[0].visible = true;
                (slack, tarea)
            }
        };
        (slack, Task::batch([tarea, capturar_si_se_pide()]))
    }

    /// Un workspace por sesión, el primero a la vista.
    fn con(todas: Vec<Credenciales>, archivo: PathBuf) -> (Self, Task<Global>) {
        let mut ajustes = Ajustes::leer(&archivo);
        ajustes.escala = ajustes.escala.clamp(ESCALA_MIN, ESCALA_MAX);
        let mut slack = Self {
            espacios: Vec::new(),
            activo: 0,
            archivo,
            inicio: Inicio::Libre,
            aviso: None,
            ajustes,
            ver_atajos: false,
            menu_cuenta: false,
            ver_preferencias: false,
            siguiente: 1,
        };
        let tareas: Vec<_> = todas.into_iter().map(|c| slack.agregar(c).1).collect();
        if let Some(e) = slack.espacios.first_mut() {
            e.visible = true;
        }
        estilo::fijar(slack.colores());
        (slack, Task::batch(tareas))
    }

    /// Suma al final el workspace de una sesión, con una clave nueva.
    fn agregar(&mut self, cred: Credenciales) -> (usize, Task<Global>) {
        let (mut e, tarea) = Espacio::new(cred);
        let clave = self.siguiente;
        self.siguiente += 1;
        e.clave = clave;
        e.ajustes = self.ajustes.clone();
        self.espacios.push(e);
        let tarea = tarea.map(move |m| Global::Espacio(clave, m));
        (self.espacios.len() - 1, tarea)
    }

    fn actual(&self) -> &Espacio {
        &self.espacios[self.activo]
    }

    fn titulo(&self) -> String {
        match self.espacios.get(self.activo) {
            Some(e) => e.titulo(),
            None => "Slack".into(),
        }
    }

    /// Workspaces con conexión de tiempo real: los que tienen sesión y no está vencida.
    fn con_tiempo_real(&self) -> Vec<u64> {
        self.espacios
            .iter()
            .filter(|e| e.cliente.is_some() && !e.sesion_vencida)
            .map(|e| e.clave)
            .collect()
    }

    fn subscription(&self) -> Subscription<Global> {
        let vivos = self.con_tiempo_real();
        let tiempo_real = self.espacios.iter().filter_map(|e| {
            let c = e.cliente.as_ref().filter(|_| vivos.contains(&e.clave))?;
            Some(
                rtm::suscripcion(c.credenciales().clone())
                    .with(e.clave)
                    .map(|(clave, ev)| Global::Espacio(clave, Msg::Rtm(Box::new(ev)))),
            )
        });
        // También las teclas que ya usó un campo de texto: Esc cierra Ctrl+K aunque el
        // campo tenga el foco.
        let teclado = iced::event::listen_with(|evento, _, _| match evento {
            iced::Event::Keyboard(k) => Some(Global::Tecla(k)),
            iced::Event::Window(iced::window::Event::FileDropped(ruta)) => {
                Some(Global::Soltado(ruta))
            }
            iced::Event::Window(iced::window::Event::Focused) => Some(Global::Foco(true)),
            iced::Event::Window(iced::window::Event::Unfocused) => Some(Global::Foco(false)),
            _ => None,
        });
        // Mientras alguien escribe, un reloj para quitar el aviso cuando deja de hacerlo.
        let relojes = self
            .espacios
            .iter()
            .filter(|e| !e.escribiendo.is_empty())
            .map(|e| {
                iced::time::every(std::time::Duration::from_secs(2))
                    .with(e.clave)
                    .map(|(clave, _)| Global::Espacio(clave, Msg::Tick))
            });
        Subscription::batch(tiempo_real.chain(relojes).chain([teclado]))
    }

    /// Colores del tema a la vista: el elegido en Preferencias gana sobre el del
    /// workspace.
    fn colores(&self) -> estilo::Colores {
        let oscuro = self.ajustes.oscuro;
        let elegido = self.ajustes.tema.as_deref().map(|primaria| paleta::Tema {
            primaria: primaria.to_string(),
            resalte: "jade".into(),
        });
        let paleta = match (&elegido, self.espacios.get(self.activo)) {
            (Some(t), _) if t.primaria == "clasico" => paleta::aubergine(),
            (Some(t), _) => paleta::del_tema(Some(t), oscuro),
            (None, e) => paleta::del_tema(e.and_then(|e| e.tema.as_ref()), oscuro),
        };
        estilo::colores(&paleta, oscuro)
    }

    fn update(&mut self, msg: Global) -> Task<Global> {
        let tarea = self.actualizar(msg);
        // El tema puede cambiar al llegar las preferencias o al cambiar de workspace.
        estilo::fijar(self.colores());
        tarea
    }

    fn actualizar(&mut self, msg: Global) -> Task<Global> {
        match msg {
            // Clic en una notificación: ese workspace pasa al frente con su canal.
            Global::Espacio(clave, Msg::AvisoClic(canal)) => {
                let Some(i) = self.espacios.iter().position(|e| e.clave == clave) else {
                    return Task::none();
                };
                let al_frente = iced::window::latest().and_then(iced::window::gain_focus);
                return Task::batch([
                    self.activar(i),
                    self.update(Global::Espacio(clave, Msg::Seleccionar(canal))),
                    al_frente,
                ]);
            }
            Global::Espacio(clave, msg) => {
                if let Some(e) = self.espacios.iter_mut().find(|e| e.clave == clave) {
                    return e.update(msg).map(move |m| Global::Espacio(clave, m));
                }
            }
            Global::Activar(i) => return self.activar(i),
            Global::CerrarSesion(clave) => return self.cerrar_sesion(clave),
            Global::VerAtajos => self.ver_atajos = !self.ver_atajos,
            Global::Pausar => {
                self.ajustes.pausado = !self.ajustes.pausado;
                self.repartir_ajustes();
            }
            Global::Foco(enfocada) => {
                for e in &mut self.espacios {
                    e.desenfocada = !enfocada;
                }
            }
            Global::Soltado(ruta) => {
                return self.al_activo(Msg::Adjuntar(espacio::Destino::Canal, ruta));
            }
            Global::MenuCuenta => self.menu_cuenta = !self.menu_cuenta,
            Global::VerPreferencias => {
                self.ver_preferencias = !self.ver_preferencias;
                self.menu_cuenta = false;
            }
            Global::Ajustar(cambio) => {
                let a = &mut self.ajustes;
                match cambio {
                    Cambio::Oscuro(v) => a.oscuro = v,
                    Cambio::Tema(t) => a.tema = t,
                    Cambio::Compacta(v) => a.compacta = v,
                    Cambio::Reloj(r) => a.reloj = r,
                    Cambio::CtrlIntro(v) => a.ctrl_intro = v,
                    Cambio::VistaPrevia(v) => a.vista_previa = v,
                }
                self.repartir_ajustes();
            }
            Global::ModoOscuro => {
                self.menu_cuenta = false;
                return self.atajo(atajos::Accion::ModoOscuro);
            }
            Global::IniciarSesion => {
                if self.inicio == Inicio::Libre {
                    self.inicio = Inicio::Esperando;
                    self.aviso = None;
                    return Task::perform(login::con_navegador(), Global::SesionIniciada);
                }
            }
            Global::SesionIniciada(resultado) => {
                self.inicio = Inicio::Libre;
                match resultado {
                    Ok(cred) => {
                        if let Err(e) = login::recordar_en(&self.archivo, cred.clone()) {
                            self.aviso = Some(format!("No pude guardar la sesión: {e}"));
                        }
                        return self.abrir(cred);
                    }
                    Err(e) => self.aviso = Some(e),
                }
            }
            Global::CerrarAviso => self.aviso = None,
            Global::Tecla(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                ..
            }) => {
                let mods = atajos::Mods {
                    ctrl: modifiers.command(),
                    alt: modifiers.alt(),
                    shift: modifiers.shift(),
                };
                let accion =
                    tecla_de_atajo(&key, physical_key).and_then(|t| atajos::accion(t, mods, false));
                if let Some(accion) = accion {
                    return self.atajo(accion);
                }
                if let Some(msg) = atajo_espacio(&key, physical_key, modifiers) {
                    return self.al_activo(msg);
                }
                // Ctrl + 1…9 cambia de workspace, como en la app de escritorio.
                if modifiers.command()
                    && let Some(n) = key.to_latin(physical_key).and_then(|c| c.to_digit(10))
                    && n > 0
                {
                    return self.activar(n as usize - 1);
                }
            }
            Global::Tecla(_) => {}
            Global::Captura(c) => {
                if let Some(ruta) = std::env::var_os("SLACK_CAPTURA") {
                    let (w, h) = (c.size.width, c.size.height);
                    if let Err(e) =
                        ::image::save_buffer(&ruta, &c.rgba, w, h, ::image::ColorType::Rgba8)
                    {
                        eprintln!("captura: {e}");
                    }
                }
                return iced::exit();
            }
        }
        Task::none()
    }

    fn al_activo(&mut self, msg: Msg) -> Task<Global> {
        match self.espacios.get(self.activo) {
            Some(e) => {
                let clave = e.clave;
                self.update(Global::Espacio(clave, msg))
            }
            None => Task::none(),
        }
    }

    fn atajo(&mut self, accion: atajos::Accion) -> Task<Global> {
        use atajos::Accion::*;
        match accion {
            Atras => return self.al_activo(Msg::Atras),
            Adelante => return self.al_activo(Msg::Adelante),
            CanalAnterior => return self.al_activo(Msg::Vecino(-1)),
            CanalSiguiente => return self.al_activo(Msg::Vecino(1)),
            NoLeidoAnterior => return self.al_activo(Msg::VecinoSinLeer(-1)),
            NoLeidoSiguiente => return self.al_activo(Msg::VecinoSinLeer(1)),
            MarcarTodoLeido => return self.al_activo(Msg::MarcarTodoLeido),
            // Esc cierra primero lo de la app; si no hay nada, va al workspace.
            MarcarLeido => {
                let cerro = std::mem::take(&mut self.ver_atajos)
                    || std::mem::take(&mut self.menu_cuenta)
                    || std::mem::take(&mut self.ver_preferencias);
                if !cerro {
                    return self.al_activo(Msg::Esc);
                }
            }
            WorkspaceSiguiente | WorkspaceAnterior => {
                let paso = if accion == WorkspaceSiguiente { 1 } else { -1 };
                let i = navegacion::rotar(self.activo, self.espacios.len(), paso);
                return self.activar(i);
            }
            PanelDerecho => return self.al_activo(Msg::CerrarHilo),
            TextoMas => self.escalar(self.ajustes.escala + 0.1),
            TextoMenos => self.escalar(self.ajustes.escala - 0.1),
            TextoNormal => self.escalar(1.0),
            Vista(1) => return self.al_activo(Msg::Vista(espacio::Vista::Inicio)),
            Vista(2) => return self.al_activo(Msg::Vista(espacio::Vista::Directos)),
            Vista(3) => return self.al_activo(Msg::AbrirPanel(espacio::Panel::Actividad)),
            Vista(4) => return self.al_activo(Msg::AbrirPanel(espacio::Panel::Archivos)),
            Vista(5) => return self.al_activo(Msg::AbrirPanel(espacio::Panel::Guardados)),
            Vista(_) | EditarUltimo => {}
            VerAtajos => self.ver_atajos = !self.ver_atajos,
            Preferencias => self.ver_preferencias = !self.ver_preferencias,
            ModoOscuro => {
                self.ajustes.oscuro = !self.ajustes.oscuro;
                self.guardar_ajustes();
            }
        }
        Task::none()
    }

    fn escalar(&mut self, escala: f32) {
        // Redondeo a décimas para que ida y vuelta no acumule error.
        self.ajustes.escala = ((escala * 10.0).round() / 10.0).clamp(ESCALA_MIN, ESCALA_MAX);
        self.guardar_ajustes();
    }

    /// Lleva los ajustes a cada workspace y los guarda.
    fn repartir_ajustes(&mut self) {
        for e in &mut self.espacios {
            e.ajustes = self.ajustes.clone();
        }
        self.guardar_ajustes();
    }

    fn guardar_ajustes(&mut self) {
        if let Err(e) = self.ajustes.guardar(&self.archivo) {
            self.aviso = Some(format!("No pude guardar la preferencia: {e}"));
        }
    }

    /// Quita el workspace y su sesión guardada; el de al lado queda a la vista.
    fn cerrar_sesion(&mut self, clave: u64) -> Task<Global> {
        self.menu_cuenta = false;
        let Some(i) = self.espacios.iter().position(|e| e.clave == clave) else {
            return Task::none();
        };
        let e = self.espacios.remove(i);
        if let Some(c) = &e.cliente
            && let Err(e) = login::olvidar_en(&self.archivo, &c.credenciales().token)
        {
            self.aviso = Some(format!("No pude borrar la sesión guardada: {e}"));
        }
        if self.espacios.is_empty() {
            self.activo = 0;
            return Task::none();
        }
        if i < self.activo || self.activo >= self.espacios.len() {
            self.activo -= 1;
        }
        if i != self.activo && self.espacios[self.activo].visible {
            return Task::none();
        }
        let clave = self.espacios[self.activo].clave;
        self.espacios[self.activo]
            .mostrar()
            .map(move |m| Global::Espacio(clave, m))
    }

    /// Abre el workspace de una sesión recién iniciada y lo pone a la vista. Si ya
    /// estaba abierto (mismo workspace o token), lo reemplaza en su lugar.
    fn abrir(&mut self, cred: Credenciales) -> Task<Global> {
        // El aviso de credenciales inválidas del entorno deja de hacer falta.
        if let [e] = self.espacios.as_slice()
            && e.cliente.is_none()
            && e.error.is_some()
        {
            self.espacios.clear();
        }
        let mismo = self.espacios.iter().position(|e| {
            e.cliente.as_ref().is_some_and(|c| {
                let c = c.credenciales();
                c.token == cred.token || c.equipo.is_some() && c.equipo == cred.equipo
            })
        });
        let (nuevo, tarea) = self.agregar(cred);
        let i = match mismo {
            Some(i) => {
                // Clave nueva: lo que siga llegando de la sesión anterior se descarta.
                self.espacios.swap_remove(i);
                i
            }
            None => nuevo,
        };
        for e in &mut self.espacios {
            e.visible = false;
        }
        self.espacios[i].visible = true;
        self.activo = i;
        tarea
    }

    fn activar(&mut self, i: usize) -> Task<Global> {
        if i >= self.espacios.len() || i == self.activo {
            return Task::none();
        }
        self.espacios[self.activo].visible = false;
        self.activo = i;
        let clave = self.espacios[i].clave;
        self.espacios[i]
            .mostrar()
            .map(move |m| Global::Espacio(clave, m))
    }
}

/// Atajos de teclado que van al workspace visible: Ctrl+K, y ↑ ↓ Esc para las capas
/// (Ctrl+K, selector de emoji); sin capa abierta el workspace los ignora. Intro lo
/// maneja el propio campo.
fn atajo_espacio(
    key: &keyboard::Key,
    fisica: keyboard::key::Physical,
    modifiers: keyboard::Modifiers,
) -> Option<Msg> {
    use keyboard::key::Named;
    if modifiers.command() && key.to_latin(fisica) == Some('k') {
        return Some(Msg::AbrirSaltar);
    }
    match key {
        keyboard::Key::Named(Named::ArrowUp) => Some(Msg::Flecha(-1)),
        keyboard::Key::Named(Named::ArrowDown) => Some(Msg::Flecha(1)),
        keyboard::Key::Named(Named::Escape) => Some(Msg::Esc),
        _ => None,
    }
}

/// La tecla en los términos de `atajos`. Con Shift, los números se leen de la tecla
/// física (Ctrl + Shift + 2 da `@` en el teclado de EE. UU.).
fn tecla_de_atajo(key: &keyboard::Key, fisica: keyboard::key::Physical) -> Option<atajos::Tecla> {
    use atajos::Tecla;
    use keyboard::key::{Code, Named, Physical};
    let digito = match fisica {
        Physical::Code(Code::Digit0) => Some('0'),
        Physical::Code(Code::Digit1) => Some('1'),
        Physical::Code(Code::Digit2) => Some('2'),
        Physical::Code(Code::Digit3) => Some('3'),
        Physical::Code(Code::Digit4) => Some('4'),
        Physical::Code(Code::Digit5) => Some('5'),
        Physical::Code(Code::Digit6) => Some('6'),
        Physical::Code(Code::Digit7) => Some('7'),
        Physical::Code(Code::Digit8) => Some('8'),
        Physical::Code(Code::Digit9) => Some('9'),
        _ => None,
    };
    Some(match key {
        keyboard::Key::Named(Named::ArrowUp) => Tecla::Arriba,
        keyboard::Key::Named(Named::ArrowDown) => Tecla::Abajo,
        keyboard::Key::Named(Named::ArrowLeft) => Tecla::Izquierda,
        keyboard::Key::Named(Named::ArrowRight) => Tecla::Derecha,
        keyboard::Key::Named(Named::Escape) => Tecla::Esc,
        keyboard::Key::Named(Named::Tab) => Tecla::Tab,
        keyboard::Key::Character(_) => Tecla::Caracter(digito.or(key.to_latin(fisica))?),
        _ => return None,
    })
}

/// Con `SLACK_CAPTURA=ruta.png` guarda una captura de la ventana a los 2 s y sale;
/// sirve para revisar la interfaz sin mirar la pantalla.
fn capturar_si_se_pide() -> Task<Global> {
    if std::env::var_os("SLACK_CAPTURA").is_none() {
        return Task::none();
    }
    Task::future(tokio::time::sleep(std::time::Duration::from_secs(2)))
        .then(|_| iced::window::latest())
        .and_then(iced::window::screenshot)
        .map(Global::Captura)
}
#[cfg(test)]
mod tests {
    use super::*;
    use keyboard::key::{Code, Named, Physical};
    use keyboard::{Key, Modifiers};

    fn letra(c: &str) -> (Key, Physical) {
        let codigo = match c {
            "k" => Code::KeyK,
            _ => Code::KeyJ,
        };
        (Key::Character(c.into()), Physical::Code(codigo))
    }

    fn nombrada(n: Named) -> (Key, Physical) {
        (Key::Named(n), Physical::Code(Code::Fn))
    }

    fn atajo((key, fisica): (Key, Physical), m: Modifiers) -> Option<Msg> {
        atajo_espacio(&key, fisica, m)
    }

    fn cred(equipo: &str, token: &str) -> Credenciales {
        Credenciales {
            token: token.into(),
            cookie: "xoxd-x".into(),
            equipo: Some(equipo.into()),
        }
    }

    /// App con estas sesiones que guarda las nuevas en una carpeta temporal propia.
    fn app(creds: Vec<Credenciales>) -> (Slack, std::path::PathBuf) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let dir = std::env::temp_dir().join(format!("slack-rs-app-{}-{nanos}", std::process::id()));
        let (slack, _) = Slack::con(creds, dir.join("credenciales.json"));
        (slack, dir)
    }

    fn tokens(s: &Slack) -> Vec<String> {
        s.espacios
            .iter()
            .map(|e| e.cliente.as_ref().unwrap().credenciales().token.clone())
            .collect()
    }

    #[test]
    fn sin_sesiones_arranca_en_la_bienvenida() {
        let (s, _) = app(vec![]);
        assert!(s.espacios.is_empty());
        assert_eq!(s.titulo(), "Slack");
        assert_eq!(s.inicio, Inicio::Libre);
    }

    #[test]
    fn iniciar_sesion_espera_una_sola_vez() {
        let (mut s, _) = app(vec![]);
        let _ = s.update(Global::IniciarSesion);
        assert_eq!(s.inicio, Inicio::Esperando);
        let _ = s.update(Global::IniciarSesion);
        assert_eq!(s.inicio, Inicio::Esperando);
    }

    #[test]
    fn la_sesion_nueva_se_suma_se_muestra_y_se_guarda() {
        let (mut s, dir) = app(vec![cred("T1", "xoxc-a")]);
        let _ = s.update(Global::IniciarSesion);
        let _ = s.update(Global::SesionIniciada(Ok(cred("T2", "xoxc-b"))));
        assert_eq!(tokens(&s), ["xoxc-a", "xoxc-b"]);
        assert_eq!(s.activo, 1);
        assert!(s.espacios[1].visible && !s.espacios[0].visible);
        assert_eq!(s.inicio, Inicio::Libre);
        let guardadas = login::cargar_de(&dir.join("credenciales.json"));
        assert_eq!(guardadas, [cred("T2", "xoxc-b")]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn volver_a_entrar_al_mismo_workspace_lo_reemplaza() {
        let (mut s, dir) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let _ = s.update(Global::SesionIniciada(Ok(cred("T1", "xoxc-a2"))));
        assert_eq!(tokens(&s), ["xoxc-a2", "xoxc-b"]);
        assert_eq!(s.activo, 0);
        assert!(s.espacios[0].visible);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn la_primera_sesion_sale_de_la_bienvenida() {
        let (mut s, dir) = app(vec![]);
        let _ = s.update(Global::SesionIniciada(Ok(cred("T1", "xoxc-a"))));
        assert_eq!(tokens(&s), ["xoxc-a"]);
        assert_eq!(s.activo, 0);
        assert!(s.espacios[0].visible);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn la_sesion_reemplaza_el_aviso_de_entorno_invalido() {
        let (mut s, dir) = app(vec![]);
        s.espacios
            .push(Espacio::con_error("falta SLACK_COOKIE".into()));
        let _ = s.update(Global::SesionIniciada(Ok(cred("T1", "xoxc-a"))));
        assert_eq!(tokens(&s), ["xoxc-a"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn si_falla_queda_el_aviso_y_lo_abierto() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a")]);
        let _ = s.update(Global::IniciarSesion);
        let _ = s.update(Global::SesionIniciada(Err("se cerró la ventana".into())));
        assert_eq!(tokens(&s), ["xoxc-a"]);
        assert_eq!(s.inicio, Inicio::Libre);
        assert_eq!(s.aviso.as_deref(), Some("se cerró la ventana"));
        let _ = s.update(Global::CerrarAviso);
        assert_eq!(s.aviso, None);
    }

    fn horchata() -> paleta::Tema {
        paleta::Tema {
            primaria: "horchata".into(),
            resalte: "jade".into(),
        }
    }

    #[test]
    fn los_colores_siguen_al_workspace_activo() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        s.espacios[1].tema = Some(horchata());
        let aubergine = estilo::colores(&paleta::aubergine(), false);
        assert_eq!(s.colores(), aubergine);
        let _ = s.update(Global::Activar(1));
        let cafe = estilo::colores(&paleta::del_tema(Some(&horchata()), false), false);
        assert_eq!(s.colores(), cafe);
    }

    fn tecla(c: &str, modifiers: Modifiers) -> Global {
        pulsar(Key::Character(c.into()), Code::KeyY, modifiers)
    }

    fn pulsar(key: Key, codigo: Code, modifiers: Modifiers) -> Global {
        Global::Tecla(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: Physical::Code(codigo),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    fn especial(n: Named, modifiers: Modifiers) -> Global {
        pulsar(Key::Named(n), Code::Fn, modifiers)
    }

    #[test]
    fn soltar_un_archivo_lo_adjunta_al_workspace_activo() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let _ = s.update(Global::Activar(1));
        let ruta = PathBuf::from("/tmp/plan.pdf");
        let _ = s.update(Global::Soltado(ruta.clone()));
        assert_eq!(s.espacios[1].adjuntos_de(espacio::Destino::Canal), [ruta]);
        assert!(s.espacios[0].adjuntos.is_empty());
    }

    #[test]
    fn pausar_avisos_llega_a_todos_y_se_recuerda() {
        let (mut s, dir) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let _ = s.update(Global::Pausar);
        assert!(s.ajustes.pausado && s.espacios.iter().all(|e| e.ajustes.pausado));
        let (otra, _) = Slack::con(vec![cred("T1", "xoxc-a")], dir.join("credenciales.json"));
        assert!(otra.espacios[0].ajustes.pausado, "se lee de prefs.json");
        let _ = s.update(Global::Pausar);
        assert!(s.espacios.iter().all(|e| !e.ajustes.pausado));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn el_foco_de_la_ventana_llega_a_todos() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let _ = s.update(Global::Foco(false));
        assert!(s.espacios.iter().all(|e| e.desenfocada));
        let _ = s.update(Global::Foco(true));
        assert!(s.espacios.iter().all(|e| !e.desenfocada));
    }

    #[test]
    fn clic_en_un_aviso_trae_su_workspace_y_su_canal() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let clave = s.espacios[1].clave;
        let _ = s.update(Global::Espacio(clave, Msg::AvisoClic("D7".into())));
        assert_eq!(s.activo, 1);
        assert_eq!(s.espacios[1].seleccionado.as_deref(), Some("D7"));
    }

    #[test]
    fn ajustar_llega_a_todos_y_se_guarda() {
        let (mut s, dir) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let _ = s.update(Global::Ajustar(Cambio::Reloj(ajustes::Reloj::Doce)));
        let _ = s.update(Global::Ajustar(Cambio::Compacta(true)));
        assert!(
            s.espacios
                .iter()
                .all(|e| e.ajustes.reloj == ajustes::Reloj::Doce && e.ajustes.compacta)
        );
        let (otra, _) = Slack::con(vec![], dir.join("credenciales.json"));
        assert_eq!(otra.ajustes.reloj, ajustes::Reloj::Doce);
        assert!(otra.ajustes.compacta);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn el_tema_elegido_gana_sobre_el_del_workspace() {
        let (mut s, dir) = app(vec![cred("T1", "xoxc-a")]);
        s.espacios[0].tema = Some(horchata());
        let _ = s.update(Global::Ajustar(Cambio::Tema(Some("clasico".into()))));
        assert_eq!(s.colores(), estilo::colores(&paleta::aubergine(), false));
        let _ = s.update(Global::Ajustar(Cambio::Tema(Some("ocean".into()))));
        let oceano = paleta::Tema {
            primaria: "ocean".into(),
            resalte: "jade".into(),
        };
        let esperado = estilo::colores(&paleta::del_tema(Some(&oceano), false), false);
        assert_eq!(s.colores(), esperado);
        let _ = s.update(Global::Ajustar(Cambio::Tema(None)));
        let cafe = estilo::colores(&paleta::del_tema(Some(&horchata()), false), false);
        assert_eq!(s.colores(), cafe, "sin elegir, el del workspace");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ctrl_coma_abre_preferencias_y_esc_las_cierra() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a")]);
        let _ = s.update(tecla(",", Modifiers::CTRL));
        assert!(s.ver_preferencias);
        let _ = s.update(especial(Named::Escape, Modifiers::empty()));
        assert!(!s.ver_preferencias);
    }

    #[test]
    fn una_sesion_vencida_deja_de_reintentar() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let (uno, dos) = (s.espacios[0].clave, s.espacios[1].clave);
        assert_eq!(s.con_tiempo_real(), [uno, dos]);
        let caida = rtm::Evento::Desconectado("rtm.connect: invalid_auth".into());
        let _ = s.update(Global::Espacio(dos, Msg::Rtm(Box::new(caida))));
        assert_eq!(s.con_tiempo_real(), [uno]);
    }

    #[test]
    fn ctrl_tab_rota_los_workspaces() {
        let creds = vec![
            cred("T1", "xoxc-a"),
            cred("T2", "xoxc-b"),
            cred("T3", "xoxc-c"),
        ];
        let (mut s, _) = app(creds);
        let _ = s.update(especial(Named::Tab, Modifiers::CTRL));
        assert_eq!(s.activo, 1);
        let _ = s.update(especial(Named::Tab, Modifiers::CTRL | Modifiers::SHIFT));
        let _ = s.update(especial(Named::Tab, Modifiers::CTRL | Modifiers::SHIFT));
        assert_eq!(s.activo, 2, "da la vuelta");
    }

    #[test]
    fn alt_izquierda_vuelve_en_el_workspace_activo() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let _ = s.update(Global::Activar(1));
        let clave = s.espacios[1].clave;
        let _ = s.update(Global::Espacio(clave, Msg::Seleccionar("C1".into())));
        let _ = s.update(Global::Espacio(clave, Msg::Seleccionar("C2".into())));
        let _ = s.update(especial(Named::ArrowLeft, Modifiers::ALT));
        assert_eq!(s.espacios[1].seleccionado.as_deref(), Some("C1"));
        assert_eq!(s.espacios[0].seleccionado, None);
    }

    #[test]
    fn ctrl_shift_2_abre_los_mensajes_directos() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a")]);
        let arroba = Key::Character("@".into());
        let _ = s.update(pulsar(
            arroba,
            Code::Digit2,
            Modifiers::CTRL | Modifiers::SHIFT,
        ));
        assert_eq!(s.espacios[0].vista, espacio::Vista::Directos);
        assert_eq!(s.activo, 0, "no es Ctrl + 2");
    }

    #[test]
    fn ctrl_shift_3_abre_actividad() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a")]);
        let tres = Key::Character("#".into());
        let _ = s.update(pulsar(
            tres,
            Code::Digit3,
            Modifiers::CTRL | Modifiers::SHIFT,
        ));
        assert_eq!(s.espacios[0].panel, Some(espacio::Panel::Actividad));
    }

    #[test]
    fn ctrl_mas_menos_y_cero_cambian_el_tamano_y_se_recuerda() {
        let (mut s, dir) = app(vec![]);
        assert_eq!(s.ajustes.escala, 1.0);
        let _ = s.update(tecla("+", Modifiers::CTRL));
        let _ = s.update(tecla("=", Modifiers::CTRL));
        assert!((s.ajustes.escala - 1.2).abs() < 1e-3);
        let (otra, _) = Slack::con(vec![], dir.join("credenciales.json"));
        assert!((otra.ajustes.escala - 1.2).abs() < 1e-3);
        for _ in 0..20 {
            let _ = s.update(tecla("-", Modifiers::CTRL));
        }
        assert!((s.ajustes.escala - ESCALA_MIN).abs() < 1e-3, "tiene tope");
        let _ = s.update(tecla("0", Modifiers::CTRL));
        assert_eq!(s.ajustes.escala, 1.0);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ctrl_barra_muestra_los_atajos_y_esc_los_cierra() {
        let (mut s, _) = app(vec![cred("T1", "xoxc-a")]);
        let _ = s.update(tecla("/", Modifiers::CTRL));
        assert!(s.ver_atajos);
        let _ = s.update(especial(Named::Escape, Modifiers::empty()));
        assert!(!s.ver_atajos);
    }

    #[test]
    fn cerrar_sesion_quita_el_workspace_y_lo_olvida() {
        let (mut s, dir) = app(vec![cred("T1", "xoxc-a"), cred("T2", "xoxc-b")]);
        let ruta = dir.join("credenciales.json");
        for c in [cred("T1", "xoxc-a"), cred("T2", "xoxc-b")] {
            login::recordar_en(&ruta, c).unwrap();
        }
        let _ = s.update(Global::Activar(1));
        let cerrada = s.espacios[0].clave;
        let _ = s.update(Global::CerrarSesion(cerrada));
        assert_eq!(tokens(&s), ["xoxc-b"]);
        assert_eq!(s.activo, 0);
        assert!(s.espacios[0].visible);
        assert_eq!(login::cargar_de(&ruta), [cred("T2", "xoxc-b")]);
        // Lo que llegue tarde del workspace cerrado no cae en otro.
        let sesion = slack::Sesion {
            user_id: "U9".into(),
            team: "Cerrado".into(),
        };
        let _ = s.update(Global::Espacio(cerrada, Msg::Sesion(Ok(sesion))));
        assert!(s.espacios[0].sesion.is_none());
        let ultima = s.espacios[0].clave;
        let _ = s.update(Global::CerrarSesion(ultima));
        assert!(s.espacios.is_empty(), "vuelve a la bienvenida");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ctrl_shift_y_alterna_el_modo_oscuro_y_se_recuerda() {
        let (mut s, dir) = app(vec![cred("T1", "xoxc-a")]);
        assert!(!s.ajustes.oscuro);
        let _ = s.update(tecla("Y", Modifiers::CTRL | Modifiers::SHIFT));
        assert!(s.ajustes.oscuro);
        assert_eq!(
            s.colores().fondo,
            estilo::colores(&paleta::aubergine(), true).fondo
        );
        let (otra, _) = Slack::con(vec![], dir.join("credenciales.json"));
        assert!(otra.ajustes.oscuro, "se lee de prefs.json al arrancar");
        let _ = s.update(tecla("Y", Modifiers::CTRL | Modifiers::SHIFT));
        assert!(!s.ajustes.oscuro);
        let _ = s.update(tecla("y", Modifiers::CTRL));
        assert!(!s.ajustes.oscuro, "sin Shift no");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ctrl_k_abre_saltar() {
        assert!(matches!(
            atajo(letra("k"), Modifiers::CTRL),
            Some(Msg::AbrirSaltar)
        ));
        assert!(atajo(letra("k"), Modifiers::empty()).is_none());
        assert!(atajo(letra("j"), Modifiers::CTRL).is_none());
    }

    #[test]
    fn flechas_y_esc() {
        let abajo = atajo(nombrada(Named::ArrowDown), Modifiers::empty());
        assert!(matches!(abajo, Some(Msg::Flecha(1))));
        let arriba = atajo(nombrada(Named::ArrowUp), Modifiers::empty());
        assert!(matches!(arriba, Some(Msg::Flecha(-1))));
        let esc = atajo(nombrada(Named::Escape), Modifiers::empty());
        assert!(matches!(esc, Some(Msg::Esc)));
        assert!(atajo(nombrada(Named::Enter), Modifiers::empty()).is_none());
    }
}
