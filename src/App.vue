<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

interface ClipboardShortcut {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
  key: string;
}

const showConfig = ref(false);
const shortcutConfig = ref<ClipboardShortcut>({
  ctrl: true,
  shift: true,
  alt: false,
  meta: false,
  key: "c",
});
const recordingKey = ref(false);
const recordingBindingId = ref<string | null>(null);

const ocrShortcutConfig = ref<ClipboardShortcut>({
  ctrl: true,
  shift: false,
  alt: true,
  meta: false,
  key: "o",
});
const recordingOCRKey = ref(false);
const recordingOCRBindingId = ref<string | null>(null);

const isSpeaking = ref(false);
const playbackSpeed = ref(1.0);
const speedOptions = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0];
const sourceLanguage = ref("auto");
const targetLanguage = ref("fr");
const sourceLanguageOptions = [
  { code: "auto", label: "Détection automatique" },
  { code: "fr", label: "Français" },
  { code: "en", label: "English" }
];
const targetLanguageOptions = [
  { code: "fr", label: "Français" },
  { code: "en", label: "English" }
];
const devMode = ref(false);
const showModelsPage = ref(false);
const models = ref<Array<{ name: string; size_mb: number }>>([]);
const loadingModels = ref(false);
const showAbout = ref(false);
const appVersion = "0.1.0";
let shortcutInProgress = false;
let unlisten: (() => void)[] = [];

// Tracker pour les touches modificateurs (pour gérer le super+key correctement sur X11)
const heldModifiers = {
  ctrl: false,
  shift: false,
  alt: false,
  meta: false,
};

async function speakSelection() {
  console.log("[speakSelection] Début");

  try {
    isSpeaking.value = true;
    console.log("[speakSelection] Appel de invoke('speak_clipboard')");
    await invoke("speak_clipboard");
    console.log("[speakSelection] speak_clipboard() terminé");
  } catch (error) {
    console.log("[speakSelection] Erreur:", error);
    isSpeaking.value = false;
  }
}

async function speakOCR() {
  console.log("[speakOCR] Début");

  try {
    isSpeaking.value = true;
    console.log("[speakOCR] Appel de invoke('speak_ocr')");
    await invoke("speak_ocr");
    console.log("[speakOCR] speak_ocr() terminé");
  } catch (error) {
    console.log("[speakOCR] Erreur:", error);
    isSpeaking.value = false;
  }
}

async function stopSpeaking() {
  console.log("[stopSpeaking] Début");
  try {
    console.log("[stopSpeaking] Appel de invoke('stop_speak')");
    await invoke("stop_speak");
    console.log("[stopSpeaking] stop_speak() terminé");
  } catch (error) {
    console.error(`[stopSpeaking] Erreur lors de l'arrêt de la lecture: ${error}`);
  } finally {
    console.log("[stopSpeaking] Mise à jour de isSpeaking à false");
    isSpeaking.value = false;
  }
}

const handleKeydown = async (event: KeyboardEvent) => {
  // Mettre à jour les touches modificateurs tracées
  if (event.key === "Control") heldModifiers.ctrl = event.ctrlKey;
  if (event.key === "Shift") heldModifiers.shift = event.shiftKey;
  if (event.key === "Alt") heldModifiers.alt = event.altKey;
  if (event.key === "Meta") heldModifiers.meta = event.metaKey;

  // Déclencher la lecture si le raccourci correspond et qu'on n'est pas en mode enregistrement
  if (!recordingKey.value && !recordingOCRKey.value) {
    const { ctrl, shift, alt, meta, key } = shortcutConfig.value;
    const keyMatches =
      ctrl === event.ctrlKey &&
      shift === event.shiftKey &&
      alt === event.altKey &&
      meta === event.metaKey &&
      key === get_key_from_code(event.code).toLowerCase();

    if (keyMatches) {
      event.preventDefault();
      speakSelection();
    }
  }
};

