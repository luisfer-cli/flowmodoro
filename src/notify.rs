use zbus::blocking::Connection;

pub fn send(summary: &str, body: &str, urgency: u8) {
    let conn = match Connection::session() {
        Ok(c) => c,
        Err(_) => return,
    };

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
