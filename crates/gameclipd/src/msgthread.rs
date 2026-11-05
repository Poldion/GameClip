// SPDX-License-Identifier: GPL-3.0-or-later
//! Nachrichten-Thread (Design, Datei 42): Message-only-Fenster mit Win32-Schleife für den
//! globalen Hotkey, die Sperr-Meldungen und das Tray.
//!
//! `WM_HOTKEY` kommt mit QPC-Zeitstempel an die Hauptschleife; `WM_WTSSESSION_CHANGE`
//! (`WTS_SESSION_LOCK`/`UNLOCK`) wird zu `SessionLocked`/`SessionUnlocked` (D27). Befehle
//! kommen über einen Kanal; `PostThreadMessageW(WM_APP)` weckt die Schleife.

use std::cell::RefCell;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use gameclip_capture_win::pacer::qpc_us;
use gameclip_core::keymap::to_vk;
use gameclip_core::settings::Hotkey;
use gameclip_protocol::messages::TestRegisterReply;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE,
    MSG, PostThreadMessageW, RegisterClassW, TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_HOTKEY,
    WM_WTSSESSION_CHANGE, WNDCLASSW, WS_OVERLAPPED,
};
use windows_core::w;

use crate::tray::{Tray, TrayCommand, TrayView};

const HOTKEY_ID: i32 = 1;
/// ID für `TestRegister`; nie dauerhaft registriert.
const TEST_ID: i32 = 2;
const WTS_SESSION_LOCK: usize = 0x7;
const WTS_SESSION_UNLOCK: usize = 0x8;

#[derive(Debug, Clone, PartialEq)]
pub enum MsgEvent {
    /// Tastendruck mit QPC-Zeit in µs.
    Hotkey {
        qpc_us: i64,
    },
    SessionLocked,
    SessionUnlocked,
    Tray(TrayCommand),
}

enum Command {
    Register(Hotkey, Sender<Result<(), String>>),
    TestRegister(Hotkey, Sender<TestRegisterReply>),
    Unregister,
    UpdateTray(TrayView),
    Quit,
}

pub struct MsgThread {
    thread_id: u32,
    commands: Sender<Command>,
    join: Option<JoinHandle<()>>,
}