const handleKeyup = async (event: KeyboardEvent) => {
  // Mettre à jour les touches modificateurs tracées quand elles sont relâchées
  if (event.key === "Control") heldModifiers.ctrl = event.ctrlKey;
  if (event.key === "Shift") heldModifiers.shift = event.shiftKey;
  if (event.key === "Alt") heldModifiers.alt = event.altKey;
  if (event.key === "Meta") heldModifiers.meta = event.metaKey;

  if (recordingKey.value) {
    event.preventDefault();

    // Ne pas enregistrer si c'est juste un modificateur seul
    if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) {
      return;
    }

    // Utiliser event.code (code physique de la touche) au lieu de event.key (caractère produit)
    // Cela fonctionne correctement avec les claviers non-QWERTY (azerty, dvorak, etc)
    const keyCode = get_key_from_code(event.code);
    shortcutConfig.value.key = keyCode;
    recordingKey.value = false;

    // Reprendre le raccourci après l'enregistrement
    if (recordingBindingId.value) {
      try {
        await invoke("resume_binding", { id: recordingBindingId.value });
      } catch (error) {
        console.error("Erreur lors de la reprise du raccourci:", error);
      }
      recordingBindingId.value = null;
    }
    return;
  }

  if (recordingOCRKey.value) {
    event.preventDefault();

    // Ne pas enregistrer si c'est juste un modificateur seul
    if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) {
      return;
    }

    // Utiliser event.code (code physique de la touche) au lieu de event.key (caractère produit)
    const keyCode = get_key_from_code(event.code);
    ocrShortcutConfig.value.key = keyCode;
    recordingOCRKey.value = false;

    // Reprendre le raccourci après l'enregistrement
    if (recordingOCRBindingId.value) {
      try {
        await invoke("resume_binding", { id: recordingOCRBindingId.value });
      } catch (error) {
        console.error("Erreur lors de la reprise du raccourci OCR:", error);
      }
      recordingOCRBindingId.value = null;
    }
    return;
  }

  const { ctrl, shift, alt, meta, key } = shortcutConfig.value;
  const matches =
    ctrl === event.ctrlKey &&
    shift === event.shiftKey &&
    alt === event.altKey &&
    meta === event.metaKey &&
    key === event.key.toLowerCase();

  if (matches) {
    event.preventDefault();
    speakSelection();
  }
};

async function loadShortcutConfig() {
  try {
    shortcutConfig.value = await invoke("load_shortcut_config");
  } catch (error) {
    console.error("Erreur lors du chargement de la configuration:", error);
  }
}

async function loadOCRShortcutConfig() {
  try {
    ocrShortcutConfig.value = await invoke("load_ocr_shortcut_config");
  } catch (error) {
    console.error("Erreur lors du chargement de la configuration OCR:", error);
  }
}

async function loadPlaybackSpeed() {
  try {
    playbackSpeed.value = await invoke("load_playback_speed");
    console.log("[loadPlaybackSpeed] Vitesse chargée:", playbackSpeed.value);
  } catch (error) {
    console.error("Erreur lors du chargement de la vitesse:", error);
  }
}

async function loadDevMode() {
  try {
    devMode.value = await invoke("load_dev_mode");
    console.log("[loadDevMode] Mode développeur chargé:", devMode.value);
  } catch (error) {
    console.error("Erreur lors du chargement du mode développeur:", error);
  }
}

async function toggleDevMode() {
  try {
    console.log("[toggleDevMode] Sauvegarde du mode développeur:", devMode.value);
    await invoke("save_dev_mode", { enabled: devMode.value });
    console.log("[toggleDevMode] Mode développeur sauvegardé:", devMode.value);
  } catch (error) {
    console.error("Erreur lors de la sauvegarde du mode développeur:", error);
  }
}

async function loadSourceLanguage() {
  try {
    sourceLanguage.value = await invoke("load_source_language");
    console.log("[loadSourceLanguage] Langue source chargée:", sourceLanguage.value);
  } catch (error) {
    console.error("Erreur lors du chargement de la langue source:", error);
  }
}

async function setSourceLanguage(lang: string) {
  console.log("[setSourceLanguage] Nouvelle langue source:", lang);
  sourceLanguage.value = lang;
  try {
    await invoke("save_source_language", { language: lang });
    console.log("[setSourceLanguage] Langue source sauvegardée");
  } catch (error) {
    console.error("Erreur lors de la sauvegarde de la langue source:", error);
  }
}

async function loadTargetLanguage() {
  try {
    targetLanguage.value = await invoke("load_target_language");
    console.log("[loadTargetLanguage] Langue cible chargée:", targetLanguage.value);
  } catch (error) {
    console.error("Erreur lors du chargement de la langue cible:", error);
  }
}

