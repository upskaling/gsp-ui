import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AvailablePiperModel, Model, PiperModel } from '@/types'

export function useModels() {
  const models = ref<Model[]>([])
  const piperModels = ref<PiperModel[]>([])
  const loadingModels = ref(false)
  const downloadingModel = ref<string | null>(null)

  const availablePiperModels: AvailablePiperModel[] = [
    { id: 'fr_FR-tom-medium', name: 'Tom (Français)', language: 'Français', quality: 'Medium' },
    { id: 'en_US-lessac-medium', name: 'Lessac (Anglais)', language: 'Anglais', quality: 'Medium' },
  ]

  async function loadModels() {
    loadingModels.value = true
    try {
      console.log('[loadModels] Chargement des modèles Bergamot et Piper')
      const [modelList, piperList] = await Promise.all([
        invoke('list_models') as Promise<Model[]>,
        invoke('list_piper_models') as Promise<PiperModel[]>
      ])
      models.value = modelList
      piperModels.value = piperList
      console.log('[loadModels] Modèles Bergamot chargés:', models.value)
      console.log('[loadModels] Modèles Piper chargés:', piperModels.value)
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

  async function deletePiperModel(modelName: string) {
    if (!confirm(`Êtes-vous sûr de vouloir supprimer le modèle Piper "${modelName}" ?`)) {
      return
    }

    try {
      console.log('[deletePiperModel] Suppression du modèle Piper:', modelName)
      await invoke('delete_piper_model', { modelName })
      console.log('[deletePiperModel] Modèle Piper supprimé')
      await loadModels()
    } catch (error) {
      console.error('[deletePiperModel] Erreur:', error)
      alert(`Erreur lors de la suppression: ${error}`)
    }
  }

  async function downloadPiperModel(modelId: string) {
    downloadingModel.value = modelId

    try {
      console.log('[downloadPiperModel] Téléchargement du modèle:', modelId)
      await invoke('download_piper_model', { modelName: modelId })
      console.log('[downloadPiperModel] Modèle téléchargé avec succès')
      await loadModels()
    } catch (error) {
      console.error('[downloadPiperModel] Erreur:', error)
      alert(`Erreur lors du téléchargement: ${error}`)
    } finally {
      downloadingModel.value = null
    }
  }

  return {
    models,
    piperModels,
    loadingModels,
    downloadingModel,
    availablePiperModels,
    loadModels,
    deleteModel,
    deletePiperModel,
    downloadPiperModel,
  }
}
