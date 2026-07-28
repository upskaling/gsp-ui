<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import './styles/theme.css'
import './App.css'

import PlaybackControls from './components/PlaybackControls.vue'
import ShortcutConfigPanel from './components/ShortcutConfigPanel.vue'
import ModelsPage from './components/ModelsPage.vue'
import AboutDialog from './components/AboutDialog.vue'

import { usePlayback } from './composables/usePlayback'
import { useShortcuts } from './composables/useShortcuts'
import { useModels } from './composables/useModels'
import { useLanguage } from './composables/useLanguage'
import { useKeyRecording } from './composables/useKeyRecording'

const appVersion = '0.1.0'
const showConfig = ref(false)
const showModelsPage = ref(false)
const showAbout = ref(false)
const devMode = ref(false)

let shortcutInProgress = false
let unlisten: (() => void)[] = []

const playback = usePlayback()
const shortcuts = useShortcuts()
const models = useModels()
const language = useLanguage()
const keyRecording = useKeyRecording(shortcuts)

async function loadDevMode() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    devMode.value = await invoke('load_dev_mode')
    console.log('[loadDevMode] Mode développeur chargé:', devMode.value)
  } catch (error) {
    console.error('Erreur lors du chargement du mode développeur:', error)
  }
}

async function toggleDevMode() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    console.log('[toggleDevMode] Sauvegarde du mode développeur:', devMode.value)
    await invoke('save_dev_mode', { enabled: devMode.value })
    console.log('[toggleDevMode] Mode développeur sauvegardé:', devMode.value)
  } catch (error) {
    console.error('Erreur lors de la sauvegarde du mode développeur:', error)
  }
}