async function setTargetLanguage(lang: string) {
  console.log("[setTargetLanguage] Nouvelle langue cible:", lang);
  targetLanguage.value = lang;
  try {
    await invoke("save_target_language", { language: lang });
    console.log("[setTargetLanguage] Langue cible sauvegardée");
  } catch (error) {
    console.error("Erreur lors de la sauvegarde de la langue cible:", error);
  }
}

async function setPlaybackSpeed(speed: number) {
  console.log("[setPlaybackSpeed] Nouvelle vitesse:", speed);
  playbackSpeed.value = speed;
  try {
    await invoke("save_playback_speed", { speed });
    console.log("[setPlaybackSpeed] Vitesse sauvegardée");
  } catch (error) {
    console.error("Erreur lors de la sauvegarde de la vitesse:", error);
  }
}

async function saveAllShortcutConfigs() {
  console.log("[saveAllShortcutConfigs] Début");
  try {
    // Formater les raccourcis avec la commande Rust pour utiliser le bon format selon le système
    const clipboardBindingStr = await invoke("format_shortcut_binding", {
      ctrl: shortcutConfig.value.ctrl,
      shift: shortcutConfig.value.shift,
      alt: shortcutConfig.value.alt,
      meta: shortcutConfig.value.meta,
      key: shortcutConfig.value.key,
    }) as string;

    const ocrBindingStr = await invoke("format_shortcut_binding", {
      ctrl: ocrShortcutConfig.value.ctrl,
      shift: ocrShortcutConfig.value.shift,
      alt: ocrShortcutConfig.value.alt,
      meta: ocrShortcutConfig.value.meta,
      key: ocrShortcutConfig.value.key,
    }) as string;

    console.log("[saveAllShortcutConfigs] Nouvelle config lecture:", clipboardBindingStr);
    await invoke("change_binding", { id: "clipboard", binding: clipboardBindingStr });

    console.log("[saveAllShortcutConfigs] Nouvelle config OCR:", ocrBindingStr);
    await invoke("change_binding", { id: "ocr", binding: ocrBindingStr });

    showConfig.value = false;
    console.log("[saveAllShortcutConfigs] Terminé");
  } catch (error) {
    console.log("[saveAllShortcutConfigs] Erreur:", error);
    alert(`Erreur lors de la sauvegarde: ${error}`);
  }
}

/**
 * Convertir le code de la touche (event.code) en format handy-keys
 * event.code retourne le code physique de la touche, indépendant de la disposition du clavier
 */
function get_key_from_code(code: string): string {
  // Touches numériques
  if (code.startsWith("Digit")) {
    return code.replace("Digit", "");
  }
  // Touches lettres
  if (code.startsWith("Key")) {
    return code.replace("Key", "").toLowerCase();
  }
  // Touches numpad
  if (code.startsWith("Numpad")) {
    return code.replace("Numpad", "numpad").toLowerCase();
  }
  // Touches spéciales - mapper vers les noms handy-keys
  const specialKeys: Record<string, string> = {
    Space: "space",
    Enter: "return",
    Tab: "tab",
    Escape: "escape",
    Backspace: "backspace",
    Delete: "delete",
    Insert: "insert",
    Home: "home",
    End: "end",
    PageUp: "pageup",
    PageDown: "pagedown",
    ArrowUp: "up",
    ArrowDown: "down",
    ArrowLeft: "left",
    ArrowRight: "right",
    Minus: "minus",
    Equal: "equal",
    BracketLeft: "bracketleft",
    BracketRight: "bracketright",
    Backslash: "backslash",
    Semicolon: "semicolon",
    Quote: "quote",
    Comma: "comma",
    Period: "period",
    Slash: "slash",
    Backquote: "backquote",
  };

  return specialKeys[code] || code.toLowerCase();
}

async function toggleRecordingKey(bindingId: string) {
  if (recordingKey.value) {
    // Arrêter l'enregistrement et sauvegarder
    recordingKey.value = false;
    await saveAllShortcutConfigs();
  } else {
    // Commencer l'enregistrement - suspendre le raccourci
    try {
      console.log("[toggleRecordingKey] Suspension du raccourci:", bindingId);
      await invoke("suspend_binding", { id: bindingId });
      recordingBindingId.value = bindingId;
      recordingKey.value = true;
    } catch (error) {
      console.error("Erreur lors de la suspension du raccourci:", error);
    }
  }
}

