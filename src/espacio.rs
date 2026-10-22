//! Estado de un workspace y su `update`: todo lo que no es pintar ni enrutar.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use iced::Task;
use iced::widget::{image, text_editor};

use crate::ajustes::{Ajustes, Reloj};
use crate::borradores::{self, Borradores};
use crate::saltar::{self, Candidato};
use crate::slack::{
    Archivo, BorradorSlack, Canal, Cliente, Coincidencia, Contador, Credenciales, ElementoRico,
    EnLinea, Equipo, InfoCanal, ItemActividad, Marcador, Mensaje, NivelAviso, Pagina, Prefs,
    Resultado, ResumenHilo, Seccion, Sesion, TipoSeccion, Usuario,
};
use crate::sugerencias::{self, Disparador, OpcionEmoji};
use crate::{atajos, avisos, escritorio, formato, lateral, navegacion, paleta, rtm, textos};

/// Reacciones rápidas de la barra que aparece al pasar sobre un mensaje.
pub(crate) const REACCIONES_RAPIDAS: [&str; 3] = ["+1", "white_check_mark", "eyes"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Destino {
    Canal,
    Hilo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Vista {
    #[default]
    Inicio,
    Directos,
}

/// Búsqueda de mensajes abierta en el centro, en lugar del canal.
pub(crate) struct Busqueda {
    pub(crate) consulta: String,
    /// `None` mientras carga.
    pub(crate) resultados: Option<Vec<Coincidencia>>,
}

/// Un grupo de la lateral: una sección del usuario o una de las dos fijas.
pub(crate) struct GrupoLateral<'a> {
    pub(crate) id: String,
    pub(crate) titulo: String,
    pub(crate) emoji: Option<String>,
    pub(crate) canales: Vec<&'a Canal>,
}

/// La ventana de Ctrl+K abierta.
pub(crate) struct Saltar {
    pub(crate) consulta: String,
    /// Índice dentro de [`Espacio::opciones_saltar`].
    pub(crate) elegido: usize,
}

/// Para qué se abrió el selector de emoji.
#[derive(Debug, Clone)]
pub(crate) enum DestinoEmoji {
    Reaccion { canal: String, ts: String },
    Compositor(Destino),
}

/// Pestaña bajo el encabezado del canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Pestana {
    #[default]
    Mensajes,
    Archivos,
    Fijados,
}

/// Vista que ocupa el centro en lugar de un canal: las de la lateral y del rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Hilos,
    Actividad,
    Borradores,
    Guardados,
    Archivos,
    Directorio,
}

/// Pestaña de la ventana de detalles del canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Detalles {
    Acerca,
    Miembros,
}

pub(crate) struct SelectorEmoji {
    pub(crate) destino: DestinoEmoji,
    pub(crate) consulta: String,
}

pub(crate) struct Edicion {
    pub(crate) canal: String,
    pub(crate) ts: String,
    pub(crate) editor: text_editor::Content,
}

/// Sugerencias abiertas en un compositor para la palabra que se está escribiendo.
pub(crate) struct Autocompletado {
    pub(crate) destino: Destino,
    pub(crate) tipo: Disparador,
    /// Línea y bytes `[inicio, fin)` a reemplazar al elegir.
    linea: usize,
    inicio: usize,
    fin: usize,
    pub(crate) opciones: Vec<Sugerencia>,
    pub(crate) elegido: usize,
}

pub(crate) struct Sugerencia {
    /// Lo que se ve en la lista: el nombre o el código del emoji.
    pub(crate) etiqueta: String,
    /// Usuario del que es el avatar, si es una mención.
    pub(crate) usuario: Option<String>,
    /// Lo que se escribe en el compositor.
    insercion: String,
    /// Cómo se manda a Slack, si cambia.
    marca: Option<String>,
}

/// ID del editor de un mensaje propio, para darle el foco.
pub(crate) const ID_EDICION: &str = "edicion";

/// ID del campo de búsqueda del selector de emoji.
pub(crate) const ID_EMOJI: &str = "emoji";

/// ID del campo de Ctrl+K, para darle el foco al abrirlo.
pub(crate) const ID_SALTAR: &str = "saltar";

/// ID del buscador de la barra superior, para "buscar en el canal".
pub(crate) const ID_BUSCADOR: &str = "buscador";

pub(crate) struct Hilo {
    pub(crate) canal: String,
    pub(crate) ts: String,
    /// Raíz seguida de respuestas; `None` mientras carga.
    pub(crate) mensajes: Option<Vec<Mensaje>>,
    pub(crate) editor: text_editor::Content,
    /// "Enviar también a #canal".
    pub(crate) tambien: bool,
}

/// Un workspace con su propia sesión, canales, tiempo real y canal abierto.
#[derive(Default)]
pub(crate) struct Espacio {
    /// Identifica al workspace en los mensajes de la app aunque cambie de lugar en el
    /// rail; la asigna `Slack`.
    pub(crate) clave: u64,
    pub(crate) cliente: Option<Cliente>,
    pub(crate) error: Option<String>,
    pub(crate) sesion: Option<Sesion>,
    pub(crate) equipo: Option<Equipo>,
    pub(crate) icono_equipo: Option<image::Handle>,
    pub(crate) canales: Vec<Canal>,
    pub(crate) contadores: HashMap<String, Contador>,
    /// Segundos del último mensaje de cada conversación, para ordenar los DMs.
    pub(crate) actividad: HashMap<String, i64>,
    pub(crate) usuarios: HashMap<String, Usuario>,
    pub(crate) usuarios_pedidos: HashSet<String>,
    pub(crate) avatares: HashMap<String, image::Handle>,
    /// Emoji propios del workspace: nombre → URL.
    pub(crate) emoji_propios: HashMap<String, String>,
    pub(crate) emoji_imagenes: HashMap<String, image::Handle>,
    pub(crate) emoji_pedidos: HashSet<String>,
    /// Miniaturas de imágenes adjuntas, por ID de archivo.
    pub(crate) miniaturas: HashMap<String, image::Handle>,
    pub(crate) miniaturas_pedidas: HashSet<String>,
    pub(crate) mensajes: HashMap<String, Vec<Mensaje>>,
    /// Si cada canal cargado tiene mensajes más viejos que pedir.
    pub(crate) hay_mas: HashMap<String, bool>,
    /// Canales a los que ya se les pidió la página anterior.
    pub(crate) cargando_mas: HashSet<String>,
    pub(crate) seleccionado: Option<String>,
    pub(crate) editor: text_editor::Content,
    pub(crate) hilo: Option<Hilo>,
    /// `ts` del mensaje bajo el mouse, para la barra de acciones.
    pub(crate) sobre: Option<String>,
    pub(crate) filtro: String,
    pub(crate) busqueda: Option<Busqueda>,
    pub(crate) saltar: Option<Saltar>,
    pub(crate) selector_emoji: Option<SelectorEmoji>,
    pub(crate) autocompletado: Option<Autocompletado>,
    /// Mensaje propio que se está editando en su lugar.
    pub(crate) edicion: Option<Edicion>,
    /// Canal y `ts` del mensaje propio que espera confirmación para borrarse.
    pub(crate) por_borrar: Option<(String, String)>,
    /// Lo que el autocompletado insertó a la vista y cómo se manda a Slack:
    /// (`@Ana Ruiz`, `<@U5>`).
    pub(crate) traducciones: Vec<(String, String)>,
    pub(crate) vista: Vista,
    /// IDs de las secciones de la lateral que están plegadas.
    pub(crate) plegadas: HashSet<String>,
    /// Secciones del usuario; `None` si Slack no las dio y se usan las dos fijas.
    pub(crate) secciones: Option<Vec<Seccion>>,
    pub(crate) silenciados: HashSet<String>,
    /// Tema elegido en Slack; `None` usa Aubergine.
    pub(crate) tema: Option<paleta::Tema>,
    /// Menú ⚙ de la lateral ("Filter and sort").
    pub(crate) menu_lateral: bool,
    pub(crate) filtro_lateral: lateral::Filtro,
    pub(crate) orden_lateral: lateral::Orden,
    pub(crate) ocultar_silenciados: bool,
    /// Texto de "Buscar conversación", que filtra la lateral por nombre.
    pub(crate) buscar_conversacion: String,
    pub(crate) historial: navegacion::Historial,
    pub(crate) menu_recientes: bool,
    /// Datos del encabezado y de los detalles, por canal.
    pub(crate) info: HashMap<String, InfoCanal>,
    pub(crate) miembros: HashMap<String, Vec<String>>,
    pub(crate) marcadores: HashMap<String, Vec<Marcador>>,
    pub(crate) archivos_canal: HashMap<String, Vec<Archivo>>,
    pub(crate) fijados: HashMap<String, Vec<Mensaje>>,
    pub(crate) pestana: Pestana,
    pub(crate) detalles: Option<Detalles>,
    /// Menú ⋮ del encabezado.
    pub(crate) menu_canal: bool,
    /// Menú 🔔 del encabezado.
    pub(crate) menu_avisos: bool,
    /// Canales en "Todos los mensajes".
    pub(crate) avisar_todo: HashSet<String>,
    pub(crate) panel: Option<Panel>,
    pub(crate) hilos_vista: Option<Vec<ResumenHilo>>,
    /// Bandeja Actividad: menciones e hilos.
    pub(crate) bandeja: Option<Vec<ItemActividad>>,
    pub(crate) guardados: Option<Vec<(String, String)>>,
    pub(crate) borradores_slack: Option<Vec<BorradorSlack>>,
    pub(crate) archivos_equipo: Option<Vec<Archivo>>,
    pub(crate) personas: Option<Vec<Usuario>>,
    pub(crate) consulta_personas: String,
    /// Mensajes sueltos de Actividad y Guardados, por canal y `ts`.
    pub(crate) sueltos: HashMap<(String, String), Mensaje>,
    /// Persona cuyo perfil se ve en el panel derecho.
    pub(crate) perfil: Option<String>,
    /// `ts` del mensaje con la barra de acciones abierta del todo (⋮).
    pub(crate) menu_mensaje: Option<String>,
    /// Archivos esperando el próximo envío, por compositor.
    pub(crate) adjuntos: Vec<(Destino, std::path::PathBuf)>,
    /// Menú ▾ de enviar abierto en ese compositor.
    pub(crate) menu_enviar: Option<Destino>,
    /// Aviso que no es error ("Mensaje programado…").
    pub(crate) nota: Option<String>,
    /// Slack rechazó la sesión (`invalid_auth`…): no se reintenta y se pide entrar de nuevo.
    pub(crate) sesion_vencida: bool,
    /// Conexión de tiempo real abierta, para mandarle cosas.
    pub(crate) enviador: Option<rtm::Enviador>,
    /// Personas en línea (punto verde).
    pub(crate) presentes: HashSet<String>,
    /// Cuándo se avisó por última vez que escribes.
    pub(crate) aviso_escribiendo: Option<std::time::Instant>,
    /// Preferencias de la app; las fija `Slack` (🌙, reloj, densidad…).
    pub(crate) ajustes: Ajustes,
    /// La ventana no tiene el foco; lo fija `Slack`.
    pub(crate) desenfocada: bool,
    /// Quién escribe y dónde, con la hora del último aviso.
    pub(crate) escribiendo: Vec<(String, String, std::time::Instant)>,
    /// Barra de formato del compositor oculta con Aa.
    pub(crate) sin_barra: bool,
    /// Hora como "6:18 PM" (preferencia `time24` apagada).
    pub(crate) reloj_12: bool,
    /// Lo escrito y no enviado en cada conversación e hilo que se dejó.
    pub(crate) borradores: Borradores,
    pub(crate) tiempo_real: bool,
    /// El tiempo real se cayó después de haber estado conectado: al volver hay que
    /// pedir lo que llegó mientras tanto.
    pub(crate) hueco: bool,
    /// Si es el workspace que se ve; uno en segundo plano no marca nada como leído.
    pub(crate) visible: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum Msg {
    Sesion(Resultado<Sesion>),
    Equipo(Resultado<Equipo>),
    IconoEquipo(Resultado<Vec<u8>>),
    Canales(Resultado<Vec<Canal>>),
    Contadores(Resultado<HashMap<String, Contador>>),
    Usuarios(Vec<Usuario>),
    Avatar(String, Resultado<Vec<u8>>),
    EmojiPropios(Resultado<HashMap<String, String>>),
    EmojiImagen(String, Resultado<Vec<u8>>),
    Miniatura(String, Resultado<Vec<u8>>),
    Seleccionar(String),
    Historial(String, Resultado<Pagina>),
    /// El historial de un canal se movió; distancia en px al tope de la lista.
    Desplazado(String, f32),
    /// Mensajes anteriores a los cargados.
    Anteriores(String, Resultado<Pagina>),
    /// Lo que llegó a un canal con el tiempo real caído.
    Posteriores(String, Resultado<Vec<Mensaje>>),
    AbrirHilo(String, String),
    CerrarHilo,
    Respuestas(String, Resultado<Vec<Mensaje>>),
    Filtro(String),
    Buscar,
    Resultados(String, Resultado<Vec<Coincidencia>>),
    CerrarBusqueda,
    AbrirSaltar,
    SaltarConsulta(String),
    SaltarElegir,
    /// ↑ ↓ con una capa abierta.
    Flecha(isize),
    /// Esc: cierra la capa que esté abierta.
    Esc,
    AbrirEmoji(DestinoEmoji),
    EmojiConsulta(String),
    ElegirEmoji(String),
    /// Intro o Tab con el autocompletado abierto.
    ElegirSugerencia,
    /// Clic en una sugerencia.
    SugerenciaEn(usize),
    EmpezarEdicion(String, String),
    /// ↑ con el compositor vacío: editar el último mensaje propio.
    EditarUltimo(Destino),
    EditarEdicion(text_editor::Action),
    GuardarEdicion,
    PedirBorrar(String, String),
    ConfirmarBorrar,
    Vista(Vista),
    Plegar(String),
    /// Alt + ← / →.
    Atras,
    Adelante,
    /// Alt + ↑ / ↓: la conversación de arriba o de abajo en la lateral.
    Vecino(i32),
    /// Alt + Shift + ↑ / ↓: la siguiente con no leídos.
    VecinoSinLeer(i32),
    /// Shift + Esc.
    MarcarTodoLeido,
    /// 🕘 de la barra superior.
    MenuRecientes,
    InfoCanal(String, Resultado<InfoCanal>),
    MiembrosCanal(String, Resultado<Vec<String>>),
    Marcadores(String, Resultado<Vec<Marcador>>),
    ArchivosCanal(String, Resultado<Vec<Archivo>>),
    FijadosCanal(String, Resultado<Vec<Mensaje>>),
    Pestana(Pestana),
    AbrirDetalles(Detalles),
    CerrarDetalles,
    MenuCanal,
    CopiarEnlaceCanal,
    /// 🔍 del encabezado: buscar mensajes en el canal abierto.
    BuscarEnCanal,
    /// ⋮ de la barra de acciones de un mensaje.
    MenuMensaje(String),
    CopiarEnlaceMensaje(String, String),
    CopiarTexto(String, String),
    MarcarNoLeido(String, String),
    Fijar(String, String),
    /// Guardar para después, o quitar de guardados.
    Guardar(String, String),
    /// ☆ del encabezado.
    Destacar(String),
    /// 🔔 del encabezado: el menú y la opción elegida.
    MenuAvisos,
    AvisosCanal(String, NivelAviso),
    MenuLateral,
    FiltrarLateral(lateral::Filtro),
    OrdenarLateral(lateral::Orden),
    AlternarSilenciados,
    BuscarConversacion(String),
    Secciones(Resultado<Vec<Seccion>>),
    Prefs(Resultado<Prefs>),
    Editar(Destino, text_editor::Action),
    Formatear(Destino, atajos::Formato),
    /// Aa: muestra u oculta la barra de formato.
    AlternarBarra,
    /// Casilla "Enviar también a #canal" del hilo.
    AlternarTambien,
    /// Reloj para olvidar a quien dejó de escribir.
    Tick,
    /// Clic en una notificación de escritorio: abre ese canal.
    AvisoClic(String),
    AbrirPanel(Panel),
    HilosVista(Resultado<Vec<ResumenHilo>>),
    ActividadVista(Resultado<Vec<ItemActividad>>),
    Guardados(Resultado<Vec<(String, String)>>),
    MensajesSueltos(Resultado<Vec<(String, Mensaje)>>),
    BorradoresSlack(Resultado<Vec<BorradorSlack>>),
    ArchivosEquipo(Resultado<Vec<Archivo>>),
    BuscarPersonas(String),
    /// Resultado de buscar personas, con la consulta que lo pidió.
    Personas(String, Resultado<Vec<Usuario>>),
    /// Abrir la DM con alguien del directorio o del perfil.
    AbrirDm(String),
    DmAbierto(String, Resultado<String>),
    VerPerfil(String),
    CerrarPerfil,
    /// ▾ junto al botón de enviar.
    MenuEnviar(Destino),
    /// Programar lo escrito para ese momento (segundos Unix), con su etiqueta.
    Programar(Destino, i64, String),
    Programado(Destino, String, String, Resultado<()>),
    CerrarNota,
    /// Archivo para subir con el próximo envío (arrastrado a la ventana o elegido).
    Adjuntar(Destino, std::path::PathBuf),
    /// + del compositor: abre el selector de archivos.
    ElegirArchivos(Destino),
    Elegidos(Destino, Vec<std::path::PathBuf>),
    QuitarAdjunto(Destino, std::path::PathBuf),
    Subido(Destino, String, Vec<std::path::PathBuf>, Resultado<()>),
    Enviar(Destino),
    /// Resultado de enviar, con el texto tal como estaba en el compositor.
    Enviado(Destino, String, Resultado<()>),
    Sobre(String),
    Salir(String),
    Reaccionar {
        canal: String,
        ts: String,
        nombre: String,
    },
    Hecho(Resultado<()>),
    AbrirEnlace(String),
    CerrarError,
    Rtm(Box<rtm::Evento>),
}

impl Espacio {
    pub(crate) fn new(cred: Credenciales) -> (Self, Task<Msg>) {
        let c = Cliente::new(cred);
        let tareas = Task::batch([
            Task::perform(c.clone().sesion(), Msg::Sesion),
            Task::perform(c.clone().equipo(), Msg::Equipo),
            Task::perform(c.clone().canales(), Msg::Canales),
            Task::perform(c.clone().contadores(), Msg::Contadores),
            Task::perform(c.clone().secciones(), Msg::Secciones),
            Task::perform(c.clone().emoji(), Msg::EmojiPropios),
            Task::perform(c.clone().prefs(), Msg::Prefs),
        ]);
        (
            Self {
                cliente: Some(c),
                ..Self::default()
            },
            tareas,
        )
    }

    pub(crate) fn con_error(e: String) -> Self {
        Self {
            error: Some(e),
            ..Self::default()
        }
    }

    /// Al volver a un workspace, el canal que quedó abierto se marca como leído si
    /// llegó algo mientras estaba en segundo plano.
    pub(crate) fn mostrar(&mut self) -> Task<Msg> {
        self.visible = true;
        let ultimo = self
            .seleccionado
            .clone()
            .filter(|c| self.contadores.contains_key(c))
            .and_then(|c| {
                let ts = self.mensajes.get(&c)?.last()?.ts.clone();
                Some((c, ts))
            });
        match ultimo {
            Some((canal, ts)) => self.marcar_leido(canal, ts),
            None => Task::none(),
        }
    }

    /// Menciones y si hay algo sin leer, para la insignia del rail.
    pub(crate) fn pendientes(&self) -> (u32, bool) {
        self.contadores.iter().fold((0, false), |(m, n), (id, c)| {
            (m + c.menciones, n || (c.no_leidos && !self.silenciado(id)))
        })
    }

    pub(crate) fn destacado(&self, canal: &str) -> bool {
        self.secciones
            .iter()
            .flatten()
            .any(|s| s.tipo == TipoSeccion::Destacados && s.canales.iter().any(|c| c == canal))
    }

    pub(crate) fn nivel_aviso(&self, canal: &str) -> NivelAviso {
        if self.silenciado(canal) {
            NivelAviso::Silencio
        } else if self.avisar_todo.contains(canal) {
            NivelAviso::Todo
        } else {
            NivelAviso::Menciones
        }
    }

    /// Según la última lista de guardados y lo que se guardó o quitó desde aquí.
    pub(crate) fn esta_guardado(&self, canal: &str, ts: &str) -> bool {
        self.guardados
            .iter()
            .flatten()
            .any(|(c, t)| c == canal && t == ts)
    }

