// SPDX-License-Identifier: GPL-3.0-or-later
//! Verbindung zum Dienst (Design, Datei 60). Ein Thread hält die Verbindung: Ist keine Pipe da,
//! startet er den Dienst und versucht es 10 s lang alle 500 ms; danach meldet er
//! `service_unreachable`. Bricht die Verbindung ab, verbindet er neu. Dienst-Ereignisse gehen als
//! Tauri-Event `service-event` ans Frontend, der Verbindungszustand als `connection`.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gameclip_protocol::messages::{Command, Response};
use gameclip_ui_platform::pipe_client::{Client, ConnectError, ServerCheck};
use gameclip_ui_platform::{package, service};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

const START_WAIT: Duration = Duration::from_secs(10);
const RETRY: Duration = Duration::from_millis(500);
/// Längste Antwortzeit des Dienstes plus Puffer (Pipe-Server: 60 s).
pub const CALL_TIMEOUT: Duration = Duration::from_secs(65);

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Connection {
    Connecting,
    Connected { service_version: String },
    ServiceUnreachable,
    VersionMismatch { message: String },
}

#[derive(Clone, Default)]
pub struct Ipc {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    client: Option<Arc<Client>>,
    connection: Connection,
}

impl Default for Inner {
    fn default() -> Self {
        Inner {
            client: None,
            connection: Connection::Connecting,
        }
    }
}

/// Im Paket wird der Server gegen `InstalledLocation\gameclipd.exe` geprüft; in der Entwicklung
/// (ohne Paket) läuft der Dienst von woanders, dann ohne Prüfung.
fn server_check() -> ServerCheck {
    match (package::family_name(), package::service_exe()) {
        (Some(_), Some(exe)) => ServerCheck::Exe(exe),
        _ => ServerCheck::None,
    }
}

impl Ipc {
    pub fn connection(&self) -> Connection {
        self.inner.lock().expect("Sperre").connection.clone()
    }

    pub fn client(&self) -> Option<Arc<Client>> {
        self.inner.lock().expect("Sperre").client.clone()
    }

    fn set(&self, app: &AppHandle, client: Option<Arc<Client>>, c: Connection) {
        {
            let mut inner = self.inner.lock().expect("Sperre");
            inner.client = client;
            inner.connection = c.clone();
        }
        let _ = app.emit("connection", c);
    }

    /// Schickt einen Befehl; blockiert bis zur Antwort.
    pub fn call(&self, cmd: Command) -> Result<Response, String> {
        let client = self.client().ok_or("Nicht mit dem Dienst verbunden")?;
        client.call(cmd, CALL_TIMEOUT).map_err(|e| e.to_string())
    }

    /// Startet den Verbindungs-Thread.
    pub fn start(&self, app: AppHandle) {
        let me = self.clone();
        std::thread::Builder::new()
            .name("gameclip-connect".into())
            .spawn(move || me.run(app))
            .expect("Thread startet");
    }

    fn run(self, app: AppHandle) {
        let check = server_check();
        loop {
            self.set(&app, None, Connection::Connecting);
            let deadline = Instant::now() + START_WAIT;
            let mut started = false;
            let (client, events) = loop {
                match Client::connect("gameclip-ui", &check) {
                    Ok(ok) => break ok,
                    Err(ConnectError::VersionMismatch(message)) => {
                        warn!("Version passt nicht: {message}");
                        self.set(&app, None, Connection::VersionMismatch { message });
                        return;
                    }
                    Err(e) => {
                        if !started {
                            started = true;
                            info!("Dienst nicht erreichbar ({e}), starte ihn");
                            if let Err(e) = service::start() {
                                warn!("Dienst startet nicht: {e}");
                            }
                        }
                        if Instant::now() >= deadline {
                            warn!("Dienst nach {START_WAIT:?} nicht erreichbar: {e}");
                            self.set(&app, None, Connection::ServiceUnreachable);
                            // Weiter versuchen, falls er später doch kommt (z. B. nach Neustart aus dem Tray).
                            std::thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                        std::thread::sleep(RETRY);
                    }
                }
            };
            let version = client.hello.service_version.clone();
            info!("Mit Dienst {version} verbunden");
            self.set(
                &app,
                Some(Arc::new(client)),
                Connection::Connected {
                    service_version: version,
                },
            );
            // Bis die Pipe schließt: Ereignisse weitergeben.
            for ev in events.iter() {
                let _ = app.emit("service-event", ev);
            }
            warn!("Verbindung zum Dienst getrennt, verbinde neu");
            std::thread::sleep(RETRY);
        }
    }
}
