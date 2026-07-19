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

const showShortcutConfig = ref(false);
const shortcutConfig = ref<ClipboardShortcut>({
  ctrl: true,
  shift: true,
  alt: false,
  meta: false,
  key: "c",
});
const recordingKey = ref(false);

const showOCRShortcutConfig = ref(false);
const ocrShortcutConfig = ref<ClipboardShortcut>({
  ctrl: true,
  shift: false,
  alt: true,
  meta: false,
  key: "o",
});
const recordingOCRKey = ref(false);

const isSpeaking = ref(false);
const playbackSpeed = ref(1.0);
const speedOptions = [0.75, 1.0, 1.25, 1.5, 2.0];
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
let shortcutInProgress = false;

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
    alert(`Erreur OCR: ${error}`);
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

const handleKeydown = (event: KeyboardEvent) => {
  if (recordingKey.value) {
    event.preventDefault();
    shortcutConfig.value.key = event.key.toLowerCase();
    shortcutConfig.value.ctrl = event.ctrlKey;
    shortcutConfig.value.shift = event.shiftKey;
    shortcutConfig.value.alt = event.altKey;
    shortcutConfig.value.meta = event.metaKey;
    recordingKey.value = false;
    return;
  }

  if (recordingOCRKey.value) {
    event.preventDefault();
    ocrShortcutConfig.value.key = event.key.toLowerCase();
    ocrShortcutConfig.value.ctrl = event.ctrlKey;
    ocrShortcutConfig.value.shift = event.shiftKey;
    ocrShortcutConfig.value.alt = event.altKey;
    ocrShortcutConfig.value.meta = event.metaKey;
    recordingOCRKey.value = false;
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

async function saveShortcutConfig() {
  console.log("[saveShortcutConfig] Début de saveShortcutConfig()");
  try {
    console.log("[saveShortcutConfig] Sauvegarde de la config");
    await invoke("save_shortcut_config", { shortcut: shortcutConfig.value });

    // Réenregistrer le raccourci global avec la nouvelle configuration
    try {
      console.log("[saveShortcutConfig] Désenregistrement du raccourci");
      await invoke("unregister_global_shortcut");
    } catch {
      // Ignorer si le désenregistrement échoue (raccourci peut ne pas être enregistré)
      console.log("[saveShortcutConfig] Désenregistrement échoué (ignoré)");
    }

    console.log("[saveShortcutConfig] Enregistrement du nouveau raccourci");
    await invoke("register_global_shortcut");
    showShortcutConfig.value = false;
    console.log("[saveShortcutConfig] Terminé");
  } catch (error) {
    console.log("[saveShortcutConfig] Erreur:", error);
    alert(`Erreur lors de la sauvegarde: ${error}`);
  }
}

async function saveOCRShortcutConfig() {
  console.log("[saveOCRShortcutConfig] Début de saveOCRShortcutConfig()");
  try {
    console.log("[saveOCRShortcutConfig] Sauvegarde de la config OCR");
    await invoke("save_ocr_shortcut_config", { shortcut: ocrShortcutConfig.value });

    // Réenregistrer le raccourci global avec la nouvelle configuration
    try {
      console.log("[saveOCRShortcutConfig] Désenregistrement du raccourci OCR");
      await invoke("unregister_global_shortcut");
    } catch {
      // Ignorer si le désenregistrement échoue (raccourci peut ne pas être enregistré)
      console.log("[saveOCRShortcutConfig] Désenregistrement échoué (ignoré)");
    }

    console.log("[saveOCRShortcutConfig] Enregistrement du nouveau raccourci OCR");
    await invoke("register_global_shortcut");
    showOCRShortcutConfig.value = false;
    console.log("[saveOCRShortcutConfig] Terminé");
  } catch (error) {
    console.log("[saveOCRShortcutConfig] Erreur:", error);
    alert(`Erreur lors de la sauvegarde: ${error}`);
  }
}

onMounted(async () => {
  await loadShortcutConfig();
  await loadOCRShortcutConfig();
  await loadPlaybackSpeed();
  await loadSourceLanguage();
  await loadTargetLanguage();
  await loadDevMode();
  window.addEventListener("keydown", handleKeydown);

  // Écouter quand la lecture se termine
  try {
    await listen("playback_finished", () => {
      console.log("[Playback] Événement playback_finished reçu, mise à jour de isSpeaking à false");
      isSpeaking.value = false;
    });
    console.log("[Playback] Listener playback_finished configuré");
  } catch (error) {
    console.error("[Playback] Erreur lors de la configuration du listener playback_finished:", error);
  }

  // Écouter l'événement du raccourci global
  try {
    await listen("global_shortcut_triggered", async () => {
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
    console.log("[Global Shortcut] Listener du raccourci global configuré");
  } catch (error) {
    console.error("[Global Shortcut] Erreur lors de la configuration du listener:", error);
  }

  // Écouter l'événement du raccourci OCR
  try {
    await listen("global_shortcut_ocr_triggered", async () => {
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
    console.log("[Global Shortcut OCR] Listener du raccourci OCR configuré");
  } catch (error) {
    console.error("[Global Shortcut OCR] Erreur lors de la configuration du listener:", error);
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <main class="container">
    <div class="clipboard-section">
      <div class="clipboard-controls">
        <div class="language-controls">
        <button @click="speakSelection" :disabled="isSpeaking" class="speak-btn">
          {{ isSpeaking ? "🔊 Lecture en cours..." : "🔊 Lire" }}
        </button>
        <button @click="speakOCR" :disabled="isSpeaking" class="speak-ocr-btn">
          {{ isSpeaking ? "📸 Capture en cours..." : "📸 Lecture OCR" }}
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
        <button @click="showShortcutConfig = !showShortcutConfig" class="config-btn">
          ⚙️ Configurer
        </button>
        <button @click="showOCRShortcutConfig = !showOCRShortcutConfig" class="config-btn">
          ⚙️ Raccourci OCR
        </button>
        <label class="dev-mode-toggle">
          <input type="checkbox" v-model="devMode" @change="toggleDevMode" />
          🧪 Dev
        </label>
      </div>

      <div v-if="showShortcutConfig" class="shortcut-config">
        <h3>Configurer le raccourci clavier - Lecture</h3>
        <div class="shortcut-options">
          <label>
            <input v-model="shortcutConfig.ctrl" type="checkbox" /> Ctrl
          </label>
          <label>
            <input v-model="shortcutConfig.alt" type="checkbox" /> Alt
          </label>
          <label>
            <input v-model="shortcutConfig.meta" type="checkbox" /> Super
          </label>
          <label>
            <input v-model="shortcutConfig.shift" type="checkbox" /> Shift
          </label>
        </div>
        <div class="key-input">
          <button
            @click="recordingKey = !recordingKey"
            :class="{ recording: recordingKey }"
            class="record-btn"
          >
            {{ recordingKey ? "Appuyez sur une touche..." : `Touche: ${shortcutConfig.key.toUpperCase()}` }}
          </button>
        </div>
        <div class="config-buttons">
          <button @click="saveShortcutConfig" class="save-btn">Enregistrer</button>
          <button @click="showShortcutConfig = false" class="cancel-btn">Annuler</button>
        </div>
      </div>

      <div v-if="showOCRShortcutConfig" class="shortcut-config">
        <h3>Configurer le raccourci clavier - OCR</h3>
        <div class="shortcut-options">
          <label>
            <input v-model="ocrShortcutConfig.ctrl" type="checkbox" /> Ctrl
          </label>
          <label>
            <input v-model="ocrShortcutConfig.alt" type="checkbox" /> Alt
          </label>
          <label>
            <input v-model="ocrShortcutConfig.meta" type="checkbox" /> Super
          </label>
          <label>
            <input v-model="ocrShortcutConfig.shift" type="checkbox" /> Shift
          </label>
        </div>
        <div class="key-input">
          <button
            @click="recordingOCRKey = !recordingOCRKey"
            :class="{ recording: recordingOCRKey }"
            class="record-btn"
          >
            {{ recordingOCRKey ? "Appuyez sur une touche..." : `Touche: ${ocrShortcutConfig.key.toUpperCase()}` }}
          </button>
        </div>
        <div class="config-buttons">
          <button @click="saveOCRShortcutConfig" class="save-btn">Enregistrer</button>
          <button @click="showOCRShortcutConfig = false" class="cancel-btn">Annuler</button>
        </div>
      </div>

    </div>
  </main>
</template>