async function toggleRecordingOCRKey(bindingId: string) {
  if (recordingOCRKey.value) {
    // Arrêter l'enregistrement et sauvegarder
    recordingOCRKey.value = false;
    await saveAllShortcutConfigs();
  } else {
    // Commencer l'enregistrement - suspendre le raccourci
    try {
      console.log("[toggleRecordingOCRKey] Suspension du raccourci:", bindingId);
      await invoke("suspend_binding", { id: bindingId });
      recordingOCRBindingId.value = bindingId;
      recordingOCRKey.value = true;
    } catch (error) {
      console.error("Erreur lors de la suspension du raccourci OCR:", error);
    }
  }
}

async function resetShortcutsToDefault() {
  console.log("[resetShortcutsToDefault] Début");
  try {
    console.log("[resetShortcutsToDefault] Réinitialisation des raccourcis par défaut");
    await invoke("reset_binding", { id: "clipboard" });
    await invoke("reset_binding", { id: "ocr" });

    console.log("[resetShortcutsToDefault] Rechargement de la configuration");
    await loadShortcutConfig();
    await loadOCRShortcutConfig();

    console.log("[resetShortcutsToDefault] Terminé");
  } catch (error) {
    console.log("[resetShortcutsToDefault] Erreur:", error);
    alert(`Erreur lors de la réinitialisation: ${error}`);
  }
}

async function loadModels() {
  loadingModels.value = true;
  try {
    console.log("[loadModels] Chargement de la liste des modèles");
    const modelList = await invoke("list_models");
    models.value = modelList as Array<{ name: string; size_mb: number }>;
    console.log("[loadModels] Modèles chargés:", models.value);
  } catch (error) {
    console.error("[loadModels] Erreur:", error);
    alert(`Erreur lors du chargement des modèles: ${error}`);
  } finally {
    loadingModels.value = false;
  }
}

async function deleteModelAction(modelName: string) {
  if (!confirm(`Êtes-vous sûr de vouloir supprimer le modèle "${modelName}" ?`)) {
    return;
  }

  try {
    console.log("[deleteModelAction] Suppression du modèle:", modelName);
    await invoke("delete_model", { modelName });
    console.log("[deleteModelAction] Modèle supprimé");
    await loadModels();
  } catch (error) {
    console.error("[deleteModelAction] Erreur:", error);
    alert(`Erreur lors de la suppression: ${error}`);
  }
}