thread_local! {
    static EVENTS: RefCell<Option<Sender<MsgEvent>>> = const { RefCell::new(None) };
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_WTSSESSION_CHANGE {
        let ev = match wparam.0 {
            WTS_SESSION_LOCK => Some(MsgEvent::SessionLocked),
            WTS_SESSION_UNLOCK => Some(MsgEvent::SessionUnlocked),
            _ => None,
        };
        if let Some(ev) = ev {
            EVENTS.with(|e| {
                if let Some(tx) = e.borrow().as_ref() {
                    let _ = tx.send(ev);
                }
            });
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

impl MsgThread {
    pub fn start(events: Sender<MsgEvent>, tray: TrayView) -> std::io::Result<Self> {
        let (cmd_tx, cmd_rx) = unbounded();
        let (id_tx, id_rx) = bounded(1);
        let join = std::thread::Builder::new()
            .name("gameclip-msg".into())
            .spawn(move || run(events, cmd_rx, id_tx, tray))?;
        let thread_id = id_rx
            .recv()
            .map_err(|_| std::io::Error::other("Nachrichten-Thread startet nicht"))?;
        Ok(MsgThread {
            thread_id,
            commands: cmd_tx,
            join: Some(join),
        })
    }

    fn send(&self, c: Command) {
        if self.commands.send(c).is_ok() {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_APP, WPARAM(0), LPARAM(0));
            }
        }
    }

    /// Registriert den Hotkey (ersetzt einen vorigen); `Err` mit Grund, z. B. belegt.
    pub fn register(&self, hotkey: &Hotkey) -> Result<(), String> {
        let (tx, rx) = bounded(1);
        self.send(Command::Register(hotkey.clone(), tx));
        rx.recv()
            .unwrap_or_else(|_| Err("Nachrichten-Thread beendet".into()))
    }

    /// Prüft, ob die Taste frei ist (registriert kurz unter eigener ID und gibt sie wieder frei).
    pub fn test_register(&self, hotkey: &Hotkey) -> TestRegisterReply {
        let (tx, rx) = bounded(1);
        self.send(Command::TestRegister(hotkey.clone(), tx));
        rx.recv().unwrap_or(TestRegisterReply {
            free: false,
            error: None,
        })
    }

    /// Hotkey vorübergehend freigeben (Hotkey-Auswahl in der Oberfläche).
    pub fn unregister(&self) {
        self.send(Command::Unregister);
    }

    pub fn update_tray(&self, view: TrayView) {
        self.send(Command::UpdateTray(view));
    }
}

impl Drop for MsgThread {
    fn drop(&mut self) {
        self.send(Command::Quit);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn run(events: Sender<MsgEvent>, commands: Receiver<Command>, id: Sender<u32>, view: TrayView) {
    EVENTS.with(|e| *e.borrow_mut() = Some(events.clone()));
    let hwnd = unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("GameClipMsgWindow"),
            ..Default::default()
        };
        RegisterClassW(&class);
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("GameClipMsgWindow"),
            w!("GameClip"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )
    };
    let Ok(hwnd) = hwnd else {
        return;
    };
    unsafe {
        let _ = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
    }
    // Das Tray gehört zu diesem Thread (tray-icon braucht die Nachrichtenschleife).
    let mut tray = Tray::new(view).ok();
    let _ = id.send(unsafe { GetCurrentThreadId() });

    let mut msg = MSG::default();
    let mut registered = false;
    'outer: while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        if msg.message == WM_HOTKEY && msg.wParam.0 as i32 == HOTKEY_ID {
            let _ = events.send(MsgEvent::Hotkey { qpc_us: qpc_us() });
        } else if msg.message == WM_APP && msg.hwnd.0.is_null() {
            for c in commands.try_iter() {
                match c {
                    Command::Register(hk, reply) => {
                        if registered {
                            unsafe {
                                let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
                            }
                        }
                        let result = to_vk(&hk).map_err(|e| format!("{e:?}")).and_then(
                            |(mods, vk)| unsafe {
                                RegisterHotKey(Some(hwnd), HOTKEY_ID, HOT_KEY_MODIFIERS(mods), vk)
                                    .map_err(|e| e.message())
                            },
                        );
                        registered = result.is_ok();
                        let _ = reply.send(result);
                    }
                    Command::TestRegister(hk, reply) => {
                        let result = match to_vk(&hk) {
                            Err(_) => TestRegisterReply {
                                free: false,
                                error: None,
                            },
                            Ok((mods, vk)) => unsafe {
                                match RegisterHotKey(
                                    Some(hwnd),
                                    TEST_ID,
                                    HOT_KEY_MODIFIERS(mods),
                                    vk,
                                ) {
                                    Ok(()) => {
                                        let _ = UnregisterHotKey(Some(hwnd), TEST_ID);
                                        TestRegisterReply {
                                            free: true,
                                            error: None,
                                        }
                                    }
                                    Err(e) => TestRegisterReply {
                                        free: false,
                                        error: Some(e.code().0 as u32 & 0xFFFF),
                                    },
                                }
                            },
                        };
                        let _ = reply.send(result);
                    }
                    Command::Unregister => unsafe {
                        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
                        registered = false;
                    },
                    Command::UpdateTray(v) => {
                        if let Some(t) = tray.as_mut() {
                            t.update(&v);
                        }
                    }
                    Command::Quit => break 'outer,
                }
            }
        } else {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if let Some(t) = tray.as_ref() {
            for cmd in t.poll() {
                let _ = events.send(MsgEvent::Tray(cmd));
            }
        }
    }
    drop(tray);
    unsafe {
        if registered {
            let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
        }
        let _ = WTSUnRegisterSessionNotification(hwnd);
        let _ = DestroyWindow(hwnd);
    }
}
