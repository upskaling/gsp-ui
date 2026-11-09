import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

/**
 * Gestion de l'autorisation macOS « Accessibilité ».
 *
 * Cette permission est nécessaire pour simuler Cmd+C et lire le texte
 * sélectionné. Depuis macOS 27, elle est présentée dans Réglages Système >
 * Confidentialité et sécurité > « Contrôle des appareils et accès aux
 * données » (anciennement « Accessibilité »).
 */
export function usePermissions() {
  const accessibilityGranted = ref(true)
  const showPermissionDialog = ref(false)

  async function refreshAccessibilityPermission(): Promise<boolean> {
    try {
      accessibilityGranted.value = await invoke<boolean>('check_accessibility_permission')
    } catch (error) {
      console.error('[permissions] Erreur lors de la vérification de la permission:', error)
    }

    return accessibilityGranted.value
  }

  async function requestAccessibilityPermission(): Promise<boolean> {
    try {
      accessibilityGranted.value = await invoke<boolean>('request_accessibility_permission')
    } catch (error) {
      console.error('[permissions] Erreur lors de la demande de la permission:', error)
    }

    return accessibilityGranted.value
  }

  /**
   * À appeler avant toute lecture du presse-papiers : vérifie la permission,
   * affiche la fenêtre et le dialogue d'aide si elle manque.
   */
  async function ensureAccessibility(): Promise<boolean> {
    if (await refreshAccessibilityPermission()) {
      return true
    }

    showPermissionDialog.value = true

    // L'application tourne souvent en arrière-plan (démarrage masqué) : sans
    // cela, l'utilisateur ne verrait aucun retour après le raccourci.
    try {
      await invoke('show_main_window')
    } catch (error) {
      console.error('[permissions] Impossible d’afficher la fenêtre principale:', error)
    }

    return await requestAccessibilityPermission()
  }

  async function openAccessibilitySettings() {
    try {
      await invoke('open_accessibility_settings')
    } catch (error) {
      console.error('[permissions] Impossible d’ouvrir les Réglages Système:', error)
    }
  }

  async function retryAccessibilityPermission(): Promise<boolean> {
    const granted = await requestAccessibilityPermission()

    if (granted) {
      showPermissionDialog.value = false
    }

    return granted
  }

  function dismissPermissionDialog() {
    showPermissionDialog.value = false
  }

  return {
    accessibilityGranted,
    showPermissionDialog,
    refreshAccessibilityPermission,
    requestAccessibilityPermission,
    ensureAccessibility,
    openAccessibilitySettings,
    retryAccessibilityPermission,
    dismissPermissionDialog,
  }
}