onMounted(async () => {
  await loadShortcutConfig();
  await loadOCRShortcutConfig();
  await loadPlaybackSpeed();
  await loadSourceLanguage();
  await loadTargetLanguage();
  await loadDevMode();
  await loadModels();
  // Écouter keydown pour tracker les modificateurs et déclencher la lecture
  window.addEventListener("keydown", handleKeydown as unknown as EventListener);
  // Écouter keyup pour l'enregistrement (utilise les modificateurs tracés)
  window.addEventListener("keyup", handleKeyup as unknown as EventListener);

  // Écouter quand la lecture se termine
  try {
    const unlistenPlayback = await listen("playback_finished", () => {
      console.log("[Playback] Événement playback_finished reçu, mise à jour de isSpeaking à false");
      isSpeaking.value = false;
    });
    unlisten.push(unlistenPlayback);
    console.log("[Playback] Listener playback_finished configuré");
  } catch (error) {
    console.error("[Playback] Erreur lors de la configuration du listener playback_finished:", error);
  }

  // Écouter l'événement du raccourci global
  try {
    const unlistenGlobalShortcut = await listen("global_shortcut_triggered", async () => {
      console.log("[Global Shortcut] Raccourci global reçu! isSpeaking =", isSpeaking.value);

      // Ignorer les appels en double si un traitement est en cours
      if (shortcutInProgress) {
        console.log("[Global Shortcut] Appel ignoré (déjà en cours)");
        return;
      }

      shortcutInProgress = true;

      if (isSpeaking.value) {
        console.log("[Global Shortcut] Appel de stopSpeaking()");
        await stopSpeaking();
      } else {
        console.log("[Global Shortcut] Appel de speakSelection()");
        await speakSelection();
      }

      // Attendre avant de réinitialiser le flag pour éviter les appels en double
      await new Promise(resolve => setTimeout(resolve, 1000));
      shortcutInProgress = false;
      console.log("[Global Shortcut] Flag réinitialisé");
    });
    unlisten.push(unlistenGlobalShortcut);
    console.log("[Global Shortcut] Listener du raccourci global configuré");
  } catch (error) {
    console.error("[Global Shortcut] Erreur lors de la configuration du listener:", error);
  }

  // Écouter l'événement du raccourci OCR
  try {
    const unlistenOCRShortcut = await listen("global_shortcut_ocr_triggered", async () => {
      console.log("[Global Shortcut OCR] Raccourci OCR reçu! isSpeaking =", isSpeaking.value);

      // Ignorer les appels en double si un traitement est en cours
      if (shortcutInProgress) {
        console.log("[Global Shortcut OCR] Appel ignoré (déjà en cours)");
        return;
      }

      shortcutInProgress = true;

      if (isSpeaking.value) {
        console.log("[Global Shortcut OCR] Appel de stopSpeaking()");
        await stopSpeaking();
      } else {
        console.log("[Global Shortcut OCR] Appel de speakOCR()");
        await speakOCR();
      }

      // Attendre avant de réinitialiser le flag pour éviter les appels en double
      await new Promise(resolve => setTimeout(resolve, 1000));
      shortcutInProgress = false;
      console.log("[Global Shortcut OCR] Flag réinitialisé");
    });
    unlisten.push(unlistenOCRShortcut);
    console.log("[Global Shortcut OCR] Listener du raccourci OCR configuré");
  } catch (error) {
    console.error("[Global Shortcut OCR] Erreur lors de la configuration du listener:", error);
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown as unknown as EventListener);
  window.removeEventListener("keyup", handleKeyup as unknown as EventListener);

  // Nettoyer tous les listeners Tauri
  for (const unlistenFn of unlisten) {
    try {
      unlistenFn();
    } catch (error) {
      console.error("Erreur lors du nettoyage du listener:", error);
    }
  }
  unlisten = [];
});
</script>

