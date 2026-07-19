//! Implémentation des raccourcis clavier via handy-keys

use handy_keys::{Hotkey, HotkeyManager};
use log::{debug, error, info};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Manager};

use super::handler::handle_shortcut_event;
use super::settings::ShortcutBinding;

/// Commandes pour le thread du gestionnaire
enum ManagerCommand {
    Register {
        binding_id: String,
        hotkey_string: String,
        response: Sender<Result<(), String>>,
    },
    Unregister {
        binding_id: String,
        response: Sender<Result<(), String>>,
    },
    Shutdown,
}

/// État global pour handy-keys
pub struct HandyKeysState {
    command_sender: Mutex<Sender<ManagerCommand>>,
    _thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl HandyKeysState {
    /// Créer un nouvel état handy-keys
    pub fn new(app: AppHandle) -> Result<Self, String> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<ManagerCommand>();

        // Démarrer le thread du gestionnaire
        let app_clone = app.clone();
        let thread_handle = thread::spawn(move || {
            Self::manager_thread(cmd_rx, app_clone);
        });

        Ok(Self {
            command_sender: Mutex::new(cmd_tx),
            _thread_handle: Mutex::new(Some(thread_handle)),
        })
    }

    /// Le thread principal du gestionnaire
    fn manager_thread(cmd_rx: Receiver<ManagerCommand>, app: AppHandle) {
        info!("handy-keys manager thread started");

        let manager = match HotkeyManager::new() {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to create HotkeyManager: {}", e);
                return;
            }
        };

        let mut binding_to_hotkey: HashMap<String, handy_keys::HotkeyId> = HashMap::new();
        let mut hotkey_to_binding: HashMap<handy_keys::HotkeyId, (String, String)> = HashMap::new();

        loop {
            // Vérifier les événements raccourci (non-bloquant)
            while let Some(event) = manager.try_recv() {
                if let Some((binding_id, hotkey_string)) = hotkey_to_binding.get(&event.id) {
                    debug!(
                        "handy-keys event: binding={}, hotkey={}, state={:?}",
                        binding_id, hotkey_string, event.state
                    );
                    handle_shortcut_event(&app, binding_id, hotkey_string);
                }
            }

            // Vérifier les commandes (non-bloquant avec timeout)
            match cmd_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(cmd) => match cmd {
                    ManagerCommand::Register {
                        binding_id,
                        hotkey_string,
                        response,
                    } => {
                        let result = Self::do_register(
                            &manager,
                            &mut binding_to_hotkey,
                            &mut hotkey_to_binding,
                            &binding_id,
                            &hotkey_string,
                        );
                        let _ = response.send(result);
                    }
                    ManagerCommand::Unregister {
                        binding_id,
                        response,
                    } => {
                        let result = Self::do_unregister(
                            &manager,
                            &mut binding_to_hotkey,
                            &mut hotkey_to_binding,
                            &binding_id,
                        );
                        let _ = response.send(result);
                    }
                    ManagerCommand::Shutdown => {
                        info!("handy-keys manager thread shutting down");
                        break;
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Pas de commande, continuer
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    info!("Command channel disconnected, shutting down");
                    break;
                }
            }
        }

