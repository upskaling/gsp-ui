import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { TtsModel } from '@/types'

export function useTtsModels() {
  const selectedTtsModel = ref<string>('piper:fr_FR-tom-medium')
  const availableTtsModels = ref<TtsModel[]>([])
  const loadingModels = ref(false)

  async function loadAvailableTtsModels() {
    loadingModels.value = true
    try {
      const models = await invoke<TtsModel[]>('get_available_tts_models')
      availableTtsModels.value = models
      console.log('[useTtsModels] Modèles TTS disponibles:', models)
    } catch (error) {
      console.error('[useTtsModels] Erreur lors du chargement des modèles:', error)
    } finally {
      loadingModels.value = false
    }
  }

  async function loadSelectedTtsModel() {
    try {
      const modelId = await invoke<string>('load_tts_model_selection')
      selectedTtsModel.value = modelId
      console.log('[useTtsModels] Modèle sélectionné chargé:', modelId)
    } catch (error) {
      console.error('[useTtsModels] Erreur lors du chargement du modèle sélectionné:', error)
    }
  }

  async function setTtsModel(modelId: string) {
    console.log('[useTtsModels] Changement du modèle TTS:', modelId)
    selectedTtsModel.value = modelId

    try {
      await invoke('save_tts_model_selection', { modelId })
      console.log('[useTtsModels] Modèle TTS sauvegardé')
    } catch (error) {
      console.error('[useTtsModels] Erreur lors de la sauvegarde du modèle:', error)
    }
  }

  const getModelLabel = (modelId: string): string => {
    const model = availableTtsModels.value.find((m) => m.id === modelId)
    return model?.label ?? modelId
  }

  return {
    selectedTtsModel,
    availableTtsModels,
    loadingModels,
    loadAvailableTtsModels,
    loadSelectedTtsModel,
    setTtsModel,
    getModelLabel,
  }
}
