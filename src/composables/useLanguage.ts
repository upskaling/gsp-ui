import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface LanguageOption {
  code: string
  label: string
}

export function useLanguage() {
  const sourceLanguage = ref('auto')
  const targetLanguage = ref('fr')

  const sourceLanguageOptions: LanguageOption[] = [
    { code: 'auto', label: 'Détection automatique' },
    { code: 'fr', label: 'Français' },
    { code: 'en', label: 'English' },
  ]

  const targetLanguageOptions: LanguageOption[] = [
    { code: 'fr', label: 'Français' },
    { code: 'en', label: 'English' },
  ]

  async function loadSourceLanguage() {
    try {
      sourceLanguage.value = await invoke('load_source_language')
      console.log('[loadSourceLanguage] Langue source chargée:', sourceLanguage.value)
    } catch (error) {
      console.error('Erreur lors du chargement de la langue source:', error)
    }
  }

  async function setSourceLanguage(lang: string) {
    console.log('[setSourceLanguage] Nouvelle langue source:', lang)
    sourceLanguage.value = lang
    try {
      await invoke('save_source_language', { language: lang })
      console.log('[setSourceLanguage] Langue source sauvegardée')
    } catch (error) {
      console.error('Erreur lors de la sauvegarde de la langue source:', error)
    }
  }

  async function loadTargetLanguage() {
    try {
      targetLanguage.value = await invoke('load_target_language')
      console.log('[loadTargetLanguage] Langue cible chargée:', targetLanguage.value)
    } catch (error) {
      console.error('Erreur lors du chargement de la langue cible:', error)
    }
  }

  async function setTargetLanguage(lang: string) {
    console.log('[setTargetLanguage] Nouvelle langue cible:', lang)
    targetLanguage.value = lang
    try {
      await invoke('save_target_language', { language: lang })
      console.log('[setTargetLanguage] Langue cible sauvegardée')
    } catch (error) {
      console.error('Erreur lors de la sauvegarde de la langue cible:', error)
    }
  }

  return {
    sourceLanguage,
    targetLanguage,
    sourceLanguageOptions,
    targetLanguageOptions,
    loadSourceLanguage,
    setSourceLanguage,
    loadTargetLanguage,
    setTargetLanguage,
  }
}
