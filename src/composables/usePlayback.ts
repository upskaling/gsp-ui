import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export function usePlayback() {
  const isSpeaking = ref(false)
  const playbackSpeed = ref(1.0)
  const speedOptions = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0]

  async function speakSelection() {
    console.log('[speakSelection] Début')
    try {
      isSpeaking.value = true
      console.log('[speakSelection] Appel de invoke(speak_clipboard)')
      await invoke('speak_clipboard')
      console.log('[speakSelection] speak_clipboard() terminé')
    } catch (error) {
      console.error('[speakSelection] Erreur:', error)
      isSpeaking.value = false
    }
  }

  async function speakOCR() {
    console.log('[speakOCR] Début')
    try {
      isSpeaking.value = true
      console.log('[speakOCR] Appel de invoke(speak_ocr)')
      await invoke('speak_ocr')
      console.log('[speakOCR] speak_ocr() terminé')
    } catch (error) {
      console.error('[speakOCR] Erreur:', error)
      isSpeaking.value = false
    }
  }

  async function stopSpeaking() {
    console.log('[stopSpeaking] Début')
    try {
      console.log('[stopSpeaking] Appel de invoke(stop_speak)')
      await invoke('stop_speak')
      console.log('[stopSpeaking] stop_speak() terminé')
    } catch (error) {
      console.error(`[stopSpeaking] Erreur lors de l'arrêt de la lecture: ${error}`)
    } finally {
      console.log('[stopSpeaking] Mise à jour de isSpeaking à false')
      isSpeaking.value = false
    }
  }

  async function loadPlaybackSpeed() {
    try {
      playbackSpeed.value = await invoke('load_playback_speed')
      console.log('[loadPlaybackSpeed] Vitesse chargée:', playbackSpeed.value)
    } catch (error) {
      console.error('Erreur lors du chargement de la vitesse:', error)
    }
  }

  async function setPlaybackSpeed(speed: number) {
    console.log('[setPlaybackSpeed] Nouvelle vitesse:', speed)
    playbackSpeed.value = speed
    try {
      await invoke('save_playback_speed', { speed })
      console.log('[setPlaybackSpeed] Vitesse sauvegardée')
    } catch (error) {
      console.error('Erreur lors de la sauvegarde de la vitesse:', error)
    }
  }

  return {
    isSpeaking,
    playbackSpeed,
    speedOptions,
    speakSelection,
    speakOCR,
    stopSpeaking,
    loadPlaybackSpeed,
    setPlaybackSpeed,
  }
}
