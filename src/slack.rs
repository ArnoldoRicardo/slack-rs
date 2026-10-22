//! Cliente de la Web API de Slack autenticado con tokens de sesión del navegador
//! (`xoxc-…` + cookie `d=xoxd-…`), igual que el cliente web oficial.

use std::collections::{HashMap, HashSet};

use futures::stream::{self, StreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::paleta;

pub type Resultado<T> = Result<T, String>;

const API: &str = "https://slack.com/api/";

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credenciales {
    pub token: String,
    pub cookie: String,
    /// Id del workspace (`T…`) cuando se conoce: lo trae el inicio de sesión desde la
    /// app, no las variables de entorno.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipo: Option<String>,
}

impl Credenciales {
    /// Un workspace por token: `SLACK_TOKEN` admite varios separados por comas o
    /// espacios. La cookie `d` es una sola para todos los workspaces abiertos en el
    /// mismo navegador. Las variables se leen con `leer`.
    pub fn desde(leer: impl Fn(&str) -> Option<String>) -> Resultado<Vec<Self>> {
        let leer = |var: &str| {
            leer(var)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| format!("falta la variable de entorno {var}"))
        };
        let tokens = leer("SLACK_TOKEN")?;
        let cookie = leer("SLACK_COOKIE")?;
        // Se acepta el valor pelón (xoxd-…) o ya con el prefijo `d=`.
        let cookie = cookie.strip_prefix("d=").unwrap_or(&cookie).to_string();
        let mut todas: Vec<Self> = Vec::new();
        for token in tokens.split(|c: char| c == ',' || c.is_whitespace()) {
            if token.is_empty() || todas.iter().any(|c| c.token == token) {
                continue;
            }
            if !token.starts_with("xoxc-") {
                return Err("cada token de SLACK_TOKEN debe empezar con xoxc-".into());
            }
            todas.push(Self {
                token: token.to_string(),
                cookie: cookie.clone(),
                equipo: None,
            });
        }
        Ok(todas)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sesion {
    pub user_id: String,
    pub team: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Equipo {
    pub name: String,
    /// `<dominio>.slack.com`, para los enlaces.
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub icon: Icono,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Icono {
    #[serde(default)]
    pub image_68: Option<String>,
    #[serde(default)]
    pub image_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Canal {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub is_im: bool,
    #[serde(default)]
    pub is_mpim: bool,
    #[serde(default)]
    pub is_private: bool,
    /// En los DMs, el usuario del otro lado.
    #[serde(default)]
    pub user: Option<String>,
}

impl Canal {
    pub fn es_directo(&self) -> bool {
        self.is_im || self.is_mpim
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Mensaje {
    pub ts: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub thread_ts: Option<String>,
    #[serde(default)]
    pub reply_count: Option<u32>,
    #[serde(default)]
    pub reply_users: Vec<String>,
    #[serde(default)]
    pub reactions: Vec<Reaccion>,
    #[serde(default)]
    pub files: Vec<Archivo>,
    #[serde(default)]
    pub attachments: Vec<Adjunto>,
    /// Canales donde está fijado.
    #[serde(default)]
    pub pinned_to: Vec<String>,
    /// Contenido de los bloques `rich_text`, si el mensaje solo trae de esos.
    #[serde(default, rename = "blocks", deserialize_with = "solo_rich_text")]
    pub rico: Option<Vec<ElementoRico>>,
}

/// Lee `blocks` sin hacer fallar el mensaje: si hay un bloque que no es `rich_text`
/// (los de bots) o no se entiende, se descarta todo y se pinta `text`.
fn solo_rich_text<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Vec<ElementoRico>>, D::Error> {
    #[derive(Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum Bloque {
        RichText { elements: Vec<ElementoRico> },
    }
    let crudos = Vec::<Value>::deserialize(d)?;
    let mut todos = Vec::new();
    for b in crudos {
        match serde_json::from_value(b) {
            Ok(Bloque::RichText { elements }) => todos.extend(elements),
            Err(_) => return Ok(None),
        }
    }
    Ok(Some(todos).filter(|t| !t.is_empty()))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ElementoRico {
    RichTextSection {
        #[serde(default)]
        elements: Vec<EnLinea>,
    },
    RichTextList {
        #[serde(default)]
        style: String,
        #[serde(default)]
        elements: Vec<ElementoRico>,
    },
    RichTextQuote {
        #[serde(default)]
        elements: Vec<EnLinea>,
    },
    RichTextPreformatted {
        #[serde(default)]
        elements: Vec<EnLinea>,
    },
    #[serde(other)]
    Otro,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EnLinea {
    Text {
        text: String,
        #[serde(default)]
        style: EstiloRico,
    },
    User {
        user_id: String,
        #[serde(default)]
        style: EstiloRico,
    },
    Channel {
        channel_id: String,
        #[serde(default)]
        style: EstiloRico,
    },
    Link {
        url: String,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        style: EstiloRico,
    },
    Emoji {
        name: String,
        #[serde(default)]
        unicode: Option<String>,
    },
    Broadcast {
        range: String,
    },
    Usergroup {
        usergroup_id: String,
    },
    #[serde(other)]
    Otro,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct EstiloRico {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub strike: bool,
    #[serde(default)]
    pub code: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Reaccion {
    pub name: String,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Archivo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub mimetype: Option<String>,
    #[serde(default)]
    pub thumb_360: Option<String>,
    #[serde(default)]
    pub thumb_360_w: u32,
    #[serde(default)]
    pub thumb_360_h: u32,
    /// Página del archivo en Slack, para abrirlo en el navegador.
    #[serde(default)]
    pub permalink: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub pretty_type: Option<String>,
    /// Quién lo subió.
    #[serde(default)]
    pub user: Option<String>,
    /// Segundos Unix en que se subió.
    #[serde(default)]
    pub created: i64,
}

impl Archivo {
    /// URL y tamaño de la miniatura si es una imagen.
    pub fn miniatura(&self) -> Option<(&str, u32, u32)> {
        if !self.mimetype.as_deref()?.starts_with("image/") {
            return None;
        }
        let url = self.thumb_360.as_deref()?;
        Some((url, self.thumb_360_w, self.thumb_360_h))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Adjunto {
    #[serde(default)]
    pub fallback: Option<String>,
    /// Barra de color a la izquierda: `#e01e5a`, `good`, `warning`, `danger`.
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub pretext: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub title_link: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default, rename = "fields")]
    pub campos: Vec<Campo>,
    #[serde(default)]
    pub footer: Option<String>,
}

/// Campo de un adjunto: título en negrita y valor; los cortos van de a dos.
#[derive(Debug, Clone, Deserialize)]
pub struct Campo {
    #[serde(default, rename = "title")]
    pub titulo: String,
    #[serde(default, rename = "value")]
    pub valor: String,
    #[serde(default, rename = "short")]
    pub corto: bool,
}

impl Adjunto {
    /// Si trae algo más que el `fallback`, para pintarlo como tarjeta.
    pub fn tiene_contenido(&self) -> bool {
        let lleno = |t: &Option<String>| t.as_deref().is_some_and(|t| !t.trim().is_empty());
        lleno(&self.title) || lleno(&self.text) || lleno(&self.pretext) || !self.campos.is_empty()
    }
}

impl Mensaje {
    /// Respuesta dentro de un hilo que no se publicó también en el canal.
    pub fn es_respuesta_de_hilo(&self) -> bool {
        self.thread_ts.as_deref().is_some_and(|t| t != self.ts)
            && self.subtype.as_deref() != Some("thread_broadcast")
    }

    pub fn segundos(&self) -> i64 {
        segundos(&self.ts)
    }

    /// Texto a mostrar; los bots a veces solo mandan adjuntos.
    pub fn cuerpo(&self) -> &str {
        if !self.text.is_empty() {
            return &self.text;
        }
        self.attachments
            .iter()
            .find_map(|a| a.fallback.as_deref())
            .unwrap_or("")
    }

    /// Agrega o quita la reacción de `usuario`; es idempotente para que el eco del
    /// tiempo real no cuente doble lo que ya se aplicó localmente.
    pub fn aplicar_reaccion(&mut self, nombre: &str, usuario: &str, agregar: bool) {
        let i = match self.reactions.iter().position(|r| r.name == nombre) {
            Some(i) => i,
            None if agregar => {
                self.reactions.push(Reaccion {
                    name: nombre.into(),
                    users: vec![],
                    count: 0,
                });
                self.reactions.len() - 1
            }
            None => return,
        };
        let r = &mut self.reactions[i];
        let ya = r.users.iter().any(|u| u == usuario);
        match (agregar, ya) {
            (true, false) => {
                r.users.push(usuario.into());
                r.count += 1;
            }
            (false, true) => {
                r.users.retain(|u| u != usuario);
                r.count = r.count.saturating_sub(1);
            }
            _ => {}
        }
        if self.reactions[i].count == 0 {
            self.reactions.remove(i);
        }
    }
}

/// Una página del historial de un canal.
#[derive(Debug, Clone, Default)]
pub struct Pagina {
    pub mensajes: Vec<Mensaje>,
    /// Si hay mensajes más viejos.
    pub hay_mas: bool,
}

#[cfg(test)]
impl From<Vec<Mensaje>> for Pagina {
    fn from(mensajes: Vec<Mensaje>) -> Self {
        Self {
            mensajes,
            hay_mas: false,
        }
    }
}

/// Un resultado de `search.messages`: el mensaje y el canal donde está.
#[derive(Debug, Clone, Deserialize)]
pub struct Coincidencia {
    #[serde(flatten)]
    pub mensaje: Mensaje,
    pub channel: CanalRef,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CanalRef {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usuario {
    pub id: String,
    /// `username` en `search.modules.people`.
    #[serde(default, alias = "username")]
    pub name: String,
    /// Zona horaria, para la hora local del perfil.
    #[serde(default)]
    pub tz: Option<String>,
    #[serde(default)]
    pub tz_offset: i64,
    #[serde(default)]
    pub profile: Perfil,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Perfil {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub real_name: Option<String>,
    #[serde(default)]
    pub image_72: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub status_emoji: Option<String>,
}

impl Usuario {
    pub fn nombre_visible(&self) -> &str {
        [&self.profile.display_name, &self.profile.real_name]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .find(|n| !n.is_empty())
            .unwrap_or(&self.name)
    }
}

/// Segundos Unix de un `ts` de Slack (`"1718000000.000100"`); 0 si no se entiende.
pub fn segundos(ts: &str) -> i64 {
    ts.split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// No leídos de un canal según `client.counts`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Contador {
    pub no_leidos: bool,
    pub menciones: u32,
    /// Segundos del último mensaje del canal; 0 si no se sabe.
    pub ultimo: i64,
}

/// Un hilo de la vista Hilos: raíz y últimas respuestas.
#[derive(Debug, Clone)]
pub struct ResumenHilo {
    pub canal: String,
    pub raiz: Mensaje,
    pub respuestas: Vec<Mensaje>,
}

/// Algo de la bandeja Actividad que apunta a un mensaje.
#[derive(Debug, Clone)]
pub struct ItemActividad {
    /// `at_user`, `at_channel`, `thread_v2`, `keyword`…
    pub tipo: String,
    pub canal: String,
    pub ts: String,
    pub autor: Option<String>,
    pub no_leido: bool,
}

/// Borrador de Slack (de cualquier dispositivo), o mensaje programado.
#[derive(Debug, Clone)]
pub struct BorradorSlack {
    pub canal: String,
    /// El texto, en bloques `rich_text`, como un mensaje.
    pub mensaje: Mensaje,
    /// Segundos Unix si está programado; 0 si es un borrador.
    pub programado: i64,
}

/// Datos del canal para el encabezado y los detalles.
#[derive(Debug, Clone, Default)]
pub struct InfoCanal {
    /// `None` en las DMs, que no lo traen.
    pub miembros: Option<u32>,
    pub tema: String,
    pub proposito: String,
    pub creado: i64,
    pub creador: Option<String>,
}

/// Un marcador (bookmark) de la barra de pestañas del canal.
#[derive(Debug, Clone, PartialEq)]
pub struct Marcador {
    pub titulo: String,
    pub enlace: String,
    /// Nombre del emoji sin dos puntos; vacío si no tiene.
    pub emoji: String,
}

/// Lo que el clon usa de `users.prefs.get`.
#[derive(Debug, Clone)]
pub struct Prefs {
    /// Canales silenciados, de `muted_channels` y de `all_notifications_prefs`.
    pub silenciados: HashSet<String>,
    /// Tema del usuario (`ia_theme`); `None` si no eligió uno.
    pub tema: Option<paleta::Tema>,
    /// Reloj de 24 horas (`time24`).
    pub reloj_24: bool,
    /// Canales en "Todos los mensajes" (`desktop: everything`).
    pub todos: HashSet<String>,
}

/// Cuánto avisa un canal: la campana del encabezado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelAviso {
    Todo,
    Menciones,
    Silencio,
}

/// Sección de la lateral del usuario (`users.channelSections.list`).
#[derive(Debug, Clone)]
pub struct Seccion {
    pub id: String,
    pub tipo: TipoSeccion,
    /// Vacío en las de Slack (Destacados, Canales…); el que puso el usuario en las suyas.
    pub nombre: String,
    pub emoji: String,
    /// Las conversaciones puestas a mano en la sección. En *Canales* y *Mensajes
    /// directos* va vacío: ahí cae todo lo que no está en otra.
    pub canales: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoSeccion {
    Destacados,
    Canales,
    Directos,
    Propia,
    /// Apps recientes, Salesforce y demás que el clon no pinta.
    Otra,
}

#[derive(Clone)]
pub struct Cliente {
    http: reqwest::Client,
    cred: Credenciales,
    /// Raíz de la Web API; en las pruebas apunta a un servidor falso.
    base: String,
}

impl Cliente {
    pub fn new(cred: Credenciales) -> Self {
        Self::con_base(cred, API)
    }

    pub fn con_base(cred: Credenciales, base: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("slack-rs/0.1")
            .build()
            .expect("cliente HTTP");
        Self {
            http,
            cred,
            base: base.into(),
        }
    }

    pub fn credenciales(&self) -> &Credenciales {
        &self.cred
    }

    async fn llamar<T: DeserializeOwned>(
        &self,
        metodo: &str,
        params: &[(&str, &str)],
    ) -> Resultado<T> {
        let mut form = vec![("token", self.cred.token.as_str())];
        form.extend_from_slice(params);
        let resp: Value = self
            .http
            .post(format!("{}{metodo}", self.base))
            .header(reqwest::header::COOKIE, format!("d={}", self.cred.cookie))
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("{metodo}: {e}"))?
            .json()
            .await
            .map_err(|e| format!("{metodo}: respuesta inválida: {e}"))?;
        if resp["ok"].as_bool() != Some(true) {
            let error = resp["error"].as_str().unwrap_or("error desconocido");
            return Err(format!("{metodo}: {error}"));
        }
        serde_json::from_value(resp).map_err(|e| format!("{metodo}: {e}"))
    }

    pub async fn sesion(self) -> Resultado<Sesion> {
        self.llamar("auth.test", &[]).await
    }

    pub async fn equipo(self) -> Resultado<Equipo> {
        #[derive(Deserialize)]
        struct Info {
            team: Equipo,
        }
        self.llamar::<Info>("team.info", &[]).await.map(|i| i.team)
    }

    /// Canales, privados, grupos y DMs de los que el usuario es miembro.
    pub async fn canales(self) -> Resultado<Vec<Canal>> {
        #[derive(Deserialize)]
        struct Pagina {
            channels: Vec<Canal>,
            #[serde(default)]
            response_metadata: Option<Metadatos>,
        }
        #[derive(Deserialize)]
        struct Metadatos {
            #[serde(default)]
            next_cursor: String,
        }

        let mut todos = Vec::new();
        let mut cursor = String::new();
        loop {
            let pagina: Pagina = self
                .llamar(
                    "users.conversations",
                    &[
                        ("types", "public_channel,private_channel,mpim,im"),
                        ("exclude_archived", "true"),
                        ("limit", "200"),
                        ("cursor", &cursor),
                    ],
                )
                .await?;
            todos.extend(pagina.channels);
            cursor = pagina
                .response_metadata
                .map(|m| m.next_cursor)
                .unwrap_or_default();
            if cursor.is_empty() {
                return Ok(todos);
            }
        }
    }

    /// No leídos y menciones por canal. `client.counts` no está documentado: es el
    /// que usa el cliente web.
    pub async fn contadores(self) -> Resultado<HashMap<String, Contador>> {
        #[derive(Deserialize)]
        struct Conteos {
            #[serde(default)]
            channels: Vec<Conteo>,
            #[serde(default)]
            mpims: Vec<Conteo>,
            #[serde(default)]
            ims: Vec<Conteo>,
        }
        #[derive(Deserialize)]
        struct Conteo {
            id: String,
            #[serde(default)]
            has_unreads: bool,
            #[serde(default)]
            mention_count: u32,
            #[serde(default)]
            latest: Option<String>,
        }
        let c: Conteos = self.llamar("client.counts", &[]).await?;
        Ok(c.channels
            .into_iter()
            .chain(c.mpims)
            .chain(c.ims)
            .map(|c| {
                let contador = Contador {
                    no_leidos: c.has_unreads,
                    menciones: c.mention_count,
                    ultimo: c.latest.as_deref().map_or(0, segundos),
                };
                (c.id, contador)
            })
            .collect())
    }

    /// Secciones de la lateral en el orden en que las acomodó el usuario. No está
    /// documentado: es el que usa el cliente web. Las secciones se encadenan con
    /// `next_channel_section_id`; las que queden fuera de la cadena van al final.
    pub async fn secciones(self) -> Resultado<Vec<Seccion>> {
        #[derive(Deserialize)]
        struct Respuesta {
            channel_sections: Vec<Cruda>,
        }
        #[derive(Deserialize)]
        struct Cruda {
            channel_section_id: String,
            #[serde(rename = "type")]
            tipo: String,
            #[serde(default)]
            name: String,
            #[serde(default)]
            emoji: String,
            #[serde(default)]
            next_channel_section_id: Option<String>,
            #[serde(default)]
            channel_ids_page: Option<Pagina>,
        }
        #[derive(Deserialize)]
        struct Pagina {
            #[serde(default)]
            channel_ids: Vec<String>,
        }
        let r: Respuesta = self.llamar("users.channelSections.list", &[]).await?;
        let mut crudas = r.channel_sections;
        let siguientes: HashSet<_> = crudas
            .iter()
            .filter_map(|c| c.next_channel_section_id.clone())
            .collect();
        let mut actual = crudas
            .iter()
            .find(|c| !siguientes.contains(&c.channel_section_id))
            .map(|c| c.channel_section_id.clone());
        let mut ordenadas = Vec::with_capacity(crudas.len());
        while let Some(id) = actual
            && let Some(i) = crudas.iter().position(|c| c.channel_section_id == id)
        {
            let c = crudas.remove(i);
            actual = c.next_channel_section_id.clone();
            ordenadas.push(c);
        }
        ordenadas.append(&mut crudas);
        Ok(ordenadas
            .into_iter()
            .map(|c| Seccion {
                tipo: match c.tipo.as_str() {
                    "stars" => TipoSeccion::Destacados,
                    "channels" => TipoSeccion::Canales,
                    "direct_messages" => TipoSeccion::Directos,
                    "standard" => TipoSeccion::Propia,
                    _ => TipoSeccion::Otra,
                },
                id: c.channel_section_id,
                nombre: c.name,
                emoji: c.emoji,
                canales: c
                    .channel_ids_page
                    .map(|p| p.channel_ids)
                    .unwrap_or_default(),
            })
            .collect())
    }

    /// Conversaciones silenciadas. Slack las ha guardado en dos preferencias: la vieja
    /// `muted_channels` (IDs separados por comas) y `all_notifications_prefs`, un JSON
    /// dentro de un texto con `channels.<id>.muted`. Se juntan las dos.
    pub async fn prefs(self) -> Resultado<Prefs> {
        #[derive(Deserialize)]
        struct Respuesta {
            prefs: Preferencias,
        }
        #[derive(Deserialize)]
        struct Preferencias {
            #[serde(default)]
            muted_channels: String,
            #[serde(default)]
            all_notifications_prefs: String,
            #[serde(default)]
            ia_theme: String,
            #[serde(default)]
            time24: bool,
        }
        #[derive(Deserialize)]
        struct Notificaciones {
            #[serde(default)]
            channels: HashMap<String, PorCanal>,
        }
        #[derive(Deserialize)]
        struct PorCanal {
            #[serde(default)]
            muted: bool,
            #[serde(default)]
            desktop: Option<String>,
        }
        let p = self
            .llamar::<Respuesta>("users.prefs.get", &[])
            .await?
            .prefs;
        let mut ids: HashSet<String> = p
            .muted_channels
            .split(',')
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect();
        let mut todos = HashSet::new();
        if let Ok(n) = serde_json::from_str::<Notificaciones>(&p.all_notifications_prefs) {
            for (id, c) in n.channels {
                if c.desktop.as_deref() == Some("everything") {
                    todos.insert(id.clone());
                }
                if c.muted {
                    ids.insert(id);
                }
            }
        }
        Ok(Prefs {
            silenciados: ids,
            tema: paleta::Tema::de_json(&p.ia_theme),
            reloj_24: p.time24,
            todos,
        })
    }

    /// Emoji propios del workspace: nombre → URL de la imagen, con los alias ya
    /// resueltos. Los alias a emoji estándar (`alias:thumbsup`) se descartan.
    pub async fn emoji(self) -> Resultado<HashMap<String, String>> {
        #[derive(Deserialize)]
        struct Lista {
            emoji: HashMap<String, String>,
        }
        let crudos = self.llamar::<Lista>("emoji.list", &[]).await?.emoji;
        let resolver = |nombre: &str| {
            let mut valor = crudos.get(nombre)?;
            // Tope por si hay un ciclo de alias.
            for _ in 0..8 {
                match valor.strip_prefix("alias:") {
                    Some(otro) => valor = crudos.get(otro)?,
                    None => return Some(valor.clone()),
                }
            }
            None
        };
        Ok(crudos
            .keys()
            .filter_map(|n| Some((n.clone(), resolver(n)?)))
            .collect())
    }

    /// Hasta 100 mensajes del canal, del más viejo al más nuevo: los últimos, o con
    /// `antes` los anteriores a ese `ts`.
    pub async fn historial(self, canal: String, antes: Option<String>) -> Resultado<Pagina> {
        #[derive(Deserialize)]
        struct Historial {
            messages: Vec<Mensaje>,
            #[serde(default)]
            has_more: bool,
        }
        let mut params = vec![("channel", canal.as_str()), ("limit", "100")];
        if let Some(ts) = &antes {
            params.push(("latest", ts));
        }
        let h: Historial = self.llamar("conversations.history", &params).await?;
        Ok(Pagina {
            mensajes: h.messages.into_iter().rev().collect(),
            hay_mas: h.has_more,
        })
    }

    /// Miembros, tema, propósito y creación (`conversations.info`).
    pub async fn info_canal(self, canal: String) -> Resultado<InfoCanal> {
        #[derive(Deserialize)]
        struct Respuesta {
            channel: Crudo,
        }
        #[derive(Deserialize)]
        struct Crudo {
            #[serde(default)]
            num_members: Option<u32>,
            #[serde(default)]
            topic: Option<Valor>,
            #[serde(default)]
            purpose: Option<Valor>,
            #[serde(default)]
            created: i64,
            #[serde(default)]
            creator: Option<String>,
        }
        #[derive(Deserialize)]
        struct Valor {
            #[serde(default)]
            value: String,
        }
        let params = [("channel", canal.as_str()), ("include_num_members", "true")];
        let c = self
            .llamar::<Respuesta>("conversations.info", &params)
            .await?
            .channel;
        Ok(InfoCanal {
            miembros: c.num_members,
            tema: c.topic.map(|t| t.value).unwrap_or_default(),
            proposito: c.purpose.map(|p| p.value).unwrap_or_default(),
            creado: c.created,
            creador: c.creator.filter(|u| !u.is_empty()),
        })
    }

    /// Ids de los miembros, hasta 1000.
    pub async fn miembros(self, canal: String) -> Resultado<Vec<String>> {
        #[derive(Deserialize)]
        struct Pagina {
            members: Vec<String>,
            #[serde(default)]
            response_metadata: Option<Metadatos>,
        }
        #[derive(Deserialize)]
        struct Metadatos {
            #[serde(default)]
            next_cursor: String,
        }
        let mut todos = Vec::new();
        let mut cursor = String::new();
        for _ in 0..5 {
            let params = [
                ("channel", canal.as_str()),
                ("limit", "200"),
                ("cursor", &cursor),
            ];
            let pagina: Pagina = self.llamar("conversations.members", &params).await?;
            todos.extend(pagina.members);
            cursor = pagina
                .response_metadata
                .map(|m| m.next_cursor)
                .unwrap_or_default();
            if cursor.is_empty() {
                break;
            }
        }
        Ok(todos)
    }

    /// Marcadores del canal que son enlaces (`bookmarks.list`).
    pub async fn marcadores(self, canal: String) -> Resultado<Vec<Marcador>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            bookmarks: Vec<Crudo>,
        }
        #[derive(Deserialize)]
        struct Crudo {
            #[serde(default)]
            title: String,
            #[serde(default)]
            link: String,
            #[serde(default)]
            emoji: String,
            #[serde(rename = "type", default)]
            tipo: String,
        }
        let r: Respuesta = self
            .llamar("bookmarks.list", &[("channel_id", canal.as_str())])
            .await?;
        Ok(r.bookmarks
            .into_iter()
            .filter(|b| b.tipo == "link" && !b.link.is_empty())
            .map(|b| Marcador {
                titulo: b.title,
                enlace: b.link,
                emoji: b.emoji.trim_matches(':').to_string(),
            })
            .collect())
    }

    /// Hilos en los que participas (`subscriptions.thread.getView`, sin documentar).
    pub async fn hilos(self) -> Resultado<Vec<ResumenHilo>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            threads: Vec<Crudo>,
        }
        #[derive(Deserialize)]
        struct Crudo {
            root_msg: Raiz,
            #[serde(default)]
            latest_replies: Vec<Mensaje>,
        }
        #[derive(Deserialize)]
        struct Raiz {
            #[serde(default)]
            channel: String,
            #[serde(flatten)]
            mensaje: Mensaje,
        }
        let params = [("limit", "20"), ("priority_mode", "all")];
        let r: Respuesta = self.llamar("subscriptions.thread.getView", &params).await?;
        Ok(r.threads
            .into_iter()
            .map(|h| ResumenHilo {
                canal: h.root_msg.channel,
                raiz: h.root_msg.mensaje,
                respuestas: h.latest_replies,
            })
            .collect())
    }

    /// Menciones e hilos de la bandeja Actividad (`activity.feed`, sin documentar).
    pub async fn actividad(self) -> Resultado<Vec<ItemActividad>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            items: Vec<Crudo>,
        }
        #[derive(Deserialize)]
        struct Crudo {
            #[serde(default)]
            is_unread: bool,
            item: Item,
        }
        #[derive(Deserialize)]
        struct Item {
            #[serde(rename = "type", default)]
            tipo: String,
            #[serde(default)]
            message: Option<Apunta>,
        }
        #[derive(Deserialize)]
        struct Apunta {
            ts: String,
            channel: String,
            #[serde(default)]
            author_user_id: Option<String>,
        }
        let params = [
            ("limit", "30"),
            ("mode", "chrono_v1"),
            (
                "types",
                "at_user,at_user_group,at_channel,at_everyone,keyword,thread_v2",
            ),
            ("is_activity_inbox", "true"),
        ];
        let r: Respuesta = self.llamar("activity.feed", &params).await?;
        Ok(r.items
            .into_iter()
            .filter_map(|c| {
                let m = c.item.message?;
                Some(ItemActividad {
                    tipo: c.item.tipo,
                    canal: m.channel,
                    ts: m.ts,
                    autor: m.author_user_id,
                    no_leido: c.is_unread,
                })
            })
            .collect())
    }

    /// Mensajes sueltos por canal y `ts` (`messages.list`, sin documentar).
    pub async fn mensajes_por_id(
        self,
        ids: Vec<(String, String)>,
    ) -> Resultado<Vec<(String, Mensaje)>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            messages: HashMap<String, Vec<Mensaje>>,
        }
        let mut por_canal: Vec<(String, Vec<String>)> = Vec::new();
        for (canal, ts) in ids {
            match por_canal.iter_mut().find(|(c, _)| *c == canal) {
                Some((_, lista)) => lista.push(ts),
                None => por_canal.push((canal, vec![ts])),
            }
        }
        let pedido: Vec<Value> = por_canal
            .iter()
            .map(|(c, ts)| serde_json::json!({ "channel": c, "timestamps": ts }))
            .collect();
        let pedido = serde_json::to_string(&pedido).expect("json válido");
        let r: Respuesta = self
            .llamar("messages.list", &[("message_ids", &pedido)])
            .await?;
        Ok(r.messages
            .into_iter()
            .flat_map(|(c, lista)| lista.into_iter().map(move |m| (c.clone(), m)))
            .collect())
    }

    /// Mensajes guardados para después: canal y `ts` (`saved.list`, sin documentar).
    pub async fn guardados(self) -> Resultado<Vec<(String, String)>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            saved_items: Vec<Item>,
        }
        #[derive(Deserialize)]
        struct Item {
            #[serde(default)]
            item_id: String,
            #[serde(default)]
            item_type: String,
            #[serde(default)]
            ts: Option<String>,
        }
        let params = [("limit", "50"), ("filter", "saved")];
        let r: Respuesta = self.llamar("saved.list", &params).await?;
        Ok(r.saved_items
            .into_iter()
            .filter(|i| i.item_type == "message")
            .filter_map(|i| Some((i.item_id, i.ts?)))
            .collect())
    }

    /// Borradores y programados de todos tus dispositivos (`drafts.list`).
    pub async fn borradores(self) -> Resultado<Vec<BorradorSlack>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            drafts: Vec<Crudo>,
        }
        #[derive(Deserialize)]
        struct Crudo {
            #[serde(default)]
            last_updated_ts: String,
            #[serde(default)]
            blocks: Value,
            #[serde(default)]
            is_sent: bool,
            #[serde(default)]
            is_deleted: bool,
            #[serde(default)]
            date_scheduled: i64,
            #[serde(default)]
            destinations: Vec<Destino>,
        }
        #[derive(Deserialize)]
        struct Destino {
            #[serde(default)]
            channel_id: String,
        }
        let r: Respuesta = self.llamar("drafts.list", &[]).await?;
        Ok(r.drafts
            .into_iter()
            .filter(|d| !d.is_sent && !d.is_deleted)
            .filter_map(|d| {
                let canal = d.destinations.into_iter().next()?.channel_id;
                // Los bloques se leen igual que los de un mensaje.
                let mensaje = serde_json::from_value(
                    serde_json::json!({ "ts": d.last_updated_ts, "blocks": d.blocks }),
                )
                .ok()?;
                Some(BorradorSlack {
                    canal,
                    mensaje,
                    programado: d.date_scheduled,
                })
            })
            .collect())
    }

    /// Archivos del equipo, los más nuevos primero (`search.modules.files`).
    pub async fn archivos_equipo(self) -> Resultado<Vec<Archivo>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            items: Vec<Archivo>,
        }
        let params = [
            ("module", "files"),
            ("query", ""),
            ("page", "1"),
            ("count", "50"),
        ];
        let r: Respuesta = self.llamar("search.modules.files", &params).await?;
        Ok(r.items)
    }

    /// Personas del directorio que coinciden con `consulta` (`search.modules.people`).
    pub async fn personas(self, consulta: String) -> Resultado<Vec<Usuario>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            items: Vec<Usuario>,
        }
        let params = [
            ("module", "people"),
            ("query", consulta.as_str()),
            ("page", "1"),
            ("count", "50"),
        ];
        let r: Respuesta = self.llamar("search.modules.people", &params).await?;
        Ok(r.items)
    }

    /// Abre (o encuentra) la DM con una persona y devuelve su id.
    pub async fn abrir_dm(self, usuario: String) -> Resultado<String> {
        #[derive(Deserialize)]
        struct Respuesta {
            channel: Id,
        }
        #[derive(Deserialize)]
        struct Id {
            id: String,
        }
        let r: Respuesta = self
            .llamar("conversations.open", &[("users", &usuario)])
            .await?;
        Ok(r.channel.id)
    }

    /// Guarda un mensaje para después o lo quita (`saved.add` / `saved.delete`, sin
    /// documentar; probados contra Slack).
    pub async fn guardar(self, canal: String, ts: String, guardar: bool) -> Resultado<()> {
        let metodo = if guardar { "saved.add" } else { "saved.delete" };
        let params = [("item_type", "message"), ("item_id", &canal), ("ts", &ts)];
        self.llamar::<Value>(metodo, &params).await.map(|_| ())
    }

    /// Destaca un canal o lo quita de Destacados (`stars.add` / `stars.remove`).
    pub async fn destacar(self, canal: String, destacar: bool) -> Resultado<()> {
        let metodo = if destacar {
            "stars.add"
        } else {
            "stars.remove"
        };
        self.llamar::<Value>(metodo, &[("channel", &canal)])
            .await
            .map(|_| ())
    }

    /// La campana del canal (`users.prefs.setNotifications`, sin documentar). Como el
    /// cliente web: silenciar es `muted`; los otros dos niveles quitan el silencio y
    /// cambian `desktop`.
    pub async fn nivel_avisos(self, canal: String, nivel: NivelAviso) -> Resultado<()> {
        let silenciar = |valor: &'static str| {
            [
                ("name", "muted"),
                ("value", valor),
                ("channel_id", canal.as_str()),
                ("global", "false"),
            ]
        };
        let desktop = match nivel {
            NivelAviso::Silencio => {
                return self
                    .llamar::<Value>("users.prefs.setNotifications", &silenciar("true"))
                    .await
                    .map(|_| ());
            }
            NivelAviso::Todo => "everything",
            NivelAviso::Menciones => "mentions_dms",
        };
        self.llamar::<Value>("users.prefs.setNotifications", &silenciar("false"))
            .await?;
        let prefs =
            serde_json::json!([{ "name": "desktop", "value": desktop, "sync": true }]).to_string();
        let params = [
            ("channel_ids", canal.as_str()),
            ("prefs", prefs.as_str()),
            ("global", "false"),
            ("sync", "true"),
        ];
        self.llamar::<Value>("users.prefs.setNotifications", &params)
            .await
            .map(|_| ())
    }

    /// Fija o desfija un mensaje (`pins.add` / `pins.remove`).
    pub async fn fijar(self, canal: String, ts: String, fijar: bool) -> Resultado<()> {
        let metodo = if fijar { "pins.add" } else { "pins.remove" };
        self.llamar::<Value>(metodo, &[("channel", &canal), ("timestamp", &ts)])
            .await
            .map(|_| ())
    }

    /// Mensajes fijados del canal (`pins.list`).
    pub async fn fijados(self, canal: String) -> Resultado<Vec<Mensaje>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            items: Vec<Item>,
        }
        #[derive(Deserialize)]
        struct Item {
            #[serde(default)]
            message: Option<Mensaje>,
        }
        let r: Respuesta = self
            .llamar("pins.list", &[("channel", canal.as_str())])
            .await?;
        Ok(r.items.into_iter().filter_map(|i| i.message).collect())
    }

    /// Los archivos compartidos en el canal, el más nuevo primero (`files.list`).
    pub async fn archivos(self, canal: String) -> Resultado<Vec<Archivo>> {
        #[derive(Deserialize)]
        struct Respuesta {
            #[serde(default)]
            files: Vec<Archivo>,
        }
        let params = [("channel", canal.as_str()), ("count", "100")];
        let r: Respuesta = self.llamar("files.list", &params).await?;
        Ok(r.files)
    }

    /// Mensajes del canal posteriores a `ts`, del más viejo al más nuevo (hasta 200),
    /// para recuperar lo que llegó con el tiempo real caído.
    pub async fn posteriores(self, canal: String, ts: String) -> Resultado<Vec<Mensaje>> {
        #[derive(Deserialize)]
        struct Historial {
            messages: Vec<Mensaje>,
        }
        let h: Historial = self
            .llamar(
                "conversations.history",
                &[("channel", &canal), ("oldest", &ts), ("limit", "200")],
            )
            .await?;
        Ok(h.messages.into_iter().rev().collect())
    }

    /// El mensaje raíz del hilo seguido de sus respuestas.
    pub async fn respuestas(self, canal: String, ts: String) -> Resultado<Vec<Mensaje>> {
        #[derive(Deserialize)]
        struct Hilo {
            messages: Vec<Mensaje>,
        }
        let h: Hilo = self
            .llamar(
                "conversations.replies",
                &[("channel", &canal), ("ts", &ts), ("limit", "200")],
            )
            .await?;
        Ok(h.messages)
    }

    pub async fn marcar_leido(self, canal: String, ts: String) -> Resultado<()> {
        self.llamar::<Value>("conversations.mark", &[("channel", &canal), ("ts", &ts)])
            .await
            .map(|_| ())
    }

    /// Resuelve varios usuarios en paralelo; los que fallan se omiten.
    pub async fn usuarios(self, ids: Vec<String>) -> Vec<Usuario> {
        #[derive(Deserialize)]
        struct Info {
            user: Usuario,
        }
        stream::iter(ids)
            .map(|id| {
                let cliente = self.clone();
                async move { cliente.llamar::<Info>("users.info", &[("user", &id)]).await }
            })
            .buffer_unordered(8)
            .filter_map(|r| async move { r.ok().map(|i| i.user) })
            .collect()
            .await
    }

    /// Mensajes que coinciden con la consulta, con la sintaxis de Slack
    /// (`in:#canal`, `from:@alguien`…), los más relevantes primero.
    pub async fn buscar(self, consulta: String) -> Resultado<Vec<Coincidencia>> {
        #[derive(Deserialize)]
        struct Busqueda {
            messages: Coincidencias,
        }
        #[derive(Deserialize)]
        struct Coincidencias {
            #[serde(default)]
            matches: Vec<Coincidencia>,
        }
        let b: Busqueda = self
            .llamar(
                "search.messages",
                &[
                    ("query", &consulta),
                    ("count", "40"),
                    ("highlight", "false"),
                ],
            )
            .await?;
        Ok(b.messages.matches)
    }

    /// Publica un mensaje; en un hilo, `tambien` lo muestra además en el canal
    /// ("Also send to #canal").
    pub async fn enviar(
        self,
        canal: String,
        texto: String,
        hilo: Option<String>,
        tambien: bool,
    ) -> Resultado<()> {
        let mut params = vec![("channel", canal.as_str()), ("text", texto.as_str())];
        if let Some(ts) = &hilo {
            params.push(("thread_ts", ts));
            if tambien {
                params.push(("reply_broadcast", "true"));
            }
        }
        self.llamar::<Value>("chat.postMessage", &params)
            .await
            .map(|_| ())
    }

    /// Sube archivos a la conversación (`files.getUploadURLExternal`, el envío de los
    /// bytes y `files.completeUploadExternal`), con `comentario` como mensaje.
    pub async fn subir(
        self,
        canal: String,
        hilo: Option<String>,
        rutas: Vec<std::path::PathBuf>,
        comentario: String,
    ) -> Resultado<()> {
        #[derive(Deserialize)]
        struct Destino {
            upload_url: String,
            file_id: String,
        }
        let mut subidos = Vec::new();
        for ruta in &rutas {
            let nombre = ruta
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "archivo".into());
            let bytes = std::fs::read(ruta).map_err(|e| format!("{}: {e}", ruta.display()))?;
            let largo = bytes.len().to_string();
            let d: Destino = self
                .llamar(
                    "files.getUploadURLExternal",
                    &[("filename", &nombre), ("length", &largo)],
                )
                .await?;
            // La URL viene firmada: va sin la sesión, y solo a Slack.
            let de_slack = d.upload_url.starts_with("https://files.slack.com/")
                || d.upload_url.starts_with(self.base.trim_end_matches("api/"));
            if !de_slack {
                return Err(format!("{} no es de Slack", d.upload_url));
            }
            self.http
                .post(&d.upload_url)
                .body(bytes)
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(|e| format!("subir {nombre}: {e}"))?;
            subidos.push(serde_json::json!({ "id": d.file_id, "title": nombre }));
        }
        let archivos = serde_json::to_string(&subidos).expect("json válido");
        let mut params = vec![("files", archivos.as_str()), ("channel_id", canal.as_str())];
        if !comentario.is_empty() {
            params.push(("initial_comment", comentario.as_str()));
        }
        if let Some(ts) = &hilo {
            params.push(("thread_ts", ts));
        }
        self.llamar::<Value>("files.completeUploadExternal", &params)
            .await
            .map(|_| ())
    }

    /// Programa un mensaje para `post_at` (segundos Unix).
    pub async fn programar(
        self,
        canal: String,
        texto: String,
        post_at: i64,
        hilo: Option<String>,
    ) -> Resultado<()> {
        let post_at = post_at.to_string();
        let mut params = vec![
            ("channel", canal.as_str()),
            ("text", texto.as_str()),
            ("post_at", post_at.as_str()),
        ];
        if let Some(ts) = &hilo {
            params.push(("thread_ts", ts));
        }
        self.llamar::<Value>("chat.scheduleMessage", &params)
            .await
            .map(|_| ())
    }

    /// Cambia el texto de un mensaje propio.
    pub async fn editar(self, canal: String, ts: String, texto: String) -> Resultado<()> {
        self.llamar::<Value>(
            "chat.update",
            &[("channel", &canal), ("ts", &ts), ("text", &texto)],
        )
        .await
        .map(|_| ())
    }

    pub async fn borrar(self, canal: String, ts: String) -> Resultado<()> {
        self.llamar::<Value>("chat.delete", &[("channel", &canal), ("ts", &ts)])
            .await
            .map(|_| ())
    }

    pub async fn reaccionar(
        self,
        canal: String,
        ts: String,
        nombre: String,
        agregar: bool,
    ) -> Resultado<()> {
        let metodo = if agregar {
            "reactions.add"
        } else {
            "reactions.remove"
        };
        self.llamar::<Value>(
            metodo,
            &[("channel", &canal), ("timestamp", &ts), ("name", &nombre)],
        )
        .await
        .map(|_| ())
    }

    /// Descarga una imagen pública (avatares e íconos del CDN de Slack). No manda la
    /// cookie de sesión: esos hosts no la necesitan.
    pub async fn imagen(self, url: String) -> Resultado<Vec<u8>> {
        descargar(self.http.get(&url)).await
    }

    /// Descarga un archivo privado (miniaturas de imágenes) con la sesión. Solo a
    /// files.slack.com o a la raíz configurada: la sesión no sale a otros hosts.
    pub async fn archivo(self, url: String) -> Resultado<Vec<u8>> {
        let de_slack = url.starts_with("https://files.slack.com/")
            || url.starts_with(self.base.trim_end_matches("api/"));
        if !de_slack {
            return Err(format!("{url} no es de Slack"));
        }
        let peticion = self
            .http
            .get(&url)
            .header(reqwest::header::COOKIE, format!("d={}", self.cred.cookie))
            .bearer_auth(&self.cred.token);
        descargar(peticion).await
    }

    pub async fn url_tiempo_real(&self) -> Resultado<String> {
        #[derive(Deserialize)]
        struct Rtm {
            url: String,
        }
        self.llamar::<Rtm>("rtm.connect", &[]).await.map(|r| r.url)
    }
}

