<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ClipboardShortcut {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
  key: string;
}

const greetMsg = ref("");
const name = ref("");
const clipboardContent = ref("");
const clipboardError = ref("");
const showShortcutConfig = ref(false);
const shortcutConfig = ref<ClipboardShortcut>({
  ctrl: true,
  shift: true,
  alt: false,
  meta: false,
  key: "c",
});
const recordingKey = ref(false);

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsg.value = await invoke("greet", { name: name.value });
}

async function getClipboardContent() {
  try {
    clipboardError.value = "";
    clipboardContent.value = await invoke("get_clipboard_content");
  } catch (error) {
    clipboardError.value = `Erreur: ${error}`;
    clipboardContent.value = "";
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

  const { ctrl, shift, alt, meta, key } = shortcutConfig.value;
  const matches =
    ctrl === event.ctrlKey &&
    shift === event.shiftKey &&
    alt === event.altKey &&
    meta === event.metaKey &&
    key === event.key.toLowerCase();

  if (matches) {
    event.preventDefault();
    getClipboardContent();
  }
};

async function loadShortcutConfig() {
  try {
    shortcutConfig.value = await invoke("load_shortcut_config");
  } catch (error) {
    console.error("Erreur lors du chargement de la configuration:", error);
  }
}

async function saveShortcutConfig() {
  try {
    await invoke("save_shortcut_config", { shortcut: shortcutConfig.value });

    // Réenregistrer le raccourci global avec la nouvelle configuration
    try {
      await invoke("unregister_global_shortcut");
    } catch {
      // Ignorer si le désenregistrement échoue (raccourci peut ne pas être enregistré)
    }

    await invoke("register_global_shortcut");
    showShortcutConfig.value = false;
  } catch (error) {
    alert(`Erreur lors de la sauvegarde: ${error}`);
  }
}

onMounted(async () => {
  await loadShortcutConfig();
  window.addEventListener("keydown", handleKeydown);

  // Écouter l'événement du raccourci global
  try {
    await listen("global_shortcut_triggered", () => {
      console.log("Événement du raccourci global reçu!");
      getClipboardContent();
    });
    console.log("Listener du raccourci global configuré");
  } catch (error) {
    console.error("Erreur lors de la configuration du listener:", error);
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <main class="container">
    <form class="row" @submit.prevent="greet">
      <input id="greet-input" v-model="name" placeholder="Enter a name..." />
      <button type="submit">Greet</button>
    </form>
    <p>{{ greetMsg }}</p>

    <div class="clipboard-section">
      <div class="clipboard-controls">
        <button @click="getClipboardContent">
          Afficher le contenu du presse-papier
          <span class="shortcut-hint">
            ({{ shortcutConfig.ctrl ? "Ctrl+" : "" }}{{ shortcutConfig.alt ? "Alt+" : "" }}{{ shortcutConfig.meta ? "Super+" : "" }}{{ shortcutConfig.shift ? "Shift+" : "" }}{{ shortcutConfig.key.toUpperCase() }})
          </span>
        </button>
        <button @click="showShortcutConfig = !showShortcutConfig" class="config-btn">
          ⚙️ Configurer
        </button>
      </div>

      <div v-if="showShortcutConfig" class="shortcut-config">
        <h3>Configurer le raccourci clavier</h3>
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

      <div v-if="clipboardError" class="clipboard-error">{{ clipboardError }}</div>
      <div v-if="clipboardContent" class="clipboard-display">
        <strong>Contenu du presse-papier:</strong>
        <p>{{ clipboardContent }}</p>
      </div>
    </div>
  </main>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

.clipboard-section {
  margin-top: 2rem;
  padding: 1rem;
  border: 1px solid #ccc;
  border-radius: 8px;
  background-color: #f9f9f9;
}

.clipboard-display {
  margin-top: 1rem;
  padding: 1rem;
  background-color: #ffffff;
  border: 1px solid #ddd;
  border-radius: 4px;
  max-height: 200px;
  overflow-y: auto;
  word-break: break-word;
}

.clipboard-error {
  margin-top: 1rem;
  padding: 1rem;
  background-color: #fee;
  color: #c33;
  border: 1px solid #fcc;
  border-radius: 4px;
}

.shortcut-hint {
  display: block;
  font-size: 0.75em;
  color: #666;
  margin-top: 0.3em;
  font-weight: normal;
}

.clipboard-controls {
  display: flex;
  gap: 1rem;
  flex-wrap: wrap;
  align-items: center;
}

.config-btn {
  font-size: 0.9em;
  padding: 0.5em 1em;
}

.shortcut-config {
  margin-top: 1rem;
  padding: 1rem;
  border: 1px solid #ddd;
  border-radius: 4px;
  background-color: #f5f5f5;
}

.shortcut-config h3 {
  margin-top: 0;
  margin-bottom: 1rem;
  font-size: 1em;
}

.shortcut-options {
  display: flex;
  gap: 1.5rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.shortcut-options label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.shortcut-options input {
  cursor: pointer;
}

.key-input {
  margin-bottom: 1rem;
}

.record-btn {
  width: 100%;
  padding: 0.8em 1.2em;
  font-size: 1em;
  background-color: #ffffff;
  transition: background-color 0.2s;
}

.record-btn.recording {
  background-color: #ffe0e0;
  border-color: #c33;
}

.config-buttons {
  display: flex;
  gap: 1rem;
  justify-content: center;
}

.save-btn {
  background-color: #28a745;
  color: white;
  border-color: #28a745;
}

.save-btn:hover {
  background-color: #218838;
  border-color: #218838;
}

.cancel-btn {
  background-color: #6c757d;
  color: white;
  border-color: #6c757d;
}

.cancel-btn:hover {
  background-color: #5a6268;
  border-color: #5a6268;
}
</style>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }

  .clipboard-section {
    border-color: #555;
    background-color: #1f1f1f;
  }

  .clipboard-display {
    background-color: #2a2a2a;
    border-color: #444;
  }

  .clipboard-error {
    background-color: #3a1a1a;
    color: #ff6b6b;
    border-color: #662222;
  }

  .shortcut-hint {
    color: #aaa;
  }

  .shortcut-config {
    border-color: #444;
    background-color: #1f1f1f;
  }

  .record-btn.recording {
    background-color: #3a1a1a;
    border-color: #ff6b6b;
  }

  .save-btn {
    background-color: #28a745;
  }

  .save-btn:hover {
    background-color: #218838;
  }

  .cancel-btn {
    background-color: #6c757d;
  }

  .cancel-btn:hover {
    background-color: #5a6268;
  }
}

</style>