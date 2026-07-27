#[cfg(target_os = "linux")]
pub fn get_clipboard_text() -> Result<String, String> {
    use x11_clipboard::Clipboard;

    let clipboard =
        Clipboard::new().map_err(|e| format!("Impossible d'accéder au presse-papier: {}", e))?;

    let atoms = clipboard.getter.atoms.clone();
    let timeout = std::time::Duration::from_secs(1);

    clipboard
        .load(
            clipboard.setter.atoms.primary,
            atoms.utf8_string,
            atoms.property,
            timeout,
        )
        .map_err(|e| format!("Erreur lors de la lecture du presse-papier: {}", e))
        .and_then(|data| {
            String::from_utf8(data).map_err(|e| format!("Erreur de décodage UTF-8: {}", e))
        })
}

#[cfg(target_os = "macos")]
pub fn get_clipboard_text() -> Result<String, String> {
    use arboard::Clipboard;
    use std::thread;
    use std::time::Duration;

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Impossible d'accéder au presse-papier: {}", e))?;

    let saved_text = clipboard.get_text().ok();

    simulate_cmd_c()?;

    for _ in 0..50 {
        thread::sleep(Duration::from_millis(10));

        if let Ok(text) = clipboard.get_text() {
            if saved_text.as_ref() != Some(&text) && !text.is_empty() {
                if let Some(ref saved) = saved_text {
                    let _ = clipboard.set_text(saved);
                }
                return Ok(text);
            }
        }
    }

    Err("Impossible de récupérer le texte sélectionné".to_string())
}

#[cfg(target_os = "macos")]
pub fn simulate_cmd_c() -> Result<(), String> {
    use core_graphics::event::CGEventFlags;
    use core_graphics::event::CGEventTapLocation;
    use core_graphics::event::{CGEvent, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use std::thread;
    use std::time::Duration;

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| "Impossible de créer la source d'événement".to_string())?;

    let keycode: CGKeyCode = 8;

    let down = CGEvent::new_keyboard_event(source.clone(), keycode, true)
        .map_err(|_| "Erreur création événement clavier down".to_string())?;
    down.set_flags(CGEventFlags::CGEventFlagCommand);
    down.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(5));

    let up = CGEvent::new_keyboard_event(source, keycode, false)
        .map_err(|_| "Erreur création événement clavier up".to_string())?;
    up.set_flags(CGEventFlags::CGEventFlagCommand);
    up.post(CGEventTapLocation::HID);

    Ok(())
}