async fn descargar(peticion: reqwest::RequestBuilder) -> Resultado<Vec<u8>> {
    let bytes = peticion
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn cred() -> Credenciales {
        Credenciales {
            token: "xoxc-1".into(),
            cookie: "xoxd-x".into(),
            equipo: None,
        }
    }

    async fn servidor() -> (MockServer, Cliente) {
        let s = MockServer::start().await;
        let c = Cliente::con_base(cred(), format!("{}/", s.uri()));
        (s, c)
    }

    fn ok(cuerpo: Value) -> ResponseTemplate {
        let mut v = cuerpo;
        v["ok"] = json!(true);
        ResponseTemplate::new(200).set_body_json(v)
    }

    #[tokio::test]
    async fn hilos_en_los_que_participas() {
        let (s, c) = servidor().await;
        Mock::given(path("/subscriptions.thread.getView"))
            .respond_with(ok(json!({ "threads": [{
                "root_msg": { "ts": "1.0", "user": "U1", "text": "raíz", "channel": "C1",
                              "reply_count": 2 },
                "latest_replies": [{ "ts": "2.0", "user": "U2", "text": "resp", "thread_ts": "1.0" }]
            }]})))
            .mount(&s)
            .await;
        let h = c.hilos().await.unwrap();
        assert_eq!(
            (h[0].canal.as_str(), h[0].raiz.text.as_str()),
            ("C1", "raíz")
        );
        assert_eq!(h[0].respuestas[0].text, "resp");
    }

    #[tokio::test]
    async fn actividad_con_menciones_e_hilos() {
        let (s, c) = servidor().await;
        Mock::given(path("/activity.feed"))
            .and(body_string_contains("mode=chrono_v1"))
            .respond_with(ok(json!({ "items": [
                { "is_unread": true, "item": { "type": "at_user",
                  "message": { "ts": "5.0", "channel": "C1", "author_user_id": "U1" } } },
                { "is_unread": false, "item": { "type": "bundle" } }
            ]})))
            .mount(&s)
            .await;
        let a = c.actividad().await.unwrap();
        assert_eq!(a.len(), 1, "lo que no trae mensaje se omite");
        assert_eq!(
            (a[0].tipo.as_str(), a[0].canal.as_str(), a[0].ts.as_str()),
            ("at_user", "C1", "5.0")
        );
        assert_eq!(a[0].autor.as_deref(), Some("U1"));
        assert!(a[0].no_leido);
    }

    #[tokio::test]
    async fn mensajes_por_canal_y_ts() {
        let (s, c) = servidor().await;
        Mock::given(path("/messages.list"))
            .and(body_string_contains("timestamps"))
            .respond_with(ok(json!({ "messages": {
                "C1": [{ "ts": "5.0", "user": "U1", "text": "hola" }]
            }})))
            .mount(&s)
            .await;
        let m = c
            .mensajes_por_id(vec![("C1".into(), "5.0".into())])
            .await
            .unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!((m[0].0.as_str(), m[0].1.text.as_str()), ("C1", "hola"));
    }

    #[tokio::test]
    async fn guardados_solo_mensajes() {
        let (s, c) = servidor().await;
        Mock::given(path("/saved.list"))
            .respond_with(ok(json!({ "saved_items": [
                { "item_id": "C1", "item_type": "message", "ts": "5.0" },
                { "item_id": "F1", "item_type": "file" }
            ]})))
            .mount(&s)
            .await;
        assert_eq!(
            c.guardados().await.unwrap(),
            [("C1".to_string(), "5.0".to_string())]
        );
    }

    #[tokio::test]
    async fn borradores_sin_los_enviados_ni_borrados() {
        let (s, c) = servidor().await;
        let rico = json!([{ "type": "rich_text", "elements": [
            { "type": "rich_text_section", "elements": [{ "type": "text", "text": "a medias" }] }
        ]}]);
        Mock::given(path("/drafts.list"))
            .respond_with(ok(json!({ "drafts": [
                { "id": "Dr1", "last_updated_ts": "9.0", "blocks": rico, "date_scheduled": 0,
                  "destinations": [{ "channel_id": "C1" }] },
                { "id": "Dr2", "is_sent": true, "blocks": [], "destinations": [{ "channel_id": "C2" }] },
                { "id": "Dr3", "is_deleted": true, "blocks": [], "destinations": [{ "channel_id": "C3" }] },
                { "id": "Dr4", "last_updated_ts": "8.0", "blocks": rico, "date_scheduled": 1800000000,
                  "destinations": [{ "channel_id": "C4" }] }
            ]})))
            .mount(&s)
            .await;
        let b = c.borradores().await.unwrap();
        let canales: Vec<_> = b.iter().map(|b| b.canal.as_str()).collect();
        assert_eq!(canales, ["C1", "C4"]);
        assert!(b[0].mensaje.rico.is_some(), "el texto viene en bloques");
        assert_eq!((b[0].programado, b[1].programado), (0, 1_800_000_000));
    }

    #[tokio::test]
    async fn archivos_del_equipo_y_personas() {
        let (s, c) = servidor().await;
        Mock::given(path("/search.modules.files"))
            .and(body_string_contains("module=files"))
            .respond_with(ok(
                json!({ "items": [{ "id": "F1", "name": "plan.pdf", "created": 5 }] }),
            ))
            .mount(&s)
            .await;
        Mock::given(path("/search.modules.people"))
            .and(body_string_contains("query=ana"))
            .respond_with(ok(json!({ "items": [{ "id": "U1", "username": "ana",
                "profile": { "real_name": "Ana López", "title": "Diseño" } }] })))
            .mount(&s)
            .await;
        assert_eq!(c.clone().archivos_equipo().await.unwrap()[0].id, "F1");
        let p = c.personas("ana".into()).await.unwrap();
        assert_eq!(
            (p[0].name.as_str(), p[0].nombre_visible()),
            ("ana", "Ana López")
        );
        assert_eq!(p[0].profile.title.as_deref(), Some("Diseño"));
    }

    #[tokio::test]
    async fn abrir_un_dm() {
        let (s, c) = servidor().await;
        Mock::given(path("/conversations.open"))
            .and(body_string_contains("users=U1"))
            .respond_with(ok(json!({ "channel": { "id": "D9" } })))
            .mount(&s)
            .await;
        assert_eq!(c.abrir_dm("U1".into()).await.unwrap(), "D9");
    }

    #[tokio::test]
    async fn responder_en_hilo_y_tambien_en_el_canal() {
        let (s, c) = servidor().await;
        Mock::given(path("/chat.postMessage"))
            .and(body_string_contains("thread_ts=1.0"))
            .and(body_string_contains("reply_broadcast=true"))
            .respond_with(ok(json!({})))
            .expect(1)
            .mount(&s)
            .await;
        c.enviar("C1".into(), "hola".into(), Some("1.0".into()), true)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn programar_un_mensaje() {
        let (s, c) = servidor().await;
        Mock::given(path("/chat.scheduleMessage"))
            .and(body_string_contains("channel=C1"))
            .and(body_string_contains("post_at=1800000000"))
            .and(body_string_contains("thread_ts=1.0"))
            .respond_with(ok(json!({ "scheduled_message_id": "Q1" })))
            .expect(1)
            .mount(&s)
            .await;
        c.programar(
            "C1".into(),
            "hola".into(),
            1_800_000_000,
            Some("1.0".into()),
        )
        .await
        .unwrap();
    }

    fn archivo_temporal(nombre: &str, contenido: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("slack-rs-subir-{}-{}", std::process::id(), nombre));
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join(nombre);
        std::fs::write(&ruta, contenido).unwrap();
        ruta
    }

    #[tokio::test]
    async fn subir_archivos_en_tres_pasos() {
        let (s, c) = servidor().await;
        let ruta = archivo_temporal("plan.txt", "hola");
        Mock::given(path("/files.getUploadURLExternal"))
            .and(body_string_contains("filename=plan.txt"))
            .and(body_string_contains("length=4"))
            .respond_with(ok(json!({
                "upload_url": format!("{}/subida/1", s.uri()), "file_id": "F1"
            })))
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/subida/1"))
            .and(body_string_contains("hola"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/files.completeUploadExternal"))
            .and(body_string_contains("channel_id=C1"))
            .and(body_string_contains("initial_comment=mira"))
            .and(body_string_contains("thread_ts=1.0"))
            .and(body_string_contains("F1"))
            .respond_with(ok(json!({ "files": [] })))
            .expect(1)
            .mount(&s)
            .await;
        c.subir(
            "C1".into(),
            Some("1.0".into()),
            vec![ruta.clone()],
            "mira".into(),
        )
        .await
        .unwrap();
        std::fs::remove_dir_all(ruta.parent().unwrap()).unwrap();
    }

    #[tokio::test]
    async fn subir_no_manda_el_archivo_fuera_de_slack() {
        let (s, c) = servidor().await;
        let ruta = archivo_temporal("secreto.txt", "x");
        Mock::given(path("/files.getUploadURLExternal"))
            .respond_with(ok(
                json!({ "upload_url": "https://otro.example/x", "file_id": "F1" }),
            ))
            .mount(&s)
            .await;
        let r = c
            .subir("C1".into(), None, vec![ruta.clone()], String::new())
            .await;
        assert!(r.unwrap_err().contains("no es de Slack"));
        std::fs::remove_dir_all(ruta.parent().unwrap()).unwrap();
    }

    #[tokio::test]
    async fn guardar_y_quitar_de_guardados() {
        let (s, c) = servidor().await;
        for metodo in ["/saved.add", "/saved.delete"] {
            Mock::given(path(metodo))
                .and(body_string_contains("item_type=message"))
                .and(body_string_contains("item_id=C1"))
                .and(body_string_contains("ts=1.5"))
                .respond_with(ok(json!({})))
                .expect(1)
                .mount(&s)
                .await;
        }
        c.clone()
            .guardar("C1".into(), "1.5".into(), true)
            .await
            .unwrap();
        c.guardar("C1".into(), "1.5".into(), false).await.unwrap();
    }

    #[tokio::test]
    async fn destacar_y_quitar() {
        let (s, c) = servidor().await;
        for metodo in ["/stars.add", "/stars.remove"] {
            Mock::given(path(metodo))
                .and(body_string_contains("channel=C1"))
                .respond_with(ok(json!({})))
                .expect(1)
                .mount(&s)
                .await;
        }
        c.clone().destacar("C1".into(), true).await.unwrap();
        c.destacar("C1".into(), false).await.unwrap();
    }

    #[tokio::test]
    async fn nivel_de_avisos_del_canal() {
        let (s, c) = servidor().await;
        Mock::given(path("/users.prefs.setNotifications"))
            .and(body_string_contains("channel_ids=C1"))
            .and(body_string_contains("everything"))
            .respond_with(ok(json!({})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/users.prefs.setNotifications"))
            .and(body_string_contains("name=muted"))
            .and(body_string_contains("channel_id=C1"))
            .respond_with(ok(json!({})))
            .expect(2)
            .mount(&s)
            .await;
        c.clone()
            .nivel_avisos("C1".into(), NivelAviso::Todo)
            .await
            .unwrap();
        c.nivel_avisos("C1".into(), NivelAviso::Silencio)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn fijar_y_desfijar() {
        let (s, c) = servidor().await;
        for metodo in ["/pins.add", "/pins.remove"] {
            Mock::given(path(metodo))
                .and(body_string_contains("channel=C1"))
                .and(body_string_contains("timestamp=1.5"))
                .respond_with(ok(json!({})))
                .expect(1)
                .mount(&s)
                .await;
        }
        c.clone()
            .fijar("C1".into(), "1.5".into(), true)
            .await
            .unwrap();
        c.fijar("C1".into(), "1.5".into(), false).await.unwrap();
    }

    #[test]
    fn adjuntos_de_bots_y_fijados() {
        let m: Mensaje = serde_json::from_value(json!({
            "ts": "1.0", "text": "", "pinned_to": ["C1"],
            "attachments": [{
                "fallback": "EC2 stopped", "color": "#e01e5a", "pretext": "Alerta",
                "title": "EC2 stopped", "title_link": "https://x/1", "text": "Instancia *i-1*",
                "fields": [{ "title": "Región", "value": "us-west-2", "short": true }],
                "footer": "ec2-bot"
            }]
        }))
        .unwrap();
        assert_eq!(m.pinned_to, ["C1"]);
        let a = &m.attachments[0];
        assert!(a.tiene_contenido());
        assert_eq!(a.color.as_deref(), Some("#e01e5a"));
        assert_eq!(a.campos[0].titulo, "Región");
        assert!(a.campos[0].corto);
        let solo_fallback: Adjunto = serde_json::from_value(json!({ "fallback": "x" })).unwrap();
        assert!(!solo_fallback.tiene_contenido());
    }

    #[tokio::test]
    async fn info_del_canal() {
        let (s, c) = servidor().await;
        Mock::given(path("/conversations.info"))
            .and(body_string_contains("include_num_members=true"))
            .respond_with(ok(json!({ "channel": {
                "id": "C1", "num_members": 40, "created": 1_600_000_000, "creator": "U1",
                "topic": { "value": "Lo del día" }, "purpose": { "value": "Todo el equipo" }
            }})))
            .mount(&s)
            .await;
        let i = c.info_canal("C1".into()).await.unwrap();
        assert_eq!(i.miembros, Some(40));
        assert_eq!(
            (i.tema.as_str(), i.proposito.as_str()),
            ("Lo del día", "Todo el equipo")
        );
        assert_eq!(
            (i.creado, i.creador.as_deref()),
            (1_600_000_000, Some("U1"))
        );
    }

    #[tokio::test]
    async fn miembros_en_varias_paginas() {
        let (s, c) = servidor().await;
        Mock::given(path("/conversations.members"))
            .and(body_string_contains("cursor=sig"))
            .respond_with(ok(
                json!({ "members": ["U3"], "response_metadata": { "next_cursor": "" } }),
            ))
            .mount(&s)
            .await;
        Mock::given(path("/conversations.members"))
            .respond_with(ok(json!({ "members": ["U1", "U2"],
                                     "response_metadata": { "next_cursor": "sig" } })))
            .mount(&s)
            .await;
        assert_eq!(c.miembros("C1".into()).await.unwrap(), ["U1", "U2", "U3"]);
    }

    #[tokio::test]
    async fn marcadores_solo_enlaces() {
        let (s, c) = servidor().await;
        Mock::given(path("/bookmarks.list"))
            .and(body_string_contains("channel_id=C1"))
            .respond_with(ok(json!({ "bookmarks": [
                { "title": "Tablero", "link": "https://ejemplo.com/t", "emoji": ":bar_chart:", "type": "link" },
                { "title": "Carpeta", "type": "folder" }
            ]})))
            .mount(&s)
            .await;
        let m = c.marcadores("C1".into()).await.unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(
            (m[0].titulo.as_str(), m[0].enlace.as_str()),
            ("Tablero", "https://ejemplo.com/t")
        );
        assert_eq!(m[0].emoji, "bar_chart");
    }

    #[tokio::test]
    async fn fijados_son_mensajes() {
        let (s, c) = servidor().await;
        Mock::given(path("/pins.list"))
            .respond_with(ok(json!({ "items": [
                { "type": "message", "message": { "ts": "2.0", "user": "U1", "text": "fijado" } },
                { "type": "file", "file": { "id": "F1" } }
            ]})))
            .mount(&s)
            .await;
        let f = c.fijados("C1".into()).await.unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].text, "fijado");
    }

    #[tokio::test]
    async fn archivos_del_canal() {
        let (s, c) = servidor().await;
        Mock::given(path("/files.list"))
            .and(body_string_contains("channel=C1"))
            .respond_with(ok(json!({ "files": [
                { "id": "F1", "name": "plan.pdf", "user": "U1", "created": 1_700_000_000,
                  "pretty_type": "PDF", "permalink": "https://x/F1" }
            ]})))
            .mount(&s)
            .await;
        let a = c.archivos("C1".into()).await.unwrap();
        assert_eq!(a[0].name.as_deref(), Some("plan.pdf"));
        assert_eq!(
            (a[0].user.as_deref(), a[0].created),
            (Some("U1"), 1_700_000_000)
        );
    }

    #[tokio::test]
    async fn equipo_con_dominio() {
        let (s, c) = servidor().await;
        Mock::given(path("/team.info"))
            .respond_with(ok(json!({ "team": { "name": "Acme", "domain": "acme" } })))
            .mount(&s)
            .await;
        assert_eq!(c.equipo().await.unwrap().domain, "acme");
    }

    #[tokio::test]
    async fn llamar_manda_token_y_cookie() {
        let (s, c) = servidor().await;
        Mock::given(method("POST"))
            .and(path("/auth.test"))
            .and(header("cookie", "d=xoxd-x"))
            .and(body_string_contains("token=xoxc-1"))
            .respond_with(ok(json!({ "user_id": "U0", "team": "Acme" })))
            .expect(1)
            .mount(&s)
            .await;
        let sesion = c.sesion().await.unwrap();
        assert_eq!(
            (sesion.user_id.as_str(), sesion.team.as_str()),
            ("U0", "Acme")
        );
    }

    #[tokio::test]
    async fn error_de_slack_lleva_el_metodo() {
        let (s, c) = servidor().await;
        Mock::given(path("/auth.test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "ok": false, "error": "invalid_auth" })),
            )
            .mount(&s)
            .await;
        assert_eq!(c.sesion().await.unwrap_err(), "auth.test: invalid_auth");
    }

    #[tokio::test]
    async fn canales_sigue_el_cursor() {
        let (s, c) = servidor().await;
        Mock::given(path("/users.conversations"))
            .and(body_string_contains("cursor=pag2"))
            .respond_with(ok(json!({ "channels": [{ "id": "C2" }] })))
            .with_priority(1)
            .mount(&s)
            .await;
        Mock::given(path("/users.conversations"))
            .respond_with(ok(json!({
                "channels": [{ "id": "C1" }],
                "response_metadata": { "next_cursor": "pag2" }
            })))
            .mount(&s)
            .await;
        let ids: Vec<_> = c
            .canales()
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, ["C1", "C2"]);
    }

    #[tokio::test]
    async fn historial_va_del_mas_viejo_al_mas_nuevo() {
        let (s, c) = servidor().await;
        Mock::given(path("/conversations.history"))
            .and(body_string_contains("channel=C1"))
            .respond_with(ok(
                json!({ "messages": [{ "ts": "3" }, { "ts": "2" }, { "ts": "1" }] }),
            ))
            .mount(&s)
            .await;
        let p = c.historial("C1".into(), None).await.unwrap();
        assert!(!p.hay_mas);
        let ts: Vec<_> = p.mensajes.into_iter().map(|m| m.ts).collect();
        assert_eq!(ts, ["1", "2", "3"]);
    }

    #[tokio::test]
    async fn historial_anterior_con_cursor() {
        let (s, c) = servidor().await;
        Mock::given(path("/conversations.history"))
            .and(body_string_contains("latest=5.0"))
            .respond_with(ok(
                json!({ "messages": [{ "ts": "4.0" }, { "ts": "3.0" }], "has_more": true }),
            ))
            .expect(1)
            .mount(&s)
            .await;
        let p = c.historial("C1".into(), Some("5.0".into())).await.unwrap();
        let ts: Vec<_> = p.mensajes.into_iter().map(|m| m.ts).collect();
        assert_eq!(
            (ts, p.hay_mas),
            (vec!["3.0".to_string(), "4.0".to_string()], true)
        );
    }

    #[tokio::test]
    async fn posteriores_a_un_ts() {
        let (s, c) = servidor().await;
        Mock::given(path("/conversations.history"))
            .and(body_string_contains("oldest=5.0"))
            .respond_with(ok(
                json!({ "messages": [{ "ts": "7.0" }, { "ts": "6.0" }] }),
            ))
            .expect(1)
            .mount(&s)
            .await;
        let ts: Vec<_> = c
            .posteriores("C1".into(), "5.0".into())
            .await
            .unwrap()
            .into_iter()
            .map(|m| m.ts)
            .collect();
        assert_eq!(ts, ["6.0", "7.0"]);
    }

    #[tokio::test]
    async fn reaccionar_elige_agregar_o_quitar() {
        let (s, c) = servidor().await;
        for metodo in ["/reactions.add", "/reactions.remove"] {
            Mock::given(path(metodo))
                .and(body_string_contains("name=eyes"))
                .and(body_string_contains("timestamp=1.2"))
                .respond_with(ok(json!({})))
                .expect(1)
                .mount(&s)
                .await;
        }
        let r = |agregar| {
            c.clone()
                .reaccionar("C1".into(), "1.2".into(), "eyes".into(), agregar)
        };
        r(true).await.unwrap();
        r(false).await.unwrap();
    }

    #[tokio::test]
    async fn imagen_no_manda_la_cookie() {
        let (s, c) = servidor().await;
        Mock::given(method("GET"))
            .and(path("/avatar.png"))
            .and(|r: &Request| !r.headers.contains_key("cookie"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"png".to_vec()))
            .expect(1)
            .mount(&s)
            .await;
        let bytes = c.imagen(format!("{}/avatar.png", s.uri())).await.unwrap();
        assert_eq!(bytes, b"png");
    }

    #[tokio::test]
    async fn buscar_mensajes() {
        let (s, c) = servidor().await;
        Mock::given(path("/search.messages"))
            .and(body_string_contains("query=deploy+in%3A%23general"))
            .respond_with(ok(json!({ "messages": {
                "total": 1,
                "matches": [{
                    "ts": "1.2", "user": "U1", "text": "el *deploy* salió",
                    "channel": { "id": "C1", "name": "general" },
                    "permalink": "https://acme.slack.com/archives/C1/p12"
                }]
            }})))
            .expect(1)
            .mount(&s)
            .await;
        let r = c.buscar("deploy in:#general".into()).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].mensaje.text, "el *deploy* salió");
        assert_eq!(r[0].channel.id, "C1");
    }

    #[tokio::test]
    async fn contadores_con_ultimo_mensaje() {
        let (s, c) = servidor().await;
        Mock::given(path("/client.counts"))
            .respond_with(ok(json!({
                "channels": [{ "id": "C1", "has_unreads": true, "mention_count": 2,
                               "latest": "1718000000.000100" }],
                "ims": [{ "id": "D1", "has_unreads": false, "mention_count": 0 }],
                "mpims": []
            })))
            .mount(&s)
            .await;
        let c = c.contadores().await.unwrap();
        assert_eq!(
            (c["C1"].no_leidos, c["C1"].menciones, c["C1"].ultimo),
            (true, 2, 1718000000)
        );
        assert_eq!(c["D1"].ultimo, 0, "sin `latest` no se sabe");
    }

    #[tokio::test]
    async fn secciones_en_orden_de_la_lista_enlazada() {
        let (s, c) = servidor().await;
        let cuerpo: Value =
            serde_json::from_str(include_str!("../tests/datos/channel_sections.json")).unwrap();
        Mock::given(path("/users.channelSections.list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(cuerpo))
            .mount(&s)
            .await;
        let secciones = c.secciones().await.unwrap();
        let resumen: Vec<_> = secciones
            .iter()
            .map(|s| (s.id.as_str(), s.tipo, s.nombre.as_str(), s.canales.len()))
            .collect();
        assert_eq!(
            resumen,
            [
                ("L1", TipoSeccion::Destacados, "", 2),
                ("L2", TipoSeccion::Propia, "Proyectos", 2),
                ("L3", TipoSeccion::Canales, "", 0),
                ("L4", TipoSeccion::Directos, "", 0),
                ("L5", TipoSeccion::Otra, "", 0),
            ]
        );
        assert_eq!(secciones[1].emoji, "rocket");
    }

    #[tokio::test]
    async fn silenciados_de_las_dos_preferencias_y_tema() {
        let (s, c) = servidor().await;
        let notificaciones = json!({ "channels": {
            "C5": { "muted": true }, "C6": { "muted": false }, "C7": {},
            "C8": { "desktop": "everything" }
        }})
        .to_string();
        let tema = json!({ "primary": { "palette": "horchata" },
                           "highlight1": { "palette": "jade" }, "brightness": 5 })
        .to_string();
        Mock::given(path("/users.prefs.get"))
            .respond_with(ok(json!({ "prefs": {
                "muted_channels": "C1,C2,",
                "all_notifications_prefs": notificaciones,
                "ia_theme": tema,
                "time24": true
            }})))
            .mount(&s)
            .await;
        let prefs = c.prefs().await.unwrap();
        let mut ids: Vec<_> = prefs.silenciados.into_iter().collect();
        ids.sort();
        assert_eq!(ids, ["C1", "C2", "C5"]);
        let tema = prefs.tema.unwrap();
        assert_eq!(
            (tema.primaria.as_str(), tema.resalte.as_str()),
            ("horchata", "jade")
        );
        assert!(prefs.reloj_24);
        assert_eq!(prefs.todos, HashSet::from(["C8".to_string()]));
    }

    #[tokio::test]
    async fn preferencias_vacias() {
        let (s, c) = servidor().await;
        Mock::given(path("/users.prefs.get"))
            .respond_with(ok(
                json!({ "prefs": { "all_notifications_prefs": "no es json", "ia_theme": "" } }),
            ))
            .mount(&s)
            .await;
        let prefs = c.prefs().await.unwrap();
        assert!(prefs.silenciados.is_empty());
        assert_eq!(prefs.tema, None);
        assert!(!prefs.reloj_24, "Slack usa 12 horas si no se eligió");
    }

    #[tokio::test]
    async fn emoji_propios_con_alias() {
        let (s, c) = servidor().await;
        Mock::given(path("/emoji.list"))
            .respond_with(ok(json!({ "emoji": {
                "loro": "https://emoji.slack-edge.com/T1/loro/abc.gif",
                "lorito": "alias:loro",
                "lorote": "alias:lorito",
                "pulgar": "alias:thumbsup",
                "roto": "alias:no-existe"
            }})))
            .mount(&s)
            .await;
        let e = c.emoji().await.unwrap();
        let url = "https://emoji.slack-edge.com/T1/loro/abc.gif";
        assert_eq!(e["loro"], url);
        assert_eq!(e["lorito"], url);
        assert_eq!(e["lorote"], url, "los alias se siguen en cadena");
        assert!(
            !e.contains_key("pulgar"),
            "alias a uno estándar: se descarta"
        );
        assert!(!e.contains_key("roto"));
    }

    #[tokio::test]
    async fn archivo_privado_con_la_sesion() {
        let (s, c) = servidor().await;
        Mock::given(method("GET"))
            .and(path("/files-tmb/T1-F1/foto_360.png"))
            .and(header("cookie", "d=xoxd-x"))
            .and(header("authorization", "Bearer xoxc-1"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"img".to_vec()))
            .expect(1)
            .mount(&s)
            .await;
        let url = format!("{}/files-tmb/T1-F1/foto_360.png", s.uri());
        assert_eq!(c.archivo(url).await.unwrap(), b"img");
    }

    #[tokio::test]
    async fn archivo_no_manda_la_sesion_a_otro_lado() {
        let (_s, c) = servidor().await;
        let r = c.archivo("https://evil.example/robar.png".into()).await;
        assert!(r.unwrap_err().contains("no es de Slack"));
    }

    #[test]
    fn archivo_de_imagen() {
        let f: Archivo = serde_json::from_value(json!({
            "id": "F1", "name": "foto.png", "mimetype": "image/png",
            "thumb_360": "https://files.slack.com/t/foto_360.png",
            "thumb_360_w": 360, "thumb_360_h": 240,
            "permalink": "https://acme.slack.com/files/U1/F1/foto.png"
        }))
        .unwrap();
        assert_eq!(
            f.miniatura(),
            Some(("https://files.slack.com/t/foto_360.png", 360, 240))
        );
        let pdf: Archivo =
            serde_json::from_value(json!({ "id": "F2", "mimetype": "application/pdf" })).unwrap();
        assert_eq!(pdf.miniatura(), None);
    }

    #[tokio::test]
    async fn editar_y_borrar() {
        let (s, c) = servidor().await;
        Mock::given(path("/chat.update"))
            .and(body_string_contains("channel=C1"))
            .and(body_string_contains("ts=1.2"))
            .and(body_string_contains("text=nuevo+texto"))
            .respond_with(ok(json!({})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/chat.delete"))
            .and(body_string_contains("channel=C1"))
            .and(body_string_contains("ts=1.2"))
            .respond_with(ok(json!({})))
            .expect(1)
            .mount(&s)
            .await;
        c.clone()
            .editar("C1".into(), "1.2".into(), "nuevo texto".into())
            .await
            .unwrap();
        c.borrar("C1".into(), "1.2".into()).await.unwrap();
    }

    fn mensaje(v: Value) -> Mensaje {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn mensaje_con_rich_text() {
        let m: Mensaje =
            serde_json::from_str(include_str!("../tests/datos/rich_text.json")).unwrap();
        assert_eq!(m.rico.as_ref().map(Vec::len), Some(5));
    }

    #[test]
    fn mensaje_sin_rich_text_usa_el_texto() {
        let seccion = json!({ "ts": "1", "text": "bot", "blocks": [
            { "type": "section", "text": { "type": "mrkdwn", "text": "bot" } }
        ]});
        assert!(mensaje(seccion).rico.is_none());
        let roto =
            json!({ "ts": "1", "text": "x", "blocks": [{ "type": "rich_text", "elements": 3 }] });
        let roto = mensaje(roto);
        assert!(roto.rico.is_none());
        assert_eq!(roto.text, "x");
        assert!(mensaje(json!({ "ts": "1", "blocks": [] })).rico.is_none());
    }

    fn entorno<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn credenciales_de_varios_tokens() {
        let todas = Credenciales::desde(entorno(&[
            ("SLACK_TOKEN", " xoxc-a, xoxc-b xoxc-a "),
            ("SLACK_COOKIE", "d=xoxd-z"),
        ]))
        .unwrap();
        let tokens: Vec<_> = todas.iter().map(|c| c.token.as_str()).collect();
        assert_eq!(tokens, ["xoxc-a", "xoxc-b"]);
        assert!(todas.iter().all(|c| c.cookie == "xoxd-z"));
    }

    #[test]
    fn credenciales_rechaza_lo_invalido() {
        let falta = Credenciales::desde(entorno(&[("SLACK_TOKEN", "xoxc-a")]));
        assert_eq!(
            falta.unwrap_err(),
            "falta la variable de entorno SLACK_COOKIE"
        );
        let vacia = Credenciales::desde(entorno(&[("SLACK_TOKEN", "  "), ("SLACK_COOKIE", "x")]));
        assert_eq!(
            vacia.unwrap_err(),
            "falta la variable de entorno SLACK_TOKEN"
        );
        let bot = Credenciales::desde(entorno(&[("SLACK_TOKEN", "xoxb-1"), ("SLACK_COOKIE", "x")]));
        assert!(bot.unwrap_err().contains("xoxc-"));
    }
}
