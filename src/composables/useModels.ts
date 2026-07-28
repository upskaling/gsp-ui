import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Model } from '@/types'

export function useModels() {
  const models = ref<Model[]>([])
  const loadingModels = ref(false)

  async function loadModels() {
    loadingModels.value = true
    try {
      console.log('[loadModels] Chargement de la liste des modèles')
      const modelList = await invoke('list_models')
      models.value = modelList as Model[]
      console.log('[loadModels] Modèles chargés:', models.value)
    } catch (error) {
      console.error('[loadModels] Erreur:', error)
      alert(`Erreur lors du chargement des modèles: ${error}`)
    } finally {
      loadingModels.value = false
    }
  }

  async function deleteModel(modelName: string) {
    if (!confirm(`Êtes-vous sûr de vouloir supprimer le modèle "${modelName}" ?`)) {
      return
    }

    try {
      console.log('[deleteModel] Suppression du modèle:', modelName)
      await invoke('delete_model', { modelName })
      console.log('[deleteModel] Modèle supprimé')
      await loadModels()
    } catch (error) {
      console.error('[deleteModel] Erreur:', error)
      alert(`Erreur lors de la suppression: ${error}`)
    }
  }

  return {
    models,
    loadingModels,
    loadModels,
    deleteModel,
  }
}
