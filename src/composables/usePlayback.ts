import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export function usePlayback() {
  const isSpeaking = ref(false)
  const playbackSpeed = ref(1.0)
  const speedOptions = [0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 2.75, 3.0]
  const volume = ref(1.0)

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

  async function loadVolume() {
    try {
      volume.value = await invoke('load_volume')
      console.log('[loadVolume] Volume chargé:', volume.value)
    } catch (error) {
      console.error('Erreur lors du chargement du volume:', error)
    }
  }

  async function setVolume(newVolume: number) {
    console.log('[setVolume] Nouveau volume:', newVolume)
    volume.value = newVolume
    try {
      // Essayer de changer le volume en cours de lecture
      try {
        await invoke('set_playing_volume', { volume: newVolume })
        console.log('[setVolume] Volume de lecture mis à jour')
      } catch (error) {
        console.log('[setVolume] Aucune lecture en cours:', error)
      }

      // Toujours sauvegarder la préférence
      await invoke('save_volume', { volume: newVolume })
      console.log('[setVolume] Volume sauvegardé')
    } catch (error) {
      console.error('Erreur lors de la sauvegarde du volume:', error)
    }
  }

  return {
    isSpeaking,
    playbackSpeed,
    speedOptions,
    volume,
    speakSelection,
    speakOCR,
    stopSpeaking,
    loadPlaybackSpeed,
    setPlaybackSpeed,
    loadVolume,
    setVolume,
  }
}
