import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ClipboardShortcut } from '@/types'

export function useShortcuts() {
  const shortcutConfig = ref<ClipboardShortcut>({
    ctrl: true,
    shift: true,
    alt: false,
    meta: false,
    key: 'c',
  })

  const ocrShortcutConfig = ref<ClipboardShortcut>({
    ctrl: true,
    shift: false,
    alt: true,
    meta: false,
    key: 'o',
  })

  const recordingKey = ref(false)
  const recordingBindingId = ref<string | null>(null)
  const recordingOCRKey = ref(false)
  const recordingOCRBindingId = ref<string | null>(null)

  async function loadShortcutConfig() {
    try {
      shortcutConfig.value = await invoke('load_shortcut_config')
    } catch (error) {
      console.error('Erreur lors du chargement de la configuration:', error)
    }
  }

  async function loadOCRShortcutConfig() {
    try {
      ocrShortcutConfig.value = await invoke('load_ocr_shortcut_config')
    } catch (error) {
      console.error('Erreur lors du chargement de la configuration OCR:', error)
    }
  }

  async function saveAllShortcutConfigs() {
    console.log('[saveAllShortcutConfigs] Début')
    try {
      const clipboardBindingStr = await invoke('format_shortcut_binding', {
        ctrl: shortcutConfig.value.ctrl,
        shift: shortcutConfig.value.shift,
        alt: shortcutConfig.value.alt,
        meta: shortcutConfig.value.meta,
        key: shortcutConfig.value.key,
      }) as string

      const ocrBindingStr = await invoke('format_shortcut_binding', {
        ctrl: ocrShortcutConfig.value.ctrl,
        shift: ocrShortcutConfig.value.shift,
        alt: ocrShortcutConfig.value.alt,
        meta: ocrShortcutConfig.value.meta,
        key: ocrShortcutConfig.value.key,
      }) as string

      console.log('[saveAllShortcutConfigs] Nouvelle config lecture:', clipboardBindingStr)
      await invoke('change_binding', { id: 'clipboard', binding: clipboardBindingStr })

      console.log('[saveAllShortcutConfigs] Nouvelle config OCR:', ocrBindingStr)
      await invoke('change_binding', { id: 'ocr', binding: ocrBindingStr })

      console.log('[saveAllShortcutConfigs] Terminé')
    } catch (error) {
      console.error('[saveAllShortcutConfigs] Erreur:', error)
      alert(`Erreur lors de la sauvegarde: ${error}`)
      throw error
    }
  }

  async function resetShortcutsToDefault() {
    console.log('[resetShortcutsToDefault] Début')
    try {
      console.log('[resetShortcutsToDefault] Réinitialisation des raccourcis par défaut')
      await invoke('reset_binding', { id: 'clipboard' })
      await invoke('reset_binding', { id: 'ocr' })

      console.log('[resetShortcutsToDefault] Rechargement de la configuration')
      await loadShortcutConfig()
      await loadOCRShortcutConfig()

      console.log('[resetShortcutsToDefault] Terminé')
    } catch (error) {
      console.error('[resetShortcutsToDefault] Erreur:', error)
      alert(`Erreur lors de la réinitialisation: ${error}`)
      throw error
    }
  }

  async function toggleRecordingKey(bindingId: string) {
    if (recordingKey.value) {
      recordingKey.value = false
      await saveAllShortcutConfigs()
    } else {
      try {
        console.log('[toggleRecordingKey] Suspension du raccourci:', bindingId)
        await invoke('suspend_binding', { id: bindingId })
        recordingBindingId.value = bindingId
        recordingKey.value = true
      } catch (error) {
        console.error('Erreur lors de la suspension du raccourci:', error)
      }
    }
  }

  async function toggleRecordingOCRKey(bindingId: string) {
    if (recordingOCRKey.value) {
      recordingOCRKey.value = false
      await saveAllShortcutConfigs()
    } else {
      try {
        console.log('[toggleRecordingOCRKey] Suspension du raccourci:', bindingId)
        await invoke('suspend_binding', { id: bindingId })
        recordingOCRBindingId.value = bindingId
        recordingOCRKey.value = true
      } catch (error) {
        console.error('Erreur lors de la suspension du raccourci OCR:', error)
      }
    }
  }

  async function resumeBindingAfterRecording(bindingId: string | null) {
    if (bindingId) {
      try {
        await invoke('resume_binding', { id: bindingId })
      } catch (error) {
        console.error('Erreur lors de la reprise du raccourci:', error)
      }
    }
  }

  return {
    shortcutConfig,
    ocrShortcutConfig,
    recordingKey,
    recordingBindingId,
    recordingOCRKey,
    recordingOCRBindingId,
    loadShortcutConfig,
    loadOCRShortcutConfig,
    saveAllShortcutConfigs,
    resetShortcutsToDefault,
    toggleRecordingKey,
    toggleRecordingOCRKey,
    resumeBindingAfterRecording,
  }
}