onMounted(async () => {
  // Load all configurations
  await playback.loadPlaybackSpeed()
  await shortcuts.loadShortcutConfig()
  await shortcuts.loadOCRShortcutConfig()
  await language.loadSourceLanguage()
  await language.loadTargetLanguage()
  await loadDevMode()
  await models.loadModels()

  // Setup keyboard event listeners
  window.addEventListener('keydown', keyRecording.handleKeydown as unknown as EventListener)
  window.addEventListener('keyup', keyRecording.handleKeyup as unknown as EventListener)

  // Listen for playback completion
  try {
    const unlistenPlayback = await listen('playback_finished', () => {
      console.log('[Playback] Événement playback_finished reçu, mise à jour de isSpeaking à false')
      playback.isSpeaking.value = false
    })
    unlisten.push(unlistenPlayback)
    console.log('[Playback] Listener playback_finished configuré')
  } catch (error) {
    console.error('[Playback] Erreur lors de la configuration du listener playback_finished:', error)
  }

  // Listen for global shortcut trigger
  try {
    const unlistenGlobalShortcut = await listen('global_shortcut_triggered', async () => {
      console.log('[Global Shortcut] Raccourci global reçu! isSpeaking =', playback.isSpeaking.value)

      if (shortcutInProgress) {
        console.log('[Global Shortcut] Appel ignoré (déjà en cours)')
        return
      }

      shortcutInProgress = true

      if (playback.isSpeaking.value) {
        console.log('[Global Shortcut] Appel de stopSpeaking()')
        await playback.stopSpeaking()
      } else {
        console.log('[Global Shortcut] Appel de speakSelection()')
        await playback.speakSelection()
      }

      await new Promise((resolve) => setTimeout(resolve, 1000))
      shortcutInProgress = false
      console.log('[Global Shortcut] Flag réinitialisé')
    })
    unlisten.push(unlistenGlobalShortcut)
    console.log('[Global Shortcut] Listener du raccourci global configuré')
  } catch (error) {
    console.error('[Global Shortcut] Erreur lors de la configuration du listener:', error)
  }

  // Listen for OCR shortcut trigger
  try {
    const unlistenOCRShortcut = await listen('global_shortcut_ocr_triggered', async () => {
      console.log('[Global Shortcut OCR] Raccourci OCR reçu! isSpeaking =', playback.isSpeaking.value)

      if (shortcutInProgress) {
        console.log('[Global Shortcut OCR] Appel ignoré (déjà en cours)')
        return
      }

      shortcutInProgress = true

      if (playback.isSpeaking.value) {
        console.log('[Global Shortcut OCR] Appel de stopSpeaking()')
        await playback.stopSpeaking()
      } else {
        console.log('[Global Shortcut OCR] Appel de speakOCR()')
        await playback.speakOCR()
      }

      await new Promise((resolve) => setTimeout(resolve, 1000))
      shortcutInProgress = false
      console.log('[Global Shortcut OCR] Flag réinitialisé')
    })
    unlisten.push(unlistenOCRShortcut)
    console.log('[Global Shortcut OCR] Listener du raccourci OCR configuré')
  } catch (error) {
    console.error('[Global Shortcut OCR] Erreur lors de la configuration du listener:', error)
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', keyRecording.handleKeydown as unknown as EventListener)
  window.removeEventListener('keyup', keyRecording.handleKeyup as unknown as EventListener)

  for (const unlistenFn of unlisten) {
    try {
      unlistenFn()
    } catch (error) {
      console.error('Erreur lors du nettoyage du listener:', error)
    }
  }
  unlisten = []
})
</script>

<template>
  <main class="min-h-screen bg-background text-text">
    <div class="max-w-2xl mx-auto p-6">
      <!-- Header -->
      <div class="mb-8">
        <h1 class="text-3xl font-bold mb-2">gsp-ui</h1>
      </div>

      <!-- Playback Controls -->
      <div class="mb-8">
        <PlaybackControls
          :is-speaking="playback.isSpeaking.value"
          :playback-speed="playback.playbackSpeed.value"
          :speed-options="playback.speedOptions"
          :source-language="language.sourceLanguage.value"
          :target-language="language.targetLanguage.value"
          :source-language-options="language.sourceLanguageOptions"
          :target-language-options="language.targetLanguageOptions"
          @speak="playback.speakSelection"
          @speak-ocr="playback.speakOCR"
          @stop="playback.stopSpeaking"
          @speed-change="playback.setPlaybackSpeed"
          @source-language-change="language.setSourceLanguage"
          @target-language-change="language.setTargetLanguage"
          @config-click="showConfig = !showConfig"
          @models-click="showModelsPage = !showModelsPage"
          @about-click="showAbout = !showAbout"
        />
      </div>

      <!-- Dev Mode Toggle (debug only) -->
      <div v-if="devMode" class="mb-6 p-4 bg-mid-gray/10 rounded">
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" v-model="devMode" @change="toggleDevMode" class="w-4 h-4" />
          <span class="text-sm">🧪 Dev Mode</span>
        </label>
      </div>

      <!-- Shortcut Config Section -->
      <div v-if="showConfig" class="mb-8 space-y-4">
        <h2 class="text-2xl font-bold mb-4">Configuration des raccourcis</h2>

        <ShortcutConfigPanel
          title="Raccourci - Lecture"
          :shortcut-config="shortcuts.shortcutConfig.value"
          :recording-key="shortcuts.recordingKey.value"
          binding-id="clipboard"
          @update:shortcut-config="(config) => (shortcuts.shortcutConfig.value = config)"
          @toggle-recording="shortcuts.toggleRecordingKey"
        />

        <ShortcutConfigPanel
          title="Raccourci - OCR"
          :shortcut-config="shortcuts.ocrShortcutConfig.value"
          :recording-key="shortcuts.recordingOCRKey.value"
          binding-id="ocr"
          @update:shortcut-config="(config) => (shortcuts.ocrShortcutConfig.value = config)"
          @toggle-recording="shortcuts.toggleRecordingOCRKey"
        />

        <!-- Config buttons -->
        <div class="flex gap-2 justify-center pt-4">
          <button
            @click="shortcuts.saveAllShortcutConfigs().then(() => (showConfig = false))"
            class="px-6 py-2 bg-background-ui text-white rounded font-medium hover:opacity-90 transition"
          >
            💾 Enregistrer
          </button>
          <button
            @click="shortcuts.resetShortcutsToDefault"
            class="px-6 py-2 bg-mid-gray/20 rounded font-medium hover:bg-mid-gray/40 transition"
          >
            🔄 Réinitialiser
          </button>
          <button
            @click="showConfig = false"
            class="px-6 py-2 bg-mid-gray/20 rounded font-medium hover:bg-mid-gray/40 transition"
          >
            ✕ Annuler
          </button>
        </div>
      </div>

      <!-- Models Section -->
      <div v-if="showModelsPage" class="mb-8">
        <ModelsPage
          :models="models.models.value"
          :loading="models.loadingModels.value"
          @delete="models.deleteModel"
          @refresh="models.loadModels"
        />
        <div class="mt-4 flex justify-center">
          <button
            @click="showModelsPage = false"
            class="px-6 py-2 bg-mid-gray/20 rounded font-medium hover:bg-mid-gray/40 transition"
          >
            ✕ Fermer
          </button>
        </div>
      </div>

      <!-- About Section -->
      <div v-if="showAbout" class="mb-8">
        <AboutDialog :version="appVersion" @close="showAbout = false" />
      </div>
    </div>
  </main>
</template>
