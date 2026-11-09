//! Vérification et demande des autorisations macOS (TCC).
//!
//! L'application a besoin de l'autorisation « Accessibilité » pour simuler
//! `Cmd+C` et récupérer le texte sélectionné. Cette permission est renommée
//! « Contrôle des appareils et accès aux données » à partir de macOS 27,
//! mais l'ancre de réglages (`Privacy_Accessibility`) reste identique.

use log::info;

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};

    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    }

    /// La boîte de dialogue système ne doit apparaître qu'une seule fois par
    /// lancement, pour ne pas harceler l'utilisateur à chaque appui.
    static PROMPT_SHOWN: AtomicBool = AtomicBool::new(false);

    /// Interroge l'état TCC courant.
    ///
    /// On utilise `AXIsProcessTrustedWithOptions` plutôt que
    /// `AXIsProcessTrusted` : le premier interroge TCC en direct, alors que le
    /// second peut renvoyer une valeur mise en cache au démarrage du process.
    fn is_trusted(prompt: bool) -> bool {
        let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::from(prompt);
        let options = CFDictionary::from_CFType_pairs(&[(key, value)]);

        unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const c_void) }
    }

    pub fn is_accessibility_granted() -> bool {
        is_trusted(false)
    }

    pub fn request_accessibility_access() -> bool {
        if PROMPT_SHOWN.swap(true, Ordering::SeqCst) {
            return is_trusted(false);
        }

        is_trusted(true)
    }

    pub fn open_accessibility_settings() -> Result<(), String> {
        // macOS 13+ (y compris macOS 27 « Golden Gate », où la section est
        // renommée « Contrôle des appareils et accès aux données »).
        const MODERN_URL: &str = "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility";
        // Anciennes versions de macOS.
        const LEGACY_URL: &str =
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

        let mut last_error = String::new();

        for url in [MODERN_URL, LEGACY_URL] {
            match std::process::Command::new("open").arg(url).status() {
                Ok(status) if status.success() => return Ok(()),
                Ok(status) => {
                    last_error = format!("La commande 'open' a échoué (code {:?})", status.code())
                }
                Err(e) => last_error = format!("Impossible d'ouvrir les Réglages Système: {}", e),
            }
        }

        Err(last_error)
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    pub fn is_accessibility_granted() -> bool {
        true
    }

    pub fn request_accessibility_access() -> bool {
        true
    }

    pub fn open_accessibility_settings() -> Result<(), String> {
        Ok(())
    }
}

/// Indique si l'application est autorisée dans Accessibilité.
pub fn is_accessibility_granted() -> bool {
    macos::is_accessibility_granted()
}

/// Affiche la boîte de dialogue système (une seule fois par lancement) et
/// renvoie l'état de l'autorisation.
pub fn request_accessibility_access() -> bool {
    info!("Demande de l'autorisation macOS « Accessibilité »");
    macos::request_accessibility_access()
}

/// Ouvre le panneau Réglages Système correspondant.
pub fn open_accessibility_settings() -> Result<(), String> {
    macos::open_accessibility_settings()
}