    /// Un mensaje de Actividad o Guardados, si ya llegó.
    pub(crate) fn suelto(&self, canal: &str, ts: &str) -> Option<&Mensaje> {
        self.sueltos.get(&(canal.to_string(), ts.to_string()))
    }

    /// Pide lo que muestra un panel; cada vez que se abre, para verlo al día.
    fn pedir_panel(&self, panel: Panel) -> Task<Msg> {
        let Some(c) = self.cliente.clone() else {
            return Task::none();
        };
        match panel {
            Panel::Hilos => Task::perform(c.hilos(), Msg::HilosVista),
            Panel::Actividad => Task::perform(c.actividad(), Msg::ActividadVista),
            Panel::Borradores => Task::perform(c.borradores(), Msg::BorradoresSlack),
            Panel::Guardados => Task::perform(c.guardados(), Msg::Guardados),
            Panel::Archivos => Task::perform(c.archivos_equipo(), Msg::ArchivosEquipo),
            Panel::Directorio => {
                let consulta = self.consulta_personas.clone();
                Task::perform(c.personas(consulta.clone()), move |r| {
                    Msg::Personas(consulta.clone(), r)
                })
            }
        }
    }

    /// Pide los mensajes de Actividad y Guardados que todavía no están.
    fn pedir_sueltos(&self, ids: Vec<(String, String)>) -> Task<Msg> {
        let faltan: Vec<_> = ids
            .into_iter()
            .filter(|id| !self.sueltos.contains_key(id))
            .collect();
        match (&self.cliente, faltan.is_empty()) {
            (Some(c), false) => {
                Task::perform(c.clone().mensajes_por_id(faltan), Msg::MensajesSueltos)
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn presente(&self, usuario: &str) -> bool {
        self.presentes.contains(usuario)
    }

    /// La notificación de escritorio que merece un mensaje recién llegado, si alguna.
    pub(crate) fn aviso_para(&self, canal: &str, m: &Mensaje) -> Option<avisos::Aviso> {
        let c = self.canales.iter().find(|c| c.id == canal);
        let es_dm = c.is_some_and(|c| c.es_directo());
        let donde = match c {
            Some(c) if c.is_im => String::new(),
            Some(c) if c.is_mpim => self.nombre_canal(canal),
            _ => format!("#{}", self.nombre_canal(canal)),
        };
        let autor = m
            .user
            .as_deref()
            .map(|u| self.nombre_usuario(u))
            .or(m.username.as_deref())
            .unwrap_or("Alguien");
        let texto = formato::texto_plano(m.cuerpo(), &self.usuarios);
        avisos::debe_avisar(&avisos::Contexto {
            autor,
            donde: &donde,
            texto: &texto,
            propio: m.user.is_some() && m.user.as_deref() == self.mi_id(),
            es_dm,
            me_menciona: self.me_menciona(m) || formato::aviso_general(&m.text),
            a_la_vista: self.visible
                && !self.desenfocada
                && self.seleccionado.as_deref() == Some(canal),
            silenciado: self.silenciado(canal),
            todo: self.avisar_todo.contains(canal),
            pausado: self.ajustes.pausado,
            vista_previa: self.ajustes.vista_previa,
        })
    }

    /// Si abrir o ver el canal debe mandar `conversations.mark`: solo con algo sin leer
    /// y el workspace a la vista, para no escribir de más.
    pub(crate) fn hay_que_marcar(&self, canal: &str) -> bool {
        self.visible && self.contadores.contains_key(canal)
    }

    /// "Fulano está escribiendo…" para los demás, como mucho cada 3 s.
    fn avisar_que_escribes(&mut self, destino: Destino) {
        let canal = match destino {
            Destino::Canal => self.seleccionado.clone(),
            Destino::Hilo => self.hilo.as_ref().map(|h| h.canal.clone()),
        };
        let reciente = self
            .aviso_escribiendo
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(3));
        if let (Some(canal), Some(e), false) = (canal, &mut self.enviador, reciente) {
            let _ = e.0.try_send(rtm::Saliente::Escribiendo(canal));
            self.aviso_escribiendo = Some(std::time::Instant::now());
        }
    }

    /// Pide la presencia de las personas de las DMs (punto verde).
    fn suscribir_presencia(&mut self) {
        let ids: Vec<String> = self
            .canales
            .iter()
            .filter(|c| c.is_im)
            .filter_map(|c| c.user.clone())
            .collect();
        if let (Some(e), false) = (&mut self.enviador, ids.is_empty()) {
            let _ = e.0.try_send(rtm::Saliente::Presencia(ids));
        }
    }

    /// Archivos que se subirán con el próximo envío de ese compositor.
    pub(crate) fn adjuntos_de(&self, destino: Destino) -> Vec<std::path::PathBuf> {
        self.adjuntos
            .iter()
            .filter(|(d, _)| *d == destino)
            .map(|(_, r)| r.clone())
            .collect()
    }

    /// Quién está escribiendo en el canal, en el orden en que empezó.
    pub(crate) fn escribiendo_en(&self, canal: &str) -> Vec<&str> {
        self.escribiendo
            .iter()
            .filter(|(c, _, _)| c == canal)
            .map(|(_, u, _)| u.as_str())
            .collect()
    }

    /// Si se ve la barra de formato del compositor (Aa).
    pub(crate) fn barra_formato(&self) -> bool {
        !self.sin_barra
    }

    /// Si el mensaje te menciona por nombre (no cuenta `@here`): fila resaltada.
    pub(crate) fn me_menciona(&self, m: &Mensaje) -> bool {
        self.mi_id()
            .is_some_and(|yo| formato::menciones(&m.text).any(|u| u == yo))
    }

    /// Formato de `chrono` para la hora de los mensajes.
    pub(crate) fn formato_hora(&self) -> &'static str {
        let doce = match self.ajustes.reloj {
            Reloj::Doce => true,
            Reloj::Veinticuatro => false,
            Reloj::Slack => self.reloj_12,
        };
        if doce { "%-I:%M %p" } else { "%H:%M" }
    }

    /// Enlace permanente de un mensaje ("Copiar enlace").
    pub(crate) fn enlace_mensaje(&self, canal: &str, m: &Mensaje) -> Option<String> {
        let dominio = &self.equipo.as_ref()?.domain;
        if dominio.is_empty() {
            return None;
        }
        let hilo = m.thread_ts.as_deref().filter(|t| *t != m.ts);
        Some(textos::permalink(
            &format!("{dominio}.slack.com"),
            canal,
            &m.ts,
            hilo,
        ))
    }

    /// El mensaje `ts` del canal, en la lista o en el hilo abierto.
    fn buscar_mensaje(&self, canal: &str, ts: &str) -> Option<&Mensaje> {
        let en_hilo = self
            .hilo
            .iter()
            .filter(|h| h.canal == canal)
            .flat_map(|h| h.mensajes.iter().flatten());
        self.mensajes
            .get(canal)
            .into_iter()
            .flatten()
            .chain(en_hilo)
            .find(|m| m.ts == ts)
    }

    /// Enlace del canal para "Copiar enlace", si ya se sabe el dominio del workspace.
    pub(crate) fn enlace_canal(&self, canal: &str) -> Option<String> {
        let dominio = &self.equipo.as_ref()?.domain;
        (!dominio.is_empty()).then(|| format!("https://{dominio}.slack.com/archives/{canal}"))
    }

    /// Conversaciones del menú 🕘, la más reciente primero.
    pub(crate) fn recientes(&self) -> Vec<&str> {
        self.historial.recientes()
    }

    /// Si la conversación tiene texto sin enviar, para el ✎ de la lateral.
    pub(crate) fn tiene_borrador(&self, canal: &str) -> bool {
        self.seleccionado.as_deref() != Some(canal) && self.borradores.tiene(canal)
    }

    /// Guarda lo escrito en el canal y en el hilo abiertos antes de dejarlos.
    fn guardar_borradores(&mut self) {
        if let Some(c) = &self.seleccionado {
            self.borradores.guardar(c, &self.editor.text());
        }
        self.guardar_borrador_hilo();
    }

    fn guardar_borrador_hilo(&mut self) {
        if let Some(h) = &self.hilo {
            let clave = borradores::clave_hilo(&h.canal, &h.ts);
            self.borradores.guardar(&clave, &h.editor.text());
        }
    }

    pub(crate) fn silenciado(&self, canal: &str) -> bool {
        self.silenciados.contains(canal)
    }

    /// Grupos de la lateral en orden. Con las secciones del usuario: Destacados (si
    /// tiene algo), las propias, y en *Canales* y *Mensajes directos* lo que no esté en
    /// otra. Sin ellas, o si faltan esas dos, se agregan las fijas para no perder nada.
    pub(crate) fn secciones_lateral(&self) -> Vec<GrupoLateral<'_>> {
        let de = |ids: &[String]| -> Vec<&Canal> {
            self.canales
                .iter()
                .filter(|c| ids.contains(&c.id))
                .collect()
        };
        // (grupo, de qué tipo se llena con lo que sobra, si lo afecta "Activos")
        let mut grupos = Vec::new();
        let mut a_mano: HashSet<&str> = HashSet::new();
        for s in self.secciones.iter().flatten() {
            let (titulo, canales, resto) = match s.tipo {
                TipoSeccion::Destacados if !de(&s.canales).is_empty() => {
                    ("Destacados".to_string(), de(&s.canales), None)
                }
                TipoSeccion::Propia => (s.nombre.clone(), de(&s.canales), None),
                TipoSeccion::Canales => ("Canales".to_string(), Vec::new(), Some(false)),
                TipoSeccion::Directos => ("Mensajes directos".to_string(), Vec::new(), Some(true)),
                TipoSeccion::Destacados | TipoSeccion::Otra => continue,
            };
            a_mano.extend(canales.iter().map(|c| c.id.as_str()));
            grupos.push((
                GrupoLateral {
                    id: s.id.clone(),
                    titulo,
                    emoji: Some(&s.emoji)
                        .filter(|e| !e.is_empty())
                        .map(|e| formato::emoji(e)),
                    canales,
                },
                resto,
                s.tipo != TipoSeccion::Destacados,
            ));
        }
        for (id, titulo, directos) in [
            ("canales", "Canales", false),
            ("directos", "Mensajes directos", true),
        ] {
            if !grupos.iter().any(|(_, resto, _)| *resto == Some(directos)) {
                grupos.push((
                    GrupoLateral {
                        id: id.into(),
                        titulo: titulo.into(),
                        emoji: None,
                        canales: Vec::new(),
                    },
                    Some(directos),
                    true,
                ));
            }
        }
        // Las de Slack (Canales, Mensajes directos) se llenan con lo que sobró.
        grupos
            .into_iter()
            .map(|(mut g, resto, filtrable)| {
                if let Some(directos) = resto {
                    g.canales = self
                        .canales
                        .iter()
                        .filter(|c| c.es_directo() == directos && !a_mano.contains(c.id.as_str()))
                        .collect();
                }
                g.canales = self.filtrar_lateral(g.canales, filtrable);
                g
            })
            .collect()
    }