        info!("handy-keys manager thread stopped");
    }

    /// Enregistrer un raccourci
    fn do_register(
        manager: &HotkeyManager,
        binding_to_hotkey: &mut HashMap<String, handy_keys::HotkeyId>,
        hotkey_to_binding: &mut HashMap<handy_keys::HotkeyId, (String, String)>,
        binding_id: &str,
        hotkey_string: &str,
    ) -> Result<(), String> {
        let hotkey: Hotkey = hotkey_string
            .parse()
            .map_err(|e| format!("Erreur d'analyse du raccourci '{}': {}", hotkey_string, e))?;

        let id = manager
            .register(hotkey)
            .map_err(|e| format!("Erreur d'enregistrement du raccourci: {}", e))?;

        binding_to_hotkey.insert(binding_id.to_string(), id);
        hotkey_to_binding.insert(id, (binding_id.to_string(), hotkey_string.to_string()));

        debug!(
            "Registered handy-keys shortcut: {} -> {:?}",
            binding_id, hotkey
        );
        Ok(())
    }

    /// Désenregistrer un raccourci
    fn do_unregister(
        manager: &HotkeyManager,
        binding_to_hotkey: &mut HashMap<String, handy_keys::HotkeyId>,
        hotkey_to_binding: &mut HashMap<handy_keys::HotkeyId, (String, String)>,
        binding_id: &str,
    ) -> Result<(), String> {
        if let Some(id) = binding_to_hotkey.remove(binding_id) {
            manager
                .unregister(id)
                .map_err(|e| format!("Erreur de désenregistrement du raccourci: {}", e))?;
            hotkey_to_binding.remove(&id);
            debug!("Unregistered handy-keys shortcut: {}", binding_id);
        }
        Ok(())
    }

    /// Enregistrer un raccourci
    pub fn register(&self, binding: &ShortcutBinding) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.command_sender
            .lock()
            .map_err(|_| "Erreur de verrouillage du command_sender")?
            .send(ManagerCommand::Register {
                binding_id: binding.id.clone(),
                hotkey_string: binding.current_binding.clone(),
                response: tx,
            })
            .map_err(|_| "Erreur d'envoi de la commande d'enregistrement")?;

        rx.recv()
            .map_err(|_| "Erreur de réception de la réponse d'enregistrement")?
    }

    /// Désenregistrer un raccourci
    pub fn unregister(&self, binding: &ShortcutBinding) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.command_sender
            .lock()
            .map_err(|_| "Erreur de verrouillage du command_sender")?
            .send(ManagerCommand::Unregister {
                binding_id: binding.id.clone(),
                response: tx,
            })
            .map_err(|_| "Erreur d'envoi de la commande de désenregistrement")?;

        rx.recv()
            .map_err(|_| "Erreur de réception de la réponse de désenregistrement")?
    }
}

/// Initialiser handy-keys
pub fn init_shortcuts(app: &AppHandle) {
    use super::settings::load_shortcut_config;

    info!("Initialisation des raccourcis handy-keys");

    match HandyKeysState::new(app.clone()) {
        Ok(state) => {
            // Enregistrer l'état global
            app.manage(state);

            // Charger et enregistrer les raccourcis
            match load_shortcut_config() {
                Ok(config) => {
                    let state = app.state::<HandyKeysState>();
                    for (id, binding) in config.bindings {
                        if let Err(e) = state.register(&binding) {
                            error!(
                                "Erreur d'enregistrement du raccourci '{}' au démarrage: {}",
                                id, e
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Erreur de chargement de la configuration: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Erreur d'initialisation de handy-keys: {}", e);
        }
    }
}

/// Enregistrer un raccourci
pub fn register_shortcut(app: &AppHandle, binding: &ShortcutBinding) -> Result<(), String> {
    if let Some(state) = app.try_state::<HandyKeysState>() {
        state.register(binding)
    } else {
        Err("HandyKeysState non initialisé".to_string())
    }
}

/// Désenregistrer un raccourci
pub fn unregister_shortcut(app: &AppHandle, binding: &ShortcutBinding) -> Result<(), String> {
    if let Some(state) = app.try_state::<HandyKeysState>() {
        state.unregister(binding)
    } else {
        Err("HandyKeysState non initialisé".to_string())
    }
}

/// Valider un raccourci pour handy-keys
pub fn validate_shortcut(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("Le raccourci ne peut pas être vide".into());
    }

    // Essayer de parser le raccourci
    match raw.parse::<Hotkey>() {
        Ok(_) => Ok(()),
        Err(e) => {
            // Essayer d'aider l'utilisateur en suggérant un format correct
            error!("Erreur de parsing du raccourci '{}': {:?}", raw, e);
            Err(format!(
                "Le raccourci '{}' n'est pas valide pour handy-keys. \
                 Format accepté: 'ctrl+shift+v', 'alt+a', etc.",
                raw
            ))
        }
    }
}