<template>
  <main class="container">
    <div class="clipboard-section">
      <div class="clipboard-actions">
        <div class="language-controls">
        <button @click="speakSelection" :disabled="isSpeaking" class="speak-btn">
          {{ isSpeaking ? "🔊 Lecture en cours..." : "🔊 Lire" }}
        </button>
        <button @click="speakOCR" :disabled="isSpeaking" class="speak-ocr-btn">
          {{ isSpeaking ? "📸 Capture en cours..." : "📸 Lecture OCR" }}
        </button>
        <button v-if="isSpeaking" @click="stopSpeaking" class="stop-btn">
          ⏹️ Arrêter
        </button>
        </div>
      </div>
      <div class="clipboard-controls">
        <div class="speed-controls">
          <label for="speed-select">Vitesse:</label>
          <select
            id="speed-select"
            v-model.number="playbackSpeed"
            @change="setPlaybackSpeed(playbackSpeed)"
            :disabled="isSpeaking"
            class="speed-select"
          >
            <option v-for="speed in speedOptions" :key="speed" :value="speed">
              {{ speed }}x
            </option>
          </select>
        </div>
        <div class="language-controls">
          <label for="source-lang-select">Source:</label>
          <select
            id="source-lang-select"
            v-model="sourceLanguage"
            @change="setSourceLanguage(sourceLanguage)"
            :disabled="isSpeaking"
            class="language-select"
          >
            <option v-for="lang in sourceLanguageOptions" :key="lang.code" :value="lang.code">
              {{ lang.label }}
            </option>
          </select>
        </div>
        <div class="language-controls">
          <label for="target-lang-select">Parler:</label>
          <select
            id="target-lang-select"
            v-model="targetLanguage"
            @change="setTargetLanguage(targetLanguage)"
            :disabled="isSpeaking"
            class="language-select"
          >
            <option v-for="lang in targetLanguageOptions" :key="lang.code" :value="lang.code">
              {{ lang.label }}
            </option>
          </select>
        </div>
        <button @click="showConfig = !showConfig" class="config-btn">
          ⚙️ Configurer
        </button>
        <button @click="showModelsPage = !showModelsPage" class="models-btn">
          📦 Modèles
        </button>
        <button @click="showAbout = !showAbout" class="about-btn">
          ℹ️ À propos
        </button>
        <label class="dev-mode-toggle">
          <input type="checkbox" v-model="devMode" @change="toggleDevMode" />
          🧪 Dev
        </label>
      </div>

      <div v-if="showConfig" class="shortcut-config-unified">
        <div class="config-section">
          <h3>Raccourci - Lecture</h3>

          <div class="modifiers-grid">
            <label class="modifier-checkbox">
              <input type="checkbox" v-model="shortcutConfig.ctrl" />
              <span>Ctrl</span>
            </label>
            <label class="modifier-checkbox">
              <input type="checkbox" v-model="shortcutConfig.shift" />
              <span>Shift</span>
            </label>
            <label class="modifier-checkbox">
              <input type="checkbox" v-model="shortcutConfig.alt" />
              <span>Alt</span>
            </label>
            <label class="modifier-checkbox">
              <input type="checkbox" v-model="shortcutConfig.meta" />
              <span>Super</span>
            </label>
          </div>

          <div class="key-input">
            <button
              @click="toggleRecordingKey('clipboard')"
              :class="{ recording: recordingKey }"
              class="record-btn"
            >
              {{ recordingKey ? "🎹 Appuyez sur une touche..." : `⌨️ Touche: ${shortcutConfig.key.toUpperCase()}` }}
            </button>
          </div>
        </div>

        <div class="config-section">
          <h3>Raccourci - OCR</h3>

          <div class="modifiers-grid">
            <label class="modifier-checkbox">
              <input type="checkbox" v-model="ocrShortcutConfig.ctrl" />
              <span>Ctrl</span>
            </label>
            <label class="modifier-checkbox">
              <input type="checkbox" v-model="ocrShortcutConfig.shift" />
              <span>Shift</span>
            </label>
            <label class="modifier-checkbox">
              <input type="checkbox" v-model="ocrShortcutConfig.alt" />
              <span>Alt</span>
            </label>
            <label class="modifier-checkbox">
              <input type="checkbox" v-model="ocrShortcutConfig.meta" />
              <span>Super</span>
            </label>
          </div>

          <div class="key-input">
            <button
              @click="toggleRecordingOCRKey('ocr')"
              :class="{ recording: recordingOCRKey }"
              class="record-btn"
            >
              {{ recordingOCRKey ? "🎹 Appuyez sur une touche..." : `⌨️ Touche: ${ocrShortcutConfig.key.toUpperCase()}` }}
            </button>
          </div>
        </div>

        <div class="config-buttons-unified">
          <button @click="saveAllShortcutConfigs" class="save-btn">Enregistrer</button>
          <button @click="resetShortcutsToDefault" class="reset-btn">Réinitialiser</button>
          <button @click="showConfig = false" class="cancel-btn">Annuler</button>
        </div>
      </div>

      <div v-if="showModelsPage" class="models-page">
        <h3>Modèles de traduction</h3>

        <div v-if="loadingModels" class="loading">
          ⏳ Chargement des modèles...
        </div>

        <div v-else-if="models.length === 0" class="no-models">
          <p>Aucun modèle installé</p>
        </div>

        <div v-else class="models-table-container">
          <table class="models-table">
            <thead>
              <tr>
                <th>Nom</th>
                <th>Taille</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="model in models" :key="model.name">
                <td class="model-name">{{ model.name }}</td>
                <td class="model-size">{{ model.size_mb.toFixed(2) }} MB</td>
                <td class="model-action">
                  <button
                    @click="deleteModelAction(model.name)"
                    class="delete-btn"
                    title="Supprimer ce modèle"
                  >
                    🗑️ Supprimer
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="models-page-buttons">
          <button @click="loadModels" class="refresh-btn">🔄 Actualiser</button>
          <button @click="showModelsPage = false" class="close-btn">Fermer</button>
        </div>
      </div>

      <div v-if="showAbout" class="about-dialog">
        <div class="about-content">
          <h2>À propos de gsp-ui</h2>
          <p class="version">Version {{ appVersion }}</p>
          <p class="description">
            Une application de lecture d'écran pour les utilisateurs dyslexiques.
          </p>
          <p class="github-link">
            <a href="https://github.com/upskaling/gsp-ui" target="_blank" rel="noopener noreferrer">
              🔗 Consulter le code source sur GitHub
            </a>
          </p>
          <button @click="showAbout = false" class="close-btn">Fermer</button>
        </div>
      </div>

    </div>
  </main>
</template>