    /// Filtro y orden del menú ⚙ y "Buscar conversación". El canal abierto se ve
    /// siempre. Sin actividad conocida (no llegó `client.counts`) no se oculta nada
    /// por inactivo, para no dejar la lateral vacía.
    fn filtrar_lateral<'a>(&self, canales: Vec<&'a Canal>, filtrable: bool) -> Vec<&'a Canal> {
        let abierto = self.seleccionado.as_deref();
        let buscado = self.buscar_conversacion.to_lowercase();
        let filtro = if filtrable && !self.actividad.is_empty() {
            self.filtro_lateral
        } else {
            lateral::Filtro::Todos
        };
        let nombres: Vec<(&Canal, String)> = canales
            .into_iter()
            .filter(|c| {
                !(self.ocultar_silenciados && self.silenciado(&c.id)) || abierto == Some(&c.id)
            })
            .map(|c| (c, self.nombre_canal(&c.id)))
            .filter(|(_, nombre)| nombre.to_lowercase().contains(&buscado))
            .collect();
        let items: Vec<lateral::Item> = nombres
            .iter()
            .map(|(c, nombre)| {
                let n = self.contadores.get(&c.id).copied().unwrap_or_default();
                lateral::Item {
                    id: &c.id,
                    nombre,
                    ultima: self.actividad.get(&c.id).map(|s| *s as f64),
                    no_leido: n.no_leidos,
                    menciones: n.menciones,
                }
            })
            .collect();
        let ahora = chrono::Local::now().timestamp() as f64;
        lateral::visibles(&items, filtro, self.orden_lateral, abierto, ahora)
            .into_iter()
            .filter_map(|id| nombres.iter().find(|(c, _)| c.id == id).map(|(c, _)| *c))
            .collect()
    }

    pub(crate) fn titulo(&self) -> String {
        let equipo = self
            .sesion
            .as_ref()
            .map(|s| s.team.as_str())
            .unwrap_or("Slack");
        match &self.seleccionado {
            Some(id) => format!("{} - {equipo} - Slack", self.nombre_canal(id)),
            None => format!("{equipo} - Slack"),
        }
    }

    pub(crate) fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Sesion(Ok(s)) => {
                let tarea = self.pedir_usuarios([s.user_id.clone()]);
                self.sesion = Some(s);
                return tarea;
            }
            Msg::Equipo(Ok(e)) => {
                let url = e.icon.image_68.clone().filter(|_| !e.icon.image_default);
                self.equipo = Some(e);
                if let (Some(url), Some(c)) = (url, &self.cliente) {
                    return Task::perform(c.clone().imagen(url), Msg::IconoEquipo);
                }
            }
            Msg::IconoEquipo(Ok(bytes)) => {
                self.icono_equipo = Some(image::Handle::from_bytes(bytes));
            }
            Msg::Canales(Ok(mut canales)) => {
                canales.sort_by_key(|c| c.name.clone());
                let ids: Vec<_> = canales.iter().filter_map(|c| c.user.clone()).collect();
                self.canales = canales;
                return self.pedir_usuarios(ids);
            }
            Msg::Contadores(Ok(c)) => {
                for (id, n) in &c {
                    self.registrar_actividad(id, n.ultimo);
                }
                self.contadores = c;
            }
            Msg::Usuarios(lista) => {
                let mut tareas = Vec::new();
                for u in lista {
                    if let (Some(url), Some(c)) = (u.profile.image_72.clone(), &self.cliente) {
                        let id = u.id.clone();
                        tareas.push(Task::perform(c.clone().imagen(url), move |r| {
                            Msg::Avatar(id.clone(), r)
                        }));
                    }
                    self.usuarios.insert(u.id.clone(), u);
                }
                return Task::batch(tareas);
            }
            Msg::Avatar(id, Ok(bytes)) => {
                self.avatares.insert(id, image::Handle::from_bytes(bytes));
            }
            Msg::Seleccionar(id) => {
                if let Some(anterior) = &self.seleccionado {
                    self.historial.visitar(anterior);
                }
                self.historial.visitar(&id);
                return self.abrir_conversacion(id);
            }
            Msg::Atras => {
                if let Some(id) = self.historial.atras().map(str::to_string) {
                    return self.abrir_conversacion(id);
                }
            }
            Msg::Adelante => {
                if let Some(id) = self.historial.adelante().map(str::to_string) {
                    return self.abrir_conversacion(id);
                }
            }
            Msg::Vecino(paso) => {
                let orden = self.orden_lateral();
                if let Some(id) = navegacion::vecino(&orden, self.seleccionado.as_deref(), paso) {
                    return self.update(Msg::Seleccionar(id.to_string()));
                }
            }
            Msg::VecinoSinLeer(paso) => {
                let orden = self.orden_lateral();
                let pendiente = |id: &str| {
                    self.contadores
                        .get(id)
                        .is_some_and(|c| c.no_leidos || c.menciones > 0)
                };
                let actual = self.seleccionado.as_deref();
                if let Some(id) = navegacion::vecino_sin_leer(&orden, pendiente, actual, paso) {
                    return self.update(Msg::Seleccionar(id.to_string()));
                }
            }
            Msg::MarcarTodoLeido => {
                let pendientes: Vec<String> = self.contadores.keys().cloned().collect();
                let tareas: Vec<_> = pendientes
                    .into_iter()
                    .map(|id| {
                        let ts = self.ts_leido(&id);
                        self.marcar_leido(id, ts)
                    })
                    .collect();
                return Task::batch(tareas);
            }
            Msg::Historial(canal, Ok(Pagina { mensajes, hay_mas })) => {
                self.hay_mas.insert(canal.clone(), hay_mas);
                let ultimo = mensajes.last().map(|m| m.ts.clone());
                let falta = faltantes(&mensajes);
                self.mensajes.insert(canal.clone(), mensajes);
                let mut tareas = vec![self.pedir(falta)];
                if self.seleccionado.as_ref() == Some(&canal)
                    && self.hay_que_marcar(&canal)
                    && let Some(ts) = ultimo
                {
                    tareas.push(self.marcar_leido(canal, ts));
                }
                return Task::batch(tareas);
            }
            Msg::Desplazado(canal, al_tope) => {
                // Como Slack: se piden los anteriores un poco antes de llegar arriba.
                let pedir = al_tope < 200.0
                    && self.hay_mas.get(&canal) == Some(&true)
                    && !self.cargando_mas.contains(&canal);
                let antes = self
                    .mensajes
                    .get(&canal)
                    .and_then(|l| l.first())
                    .map(|m| m.ts.clone());
                if let (true, Some(antes), Some(cliente)) = (pedir, antes, &self.cliente) {
                    self.cargando_mas.insert(canal.clone());
                    return Task::perform(
                        cliente.clone().historial(canal.clone(), Some(antes)),
                        move |r| Msg::Anteriores(canal.clone(), r),
                    );
                }
            }
            Msg::Anteriores(canal, r) => {
                self.cargando_mas.remove(&canal);
                let pagina = match r {
                    Ok(p) => p,
                    Err(e) => {
                        self.error = Some(e);
                        return Task::none();
                    }
                };
                self.hay_mas.insert(canal.clone(), pagina.hay_mas);
                let falta = faltantes(&pagina.mensajes);
                if let Some(lista) = self.mensajes.get_mut(&canal) {
                    fusionar(lista, pagina.mensajes);
                }
                return self.pedir(falta);
            }
            Msg::Posteriores(canal, Ok(nuevos)) => {
                let falta = faltantes(&nuevos);
                let Some(lista) = self.mensajes.get_mut(&canal) else {
                    return Task::none();
                };
                fusionar(lista, nuevos);
                let ultimo = lista.last().map(|m| m.ts.clone());
                let mut tareas = vec![self.pedir(falta)];
                if self.visible
                    && self.seleccionado.as_ref() == Some(&canal)
                    && let Some(ts) = ultimo
                {
                    tareas.push(self.marcar_leido(canal, ts));
                }
                return Task::batch(tareas);
            }
            Msg::AbrirHilo(canal, ts) => {
                self.guardar_borrador_hilo();
                let borrador = self.borradores.tomar(&borradores::clave_hilo(&canal, &ts));
                self.hilo = Some(Hilo {
                    canal: canal.clone(),
                    ts: ts.clone(),
                    mensajes: None,
                    editor: con_texto(&borrador),
                    tambien: false,
                });
                return self.pedir_respuestas(canal, ts);
            }
            Msg::CerrarHilo => {
                self.guardar_borrador_hilo();
                self.hilo = None;
            }
            Msg::MenuRecientes => self.menu_recientes = !self.menu_recientes,
            Msg::InfoCanal(canal, Ok(info)) => {
                self.info.insert(canal, info);
            }
            Msg::MiembrosCanal(canal, Ok(ids)) => {
                self.miembros.insert(canal, ids.clone());
                return self.pedir_usuarios(ids);
            }
            Msg::Marcadores(canal, Ok(lista)) => {
                self.marcadores.insert(canal, lista);
            }
            Msg::ArchivosCanal(canal, Ok(lista)) => {
                let autores: Vec<String> = lista.iter().filter_map(|a| a.user.clone()).collect();
                self.archivos_canal.insert(canal, lista);
                return self.pedir_usuarios(autores);
            }
            Msg::FijadosCanal(canal, Ok(lista)) => {
                let falta = faltantes(&lista);
                self.fijados.insert(canal, lista);
                return self.pedir(falta);
            }
            Msg::Pestana(pestana) => {
                self.pestana = pestana;
                let (Some(canal), Some(cliente)) = (self.seleccionado.clone(), &self.cliente)
                else {
                    return Task::none();
                };
                let c = cliente.clone();
                match pestana {
                    Pestana::Archivos if !self.archivos_canal.contains_key(&canal) => {
                        return Task::perform(c.archivos(canal.clone()), move |r| {
                            Msg::ArchivosCanal(canal.clone(), r)
                        });
                    }
                    Pestana::Fijados if !self.fijados.contains_key(&canal) => {
                        return Task::perform(c.fijados(canal.clone()), move |r| {
                            Msg::FijadosCanal(canal.clone(), r)
                        });
                    }
                    _ => {}
                }
            }
            Msg::AbrirDetalles(detalles) => {
                self.detalles = Some(detalles);
                self.menu_canal = false;
                if let (Some(canal), Some(c)) = (self.seleccionado.clone(), &self.cliente)
                    && !self.miembros.contains_key(&canal)
                {
                    return Task::perform(c.clone().miembros(canal.clone()), move |r| {
                        Msg::MiembrosCanal(canal.clone(), r)
                    });
                }
            }
            Msg::CerrarDetalles => self.detalles = None,
            Msg::MenuCanal => self.menu_canal = !self.menu_canal,
            Msg::MenuMensaje(ts) => {
                self.menu_mensaje = match self.menu_mensaje.take() {
                    Some(abierto) if abierto == ts => None,
                    _ => Some(ts),
                };
            }
            Msg::CopiarEnlaceMensaje(canal, ts) => {
                self.menu_mensaje = None;
                let enlace = self
                    .buscar_mensaje(&canal, &ts)
                    .and_then(|m| self.enlace_mensaje(&canal, m));
                if let Some(enlace) = enlace {
                    return iced::clipboard::write(enlace);
                }
            }
            Msg::CopiarTexto(canal, ts) => {
                self.menu_mensaje = None;
                if let Some(m) = self.buscar_mensaje(&canal, &ts) {
                    return iced::clipboard::write(formato::texto_plano(
                        m.cuerpo(),
                        &self.usuarios,
                    ));
                }
            }
            Msg::MarcarNoLeido(canal, ts) => {
                self.menu_mensaje = None;
                // Leído hasta el mensaje anterior: este y los siguientes quedan sin leer.
                let anterior = self.mensajes.get(&canal).and_then(|lista| {
                    let i = lista.iter().position(|m| m.ts == ts)?;
                    Some(lista.get(i.checked_sub(1)?)?.ts.clone())
                });
                let hasta = anterior.unwrap_or_else(|| "0000000000.000000".into());
                self.contadores.entry(canal.clone()).or_default().no_leidos = true;
                if let Some(c) = &self.cliente {
                    return Task::perform(c.clone().marcar_leido(canal, hasta), Msg::Hecho);
                }
            }
            Msg::Fijar(canal, ts) => {
                self.menu_mensaje = None;
                let fijar = !self
                    .buscar_mensaje(&canal, &ts)
                    .is_some_and(|m| m.pinned_to.contains(&canal));
                for m in self.copias(&canal, &ts) {
                    if fijar {
                        m.pinned_to.push(canal.clone());
                    } else {
                        m.pinned_to.retain(|c| *c != canal);
                    }
                }
                // Lo fijado cambió: la pestaña se vuelve a pedir al abrirla.
                self.fijados.remove(&canal);
                if let Some(c) = &self.cliente {
                    return Task::perform(c.clone().fijar(canal, ts, fijar), Msg::Hecho);
                }
            }
            Msg::CopiarEnlaceCanal => {
                self.menu_canal = false;
                if let Some(enlace) = self
                    .seleccionado
                    .as_deref()
                    .and_then(|c| self.enlace_canal(c))
                {
                    return iced::clipboard::write(enlace);
                }
            }
            Msg::BuscarEnCanal => {
                if let Some(canal) = self
                    .canales
                    .iter()
                    .find(|c| Some(&c.id) == self.seleccionado.as_ref())
                {
                    let prefijo = if canal.es_directo() { "@" } else { "#" };
                    self.filtro = format!("in:{prefijo}{} ", self.nombre_canal(&canal.id));
                    return iced::widget::operation::focus(ID_BUSCADOR);
                }
            }
            Msg::MenuLateral => self.menu_lateral = !self.menu_lateral,
            Msg::FiltrarLateral(filtro) => {
                self.filtro_lateral = filtro;
                self.menu_lateral = false;
            }
            Msg::OrdenarLateral(orden) => {
                self.orden_lateral = orden;
                self.menu_lateral = false;
            }
            Msg::AlternarSilenciados => {
                self.ocultar_silenciados = !self.ocultar_silenciados;
                self.menu_lateral = false;
            }
            Msg::BuscarConversacion(texto) => self.buscar_conversacion = texto,
            Msg::Respuestas(ts, Ok(mensajes)) => {
                let falta = faltantes(&mensajes);
                if let Some(h) = self.hilo.as_mut().filter(|h| h.ts == ts) {
                    h.mensajes = Some(mensajes);
                }
                return self.pedir(falta);
            }
            Msg::EmojiPropios(Ok(mapa)) => {
                self.emoji_propios = mapa;
                let cargados = self
                    .mensajes
                    .values()
                    .flatten()
                    .chain(self.hilo.iter().flat_map(|h| h.mensajes.iter().flatten()));
                let falta = faltantes(cargados);
                return self.pedir(falta);
            }
            Msg::Miniatura(id, Ok(bytes)) => {
                self.miniaturas.insert(id, image::Handle::from_bytes(bytes));
            }
            Msg::EmojiImagen(nombre, Ok(bytes)) => {
                self.emoji_imagenes
                    .insert(nombre, image::Handle::from_bytes(bytes));
            }
            Msg::Filtro(f) => self.filtro = f,
            Msg::Buscar => return self.buscar(),
            Msg::Resultados(consulta, r) => {
                let Some(b) = self.busqueda.as_mut().filter(|b| b.consulta == consulta) else {
                    return Task::none();
                };
                let resultados = r.unwrap_or_else(|e| {
                    self.error = Some(e);
                    Vec::new()
                });
                let falta = faltantes(resultados.iter().map(|c| &c.mensaje));
                b.resultados = Some(resultados);
                return self.pedir(falta);
            }
            Msg::CerrarBusqueda => self.busqueda = None,
            Msg::AbrirSaltar => {
                self.saltar = Some(Saltar {
                    consulta: String::new(),
                    elegido: 0,
                });
                return iced::widget::operation::focus(ID_SALTAR);
            }
            Msg::SaltarConsulta(consulta) => {
                if let Some(s) = &mut self.saltar {
                    *s = Saltar {
                        consulta,
                        elegido: 0,
                    };
                }
            }
            Msg::Flecha(paso) => {
                let ultimo = self.opciones_saltar().len().saturating_sub(1);
                if let Some(s) = &mut self.saltar {
                    s.elegido = s.elegido.saturating_add_signed(paso).min(ultimo);
                } else if let Some(a) = &mut self.autocompletado {
                    let ultimo = a.opciones.len().saturating_sub(1);
                    a.elegido = a.elegido.saturating_add_signed(paso).min(ultimo);
                }
            }
            Msg::SaltarElegir => {
                let elegido = self.saltar.as_ref().map_or(0, |s| s.elegido);
                if let Some(id) = self.opciones_saltar().get(elegido).map(|id| id.to_string()) {
                    return self.update(Msg::Seleccionar(id));
                }
            }
            // Cierra solo lo de más arriba: capas, confirmación, edición, sugerencias.
            Msg::Esc => {
                let cerro = self.saltar.take().is_some()
                    || self.selector_emoji.take().is_some()
                    || self.por_borrar.take().is_some()
                    || self.edicion.take().is_some()
                    || self.autocompletado.take().is_some()
                    || std::mem::take(&mut self.menu_lateral)
                    || std::mem::take(&mut self.menu_recientes)
                    || std::mem::take(&mut self.menu_canal)
                    || std::mem::take(&mut self.menu_avisos)
                    || self.detalles.take().is_some()
                    || self.perfil.take().is_some();
                // Sin nada que cerrar, Esc marca como leído lo abierto, como Slack.
                if !cerro
                    && let Some(id) = self.seleccionado.clone()
                    && self.contadores.contains_key(&id)
                {
                    let ts = self.ts_leido(&id);
                    return self.marcar_leido(id, ts);
                }
            }
            Msg::EmpezarEdicion(canal, ts) => return self.empezar_edicion(canal, ts),
            Msg::EditarUltimo(destino) => {
                if let Some((canal, ts)) = self.ultimo_propio(destino) {
                    return self.empezar_edicion(canal, ts);
                }
            }
            Msg::EditarEdicion(accion) => {
                if let Some(ed) = &mut self.edicion {
                    ed.editor.perform(accion);
                }
            }
            Msg::GuardarEdicion => return self.guardar_edicion(),
            Msg::PedirBorrar(canal, ts) => {
                if self.es_propio(&canal, &ts) {
                    self.por_borrar = Some((canal, ts));
                }
            }
            Msg::ConfirmarBorrar => {
                let (Some((canal, ts)), Some(cliente)) = (self.por_borrar.take(), &self.cliente)
                else {
                    return Task::none();
                };
                let tarea = Task::perform(
                    cliente.clone().borrar(canal.clone(), ts.clone()),
                    Msg::Hecho,
                );
                self.quitar(&canal, &ts);
                return tarea;
            }
            Msg::AbrirEmoji(destino) => {
                self.saltar = None;
                self.selector_emoji = Some(SelectorEmoji {
                    destino,
                    consulta: String::new(),
                });
                return iced::widget::operation::focus(ID_EMOJI);
            }
            Msg::EmojiConsulta(consulta) => {
                if let Some(s) = &mut self.selector_emoji {
                    s.consulta = consulta;
                }
            }
            Msg::ElegirEmoji(nombre) => {
                let Some(s) = self.selector_emoji.take() else {
                    return Task::none();
                };
                match s.destino {
                    DestinoEmoji::Reaccion { canal, ts } => {
                        return self.reaccionar(canal, ts, nombre);
                    }
                    DestinoEmoji::Compositor(destino) => {
                        if let Some(editor) = self.editor_mut(destino) {
                            let texto = Arc::new(format!(":{nombre}: "));
                            editor.perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                                texto,
                            )));
                        }
                    }
                }
            }
            Msg::Vista(v) => self.vista = v,
            Msg::Plegar(id) => {
                if !self.plegadas.remove(&id) {
                    self.plegadas.insert(id);
                }
            }
            Msg::Secciones(Ok(s)) => self.secciones = Some(s),
            Msg::Prefs(Ok(p)) => {
                self.silenciados = p.silenciados;
                self.tema = p.tema;
                self.reloj_12 = !p.reloj_24;
                self.avisar_todo = p.todos;
            }
            Msg::Editar(destino, accion) => {
                let edita = accion.is_edit();
                if let Some(editor) = self.editor_mut(destino) {
                    editor.perform(accion);
                }
                // Solo al escribir: mover el cursor o seleccionar no abre sugerencias.
                if edita {
                    self.autocompletado = self.autocompletar(destino);
                    self.avisar_que_escribes(destino);
                }
            }
            Msg::ElegirSugerencia => return self.elegir_sugerencia(),
            Msg::SugerenciaEn(i) => {
                if let Some(a) = &mut self.autocompletado {
                    a.elegido = i;
                }
                return self.elegir_sugerencia();
            }
            Msg::AlternarBarra => self.sin_barra = !self.sin_barra,
            // Slack repite el aviso cada ~3 s mientras se escribe; 6 s sin él es que paró.
            Msg::MenuEnviar(destino) => {
                self.menu_enviar = match self.menu_enviar {
                    Some(d) if d == destino => None,
                    _ => Some(destino),
                };
            }
            Msg::Programar(destino, post_at, etiqueta) => {
                self.menu_enviar = None;
                let Some((cliente, canal, hilo, texto)) = self.tomar_envio(destino, false) else {
                    return Task::none();
                };
                let para_slack = self.para_enviar(&texto);
                return Task::perform(
                    cliente.programar(canal, para_slack, post_at, hilo),
                    move |r| Msg::Programado(destino, texto.clone(), etiqueta.clone(), r),
                );
            }
            Msg::Programado(_, _, etiqueta, Ok(())) => {
                let mut cuando = etiqueta.chars();
                let cuando: String = cuando
                    .next()
                    .map(|c| c.to_lowercase().chain(cuando).collect())
                    .unwrap_or_default();
                self.nota = Some(format!("Mensaje programado: {cuando}"));
            }
            Msg::Programado(destino, texto, _, Err(e)) => {
                return self.update(Msg::Enviado(destino, texto, Err(e)));
            }
            Msg::CerrarNota => self.nota = None,
            Msg::AvisoClic(canal) => return self.update(Msg::Seleccionar(canal)),
            Msg::AbrirPanel(panel) => {
                self.panel = Some(panel);
                self.busqueda = None;
                return self.pedir_panel(panel);
            }
            Msg::HilosVista(Ok(lista)) => {
                let falta = faltantes(
                    lista
                        .iter()
                        .flat_map(|h| [&h.raiz].into_iter().chain(&h.respuestas)),
                );
                self.hilos_vista = Some(lista);
                return self.pedir(falta);
            }
            Msg::ActividadVista(Ok(items)) => {
                let autores: Vec<String> = items.iter().filter_map(|i| i.autor.clone()).collect();
                let ids = items
                    .iter()
                    .map(|i| (i.canal.clone(), i.ts.clone()))
                    .collect();
                self.bandeja = Some(items);
                return Task::batch([self.pedir_usuarios(autores), self.pedir_sueltos(ids)]);
            }
            Msg::Guardados(Ok(ids)) => {
                self.guardados = Some(ids.clone());
                return self.pedir_sueltos(ids);
            }
            Msg::MensajesSueltos(Ok(lista)) => {
                let falta = faltantes(lista.iter().map(|(_, m)| m));
                for (canal, m) in lista {
                    self.sueltos.insert((canal, m.ts.clone()), m);
                }
                return self.pedir(falta);
            }
            Msg::BorradoresSlack(Ok(lista)) => {
                let falta = faltantes(lista.iter().map(|b| &b.mensaje));
                self.borradores_slack = Some(lista);
                return self.pedir(falta);
            }
            Msg::ArchivosEquipo(Ok(lista)) => {
                let autores: Vec<String> = lista.iter().filter_map(|a| a.user.clone()).collect();
                self.archivos_equipo = Some(lista);
                return self.pedir_usuarios(autores);
            }
            Msg::BuscarPersonas(consulta) => {
                self.consulta_personas = consulta.clone();
                if let Some(c) = &self.cliente {
                    return Task::perform(c.clone().personas(consulta.clone()), move |r| {
                        Msg::Personas(consulta.clone(), r)
                    });
                }
            }
            Msg::Personas(consulta, Ok(lista)) => {
                // Solo la respuesta de lo que se busca ahora; las viejas llegan tarde.
                if consulta == self.consulta_personas {
                    self.personas = Some(lista.clone());
                    return self.update(Msg::Usuarios(lista));
                }
            }
            Msg::AbrirDm(usuario) => {
                if let Some(c) = &self.cliente {
                    return Task::perform(c.clone().abrir_dm(usuario.clone()), move |r| {
                        Msg::DmAbierto(usuario.clone(), r)
                    });
                }
            }
            Msg::DmAbierto(usuario, Ok(id)) => {
                if !self.canales.iter().any(|c| c.id == id)
                    && let Ok(c) = serde_json::from_value(
                        serde_json::json!({ "id": id, "is_im": true, "user": usuario }),
                    )
                {
                    self.canales.push(c);
                }
                self.perfil = None;
                return self.update(Msg::Seleccionar(id));
            }
            Msg::VerPerfil(usuario) => {
                self.perfil = Some(usuario.clone());
                return self.pedir_usuarios([usuario]);
            }
            Msg::CerrarPerfil => self.perfil = None,
            Msg::Destacar(canal) => {
                let destacar = !self.destacado(&canal);
                let secciones = self.secciones.get_or_insert_with(Vec::new);
                if !secciones.iter().any(|s| s.tipo == TipoSeccion::Destacados) {
                    secciones.insert(
                        0,
                        Seccion {
                            id: "destacados".into(),
                            tipo: TipoSeccion::Destacados,
                            nombre: String::new(),
                            emoji: String::new(),
                            canales: Vec::new(),
                        },
                    );
                }
                for s in secciones.iter_mut() {
                    s.canales.retain(|c| *c != canal);
                    if destacar && s.tipo == TipoSeccion::Destacados {
                        s.canales.push(canal.clone());
                    }
                }
                if let Some(c) = &self.cliente {
                    return Task::perform(c.clone().destacar(canal, destacar), Msg::Hecho);
                }
            }
            Msg::MenuAvisos => self.menu_avisos = !self.menu_avisos,
            Msg::AvisosCanal(canal, nivel) => {
                self.menu_avisos = false;
                self.silenciados.remove(&canal);
                self.avisar_todo.remove(&canal);
                match nivel {
                    NivelAviso::Silencio => {
                        self.silenciados.insert(canal.clone());
                    }
                    NivelAviso::Todo => {
                        self.avisar_todo.insert(canal.clone());
                    }
                    NivelAviso::Menciones => {}
                }
                if let Some(c) = &self.cliente {
                    return Task::perform(c.clone().nivel_avisos(canal, nivel), Msg::Hecho);
                }
            }
            Msg::Guardar(canal, ts) => {
                self.menu_mensaje = None;
                let guardar = !self.esta_guardado(&canal, &ts);
                let lista = self.guardados.get_or_insert_with(Vec::new);
                let par = (canal.clone(), ts.clone());
                if guardar {
                    lista.insert(0, par);
                } else {
                    lista.retain(|g| *g != par);
                }
                if let Some(c) = &self.cliente {
                    return Task::perform(c.clone().guardar(canal, ts, guardar), Msg::Hecho);
                }
            }
            Msg::Adjuntar(destino, ruta) => {
                if !self
                    .adjuntos
                    .iter()
                    .any(|(d, r)| *d == destino && *r == ruta)
                {
                    self.adjuntos.push((destino, ruta));
                }
            }
            Msg::Elegidos(destino, rutas) => {
                let tareas: Vec<_> = rutas
                    .into_iter()
                    .map(|r| self.update(Msg::Adjuntar(destino, r)))
                    .collect();
                return Task::batch(tareas);
            }
            Msg::ElegirArchivos(destino) => {
                let elegir = async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Adjuntar archivos")
                        .pick_files()
                        .await
                        .map(|v| v.iter().map(|f| f.path().to_path_buf()).collect())
                        .unwrap_or_default()
                };
                return Task::perform(elegir, move |rutas| Msg::Elegidos(destino, rutas));
            }
            Msg::QuitarAdjunto(destino, ruta) => {
                self.adjuntos
                    .retain(|(d, r)| !(*d == destino && *r == ruta));
            }
            Msg::Subido(destino, texto, _, Ok(())) => {
                return self.update(Msg::Enviado(destino, texto, Ok(())));
            }
            Msg::Subido(destino, texto, rutas, Err(e)) => {
                for r in rutas {
                    let _ = self.update(Msg::Adjuntar(destino, r));
                }
                return self.update(Msg::Enviado(destino, texto, Err(e)));
            }
            Msg::Tick => self
                .escribiendo
                .retain(|(_, _, desde)| desde.elapsed() < std::time::Duration::from_secs(6)),
            Msg::AlternarTambien => {
                if let Some(h) = &mut self.hilo {
                    h.tambien = !h.tambien;
                }
            }
            Msg::Formatear(destino, formato) => {
                if let Some(editor) = self.editor_mut(destino) {
                    formatear(editor, formato);
                }
            }
            Msg::Enviar(destino) => return self.enviar(destino),
            Msg::Enviado(destino, texto, Err(e)) => {
                self.error = Some(e);
                // Se devuelve lo que se intentó mandar, salvo que ya se esté escribiendo
                // otra cosa.
                if let Some(editor) = self.editor_mut(destino)
                    && editor.text().trim().is_empty()
                {
                    *editor = text_editor::Content::with_text(&texto);
                    editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
                }
            }
            Msg::Enviado(destino, _, Ok(())) => {
                // Con tiempo real el mensaje llega solo por el websocket.
                if !self.tiempo_real {
                    return match (destino, &self.hilo, &self.seleccionado) {
                        (Destino::Hilo, Some(h), _) => {
                            self.pedir_respuestas(h.canal.clone(), h.ts.clone())
                        }
                        (Destino::Canal, _, Some(c)) => self.pedir_historial(c.clone()),
                        _ => Task::none(),
                    };
                }
            }
            Msg::Sobre(ts) => self.sobre = Some(ts),
            Msg::Salir(ts) => {
                if self.sobre.as_ref() == Some(&ts) {
                    self.sobre = None;
                }
                if self.menu_mensaje.as_ref() == Some(&ts) {
                    self.menu_mensaje = None;
                }
            }
            Msg::Reaccionar { canal, ts, nombre } => return self.reaccionar(canal, ts, nombre),
            Msg::AbrirEnlace(url) => {
                let seguro = ["https://", "http://", "mailto:"]
                    .iter()
                    .any(|p| url.starts_with(p));
                if seguro {
                    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
                }
            }
            Msg::CerrarError => self.error = None,
            Msg::Rtm(evento) => return self.evento_rtm(*evento),
            Msg::Sesion(Err(e)) => {
                self.sesion_vencida |= rechazada(&e);
                self.error = Some(e);
            }
            Msg::Canales(Err(e))
            | Msg::Historial(_, Err(e))
            | Msg::Posteriores(_, Err(e))
            | Msg::Respuestas(_, Err(e))
            | Msg::Hecho(Err(e))
            | Msg::MiembrosCanal(_, Err(e))
            | Msg::ArchivosCanal(_, Err(e))
            | Msg::FijadosCanal(_, Err(e))
            | Msg::HilosVista(Err(e))
            | Msg::ActividadVista(Err(e))
            | Msg::Guardados(Err(e))
            | Msg::MensajesSueltos(Err(e))
            | Msg::BorradoresSlack(Err(e))
            | Msg::ArchivosEquipo(Err(e))
            | Msg::Personas(_, Err(e))
            | Msg::DmAbierto(_, Err(e)) => self.error = Some(e),
            // Sin ícono, contadores o avatar la interfaz sigue funcionando.
            Msg::Equipo(Err(_))
            | Msg::InfoCanal(_, Err(_))
            | Msg::Marcadores(_, Err(_))
            | Msg::IconoEquipo(Err(_))
            | Msg::Contadores(Err(_))
            | Msg::Secciones(Err(_))
            | Msg::Prefs(Err(_))
            | Msg::Avatar(_, Err(_))
            | Msg::EmojiPropios(Err(_))
            | Msg::EmojiImagen(_, Err(_))
            | Msg::Miniatura(_, Err(_))
            | Msg::Hecho(Ok(())) => {}
        }
        Task::none()
    }

    fn evento_rtm(&mut self, evento: rtm::Evento) -> Task<Msg> {
        match evento {
            rtm::Evento::Conectado => {
                self.tiempo_real = true;
                self.suscribir_presencia();
                if std::mem::take(&mut self.hueco) {
                    return self.recuperar();
                }
            }
            rtm::Evento::Desconectado(motivo) => {
                // Solo es hueco si se cayó estando conectado; antes de la primera
                // conexión lo cargado al arrancar ya es lo último.
                self.hueco |= self.tiempo_real;
                self.tiempo_real = false;
                self.sesion_vencida |= rechazada(&motivo);
                eprintln!("tiempo real desconectado: {motivo}");
            }
            rtm::Evento::Nuevo { canal, mensaje } => {
                if let Some(u) = &mensaje.user {
                    self.escribiendo
                        .retain(|(c, w, _)| !(*c == canal && w == u));
                }
                return self.mensaje_nuevo(canal, mensaje);
            }
            rtm::Evento::Editado { canal, mensaje } => {
                let ts = mensaje.ts.clone();
                for m in self.copias(&canal, &ts) {
                    *m = mensaje.clone();
                }
            }
            rtm::Evento::Borrado { canal, ts } => self.quitar(&canal, &ts),
            rtm::Evento::Reaccion {
                canal,
                ts,
                nombre,
                usuario,
                agregada,
            } => {
                for m in self.copias(&canal, &ts) {
                    m.aplicar_reaccion(&nombre, &usuario, agregada);
                }
                return self.pedir_emoji([base_emoji(&nombre).to_string()]);
            }
            rtm::Evento::Canal(enviador) => self.enviador = Some(enviador),
            rtm::Evento::Presencia { usuarios, activo } => {
                for u in usuarios {
                    if activo {
                        self.presentes.insert(u);
                    } else {
                        self.presentes.remove(&u);
                    }
                }
            }
            rtm::Evento::Leido {
                canal,
                sin_leer,
                menciones,
            } => {
                if sin_leer == 0 && menciones == 0 {
                    self.contadores.remove(&canal);
                } else {
                    let c = self.contadores.entry(canal).or_default();
                    c.no_leidos = sin_leer > 0;
                    c.menciones = menciones;
                }
            }
            rtm::Evento::Escribiendo { canal, usuario } => {
                let ahora = std::time::Instant::now();
                match self
                    .escribiendo
                    .iter_mut()
                    .find(|(c, u, _)| *c == canal && *u == usuario)
                {
                    Some((_, _, desde)) => *desde = ahora,
                    None => self.escribiendo.push((canal, usuario, ahora)),
                }
            }
        }
        Task::none()
    }

    /// Canal y último `ts` de cada canal cargado, para pedir lo posterior.
    pub(crate) fn por_recuperar(&self) -> Vec<(String, String)> {
        self.mensajes
            .iter()
            .filter_map(|(c, l)| Some((c.clone(), l.last()?.ts.clone())))
            .collect()
    }

    /// Después de un corte: lo nuevo de cada canal cargado, los contadores y el hilo
    /// abierto.
    fn recuperar(&mut self) -> Task<Msg> {
        let Some(cliente) = self.cliente.clone() else {
            return Task::none();
        };
        let mut tareas: Vec<_> = self
            .por_recuperar()
            .into_iter()
            .map(|(canal, ts)| {
                Task::perform(cliente.clone().posteriores(canal.clone(), ts), move |r| {
                    Msg::Posteriores(canal.clone(), r)
                })
            })
            .collect();
        tareas.push(Task::perform(cliente.clone().contadores(), Msg::Contadores));
        if let Some(h) = &self.hilo {
            tareas.push(self.pedir_respuestas(h.canal.clone(), h.ts.clone()));
        }
        Task::batch(tareas)
    }

    fn mensaje_nuevo(&mut self, canal: String, mensaje: Mensaje) -> Task<Msg> {
        let mut tareas = vec![self.pedir(faltantes([&mensaje]))];
        if let Some(aviso) = self.aviso_para(&canal, &mensaje) {
            let destino = canal.clone();
            tareas.push(Task::perform(escritorio::mostrar(aviso), move |clic| {
                if clic {
                    Msg::AvisoClic(destino.clone())
                } else {
                    Msg::Hecho(Ok(()))
                }
            }));
        }

        if mensaje.es_respuesta_de_hilo() {
            let raiz = mensaje.thread_ts.clone().unwrap_or_default();
            let autor = mensaje.user.clone();
            for m in self.copias(&canal, &raiz) {
                m.reply_count = Some(m.reply_count.unwrap_or(0) + 1);
                if let Some(u) = autor.as_ref().filter(|u| !m.reply_users.contains(u)) {
                    m.reply_users.push(u.clone());
                }
            }
            if let Some(h) = self
                .hilo
                .as_mut()
                .filter(|h| h.canal == canal && h.ts == raiz)
                && let Some(lista) = h.mensajes.as_mut()
                && !lista.iter().any(|m| m.ts == mensaje.ts)
            {
                lista.push(mensaje);
            }
            return Task::batch(tareas);
        }

        let mio = mensaje.user.is_some() && mensaje.user.as_deref() == self.mi_id();
        // Como Slack: `@here` y `@channel` también suman a las menciones.
        let me_menciona = self.me_menciona(&mensaje) || formato::aviso_general(&mensaje.text);
        let ts = mensaje.ts.clone();
        self.registrar_actividad(&canal, mensaje.segundos());
        // Solo se agregan a canales ya cargados; los demás se piden al abrirlos.
        if let Some(lista) = self.mensajes.get_mut(&canal)
            && !lista.iter().any(|m| m.ts == mensaje.ts)
        {
            lista.push(mensaje);
        }
        if self.visible && self.seleccionado.as_ref() == Some(&canal) {
            tareas.push(self.marcar_leido(canal, ts));
        } else if !mio {
            let directo = self.canales.iter().any(|c| c.id == canal && c.es_directo());
            let contador = self.contadores.entry(canal).or_default();
            contador.no_leidos = true;
            if directo || me_menciona {
                contador.menciones += 1;
            }
        }
        Task::batch(tareas)
    }

    fn buscar(&mut self) -> Task<Msg> {
        let consulta = self.filtro.trim().to_string();
        let Some(cliente) = self.cliente.clone().filter(|_| !consulta.is_empty()) else {
            return Task::none();
        };
        self.busqueda = Some(Busqueda {
            consulta: consulta.clone(),
            resultados: None,
        });
        Task::perform(cliente.buscar(consulta.clone()), move |r| {
            Msg::Resultados(consulta.clone(), r)
        })
    }

    /// Saca lo escrito del compositor para mandarlo: cliente, canal, hilo y texto.
    fn tomar_envio(
        &mut self,
        destino: Destino,
        permitir_vacio: bool,
    ) -> Option<(Cliente, String, Option<String>, String)> {
        let cliente = self.cliente.clone()?;
        let (canal, hilo) = match (destino, &self.hilo, &self.seleccionado) {
            (Destino::Hilo, Some(h), _) => (h.canal.clone(), Some(h.ts.clone())),
            (Destino::Canal, _, Some(c)) => (c.clone(), None),
            _ => return None,
        };
        let editor = self.editor_mut(destino)?;
        let texto = editor.text().trim().to_string();
        if texto.is_empty() && !permitir_vacio {
            return None;
        }
        *editor = text_editor::Content::new();
        self.autocompletado = None;
        Some((cliente, canal, hilo, texto))
    }

    fn enviar(&mut self, destino: Destino) -> Task<Msg> {
        let rutas = self.adjuntos_de(destino);
        if !rutas.is_empty() {
            // Con archivos el texto va como comentario y puede faltar.
            let Some((cliente, canal, hilo, texto)) = self.tomar_envio(destino, true) else {
                return Task::none();
            };
            self.adjuntos.retain(|(d, _)| *d != destino);
            let comentario = self.para_enviar(&texto);
            return Task::perform(
                cliente.subir(canal, hilo, rutas.clone(), comentario),
                move |r| Msg::Subido(destino, texto.clone(), rutas.clone(), r),
            );
        }
        let Some((cliente, canal, hilo, texto)) = self.tomar_envio(destino, false) else {
            return Task::none();
        };
        // La casilla se apaga después de cada respuesta, como en Slack.
        let tambien = destino == Destino::Hilo
            && self
                .hilo
                .as_mut()
                .is_some_and(|h| std::mem::take(&mut h.tambien));
        let para_slack = self.para_enviar(&texto);
        Task::perform(cliente.enviar(canal, para_slack, hilo, tambien), move |r| {
            Msg::Enviado(destino, texto.clone(), r)
        })
    }

    /// Quita un mensaje del canal y del hilo abierto.
    fn quitar(&mut self, canal: &str, ts: &str) {
        if let Some(lista) = self.mensajes.get_mut(canal) {
            lista.retain(|m| m.ts != ts);
        }
        if let Some(lista) = self
            .hilo
            .as_mut()
            .filter(|h| h.canal == canal)
            .and_then(|h| h.mensajes.as_mut())
        {
            lista.retain(|m| m.ts != ts);
        }
    }

    fn es_propio(&mut self, canal: &str, ts: &str) -> bool {
        let yo = self.mi_id().map(str::to_string);
        self.copias(canal, ts)
            .next()
            .is_some_and(|m| m.user.is_some() && m.user == yo)
    }

    fn empezar_edicion(&mut self, canal: String, ts: String) -> Task<Msg> {
        if !self.es_propio(&canal, &ts) {
            return Task::none();
        }
        let Some(crudo) = self.copias(&canal, &ts).next().map(|m| m.text.clone()) else {
            return Task::none();
        };
        let texto = self.para_editar(&crudo);
        let mut editor = text_editor::Content::with_text(&texto);
        editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
        self.edicion = Some(Edicion { canal, ts, editor });
        iced::widget::operation::focus(ID_EDICION)
    }

    fn guardar_edicion(&mut self) -> Task<Msg> {
        let Some(ed) = self.edicion.take() else {
            return Task::none();
        };
        let texto = ed.editor.text().trim().to_string();
        // Como en Slack: dejarlo vacío es pedir borrarlo.
        if texto.is_empty() {
            self.por_borrar = Some((ed.canal, ed.ts));
            return Task::none();
        }
        let para_slack = self.para_enviar(&texto);
        for m in self.copias(&ed.canal, &ed.ts) {
            m.text = para_slack.clone();
            m.rico = None;
        }
        let Some(cliente) = &self.cliente else {
            return Task::none();
        };
        Task::perform(
            cliente.clone().editar(ed.canal, ed.ts, para_slack),
            Msg::Hecho,
        )
    }

    /// Último mensaje propio del canal abierto o del hilo.
    fn ultimo_propio(&self, destino: Destino) -> Option<(String, String)> {
        let yo = self.mi_id()?;
        let (canal, lista) = match destino {
            Destino::Canal => {
                let c = self.seleccionado.as_ref()?;
                (c, self.mensajes.get(c)?)
            }
            Destino::Hilo => {
                let h = self.hilo.as_ref()?;
                (&h.canal, h.mensajes.as_ref()?)
            }
        };
        let m = lista
            .iter()
            .rev()
            .filter(|m| destino == Destino::Hilo || !m.es_respuesta_de_hilo())
            .find(|m| m.user.as_deref() == Some(yo))?;
        Some((canal.clone(), m.ts.clone()))
    }

    /// El mrkdwn de un mensaje como se ve al editarlo: `<@U5>` → `@Ana Ruiz` y
    /// `<#C2|random>` → `#random`, recordando cómo volver a traducirlos al guardar.
    fn para_editar(&mut self, crudo: &str) -> String {
        let mut salida = String::with_capacity(crudo.len());
        let mut resto = crudo;
        while let Some(i) = resto.find('<')
            && let Some(f) = resto[i..].find('>')
        {
            salida.push_str(&formato::entidades(&resto[..i]));
            let dentro = &resto[i + 1..i + f];
            let destino = dentro.split('|').next().unwrap_or(dentro);
            let visible = if let Some(id) = destino.strip_prefix('@') {
                Some(format!("@{}", self.nombre_usuario(id)))
            } else {
                destino
                    .strip_prefix('#')
                    .map(|id| format!("#{}", self.nombre_canal(id)))
            };
            match visible {
                Some(v) => {
                    if !self.traducciones.iter().any(|(t, _)| *t == v) {
                        self.traducciones.push((v.clone(), format!("<{destino}>")));
                    }
                    salida.push_str(&v);
                }
                None => salida.push_str(&resto[i..=i + f]),
            }
            resto = &resto[i + f + 1..];
        }
        salida.push_str(&formato::entidades(resto));
        salida
    }

    /// El texto del compositor como lo entiende Slack.
    pub(crate) fn para_enviar(&self, texto: &str) -> String {
        sugerencias::traducir_menciones(texto, &self.traducciones)
    }

    /// Sugerencias para la palabra junto al cursor del compositor `destino`.
    fn autocompletar(&mut self, destino: Destino) -> Option<Autocompletado> {
        let editor = self.editor_mut(destino)?;
        let cursor = editor.cursor().position;
        let linea = editor.line(cursor.line)?.text.to_string();
        let token = sugerencias::disparador(&linea, cursor.column)?;
        let opciones: Vec<Sugerencia> = match token.tipo {
            Disparador::Usuario => {
                let mut pares: Vec<_> = self
                    .usuarios
                    .values()
                    .map(|u| (u.id.as_str(), u.nombre_visible().to_string()))
                    .collect();
                // `usuarios` es un HashMap: sin ordenar, los empates saldrían al azar.
                pares.sort_by(|a, b| a.1.cmp(&b.1));
                let candidatos = candidatos_de(pares);
                saltar::coincidencias(&candidatos, token.prefijo)
                    .into_iter()
                    .filter_map(|id| {
                        let nombre = self.usuarios.get(id)?.nombre_visible();
                        Some(Sugerencia {
                            etiqueta: nombre.to_string(),
                            usuario: Some(id.to_string()),
                            insercion: format!("@{nombre}"),
                            marca: Some(format!("<@{id}>")),
                        })
                    })
                    .collect()
            }
            Disparador::Canal => {
                let pares = self
                    .canales
                    .iter()
                    .filter(|c| !c.es_directo())
                    .map(|c| (c.id.as_str(), self.nombre_canal(&c.id)))
                    .collect();
                let candidatos = candidatos_de(pares);
                saltar::coincidencias(&candidatos, token.prefijo)
                    .into_iter()
                    .map(|id| {
                        let nombre = self.nombre_canal(id);
                        Sugerencia {
                            insercion: format!("#{nombre}"),
                            etiqueta: nombre,
                            usuario: None,
                            marca: Some(format!("<#{id}>")),
                        }
                    })
                    .collect()
            }
            Disparador::Emoji => {
                let propios = self.emoji_propios.keys().map(String::as_str);
                sugerencias::buscar_emoji(token.prefijo, propios, 8)
                    .into_iter()
                    .map(|o| Sugerencia {
                        insercion: format!(":{}:", o.nombre),
                        etiqueta: o.nombre,
                        usuario: None,
                        marca: None,
                    })
                    .collect()
            }
        };
        if opciones.is_empty() {
            return None;
        }
        Some(Autocompletado {
            destino,
            tipo: token.tipo,
            linea: cursor.line,
            inicio: token.inicio,
            fin: cursor.column,
            opciones: opciones.into_iter().take(8).collect(),
            elegido: 0,
        })
    }

    /// Reemplaza la palabra por la sugerencia elegida y recuerda cómo traducirla.
    fn elegir_sugerencia(&mut self) -> Task<Msg> {
        let Some(mut a) = self.autocompletado.take() else {
            return Task::none();
        };
        if a.elegido >= a.opciones.len() {
            return Task::none();
        }
        let s = a.opciones.swap_remove(a.elegido);
        if let Some(marca) = s.marca
            && !self.traducciones.iter().any(|(v, _)| *v == s.insercion)
        {
            self.traducciones.push((s.insercion.clone(), marca));
        }
        let Some(editor) = self.editor_mut(a.destino) else {
            return Task::none();
        };
        let posicion = |column| text_editor::Position {
            line: a.linea,
            column,
        };
        editor.move_to(text_editor::Cursor {
            position: posicion(a.fin),
            selection: Some(posicion(a.inicio)),
        });
        let texto = Arc::new(format!("{} ", s.insercion));
        editor.perform(text_editor::Action::Edit(text_editor::Edit::Paste(texto)));
        Task::none()
    }

    /// Alterna la reacción propia: se aplica al instante y luego se manda a Slack.
    fn reaccionar(&mut self, canal: String, ts: String, nombre: String) -> Task<Msg> {
        let (Some(cliente), Some(yo)) = (self.cliente.clone(), self.mi_id().map(str::to_string))
        else {
            return Task::none();
        };
        let agregar = !self.copias(&canal, &ts).any(|m| {
            m.reactions
                .iter()
                .any(|r| r.name == nombre && r.users.contains(&yo))
        });
        for m in self.copias(&canal, &ts) {
            m.aplicar_reaccion(&nombre, &yo, agregar);
        }
        Task::perform(cliente.reaccionar(canal, ts, nombre, agregar), Msg::Hecho)
    }

    /// Abre una conversación sin tocar el historial: guarda los borradores de lo que
    /// se deja y pide los mensajes.
    fn abrir_conversacion(&mut self, id: String) -> Task<Msg> {
        self.menu_recientes = false;
        self.panel = None;
        self.busqueda = None;
        self.saltar = None;
        self.guardar_borradores();
        self.seleccionado = Some(id.clone());
        self.editor = con_texto(&self.borradores.tomar(&id));
        self.hilo = None;
        self.pestana = Pestana::Mensajes;
        self.detalles = None;
        self.menu_canal = false;
        Task::batch([self.pedir_encabezado(&id), self.pedir_historial(id)])
    }

    /// Miembros, tema y marcadores del canal, la primera vez que se abre.
    fn pedir_encabezado(&self, canal: &str) -> Task<Msg> {
        let Some(cliente) = &self.cliente else {
            return Task::none();
        };
        let mut tareas = Vec::new();
        if !self.info.contains_key(canal) {
            let id = canal.to_string();
            tareas.push(Task::perform(
                cliente.clone().info_canal(id.clone()),
                move |r| Msg::InfoCanal(id.clone(), r),
            ));
        }
        if !self.marcadores.contains_key(canal) {
            let id = canal.to_string();
            tareas.push(Task::perform(
                cliente.clone().marcadores(id.clone()),
                move |r| Msg::Marcadores(id.clone(), r),
            ));
        }
        Task::batch(tareas)
    }

    /// Ids de la lateral en el orden en que se ven, sin las secciones plegadas.
    fn orden_lateral(&self) -> Vec<&str> {
        self.secciones_lateral()
            .into_iter()
            .filter(|g| !self.plegadas.contains(&g.id))
            .flat_map(|g| g.canales.into_iter().map(|c| c.id.as_str()))
            .collect()
    }

    /// `ts` para marcar todo como leído: el último mensaje cargado o, si no hay, el
    /// final del segundo del último mensaje según `client.counts`.
    fn ts_leido(&self, canal: &str) -> String {
        if let Some(m) = self.mensajes.get(canal).and_then(|m| m.last()) {
            return m.ts.clone();
        }
        let segundos = match self.contadores.get(canal).map(|c| c.ultimo) {
            Some(s) if s > 0 => s,
            _ => chrono::Local::now().timestamp(),
        };
        format!("{segundos}.999999")
    }

    fn marcar_leido(&mut self, canal: String, ts: String) -> Task<Msg> {
        self.contadores.remove(&canal);
        let Some(cliente) = &self.cliente else {
            return Task::none();
        };
        Task::perform(cliente.clone().marcar_leido(canal, ts), Msg::Hecho)
    }

    fn pedir_historial(&self, canal: String) -> Task<Msg> {
        let Some(cliente) = &self.cliente else {
            return Task::none();
        };
        Task::perform(cliente.clone().historial(canal.clone(), None), move |r| {
            Msg::Historial(canal.clone(), r)
        })
    }

    fn pedir_respuestas(&self, canal: String, ts: String) -> Task<Msg> {
        let Some(cliente) = &self.cliente else {
            return Task::none();
        };
        Task::perform(cliente.clone().respuestas(canal, ts.clone()), move |r| {
            Msg::Respuestas(ts.clone(), r)
        })
    }

    fn pedir_usuarios(&mut self, ids: impl IntoIterator<Item = String>) -> Task<Msg> {
        let Some(cliente) = &self.cliente else {
            return Task::none();
        };
        let nuevos: Vec<_> = ids
            .into_iter()
            .filter(|id| self.usuarios_pedidos.insert(id.clone()))
            .collect();
        if nuevos.is_empty() {
            return Task::none();
        }
        Task::perform(cliente.clone().usuarios(nuevos), Msg::Usuarios)
    }

    /// Pide lo que falta para pintar unos mensajes.
    fn pedir(&mut self, f: Faltantes) -> Task<Msg> {
        Task::batch([
            self.pedir_usuarios(f.usuarios),
            self.pedir_emoji(f.emoji),
            self.pedir_miniaturas(f.miniaturas),
        ])
    }

    fn pedir_miniaturas(&mut self, archivos: Vec<(String, String)>) -> Task<Msg> {
        let Some(cliente) = &self.cliente else {
            return Task::none();
        };
        let tareas: Vec<_> = archivos
            .into_iter()
            .filter(|(id, _)| self.miniaturas_pedidas.insert(id.clone()))
            .map(|(id, url)| {
                Task::perform(cliente.clone().archivo(url), move |r| {
                    Msg::Miniatura(id.clone(), r)
                })
            })
            .collect();
        Task::batch(tareas)
    }

    /// Baja una vez la imagen de cada emoji propio que aparece; los estándar se
    /// pintan con la fuente.
    fn pedir_emoji(&mut self, nombres: impl IntoIterator<Item = String>) -> Task<Msg> {
        let Some(cliente) = &self.cliente else {
            return Task::none();
        };
        let tareas: Vec<_> = nombres
            .into_iter()
            .filter_map(|n| Some((self.emoji_propios.get(&n)?.clone(), n)))
            .filter(|(_, n)| self.emoji_pedidos.insert(n.clone()))
            .map(|(url, n)| {
                Task::perform(cliente.clone().imagen(url), move |r| {
                    Msg::EmojiImagen(n.clone(), r)
                })
            })
            .collect();
        Task::batch(tareas)
    }

    fn editor_mut(&mut self, destino: Destino) -> Option<&mut text_editor::Content> {
        match destino {
            Destino::Canal => Some(&mut self.editor),
            Destino::Hilo => self.hilo.as_mut().map(|h| &mut h.editor),
        }
    }

    /// Todas las copias cargadas de un mensaje: en el canal y en el hilo abierto.
    fn copias<'a>(
        &'a mut self,
        canal: &'a str,
        ts: &'a str,
    ) -> impl Iterator<Item = &'a mut Mensaje> {
        let principal = self.mensajes.get_mut(canal).into_iter().flatten();
        let hilo = self
            .hilo
            .iter_mut()
            .filter(move |h| h.canal == canal)
            .flat_map(|h| h.mensajes.iter_mut().flatten());
        principal.chain(hilo).filter(move |m| m.ts == ts)
    }

    fn registrar_actividad(&mut self, canal: &str, segundos: i64) {
        let ultimo = self.actividad.entry(canal.to_string()).or_default();
        *ultimo = (*ultimo).max(segundos);
    }

    /// DMs y grupos, el de actividad más reciente primero.
    pub(crate) fn directos_recientes(&self) -> Vec<&Canal> {
        let mut directos: Vec<_> = self.canales.iter().filter(|c| c.es_directo()).collect();
        directos
            .sort_by_key(|c| std::cmp::Reverse(self.actividad.get(&c.id).copied().unwrap_or(0)));
        directos
    }

    /// Texto del último mensaje cargado de una conversación, en una línea y sin marcas,
    /// con "Tú: " si es propio.
    pub(crate) fn vista_previa(&self, canal: &str) -> Option<String> {
        let m = self
            .mensajes
            .get(canal)?
            .iter()
            .rev()
            .find(|m| !m.es_respuesta_de_hilo())?;
        let texto: String = formato::segmentos(m.cuerpo(), &self.usuarios)
            .into_iter()
            .map(|s| s.texto)
            .collect();
        let texto = texto.split_whitespace().collect::<Vec<_>>().join(" ");
        let mio = m.user.is_some() && m.user.as_deref() == self.mi_id();
        Some(if mio { format!("Tú: {texto}") } else { texto })
    }

    /// Emoji que el selector muestra para lo escrito.
    pub(crate) fn opciones_emoji(&self) -> Vec<OpcionEmoji> {
        let consulta = self
            .selector_emoji
            .as_ref()
            .map_or("", |s| s.consulta.as_str());
        sugerencias::buscar_emoji(consulta, self.emoji_propios.keys().map(String::as_str), 64)
    }

    /// Conversaciones que coinciden con lo escrito en Ctrl+K, la mejor primero.
    pub(crate) fn opciones_saltar(&self) -> Vec<&str> {
        let Some(s) = &self.saltar else {
            return Vec::new();
        };
        let candidatos: Vec<_> = self
            .canales
            .iter()
            .map(|c| Candidato {
                id: &c.id,
                nombre: self.nombre_canal(&c.id),
                no_leido: self.contadores.get(&c.id).is_some_and(|n| n.no_leidos),
            })
            .collect();
        saltar::coincidencias(&candidatos, &s.consulta)
    }

    pub(crate) fn mi_id(&self) -> Option<&str> {
        self.sesion.as_ref().map(|s| s.user_id.as_str())
    }

    pub(crate) fn nombre_usuario<'a>(&'a self, id: &'a str) -> &'a str {
        self.usuarios
            .get(id)
            .map(Usuario::nombre_visible)
            .unwrap_or(id)
    }

    pub(crate) fn nombre_canal(&self, id: &str) -> String {
        let Some(c) = self.canales.iter().find(|c| c.id == id) else {
            return id.to_string();
        };
        match (&c.user, &c.name) {
            (Some(u), _) => self.nombre_usuario(u).to_string(),
            // `mpdm-ana--beto--carla-1`: se traducen los handles a nombres visibles.
            (None, Some(n)) if c.is_mpim => n
                .trim_start_matches("mpdm-")
                .trim_end_matches("-1")
                .split("--")
                .map(|h| {
                    self.usuarios
                        .values()
                        .find(|u| u.name == h)
                        .map_or(h, Usuario::nombre_visible)
                })
                .collect::<Vec<_>>()
                .join(", "),
            (None, Some(n)) => n.clone(),
            (None, None) => c.id.clone(),
        }
    }
}

