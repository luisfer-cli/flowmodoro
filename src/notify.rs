use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;
use zbus::blocking::Connection;

type Notification = (String, String, u8);

fn sender() -> &'static Sender<Notification> {
    static SENDER: OnceLock<Sender<Notification>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Notification>();
        std::thread::spawn(move || {
            let conn = Connection::session().ok();
            while let Ok((summary, body, urgency)) = rx.recv() {
                if let Some(conn) = &conn {
                    let _ = conn.call_method(
                        Some("org.freedesktop.Notifications"),
                        "/org/freedesktop/Notifications",
                        Some("org.freedesktop.Notifications"),
                        "Notify",
                        &(
                            "flow",
                            0u32,
                            "",
                            summary,
                            body,
                            Vec::<String>::new(),
                            std::collections::HashMap::<&str, zbus::zvariant::Value>::from([(
                                "urgency",
                                zbus::zvariant::Value::U8(urgency),
                            )]),
                            -1i32,
                        ),
                    );
                }
            }
        });
        tx
    })
}

pub fn send(summary: &str, body: &str, urgency: u8) {
    let _ = sender().send((summary.to_string(), body.to_string(), urgency));
}