/// Usuarios que hay que resolver para pintar estos mensajes.
fn ids_de<'a>(mensajes: impl IntoIterator<Item = &'a Mensaje>) -> Vec<String> {
    mensajes
        .into_iter()
        .flat_map(|m| {
            m.user
                .iter()
                .chain(&m.reply_users)
                .map(String::as_str)
                .chain(formato::menciones(&m.text))
        })
        .map(str::to_string)
        .collect()
}

fn candidatos_de(pares: Vec<(&str, String)>) -> Vec<Candidato<'_>> {
    pares
        .into_iter()
        .map(|(id, nombre)| Candidato {
            id,
            nombre,
            no_leido: false,
        })
        .collect()
}

/// Lo que hay que pedir para pintar unos mensajes.
struct Faltantes {
    usuarios: Vec<String>,
    emoji: Vec<String>,
    /// ID y URL de las miniaturas de imágenes.
    miniaturas: Vec<(String, String)>,
}

fn faltantes<'a>(mensajes: impl IntoIterator<Item = &'a Mensaje> + Clone) -> Faltantes {
    Faltantes {
        usuarios: ids_de(mensajes.clone()),
        emoji: emoji_de(mensajes.clone()),
        miniaturas: mensajes
            .into_iter()
            .flat_map(|m| &m.files)
            .filter_map(|f| Some((f.id.clone(), f.miniatura()?.0.to_string())))
            .collect(),
    }
}

/// Mete `otros` (anteriores o recuperados) en `lista` en orden de `ts`, sin repetir.
fn fusionar(lista: &mut Vec<Mensaje>, otros: Vec<Mensaje>) {
    let nuevos: Vec<_> = otros
        .into_iter()
        .filter(|v| !lista.iter().any(|m| m.ts == v.ts))
        .collect();
    lista.extend(nuevos);
    lista.sort_by(|a, b| orden_ts(&a.ts).cmp(&orden_ts(&b.ts)));
}

/// Un `ts` de Slack (`segundos.microsegundos`) como algo que se ordena bien.
fn orden_ts(ts: &str) -> (i64, i64) {
    let (s, f) = ts.split_once('.').unwrap_or((ts, "0"));
    (s.parse().unwrap_or(0), f.parse().unwrap_or(0))
}

/// Nombre de emoji sin el tono de piel: `+1::skin-tone-2` → `+1`.
fn base_emoji(nombre: &str) -> &str {
    nombre.split("::").next().unwrap_or(nombre)
}

/// Emoji que aparecen en estos mensajes: en el texto, en el rich text y en las
/// reacciones.
fn emoji_de<'a>(mensajes: impl IntoIterator<Item = &'a Mensaje>) -> Vec<String> {
    let mut nombres = Vec::new();
    for m in mensajes {
        nombres.extend(formato::codigos_emoji(m.cuerpo()).map(str::to_string));
        nombres.extend(m.reactions.iter().map(|r| base_emoji(&r.name).to_string()));
        for e in m.rico.iter().flatten() {
            if let ElementoRico::RichTextSection { elements }
            | ElementoRico::RichTextQuote { elements } = e
            {
                nombres.extend(elements.iter().filter_map(|e| match e {
                    EnLinea::Emoji {
                        name,
                        unicode: None,
                    } => Some(name.clone()),
                    _ => None,
                }));
            }
        }
    }
    nombres
}

/// Envuelve la selección con la marca de mrkdwn, o deja el cursor entre dos marcas.
fn formatear(editor: &mut text_editor::Content, formato: atajos::Formato) {
    use atajos::Formato::*;
    match formato {
        Negrita => marcar(editor, '*'),
        Cursiva => marcar(editor, '_'),
        Tachado => marcar(editor, '~'),
        Codigo => marcar(editor, '`'),
        BloqueCodigo | Numerada | Vinetas | Cita => bloque(editor, formato),
    }
}

/// Formato por línea: viñetas, números o cita sobre lo seleccionado; sin selección
/// pone el prefijo. El bloque de código envuelve la selección entre ```.
fn bloque(editor: &mut text_editor::Content, formato: atajos::Formato) {
    use atajos::Formato::*;
    use text_editor::{Action, Edit, Motion};
    let pegar = |texto: String| Action::Edit(Edit::Paste(Arc::new(texto)));
    let prefijo = |i: usize| match formato {
        Numerada => format!("{}. ", i + 1),
        Vinetas => "• ".to_string(),
        _ => "> ".to_string(),
    };
    match (editor.selection(), formato) {
        (Some(sel), BloqueCodigo) => editor.perform(pegar(format!("```\n{sel}\n```"))),
        (None, BloqueCodigo) => {
            editor.perform(pegar("```\n\n```".into()));
            editor.perform(Action::Move(Motion::Up));
        }
        (Some(sel), _) => {
            let lineas: Vec<String> = sel
                .lines()
                .enumerate()
                .map(|(i, l)| format!("{}{l}", prefijo(i)))
                .collect();
            editor.perform(pegar(lineas.join("\n")));
        }
        (None, _) => editor.perform(pegar(prefijo(0))),
    }
}

fn marcar(editor: &mut text_editor::Content, marca: char) {
    use text_editor::{Action, Edit, Motion};
    let pegar = |texto: String| Action::Edit(Edit::Paste(Arc::new(texto)));
    match editor.selection() {
        Some(sel) => editor.perform(pegar(format!("{marca}{sel}{marca}"))),
        None => {
            editor.perform(pegar(format!("{marca}{marca}")));
            editor.perform(Action::Move(Motion::Left));
        }
    }
}
/// Cuándo programar un mensaje, como el ▾ de enviar de Slack: mañana a las 9 y el
/// lunes a las 9 (si no es mañana).
pub(crate) fn opciones_programar(
    ahora: chrono::NaiveDateTime,
) -> Vec<(String, chrono::NaiveDateTime)> {
    use chrono::{Datelike, Days, NaiveTime, Weekday};
    let nueve = NaiveTime::from_hms_opt(9, 0, 0).expect("hora válida");
    let manana = ahora.date() + Days::new(1);
    let mut opciones = vec![("Mañana a las 9:00".to_string(), manana.and_time(nueve))];
    if manana.weekday() != Weekday::Mon {
        let faltan = 7 - ahora.weekday().num_days_from_monday();
        let lunes = ahora.date() + Days::new(u64::from(faltan));
        opciones.push(("El lunes a las 9:00".to_string(), lunes.and_time(nueve)));
    }
    opciones
}

/// Si Slack rechazó la sesión: ya no sirve reintentar, hay que volver a entrar.
fn rechazada(error: &str) -> bool {
    [
        "invalid_auth",
        "not_authed",
        "token_revoked",
        "token_expired",
        "account_inactive",
    ]
    .iter()
    .any(|e| error.ends_with(e))
}

/// Editor con `texto` y el cursor al final.
fn con_texto(texto: &str) -> text_editor::Content {
    let mut editor = text_editor::Content::with_text(texto);
    editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    editor
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Workspace visible con sesión `U0`, `#general` (C1) abierto y cargado, `#random`
    /// (C2), un DM (D1) y un grupo (G1). El cliente apunta a una dirección muerta: las
    /// tareas se crean pero nunca se ejecutan.
    fn de_prueba() -> Espacio {
        let cred = Credenciales {
            token: "xoxc-prueba".into(),
            cookie: "xoxd-prueba".into(),
            equipo: None,
        };
        let canales = json!([
            { "id": "C1", "name": "general" },
            { "id": "C2", "name": "random" },
            { "id": "D1", "is_im": true, "user": "U1" },
            { "id": "G1", "is_mpim": true, "name": "mpdm-ana--beto-1" },
        ]);
        Espacio {
            cliente: Some(Cliente::con_base(cred, "http://127.0.0.1:9/")),
            sesion: Some(Sesion {
                user_id: "U0".into(),
                team: "Acme".into(),
            }),
            canales: serde_json::from_value(canales).unwrap(),
            mensajes: HashMap::from([("C1".into(), vec![msg("1.0", "U1", "raíz")])]),
            seleccionado: Some("C1".into()),
            visible: true,
            ..Espacio::default()
        }
    }

    fn msg(ts: &str, usuario: &str, texto: &str) -> Mensaje {
        serde_json::from_value(json!({ "ts": ts, "user": usuario, "text": texto })).unwrap()
    }

    fn respuesta(ts: &str, raiz: &str, usuario: &str) -> Mensaje {
        Mensaje {
            thread_ts: Some(raiz.into()),
            ..msg(ts, usuario, "respuesta")
        }
    }

    fn rtm(e: &mut Espacio, evento: rtm::Evento) {
        let _ = e.update(Msg::Rtm(Box::new(evento)));
    }

    fn nuevo(e: &mut Espacio, canal: &str, mensaje: Mensaje) {
        rtm(
            e,
            rtm::Evento::Nuevo {
                canal: canal.into(),
                mensaje,
            },
        );
    }

    fn abrir_hilo(e: &mut Espacio, canal: &str, ts: &str) {
        let _ = e.update(Msg::AbrirHilo(canal.into(), ts.into()));
        let raiz = e.mensajes[canal]
            .iter()
            .find(|m| m.ts == ts)
            .cloned()
            .unwrap();
        let _ = e.update(Msg::Respuestas(ts.into(), Ok(vec![raiz])));
    }

    fn textos(lista: &[Mensaje]) -> Vec<&str> {
        lista.iter().map(|m| m.text.as_str()).collect()
    }

    #[test]
    fn mensaje_en_otro_canal_queda_sin_leer() {
        let mut e = de_prueba();
        nuevo(&mut e, "C2", msg("2.0", "U1", "hola"));
        let c = e.contadores["C2"];
        assert!(c.no_leidos);
        assert_eq!(c.menciones, 0);
    }

    #[test]
    fn dm_y_mencion_suman_menciones() {
        let mut e = de_prueba();
        nuevo(&mut e, "D1", msg("2.0", "U1", "oye"));
        nuevo(&mut e, "C2", msg("3.0", "U1", "<@U0> mira"));
        nuevo(&mut e, "C2", msg("4.0", "U1", "<@U9> no a ti"));
        assert_eq!(e.contadores["D1"].menciones, 1);
        assert_eq!(e.contadores["C2"].menciones, 1);
        assert_eq!(e.pendientes(), (2, true));
    }

    #[test]
    fn lo_propio_no_queda_sin_leer() {
        let mut e = de_prueba();
        nuevo(&mut e, "C2", msg("2.0", "U0", "yo"));
        assert!(!e.contadores.contains_key("C2"));
    }

    #[test]
    fn canal_abierto_agrega_y_marca_leido_sin_duplicar() {
        let mut e = de_prueba();
        e.contadores.insert(
            "C1".into(),
            Contador {
                no_leidos: true,
                menciones: 1,
                ultimo: 0,
            },
        );
        nuevo(&mut e, "C1", msg("2.0", "U1", "nuevo"));
        nuevo(&mut e, "C1", msg("2.0", "U1", "nuevo"));
        assert_eq!(textos(&e.mensajes["C1"]), ["raíz", "nuevo"]);
        assert!(!e.contadores.contains_key("C1"));
    }

    #[test]
    fn en_segundo_plano_no_marca_leido() {
        let mut e = de_prueba();
        e.visible = false;
        nuevo(&mut e, "C1", msg("2.0", "U1", "nuevo"));
        assert_eq!(e.mensajes["C1"].len(), 2);
        assert!(e.contadores["C1"].no_leidos);
    }

    #[test]
    fn canal_no_cargado_no_guarda_el_mensaje() {
        let mut e = de_prueba();
        nuevo(&mut e, "C2", msg("2.0", "U1", "hola"));
        assert!(!e.mensajes.contains_key("C2"));
    }

    #[test]
    fn respuesta_de_hilo_sube_el_contador_y_llega_al_hilo_abierto() {
        let mut e = de_prueba();
        abrir_hilo(&mut e, "C1", "1.0");
        nuevo(&mut e, "C1", respuesta("2.0", "1.0", "U2"));
        nuevo(&mut e, "C1", respuesta("3.0", "1.0", "U2"));

        let raiz = &e.mensajes["C1"][0];
        assert_eq!(e.mensajes["C1"].len(), 1, "las respuestas no van al canal");
        assert_eq!(raiz.reply_count, Some(2));
        assert_eq!(raiz.reply_users, ["U2"]);
        let hilo = e.hilo.as_ref().unwrap().mensajes.as_ref().unwrap();
        assert_eq!(
            hilo.iter().map(|m| m.ts.as_str()).collect::<Vec<_>>(),
            ["1.0", "2.0", "3.0"]
        );
        assert_eq!(hilo[0].reply_count, Some(2));
    }

    #[test]
    fn editado_y_borrado_en_canal_y_en_hilo() {
        let mut e = de_prueba();
        abrir_hilo(&mut e, "C1", "1.0");
        rtm(
            &mut e,
            rtm::Evento::Editado {
                canal: "C1".into(),
                mensaje: msg("1.0", "U1", "editado"),
            },
        );
        assert_eq!(textos(&e.mensajes["C1"]), ["editado"]);
        assert_eq!(
            textos(e.hilo.as_ref().unwrap().mensajes.as_ref().unwrap()),
            ["editado"]
        );

        rtm(
            &mut e,
            rtm::Evento::Borrado {
                canal: "C1".into(),
                ts: "1.0".into(),
            },
        );
        assert!(e.mensajes["C1"].is_empty());
        assert!(
            e.hilo
                .as_ref()
                .unwrap()
                .mensajes
                .as_ref()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn reaccion_optimista_y_eco_idempotente() {
        let mut e = de_prueba();
        let reaccionar = |e: &mut Espacio| {
            let _ = e.update(Msg::Reaccionar {
                canal: "C1".into(),
                ts: "1.0".into(),
                nombre: "eyes".into(),
            });
        };
        let eco = |e: &mut Espacio, agregada| {
            rtm(
                e,
                rtm::Evento::Reaccion {
                    canal: "C1".into(),
                    ts: "1.0".into(),
                    nombre: "eyes".into(),
                    usuario: "U0".into(),
                    agregada,
                },
            )
        };

        reaccionar(&mut e);
        eco(&mut e, true);
        let r = &e.mensajes["C1"][0].reactions;
        assert_eq!(
            (r.len(), r[0].count, r[0].users.as_slice()),
            (1, 1, &["U0".to_string()][..])
        );

        reaccionar(&mut e);
        eco(&mut e, false);
        assert!(e.mensajes["C1"][0].reactions.is_empty());
    }

    #[test]
    fn nombre_de_grupo_con_nombres_visibles() {
        let mut e = de_prueba();
        let ana = json!({ "id": "U5", "name": "ana", "profile": { "real_name": "Ana Ruiz" } });
        let _ = e.update(Msg::Usuarios(vec![serde_json::from_value(ana).unwrap()]));
        assert_eq!(e.nombre_canal("G1"), "Ana Ruiz, beto");
        assert_eq!(e.nombre_canal("C1"), "general");
        assert_eq!(e.nombre_canal("X9"), "X9");
    }

    fn coincidencia(canal: &str, ts: &str, texto: &str) -> Coincidencia {
        let v = json!({ "ts": ts, "user": "U1", "text": texto, "channel": { "id": canal } });
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn buscar_con_el_filtro_de_arriba() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Buscar);
        assert!(e.busqueda.is_none(), "sin texto no se busca");

        let _ = e.update(Msg::Filtro("  deploy ".into()));
        let _ = e.update(Msg::Buscar);
        let b = e.busqueda.as_ref().unwrap();
        assert_eq!(
            (b.consulta.as_str(), b.resultados.is_none()),
            ("deploy", true)
        );

        let vieja = vec![coincidencia("C1", "1.0", "viejo")];
        let _ = e.update(Msg::Resultados("otra".into(), Ok(vieja)));
        assert!(e.busqueda.as_ref().unwrap().resultados.is_none());

        let nueva = vec![coincidencia("C2", "2.0", "nuevo")];
        let _ = e.update(Msg::Resultados("deploy".into(), Ok(nueva)));
        let r = e.busqueda.as_ref().unwrap().resultados.as_ref().unwrap();
        assert_eq!(r[0].mensaje.text, "nuevo");
    }

    #[test]
    fn busqueda_fallida_muestra_error_y_lista_vacia() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Filtro("x".into()));
        let _ = e.update(Msg::Buscar);
        let _ = e.update(Msg::Resultados(
            "x".into(),
            Err("search.messages: ratelimited".into()),
        ));
        assert_eq!(e.error.as_deref(), Some("search.messages: ratelimited"));
        assert!(
            e.busqueda
                .as_ref()
                .unwrap()
                .resultados
                .as_ref()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn abrir_canal_cierra_la_busqueda_y_ctrl_k() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Filtro("x".into()));
        let _ = e.update(Msg::Buscar);
        let _ = e.update(Msg::AbrirSaltar);
        let _ = e.update(Msg::Seleccionar("C2".into()));
        assert!(e.busqueda.is_none());
        assert!(e.saltar.is_none());
        let _ = e.update(Msg::Buscar);
        let _ = e.update(Msg::CerrarBusqueda);
        assert!(e.busqueda.is_none());
    }

    #[test]
    fn saltar_con_ctrl_k() {
        let mut e = de_prueba();
        let _ = e.update(Msg::AbrirSaltar);
        assert_eq!(e.opciones_saltar().len(), 4, "sin consulta salen todas");

        let _ = e.update(Msg::SaltarConsulta("r".into()));
        assert_eq!(
            e.opciones_saltar(),
            ["C2", "C1"],
            "random empieza con r; general la contiene"
        );
        let _ = e.update(Msg::Flecha(1));
        let _ = e.update(Msg::Flecha(5));
        assert_eq!(e.saltar.as_ref().unwrap().elegido, 1, "no pasa del último");
        let _ = e.update(Msg::Flecha(-3));
        assert_eq!(e.saltar.as_ref().unwrap().elegido, 0, "no baja de cero");
        let _ = e.update(Msg::Flecha(1));

        let _ = e.update(Msg::SaltarConsulta("ra".into()));
        assert_eq!(
            e.saltar.as_ref().unwrap().elegido,
            0,
            "escribir vuelve al primero"
        );
        let _ = e.update(Msg::SaltarConsulta("r".into()));
        let _ = e.update(Msg::Flecha(1));
        let _ = e.update(Msg::SaltarElegir);
        assert!(e.saltar.is_none());
        assert_eq!(e.seleccionado.as_deref(), Some("C1"));
    }

    #[test]
    fn saltar_sin_coincidencias_no_cambia_de_canal() {
        let mut e = de_prueba();
        let _ = e.update(Msg::AbrirSaltar);
        let _ = e.update(Msg::SaltarConsulta("zzz".into()));
        let _ = e.update(Msg::SaltarElegir);
        assert!(e.saltar.is_some());
        assert_eq!(e.seleccionado.as_deref(), Some("C1"));
        let _ = e.update(Msg::Esc);
        assert!(e.saltar.is_none());
    }

    #[test]
    fn directos_por_actividad_reciente() {
        let mut e = de_prueba();
        let dm2: Canal =
            serde_json::from_value(json!({ "id": "D2", "is_im": true, "user": "U2" })).unwrap();
        e.canales.push(dm2);
        let ids = |e: &Espacio| {
            e.directos_recientes()
                .iter()
                .map(|c| c.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(&e),
            ["D1", "G1", "D2"],
            "sin actividad: el orden de la lateral"
        );

        let conteo = |ultimo| Contador {
            ultimo,
            ..Contador::default()
        };
        let _ = e.update(Msg::Contadores(Ok(HashMap::from([
            ("D1".to_string(), conteo(100)),
            ("D2".to_string(), conteo(300)),
            ("G1".to_string(), conteo(200)),
        ]))));
        assert_eq!(ids(&e), ["D2", "G1", "D1"]);

        nuevo(&mut e, "D1", msg("400.0", "U1", "¿sigues ahí?"));
        assert_eq!(ids(&e), ["D1", "D2", "G1"], "un mensaje nuevo lo sube");
        nuevo(&mut e, "G1", respuesta("500.0", "100.0", "U1"));
        assert_eq!(ids(&e)[0], "D1", "una respuesta de hilo no cuenta");
    }

    #[test]
    fn vista_previa_del_ultimo_mensaje() {
        let mut e = de_prueba();
        assert_eq!(
            e.vista_previa("D1"),
            None,
            "sin historial no hay vista previa"
        );
        e.mensajes.insert(
            "D1".into(),
            vec![
                msg("1.0", "U1", "hola *Arturo*\n¿cómo vas?"),
                respuesta("2.0", "1.0", "U1"),
            ],
        );
        assert_eq!(
            e.vista_previa("D1").as_deref(),
            Some("hola Arturo ¿cómo vas?")
        );
        e.mensajes
            .get_mut("D1")
            .unwrap()
            .push(msg("3.0", "U0", "bien :+1:"));
        assert_eq!(e.vista_previa("D1").as_deref(), Some("Tú: bien 👍"));
    }

    fn grupos(e: &Espacio) -> Vec<(String, Vec<&str>)> {
        e.secciones_lateral()
            .into_iter()
            .map(|g| (g.titulo, g.canales.iter().map(|c| c.id.as_str()).collect()))
            .collect()
    }

    fn seccion(id: &str, tipo: TipoSeccion, nombre: &str, canales: &[&str]) -> Seccion {
        Seccion {
            id: id.into(),
            tipo,
            nombre: nombre.into(),
            emoji: String::new(),
            canales: canales.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn lateral_sin_secciones_de_slack() {
        let e = de_prueba();
        assert_eq!(
            grupos(&e),
            [
                ("Canales".to_string(), vec!["C1", "C2"]),
                // A–Z por nombre visible: "ana, beto" antes que "U1".
                ("Mensajes directos".to_string(), vec!["G1", "D1"]),
            ]
        );
    }

    #[test]
    fn lateral_con_las_secciones_del_usuario() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Secciones(Ok(vec![
            seccion("L1", TipoSeccion::Destacados, "", &["D1", "C9"]),
            seccion("L2", TipoSeccion::Propia, "Proyectos", &["C2"]),
            seccion("L3", TipoSeccion::Canales, "", &[]),
            seccion("L4", TipoSeccion::Directos, "", &[]),
            seccion("L5", TipoSeccion::Otra, "", &[]),
        ])));
        assert_eq!(
            grupos(&e),
            [
                ("Destacados".to_string(), vec!["D1"]),
                ("Proyectos".to_string(), vec!["C2"]),
                ("Canales".to_string(), vec!["C1"]),
                ("Mensajes directos".to_string(), vec!["G1"]),
            ]
        );
    }

    #[test]
    fn lo_que_no_cae_en_ninguna_seccion_no_se_pierde() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Secciones(Ok(vec![
            seccion("L1", TipoSeccion::Destacados, "", &[]),
            seccion("L2", TipoSeccion::Propia, "Vacía", &[]),
        ])));
        assert_eq!(
            grupos(&e),
            [
                ("Vacía".to_string(), vec![]),
                ("Canales".to_string(), vec!["C1", "C2"]),
                ("Mensajes directos".to_string(), vec!["G1", "D1"]),
            ],
            "Destacados vacía no se pinta; las propias sí"
        );
    }

    #[test]
    fn seccion_propia_con_emoji() {
        let mut e = de_prueba();
        let cohete = Seccion {
            emoji: "rocket".into(),
            ..seccion("L2", TipoSeccion::Propia, "Proyectos", &["C2"])
        };
        let _ = e.update(Msg::Secciones(Ok(vec![cohete])));
        let g = e.secciones_lateral();
        assert_eq!(g[0].emoji.as_deref(), Some("🚀"));
        assert_eq!(g[1].emoji, None);
    }

    /// Actividad de hace `dias` días para cada canal.
    fn con_actividad(e: &mut Espacio, dias: &[(&str, i64)]) {
        let ahora = chrono::Local::now().timestamp();
        for (id, d) in dias {
            e.actividad.insert(id.to_string(), ahora - d * 86_400);
        }
    }

    #[test]
    fn activos_oculta_lo_viejo_pero_no_lo_abierto() {
        let mut e = de_prueba();
        con_actividad(&mut e, &[("C1", 40), ("C2", 1), ("D1", 2), ("G1", 90)]);
        assert_eq!(e.filtro_lateral, lateral::Filtro::Activos);
        assert_eq!(
            grupos(&e),
            [
                ("Canales".to_string(), vec!["C1", "C2"]),
                ("Mensajes directos".to_string(), vec!["D1"]),
            ],
            "C1 es viejo pero está abierto"
        );
        let _ = e.update(Msg::Seleccionar("C2".into()));
        assert_eq!(grupos(&e)[0].1, ["C2"]);
    }

    #[test]
    fn destacados_no_se_filtran() {
        let mut e = de_prueba();
        con_actividad(&mut e, &[("C1", 1), ("D1", 90)]);
        let _ = e.update(Msg::Secciones(Ok(vec![seccion(
            "L1",
            TipoSeccion::Destacados,
            "",
            &["D1"],
        )])));
        assert_eq!(grupos(&e)[0], ("Destacados".to_string(), vec!["D1"]));
    }

    #[test]
    fn el_menu_cambia_filtro_y_orden() {
        let mut e = de_prueba();
        con_actividad(&mut e, &[("C1", 3), ("C2", 1), ("D1", 90), ("G1", 2)]);
        let _ = e.update(Msg::MenuLateral);
        assert!(e.menu_lateral);
        let _ = e.update(Msg::FiltrarLateral(lateral::Filtro::Todos));
        assert!(!e.menu_lateral, "elegir cierra el menú");
        // A–Z por nombre visible: "ana, beto" (G1) antes que "U1" (D1, sin usuario cargado).
        assert_eq!(grupos(&e)[1].1, ["G1", "D1"]);
        let _ = e.update(Msg::OrdenarLateral(lateral::Orden::Reciente));
        assert_eq!(grupos(&e)[0].1, ["C2", "C1"]);
        assert_eq!(grupos(&e)[1].1, ["G1", "D1"]);
    }

    #[test]
    fn esc_cierra_el_menu_de_la_lateral() {
        let mut e = de_prueba();
        let _ = e.update(Msg::MenuLateral);
        let _ = e.update(Msg::Esc);
        assert!(!e.menu_lateral);
    }

    #[test]
    fn ocultar_silenciados_salvo_el_abierto() {
        let mut e = de_prueba();
        silenciar(&mut e, &["C1", "C2"]);
        let _ = e.update(Msg::AlternarSilenciados);
        assert!(e.ocultar_silenciados);
        assert_eq!(grupos(&e)[0].1, ["C1"]);
        let _ = e.update(Msg::AlternarSilenciados);
        assert_eq!(grupos(&e)[0].1, ["C1", "C2"]);
    }

    #[test]
    fn buscar_conversacion_filtra_por_nombre() {
        let mut e = de_prueba();
        let _ = e.update(Msg::BuscarConversacion("RAN".into()));
        assert_eq!(
            grupos(&e),
            [
                ("Canales".to_string(), vec!["C2"]),
                ("Mensajes directos".to_string(), vec![]),
            ]
        );
    }

    #[test]
    fn el_borrador_vuelve_al_regresar_al_canal() {
        let mut e = de_prueba();
        escribir(&mut e, Destino::Canal, "a medias");
        let _ = e.update(Msg::Seleccionar("C2".into()));
        assert_eq!(e.editor.text().trim(), "");
        assert!(e.tiene_borrador("C1"));
        assert!(!e.tiene_borrador("C2"));
        let _ = e.update(Msg::Seleccionar("C1".into()));
        assert_eq!(e.editor.text().trim(), "a medias");
        assert!(!e.tiene_borrador("C1"), "el abierto no lleva ✎");
        // Ir y volver otra vez no le agrega líneas.
        let antes = e.editor.text();
        let _ = e.update(Msg::Seleccionar("C2".into()));
        let _ = e.update(Msg::Seleccionar("C1".into()));
        assert_eq!(e.editor.text(), antes);
    }

    #[test]
    fn el_hilo_tiene_su_propio_borrador() {
        let mut e = de_prueba();
        escribir(&mut e, Destino::Canal, "al canal");
        abrir_hilo(&mut e, "C1", "1.0");
        escribir(&mut e, Destino::Hilo, "al hilo");
        let _ = e.update(Msg::CerrarHilo);
        abrir_hilo(&mut e, "C1", "1.0");
        assert_eq!(e.hilo.as_ref().unwrap().editor.text().trim(), "al hilo");
        assert_eq!(e.editor.text().trim(), "al canal");
        // Cambiar de canal guarda los dos.
        let _ = e.update(Msg::Seleccionar("C2".into()));
        let _ = e.update(Msg::Seleccionar("C1".into()));
        assert_eq!(e.editor.text().trim(), "al canal");
        abrir_hilo(&mut e, "C1", "1.0");
        assert_eq!(e.hilo.as_ref().unwrap().editor.text().trim(), "al hilo");
    }

    fn silenciar(e: &mut Espacio, ids: &[&str]) {
        let _ = e.update(Msg::Prefs(Ok(Prefs {
            silenciados: ids.iter().map(|id| id.to_string()).collect(),
            tema: None,
            reloj_24: true,
            todos: HashSet::new(),
        })));
    }

    #[test]
    fn las_preferencias_traen_el_tema() {
        let mut e = de_prueba();
        let tema = paleta::Tema {
            primaria: "horchata".into(),
            resalte: "jade".into(),
        };
        let _ = e.update(Msg::Prefs(Ok(Prefs {
            silenciados: HashSet::new(),
            tema: Some(tema.clone()),
            reloj_24: true,
            todos: HashSet::new(),
        })));
        assert_eq!(e.tema, Some(tema));
    }

    fn abierto(e: &Espacio) -> &str {
        e.seleccionado.as_deref().unwrap()
    }

    #[test]
    fn atras_y_adelante_como_el_navegador() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Seleccionar("C2".into()));
        let _ = e.update(Msg::Seleccionar("D1".into()));
        let _ = e.update(Msg::Atras);
        assert_eq!(abierto(&e), "C2");
        let _ = e.update(Msg::Atras);
        assert_eq!(abierto(&e), "C1", "el que estaba abierto al empezar");
        let _ = e.update(Msg::Atras);
        assert_eq!(abierto(&e), "C1");
        let _ = e.update(Msg::Adelante);
        assert_eq!(abierto(&e), "C2");
        let _ = e.update(Msg::Seleccionar("G1".into()));
        let _ = e.update(Msg::Adelante);
        assert_eq!(abierto(&e), "G1", "abrir otro borra el adelante");
        assert_eq!(e.recientes(), ["C2", "C1"]);
    }

    #[test]
    fn encabezado_con_info_y_marcadores() {
        let mut e = de_prueba();
        let info = InfoCanal {
            miembros: Some(40),
            tema: "Lo del día".into(),
            ..Default::default()
        };
        let _ = e.update(Msg::InfoCanal("C1".into(), Ok(info)));
        assert_eq!(e.info.get("C1").and_then(|i| i.miembros), Some(40));
        let tablero = Marcador {
            titulo: "Tablero".into(),
            enlace: "https://ejemplo.com".into(),
            emoji: String::new(),
        };
        let _ = e.update(Msg::Marcadores("C1".into(), Ok(vec![tablero.clone()])));
        assert_eq!(e.marcadores["C1"], [tablero]);
    }

    #[test]
    fn pestanas_del_canal() {
        let mut e = de_prueba();
        assert_eq!(e.pestana, Pestana::Mensajes);
        let _ = e.update(Msg::Pestana(Pestana::Archivos));
        assert_eq!(e.pestana, Pestana::Archivos);
        let plan: Archivo =
            serde_json::from_value(json!({ "id": "F1", "name": "plan.pdf" })).unwrap();
        let _ = e.update(Msg::ArchivosCanal("C1".into(), Ok(vec![plan])));
        assert_eq!(e.archivos_canal["C1"].len(), 1);
        let _ = e.update(Msg::FijadosCanal(
            "C1".into(),
            Ok(vec![msg("5.0", "U1", "fijado")]),
        ));
        assert_eq!(e.fijados["C1"][0].text, "fijado");
        let _ = e.update(Msg::Seleccionar("C2".into()));
        assert_eq!(e.pestana, Pestana::Mensajes, "otro canal vuelve a Mensajes");
    }

    #[test]
    fn detalles_con_miembros() {
        let mut e = de_prueba();
        let _ = e.update(Msg::AbrirDetalles(Detalles::Miembros));
        assert_eq!(e.detalles, Some(Detalles::Miembros));
        let _ = e.update(Msg::MiembrosCanal(
            "C1".into(),
            Ok(vec!["U1".into(), "U7".into()]),
        ));
        assert_eq!(e.miembros["C1"], ["U1", "U7"]);
        assert!(e.usuarios_pedidos.contains("U7"), "pide los que no conoce");
        let _ = e.update(Msg::Esc);
        assert_eq!(e.detalles, None);
    }

    #[test]
    fn buscar_en_el_canal() {
        let mut e = de_prueba();
        let _ = e.update(Msg::BuscarEnCanal);
        assert_eq!(e.filtro, "in:#general ");
        let _ = e.update(Msg::Seleccionar("D1".into()));
        let _ = e.update(Msg::BuscarEnCanal);
        assert_eq!(e.filtro, "in:@U1 ");
    }

    #[test]
    fn enlace_del_canal_con_el_dominio() {
        let mut e = de_prueba();
        assert_eq!(e.enlace_canal("C1"), None, "sin dominio todavía");
        e.equipo = Some(Equipo {
            name: "Acme".into(),
            domain: "acme".into(),
            icon: Default::default(),
        });
        assert_eq!(
            e.enlace_canal("C1").as_deref(),
            Some("https://acme.slack.com/archives/C1")
        );
        let _ = e.update(Msg::MenuCanal);
        assert!(e.menu_canal);
        let _ = e.update(Msg::CopiarEnlaceCanal);
        assert!(!e.menu_canal);
    }

    fn formatear(texto: &str, f: atajos::Formato) -> String {
        let mut e = de_prueba();
        e.editor = text_editor::Content::with_text(texto);
        e.editor.perform(text_editor::Action::SelectAll);
        let _ = e.update(Msg::Formatear(Destino::Canal, f));
        e.editor.text()
    }

    #[test]
    fn formato_en_linea_envuelve_la_seleccion() {
        use atajos::Formato::*;
        assert_eq!(formatear("hola", Negrita).trim_end(), "*hola*");
        assert_eq!(formatear("hola", Codigo).trim_end(), "`hola`");
    }

    #[test]
    fn formato_de_bloque_por_linea() {
        use atajos::Formato::*;
        assert_eq!(formatear("a\nb", Vinetas).trim_end(), "• a\n• b");
        assert_eq!(formatear("a\nb", Numerada).trim_end(), "1. a\n2. b");
        assert_eq!(formatear("a\nb", Cita).trim_end(), "> a\n> b");
        assert_eq!(formatear("x", BloqueCodigo).trim_end(), "```\nx\n```");
    }

    #[test]
    fn formato_de_bloque_sin_seleccion_pone_el_prefijo() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Formatear(Destino::Canal, atajos::Formato::Cita));
        assert_eq!(e.editor.text().trim_end_matches('\n'), "> ");
    }

    #[test]
    fn aa_oculta_la_barra_de_formato() {
        let mut e = de_prueba();
        assert!(e.barra_formato());
        let _ = e.update(Msg::AlternarBarra);
        assert!(!e.barra_formato());
    }

    #[test]
    fn respuesta_tambien_al_canal_y_se_apaga_al_enviar() {
        let mut e = de_prueba();
        abrir_hilo(&mut e, "C1", "1.0");
        let _ = e.update(Msg::AlternarTambien);
        assert!(e.hilo.as_ref().unwrap().tambien);
        escribir(&mut e, Destino::Hilo, "va");
        let _ = e.update(Msg::Enviar(Destino::Hilo));
        assert!(!e.hilo.as_ref().unwrap().tambien);
    }

    #[test]
    fn escribiendo_hasta_que_manda_o_pasan_6_segundos() {
        let mut e = de_prueba();
        let escribe = |canal: &str, usuario: &str| rtm::Evento::Escribiendo {
            canal: canal.into(),
            usuario: usuario.into(),
        };
        rtm(&mut e, escribe("C1", "U1"));
        rtm(&mut e, escribe("C1", "U1"));
        rtm(&mut e, escribe("C1", "U2"));
        rtm(&mut e, escribe("C2", "U3"));
        assert_eq!(e.escribiendo_en("C1"), ["U1", "U2"]);
        nuevo(&mut e, "C1", msg("2.0", "U1", "listo"));
        assert_eq!(e.escribiendo_en("C1"), ["U2"], "al mandar deja de escribir");
        let viejo = std::time::Instant::now() - std::time::Duration::from_secs(7);
        for (_, _, desde) in &mut e.escribiendo {
            *desde = viejo;
        }
        let _ = e.update(Msg::Tick);
        assert!(e.escribiendo.is_empty());
    }

    fn dia(d: u32, h: u32, min: u32) -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 10, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    #[test]
    fn opciones_para_programar() {
        // Miércoles 21 a las 15:30.
        let o = opciones_programar(dia(21, 15, 30));
        assert_eq!(o[0], ("Mañana a las 9:00".to_string(), dia(22, 9, 0)));
        assert_eq!(o[1], ("El lunes a las 9:00".to_string(), dia(26, 9, 0)));
        // Domingo 25: mañana ya es lunes, no se repite.
        let o = opciones_programar(dia(25, 20, 0));
        assert_eq!(o, [("Mañana a las 9:00".to_string(), dia(26, 9, 0))]);
    }

    #[test]
    fn programar_vacia_el_compositor_y_avisa() {
        let mut e = de_prueba();
        let _ = e.update(Msg::MenuEnviar(Destino::Canal));
        assert_eq!(e.menu_enviar, Some(Destino::Canal));
        escribir(&mut e, Destino::Canal, "para mañana");
        let _ = e.update(Msg::Programar(
            Destino::Canal,
            1_800_000_000,
            "Mañana a las 9:00".into(),
        ));
        assert_eq!(e.menu_enviar, None);
        assert_eq!(e.editor.text().trim(), "");
        let _ = e.update(Msg::Programado(
            Destino::Canal,
            "para mañana".into(),
            "Mañana a las 9:00".into(),
            Ok(()),
        ));
        assert_eq!(
            e.nota.as_deref(),
            Some("Mensaje programado: mañana a las 9:00")
        );
        let _ = e.update(Msg::CerrarNota);
        assert_eq!(e.nota, None);
    }

    #[test]
    fn programar_fallido_devuelve_el_texto() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Programado(
            Destino::Canal,
            "para mañana".into(),
            "Mañana a las 9:00".into(),
            Err("chat.scheduleMessage: time_in_past".into()),
        ));
        assert_eq!(e.editor.text().trim(), "para mañana");
        assert!(e.error.is_some());
    }

    fn ruta(nombre: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("/tmp/{nombre}"))
    }

    #[test]
    fn adjuntar_y_quitar_archivos() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Adjuntar(Destino::Canal, ruta("a.png")));
        let _ = e.update(Msg::Elegidos(
            Destino::Canal,
            vec![ruta("a.png"), ruta("b.pdf")],
        ));
        assert_eq!(
            e.adjuntos_de(Destino::Canal),
            [ruta("a.png"), ruta("b.pdf")]
        );
        assert!(e.adjuntos_de(Destino::Hilo).is_empty());
        let _ = e.update(Msg::QuitarAdjunto(Destino::Canal, ruta("a.png")));
        assert_eq!(e.adjuntos_de(Destino::Canal), [ruta("b.pdf")]);
    }

    #[test]
    fn enviar_solo_adjuntos_sin_texto() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Adjuntar(Destino::Canal, ruta("a.png")));
        let _ = e.update(Msg::Enviar(Destino::Canal));
        assert!(
            e.adjuntos_de(Destino::Canal).is_empty(),
            "se tomaron para subir"
        );
    }

    #[test]
    fn subida_fallida_devuelve_adjuntos_y_texto() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Subido(
            Destino::Canal,
            "mira".into(),
            vec![ruta("a.png")],
            Err("files.getUploadURLExternal: not_allowed".into()),
        ));
        assert_eq!(e.adjuntos_de(Destino::Canal), [ruta("a.png")]);
        assert_eq!(e.editor.text().trim(), "mira");
        assert!(e.error.is_some());
    }

    fn conectar(e: &mut Espacio) -> futures::channel::mpsc::Receiver<rtm::Saliente> {
        let (tx, rx) = futures::channel::mpsc::channel(8);
        rtm(e, rtm::Evento::Canal(rtm::Enviador(tx)));
        rx
    }

    #[test]
    fn presencia_de_las_personas() {
        let mut e = de_prueba();
        let presencia = |activo| rtm::Evento::Presencia {
            usuarios: vec!["U1".into()],
            activo,
        };
        rtm(&mut e, presencia(true));
        assert!(e.presente("U1"));
        rtm(&mut e, presencia(false));
        assert!(!e.presente("U1"));
    }

    #[test]
    fn al_conectar_pide_la_presencia_de_las_dms() {
        let mut e = de_prueba();
        let mut rx = conectar(&mut e);
        rtm(&mut e, rtm::Evento::Conectado);
        assert_eq!(
            rx.try_recv().unwrap(),
            rtm::Saliente::Presencia(vec!["U1".into()])
        );
    }

    #[test]
    fn avisa_que_escribes_sin_inundar() {
        let mut e = de_prueba();
        let mut rx = conectar(&mut e);
        escribir(&mut e, Destino::Canal, "hola");
        assert_eq!(
            rx.try_recv().unwrap(),
            rtm::Saliente::Escribiendo("C1".into())
        );
        assert!(rx.try_recv().is_err(), "una vez cada 3 s, no por tecla");
    }

    #[test]
    fn leido_en_otro_dispositivo() {
        let mut e = de_prueba();
        e.contadores.insert(
            "C2".into(),
            Contador {
                no_leidos: true,
                menciones: 2,
                ultimo: 0,
            },
        );
        let leido = |sin_leer, menciones| rtm::Evento::Leido {
            canal: "C2".into(),
            sin_leer,
            menciones,
        };
        rtm(&mut e, leido(3, 1));
        assert_eq!(e.contadores["C2"].menciones, 1);
        rtm(&mut e, leido(0, 0));
        assert!(!e.contadores.contains_key("C2"));
    }

    #[test]
    fn notificacion_de_dm_y_de_mencion() {
        let e = de_prueba();
        let dm = e.aviso_para("D1", &msg("2.0", "U1", "hola")).unwrap();
        assert_eq!((dm.titulo.as_str(), dm.cuerpo.as_str()), ("U1", "hola"));
        assert!(e.aviso_para("C2", &msg("2.0", "U1", "hola")).is_none());
        let mencion = e.aviso_para("C2", &msg("2.0", "U1", "oye <@U0>")).unwrap();
        assert_eq!(mencion.titulo, "U1 en #random");
        assert!(e.aviso_para("D1", &msg("2.0", "U0", "mío")).is_none());
    }

    #[test]
    fn sin_notificacion_si_lo_estas_viendo_o_pausaste() {
        let mut e = de_prueba();
        let mencion = msg("2.0", "U1", "oye <@U0>");
        assert!(
            e.aviso_para("C1", &mencion).is_none(),
            "C1 está abierto y a la vista"
        );
        e.desenfocada = true;
        assert!(
            e.aviso_para("C1", &mencion).is_some(),
            "con la ventana atrás sí"
        );
        e.ajustes.pausado = true;
        assert!(e.aviso_para("C1", &mencion).is_none());
    }

    #[test]
    fn marcar_leido_al_abrir_solo_si_habia_algo() {
        let mut e = de_prueba();
        assert!(!e.hay_que_marcar("C1"));
        e.contadores.insert(
            "C1".into(),
            Contador {
                no_leidos: true,
                ..Default::default()
            },
        );
        assert!(e.hay_que_marcar("C1"));
        e.visible = false;
        assert!(!e.hay_que_marcar("C1"));
    }

    #[test]
    fn un_panel_ocupa_el_centro_hasta_abrir_un_canal() {
        let mut e = de_prueba();
        let _ = e.update(Msg::AbrirPanel(Panel::Hilos));
        assert_eq!(e.panel, Some(Panel::Hilos));
        let _ = e.update(Msg::Seleccionar("C2".into()));
        assert_eq!(e.panel, None);
    }

    #[test]
    fn hilos_y_borradores_llegan() {
        let mut e = de_prueba();
        let hilo = ResumenHilo {
            canal: "C1".into(),
            raiz: msg("1.0", "U7", "raíz"),
            respuestas: vec![respuesta("2.0", "1.0", "U8")],
        };
        let _ = e.update(Msg::HilosVista(Ok(vec![hilo])));
        assert_eq!(e.hilos_vista.as_ref().unwrap().len(), 1);
        assert!(e.usuarios_pedidos.contains("U8"), "pide los autores");
        let b = BorradorSlack {
            canal: "C2".into(),
            mensaje: msg("9.0", "U0", ""),
            programado: 0,
        };
        let _ = e.update(Msg::BorradoresSlack(Ok(vec![b])));
        assert_eq!(e.borradores_slack.as_ref().unwrap()[0].canal, "C2");
    }

    #[test]
    fn actividad_y_guardados_traen_sus_mensajes() {
        let mut e = de_prueba();
        let item = ItemActividad {
            tipo: "at_user".into(),
            canal: "C2".into(),
            ts: "5.0".into(),
            autor: Some("U1".into()),
            no_leido: true,
        };
        let _ = e.update(Msg::ActividadVista(Ok(vec![item])));
        assert_eq!(e.bandeja.as_ref().unwrap().len(), 1);
        let _ = e.update(Msg::Guardados(Ok(vec![("C2".into(), "6.0".into())])));
        assert_eq!(
            e.guardados.as_ref().unwrap(),
            &[("C2".to_string(), "6.0".to_string())]
        );
        let _ = e.update(Msg::MensajesSueltos(Ok(vec![
            ("C2".into(), msg("5.0", "U1", "mención")),
            ("C2".into(), msg("6.0", "U2", "guardado")),
        ])));
        assert_eq!(e.suelto("C2", "6.0").unwrap().text, "guardado");
    }

    #[test]
    fn directorio_ignora_respuestas_viejas() {
        let mut e = de_prueba();
        let _ = e.update(Msg::BuscarPersonas("an".into()));
        assert_eq!(e.consulta_personas, "an");
        let ana: Usuario =
            serde_json::from_value(json!({ "id": "U5", "username": "ana" })).unwrap();
        let _ = e.update(Msg::Personas("a".into(), Ok(vec![ana.clone()])));
        assert!(e.personas.is_none(), "era de una búsqueda anterior");
        let _ = e.update(Msg::Personas("an".into(), Ok(vec![ana])));
        assert_eq!(e.personas.as_ref().unwrap()[0].id, "U5");
    }

    #[test]
    fn mensaje_directo_desde_el_directorio() {
        let mut e = de_prueba();
        let _ = e.update(Msg::AbrirPanel(Panel::Directorio));
        let _ = e.update(Msg::DmAbierto("U5".into(), Ok("D9".into())));
        assert_eq!(e.seleccionado.as_deref(), Some("D9"));
        assert!(
            e.canales
                .iter()
                .any(|c| c.id == "D9" && c.user.as_deref() == Some("U5"))
        );
        assert_eq!(e.panel, None);
    }

    #[test]
    fn perfil_de_una_persona() {
        let mut e = de_prueba();
        let _ = e.update(Msg::VerPerfil("U1".into()));
        assert_eq!(e.perfil.as_deref(), Some("U1"));
        assert!(e.usuarios_pedidos.contains("U1"));
        let _ = e.update(Msg::Esc);
        assert_eq!(e.perfil, None);
    }

    #[test]
    fn guardar_y_quitar_al_momento() {
        let mut e = de_prueba();
        assert!(!e.esta_guardado("C1", "1.0"));
        let _ = e.update(Msg::MenuMensaje("1.0".into()));
        let _ = e.update(Msg::Guardar("C1".into(), "1.0".into()));
        assert!(e.esta_guardado("C1", "1.0"));
        assert_eq!(e.menu_mensaje, None);
        let _ = e.update(Msg::Guardar("C1".into(), "1.0".into()));
        assert!(!e.esta_guardado("C1", "1.0"));
    }

    #[test]
    fn destacar_al_momento() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Secciones(Ok(vec![seccion(
            "L1",
            TipoSeccion::Destacados,
            "",
            &[],
        )])));
        let _ = e.update(Msg::Destacar("C2".into()));
        assert!(e.destacado("C2"));
        assert_eq!(grupos(&e)[0], ("Destacados".to_string(), vec!["C2"]));
        let _ = e.update(Msg::Destacar("C2".into()));
        assert!(!e.destacado("C2"));
    }

    #[test]
    fn la_campana_cambia_el_nivel_y_las_notificaciones() {
        let mut e = de_prueba();
        assert_eq!(e.nivel_aviso("C2"), NivelAviso::Menciones);
        assert!(e.aviso_para("C2", &msg("2.0", "U1", "hola")).is_none());
        let _ = e.update(Msg::AvisosCanal("C2".into(), NivelAviso::Todo));
        assert_eq!(e.nivel_aviso("C2"), NivelAviso::Todo);
        assert!(e.aviso_para("C2", &msg("2.0", "U1", "hola")).is_some());
        let _ = e.update(Msg::AvisosCanal("C2".into(), NivelAviso::Silencio));
        assert!(e.silenciado("C2"));
        assert_eq!(e.nivel_aviso("C2"), NivelAviso::Silencio);
        let _ = e.update(Msg::AvisosCanal("C2".into(), NivelAviso::Menciones));
        assert!(!e.silenciado("C2"));
        assert_eq!(e.nivel_aviso("C2"), NivelAviso::Menciones);
    }

    #[test]
    fn el_reloj_elegido_manda_sobre_el_de_slack() {
        let mut e = de_prueba();
        e.ajustes.reloj = Reloj::Doce;
        assert_eq!(e.formato_hora(), "%-I:%M %p");
        e.reloj_12 = true;
        e.ajustes.reloj = Reloj::Veinticuatro;
        assert_eq!(e.formato_hora(), "%H:%M");
        e.ajustes.reloj = Reloj::Slack;
        assert_eq!(e.formato_hora(), "%-I:%M %p", "según Slack");
    }

    #[test]
    fn aviso_sin_el_texto_si_asi_se_eligio() {
        let mut e = de_prueba();
        e.ajustes.vista_previa = false;
        let aviso = e.aviso_para("D1", &msg("2.0", "U1", "secreto")).unwrap();
        assert_eq!(aviso.cuerpo, "Mensaje nuevo");
    }

    #[test]
    fn sesion_vencida_por_invalid_auth_y_no_por_la_red() {
        let mut e = de_prueba();
        rtm(
            &mut e,
            rtm::Evento::Desconectado("websocket: tiempo agotado".into()),
        );
        assert!(!e.sesion_vencida);
        rtm(
            &mut e,
            rtm::Evento::Desconectado("rtm.connect: invalid_auth".into()),
        );
        assert!(e.sesion_vencida);
        let mut otra = de_prueba();
        let _ = otra.update(Msg::Sesion(Err("auth.test: token_revoked".into())));
        assert!(otra.sesion_vencida);
    }

    #[test]
    fn here_en_otro_canal_cuenta_como_mencion() {
        let mut e = de_prueba();
        nuevo(&mut e, "C2", msg("2.0", "U1", "<!here> junta en 5"));
        assert_eq!(e.contadores["C2"].menciones, 1);
    }

    #[test]
    fn tu_mencion_resalta_la_fila() {
        let e = de_prueba();
        assert!(e.me_menciona(&msg("2.0", "U1", "oye <@U0> mira")));
        assert!(!e.me_menciona(&msg("2.0", "U1", "oye <@U1>")));
        assert!(
            !e.me_menciona(&msg("2.0", "U1", "<!here> junta")),
            "@here solo se resalta en el texto"
        );
    }

    #[test]
    fn hora_en_12_o_24_segun_la_preferencia() {
        let mut e = de_prueba();
        assert_eq!(e.formato_hora(), "%H:%M", "antes de las preferencias, 24 h");
        let _ = e.update(Msg::Prefs(Ok(Prefs {
            silenciados: HashSet::new(),
            tema: None,
            reloj_24: false,
            todos: HashSet::new(),
        })));
        assert_eq!(e.formato_hora(), "%-I:%M %p");
    }

    #[test]
    fn enlace_de_un_mensaje_y_de_una_respuesta() {
        let mut e = de_prueba();
        assert_eq!(e.enlace_mensaje("C1", &msg("1.0", "U1", "x")), None);
        e.equipo = Some(Equipo {
            name: "Acme".into(),
            domain: "acme".into(),
            icon: Default::default(),
        });
        let raiz = msg("1726086000.123456", "U1", "x");
        assert_eq!(
            e.enlace_mensaje("C1", &raiz).as_deref(),
            Some("https://acme.slack.com/archives/C1/p1726086000123456")
        );
        let r = respuesta("1726086100.000200", "1726086000.123456", "U2");
        assert_eq!(
            e.enlace_mensaje("C1", &r).as_deref(),
            Some(
                "https://acme.slack.com/archives/C1/p1726086100000200\
                 ?thread_ts=1726086000.123456&cid=C1"
            )
        );
    }

    #[test]
    fn menu_del_mensaje() {
        let mut e = de_prueba();
        let _ = e.update(Msg::MenuMensaje("1.0".into()));
        assert_eq!(e.menu_mensaje.as_deref(), Some("1.0"));
        let _ = e.update(Msg::MenuMensaje("1.0".into()));
        assert_eq!(e.menu_mensaje, None);
        let _ = e.update(Msg::MenuMensaje("1.0".into()));
        let _ = e.update(Msg::Salir("1.0".into()));
        assert_eq!(e.menu_mensaje, None, "al salir del mensaje se cierra");
    }

    #[test]
    fn marcar_como_no_leido_desde_un_mensaje() {
        let mut e = de_prueba();
        e.mensajes
            .get_mut("C1")
            .unwrap()
            .push(msg("2.0", "U1", "nuevo"));
        let _ = e.update(Msg::MarcarNoLeido("C1".into(), "2.0".into()));
        assert!(e.contadores["C1"].no_leidos);
    }

    #[test]
    fn fijar_y_desfijar_al_momento() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Fijar("C1".into(), "1.0".into()));
        assert_eq!(e.mensajes["C1"][0].pinned_to, ["C1"]);
        let _ = e.update(Msg::Fijar("C1".into(), "1.0".into()));
        assert!(e.mensajes["C1"][0].pinned_to.is_empty());
    }

    #[test]
    fn menu_de_recientes() {
        let mut e = de_prueba();
        let _ = e.update(Msg::MenuRecientes);
        assert!(e.menu_recientes);
        let _ = e.update(Msg::Esc);
        assert!(!e.menu_recientes);
        let _ = e.update(Msg::MenuRecientes);
        let _ = e.update(Msg::Seleccionar("C2".into()));
        assert!(!e.menu_recientes, "elegir cierra el menú");
    }

    #[test]
    fn alt_flechas_recorren_la_lateral() {
        let mut e = de_prueba();
        // Orden visible: Canales C1, C2; Mensajes directos G1 ("ana, beto"), D1 ("U1").
        let _ = e.update(Msg::Vecino(1));
        assert_eq!(abierto(&e), "C2");
        let _ = e.update(Msg::Vecino(1));
        assert_eq!(abierto(&e), "G1");
        let _ = e.update(Msg::Vecino(-1));
        assert_eq!(abierto(&e), "C2");
        let _ = e.update(Msg::Plegar("directos".into()));
        let _ = e.update(Msg::Vecino(1));
        assert_eq!(abierto(&e), "C2", "lo plegado no cuenta");
    }

    #[test]
    fn alt_shift_flechas_saltan_al_siguiente_no_leido() {
        let mut e = de_prueba();
        e.contadores.insert(
            "D1".into(),
            Contador {
                no_leidos: true,
                ..Default::default()
            },
        );
        let _ = e.update(Msg::VecinoSinLeer(1));
        assert_eq!(abierto(&e), "D1");
        let _ = e.update(Msg::VecinoSinLeer(1));
        assert_eq!(abierto(&e), "D1", "no hay más abajo");
    }

    #[test]
    fn esc_sin_capas_marca_leido_el_canal_abierto() {
        let mut e = de_prueba();
        let pendiente = Contador {
            no_leidos: true,
            ..Default::default()
        };
        e.contadores.insert("C1".into(), pendiente);
        let _ = e.update(Msg::AbrirSaltar);
        let _ = e.update(Msg::Esc);
        assert!(
            e.contadores.contains_key("C1"),
            "el primer Esc cierra Ctrl+K"
        );
        let _ = e.update(Msg::Esc);
        assert!(!e.contadores.contains_key("C1"));
    }

    #[test]
    fn shift_esc_marca_todo_leido() {
        let mut e = de_prueba();
        for id in ["C2", "D1"] {
            e.contadores.insert(
                id.into(),
                Contador {
                    no_leidos: true,
                    menciones: 1,
                    ultimo: 1_800_000_000,
                },
            );
        }
        let _ = e.update(Msg::MarcarTodoLeido);
        assert_eq!(e.pendientes(), (0, false));
    }

    #[test]
    fn plegar_por_seccion() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Plegar("directos".into()));
        assert!(e.plegadas.contains("directos"));
        let _ = e.update(Msg::Plegar("directos".into()));
        assert!(e.plegadas.is_empty());
    }

    #[test]
    fn silenciados_no_cuentan_como_pendientes_salvo_menciones() {
        let mut e = de_prueba();
        silenciar(&mut e, &["C2"]);
        nuevo(&mut e, "C2", msg("2.0", "U1", "ruido"));
        assert_eq!(e.pendientes(), (0, false));
        assert!(e.silenciado("C2"));
        nuevo(&mut e, "C2", msg("3.0", "U1", "<@U0> esto sí"));
        assert_eq!(e.pendientes(), (1, false));
    }

    fn propios(e: &mut Espacio) {
        let mapa = HashMap::from([
            ("loro".to_string(), "https://x/loro.gif".to_string()),
            ("gato".to_string(), "https://x/gato.png".to_string()),
        ]);
        let _ = e.update(Msg::EmojiPropios(Ok(mapa)));
    }

    fn pedidos(e: &Espacio) -> Vec<&str> {
        let mut v: Vec<_> = e.emoji_pedidos.iter().map(String::as_str).collect();
        v.sort();
        v
    }

    #[test]
    fn pide_solo_los_emoji_propios_que_aparecen_y_una_vez() {
        let mut e = de_prueba();
        propios(&mut e);
        assert!(pedidos(&e).is_empty());
        let mut m = msg("2.0", "U1", "fiesta :loro: :+1: :nada:");
        m.aplicar_reaccion("gato::skin-tone-2", "U2", true);
        let _ = e.update(Msg::Historial("C2".into(), Ok(vec![m].into())));
        assert_eq!(pedidos(&e), ["gato", "loro"]);
        nuevo(&mut e, "C2", msg("3.0", "U1", ":loro:"));
        assert_eq!(pedidos(&e), ["gato", "loro"]);
    }

    #[test]
    fn emoji_propios_que_llegan_tarde_revisan_lo_cargado() {
        let mut e = de_prueba();
        e.mensajes
            .get_mut("C1")
            .unwrap()
            .push(msg("2.0", "U1", ":gato:"));
        propios(&mut e);
        assert_eq!(pedidos(&e), ["gato"]);
        let _ = e.update(Msg::EmojiImagen("gato".into(), Ok(vec![1, 2, 3])));
        assert!(e.emoji_imagenes.contains_key("gato"));
    }

    #[test]
    fn pide_las_miniaturas_de_imagenes_una_vez() {
        let mut e = de_prueba();
        let con_archivos: Mensaje = serde_json::from_value(json!({
            "ts": "2.0", "user": "U1", "text": "",
            "files": [
                { "id": "F1", "mimetype": "image/png", "thumb_360": "https://files.slack.com/f1.png" },
                { "id": "F2", "mimetype": "application/pdf" }
            ]
        }))
        .unwrap();
        let _ = e.update(Msg::Historial(
            "C2".into(),
            Ok(vec![con_archivos.clone()].into()),
        ));
        let _ = e.update(Msg::Historial("C2".into(), Ok(vec![con_archivos].into())));
        assert_eq!(e.miniaturas_pedidas.iter().collect::<Vec<_>>(), ["F1"]);
        let _ = e.update(Msg::Miniatura("F1".into(), Ok(vec![0])));
        assert!(e.miniaturas.contains_key("F1"));
    }

    #[test]
    fn flechas_sin_capa_no_hacen_nada() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Flecha(1));
        let _ = e.update(Msg::Esc);
        assert!(e.saltar.is_none() && e.selector_emoji.is_none());
    }

    #[test]
    fn reaccionar_con_el_selector() {
        let mut e = de_prueba();
        let destino = DestinoEmoji::Reaccion {
            canal: "C1".into(),
            ts: "1.0".into(),
        };
        let _ = e.update(Msg::AbrirEmoji(destino));
        let _ = e.update(Msg::EmojiConsulta("tad".into()));
        assert_eq!(e.selector_emoji.as_ref().unwrap().consulta, "tad");
        let _ = e.update(Msg::ElegirEmoji("tada".into()));
        assert!(e.selector_emoji.is_none());
        let r = &e.mensajes["C1"][0].reactions;
        assert_eq!(
            (r[0].name.as_str(), r[0].users.as_slice()),
            ("tada", &["U0".to_string()][..])
        );
    }

    #[test]
    fn emoji_del_selector_al_compositor() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Editar(
            Destino::Canal,
            text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new("listo ".into()))),
        ));
        let _ = e.update(Msg::AbrirEmoji(DestinoEmoji::Compositor(Destino::Canal)));
        let _ = e.update(Msg::ElegirEmoji("tada".into()));
        assert_eq!(e.editor.text(), "listo :tada: ");
        let _ = e.update(Msg::AbrirEmoji(DestinoEmoji::Compositor(Destino::Canal)));
        let _ = e.update(Msg::Esc);
        assert!(e.selector_emoji.is_none());
    }

    fn escribir(e: &mut Espacio, destino: Destino, texto: &str) {
        for c in texto.chars() {
            let _ = e.update(Msg::Editar(
                destino,
                text_editor::Action::Edit(text_editor::Edit::Insert(c)),
            ));
        }
    }

    fn con_ana(e: &mut Espacio) {
        let ana = json!({ "id": "U5", "name": "ana", "profile": { "real_name": "Ana Ruiz" } });
        let _ = e.update(Msg::Usuarios(vec![serde_json::from_value(ana).unwrap()]));
    }

    #[test]
    fn autocompleta_menciones_y_las_traduce_al_enviar() {
        let mut e = de_prueba();
        con_ana(&mut e);
        escribir(&mut e, Destino::Canal, "hola @an");
        let a = e.autocompletado.as_ref().unwrap();
        assert_eq!(a.opciones[0].etiqueta, "Ana Ruiz");
        let _ = e.update(Msg::ElegirSugerencia);
        assert!(e.autocompletado.is_none());
        assert_eq!(e.editor.text(), "hola @Ana Ruiz ");
        escribir(&mut e, Destino::Canal, "y ve a #gen");
        assert_eq!(
            e.autocompletado.as_ref().unwrap().opciones[0].etiqueta,
            "general"
        );
        let _ = e.update(Msg::ElegirSugerencia);
        assert_eq!(e.editor.text(), "hola @Ana Ruiz y ve a #general ");
        assert_eq!(e.para_enviar(&e.editor.text()), "hola <@U5> y ve a <#C1> ");
    }

    #[test]
    fn autocompleta_emoji_en_el_hilo() {
        let mut e = de_prueba();
        abrir_hilo(&mut e, "C1", "1.0");
        escribir(&mut e, Destino::Hilo, "listo :tad");
        let a = e.autocompletado.as_ref().unwrap();
        assert_eq!(
            (a.destino, a.opciones[0].etiqueta.as_str()),
            (Destino::Hilo, "tada")
        );
        let _ = e.update(Msg::ElegirSugerencia);
        assert_eq!(e.hilo.as_ref().unwrap().editor.text(), "listo :tada: ");
        assert_eq!(e.editor.text(), "", "el del canal no se toca");
    }

    #[test]
    fn flechas_esc_y_espacio_en_el_autocompletado() {
        let mut e = de_prueba();
        escribir(&mut e, Destino::Canal, "#");
        assert_eq!(e.autocompletado.as_ref().unwrap().opciones.len(), 2);
        let _ = e.update(Msg::Flecha(1));
        let _ = e.update(Msg::Flecha(1));
        assert_eq!(e.autocompletado.as_ref().unwrap().elegido, 1);
        let _ = e.update(Msg::SugerenciaEn(0));
        assert_eq!(
            e.editor.text(),
            "#general ",
            "el clic elige esa, no la marcada"
        );
        escribir(&mut e, Destino::Canal, "@");
        assert!(
            e.autocompletado.is_none(),
            "sin usuarios conocidos no hay opciones"
        );
        con_ana(&mut e);
        escribir(&mut e, Destino::Canal, "a");
        assert!(e.autocompletado.is_some());
        let _ = e.update(Msg::Esc);
        assert!(e.autocompletado.is_none());
        escribir(&mut e, Destino::Canal, "n");
        assert!(
            e.autocompletado.is_some(),
            "seguir escribiendo lo vuelve a abrir"
        );
        escribir(&mut e, Destino::Canal, " ");
        assert!(e.autocompletado.is_none());
    }

    #[test]
    fn envio_fallido_devuelve_el_texto_al_compositor() {
        let mut e = de_prueba();
        escribir(&mut e, Destino::Canal, "hola");
        let _ = e.update(Msg::Enviar(Destino::Canal));
        assert_eq!(e.editor.text(), "");
        let _ = e.update(Msg::Enviado(
            Destino::Canal,
            "hola".into(),
            Err("chat.postMessage: x".into()),
        ));
        assert_eq!(e.editor.text(), "hola");
        assert_eq!(e.error.as_deref(), Some("chat.postMessage: x"));
    }

    #[test]
    fn envio_fallido_no_pisa_lo_nuevo() {
        let mut e = de_prueba();
        escribir(&mut e, Destino::Canal, "otro");
        let _ = e.update(Msg::Enviado(
            Destino::Canal,
            "viejo".into(),
            Err("x".into()),
        ));
        assert_eq!(e.editor.text(), "otro");
        let _ = e.update(Msg::Enviado(Destino::Canal, "ok".into(), Ok(())));
        assert_eq!(e.editor.text(), "otro");
    }

    /// `de_prueba` con un mensaje propio en `#general` que menciona a Ana y a
    /// `#random`, también abierto en el hilo.
    fn con_mensaje_propio() -> Espacio {
        let mut e = de_prueba();
        con_ana(&mut e);
        let propio = msg("2.0", "U0", "hola <@U5>, ve <#C2|random>");
        e.mensajes.get_mut("C1").unwrap().push(propio);
        abrir_hilo(&mut e, "C1", "2.0");
        e
    }

    fn texto_de(e: &Espacio, ts: &str) -> String {
        e.mensajes["C1"]
            .iter()
            .find(|m| m.ts == ts)
            .unwrap()
            .text
            .clone()
    }

    #[test]
    fn editar_un_mensaje_propio() {
        let mut e = con_mensaje_propio();
        let _ = e.update(Msg::EmpezarEdicion("C1".into(), "1.0".into()));
        assert!(e.edicion.is_none(), "el de otro no se edita");

        let _ = e.update(Msg::EmpezarEdicion("C1".into(), "2.0".into()));
        let ed = e.edicion.as_ref().unwrap();
        assert_eq!(ed.editor.text(), "hola @Ana Ruiz, ve #random");

        let _ = e.update(Msg::EditarEdicion(text_editor::Action::Edit(
            text_editor::Edit::Insert('!'),
        )));
        let _ = e.update(Msg::GuardarEdicion);
        assert!(e.edicion.is_none());
        assert_eq!(texto_de(&e, "2.0"), "hola <@U5>, ve <#C2>!");
        let hilo = e.hilo.as_ref().unwrap().mensajes.as_ref().unwrap();
        assert_eq!(
            hilo[0].text, "hola <@U5>, ve <#C2>!",
            "también la copia del hilo"
        );
    }

    #[test]
    fn cancelar_la_edicion_no_cambia_nada() {
        let mut e = con_mensaje_propio();
        let _ = e.update(Msg::EmpezarEdicion("C1".into(), "2.0".into()));
        let _ = e.update(Msg::EditarEdicion(text_editor::Action::Edit(
            text_editor::Edit::Insert('x'),
        )));
        let _ = e.update(Msg::Esc);
        assert!(e.edicion.is_none());
        assert_eq!(texto_de(&e, "2.0"), "hola <@U5>, ve <#C2|random>");
    }

    #[test]
    fn flecha_arriba_edita_el_ultimo_propio() {
        let mut e = con_mensaje_propio();
        e.mensajes
            .get_mut("C1")
            .unwrap()
            .push(msg("3.0", "U1", "ajeno"));
        let _ = e.update(Msg::EditarUltimo(Destino::Canal));
        assert_eq!(e.edicion.as_ref().map(|ed| ed.ts.as_str()), Some("2.0"));
    }

    #[test]
    fn borrar_pide_confirmacion() {
        let mut e = con_mensaje_propio();
        let _ = e.update(Msg::PedirBorrar("C1".into(), "2.0".into()));
        let _ = e.update(Msg::Esc);
        assert!(e.por_borrar.is_none());
        assert_eq!(e.mensajes["C1"].len(), 2);

        let _ = e.update(Msg::PedirBorrar("C1".into(), "2.0".into()));
        assert!(e.por_borrar.is_some());
        let _ = e.update(Msg::ConfirmarBorrar);
        assert!(e.por_borrar.is_none());
        assert_eq!(e.mensajes["C1"].len(), 1);
        assert!(
            e.hilo
                .as_ref()
                .unwrap()
                .mensajes
                .as_ref()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn no_se_borra_lo_ajeno() {
        let mut e = con_mensaje_propio();
        let _ = e.update(Msg::PedirBorrar("C1".into(), "1.0".into()));
        assert!(e.por_borrar.is_none());
    }

    #[test]
    fn fusionar_ordena_y_no_duplica() {
        let mut lista = vec![msg("3.0", "U1", "c"), msg("10.0", "U1", "d")];
        fusionar(
            &mut lista,
            vec![
                msg("1.0", "U1", "a"),
                msg("2.5", "U1", "b"),
                msg("3.0", "U1", "c"),
            ],
        );
        assert_eq!(
            textos(&lista),
            ["a", "b", "c", "d"],
            "10.0 va después de 3.0 aunque como texto no"
        );
    }

    #[test]
    fn subir_hasta_arriba_pide_mas_historial() {
        let mut e = de_prueba();
        let pagina = Pagina {
            mensajes: vec![msg("5.0", "U1", "e")],
            hay_mas: true,
        };
        let _ = e.update(Msg::Historial("C1".into(), Ok(pagina)));
        let _ = e.update(Msg::Desplazado("C1".into(), 900.0));
        assert!(e.cargando_mas.is_empty(), "lejos del tope no pide");
        let _ = e.update(Msg::Desplazado("C1".into(), 50.0));
        assert!(e.cargando_mas.contains("C1"));

        let viejos = Pagina {
            mensajes: vec![msg("3.0", "U1", "c"), msg("4.0", "U1", "d")],
            hay_mas: false,
        };
        let _ = e.update(Msg::Anteriores("C1".into(), Ok(viejos)));
        assert!(e.cargando_mas.is_empty());
        assert_eq!(textos(&e.mensajes["C1"]), ["c", "d", "e"]);
        let _ = e.update(Msg::Desplazado("C1".into(), 0.0));
        assert!(e.cargando_mas.is_empty(), "ya no hay más");
    }

    #[test]
    fn sin_hay_mas_no_pide() {
        let mut e = de_prueba();
        let _ = e.update(Msg::Desplazado("C1".into(), 0.0));
        assert!(
            e.cargando_mas.is_empty(),
            "el canal de prueba no dijo que hubiera más"
        );
    }

    fn conexion(e: &mut Espacio, conectado: bool) {
        let evento = if conectado {
            rtm::Evento::Conectado
        } else {
            rtm::Evento::Desconectado("prueba".into())
        };
        rtm(e, evento);
    }

    #[test]
    fn la_primera_conexion_no_es_un_hueco() {
        let mut e = de_prueba();
        conexion(&mut e, false);
        conexion(&mut e, true);
        assert!(
            !e.hueco,
            "nunca estuvo conectado: lo cargado ya es lo último"
        );
    }

    #[test]
    fn al_reconectar_se_recupera_lo_perdido() {
        let mut e = de_prueba();
        e.mensajes.insert("C2".into(), vec![msg("7.0", "U1", "x")]);
        e.mensajes.insert("D1".into(), vec![]);
        conexion(&mut e, true);
        conexion(&mut e, false);
        assert!(e.hueco);
        let mut pedidos = e.por_recuperar();
        pedidos.sort();
        assert_eq!(
            pedidos,
            [
                ("C1".to_string(), "1.0".to_string()),
                ("C2".to_string(), "7.0".to_string())
            ],
            "un canal vacío se vuelve a pedir al abrirlo"
        );
        conexion(&mut e, true);
        assert!(!e.hueco);

        let _ = e.update(Msg::Posteriores(
            "C1".into(),
            Ok(vec![
                msg("1.0", "U1", "raíz"),
                msg("2.0", "U2", "durante el corte"),
            ]),
        ));
        assert_eq!(textos(&e.mensajes["C1"]), ["raíz", "durante el corte"]);
    }

    #[test]
    fn seleccionar_cierra_el_hilo_y_titulo() {
        let mut e = de_prueba();
        abrir_hilo(&mut e, "C1", "1.0");
        let _ = e.update(Msg::Seleccionar("C2".into()));
        assert!(e.hilo.is_none());
        assert_eq!(e.titulo(), "random - Acme - Slack");
    }
}
