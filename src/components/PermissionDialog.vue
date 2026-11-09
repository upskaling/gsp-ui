<script setup lang="ts">
import { ref } from 'vue'

interface Emits {
  (e: 'close'): void
  (e: 'open-settings'): void
  (e: 'retry'): void
}

const emit = defineEmits<Emits>()

const retrying = ref(false)

function retry() {
  retrying.value = true
  emit('retry')
  // Le retour visuel évite de laisser le bouton bloqué si la permission
  // n'est pas encore accordée.
  setTimeout(() => {
    retrying.value = false
  }, 1000)
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
    <div class="w-full max-w-lg rounded-lg border border-mid-gray/30 bg-background p-6 shadow-xl">
      <div class="mb-3 flex items-center gap-3">
        <span class="text-2xl">🔐</span>
        <h2 class="text-xl font-bold">Autorisation macOS requise</h2>
      </div>

      <p class="mb-4 text-sm leading-relaxed">
        gsp-ui doit pouvoir simuler <kbd class="rounded bg-mid-gray/20 px-1">⌘ C</kbd> pour lire le
        texte sélectionné. Sans cette autorisation, le raccourci global se déclenche mais rien n'est
        lu.
      </p>

      <ol class="mb-4 list-decimal space-y-2 pl-5 text-sm leading-relaxed">
        <li>
          Ouvrez <strong>Réglages Système</strong> → <strong>Confidentialité et sécurité</strong>.
        </li>
        <li>
          Ouvrez la section
          <strong>« Contrôle des appareils et accès aux données »</strong> sur macOS 27 et plus,
          ou <strong>« Accessibilité »</strong> sur les versions précédentes.
        </li>
        <li>Activez l'interrupteur de <strong>gsp-ui</strong>.</li>
        <li>Quittez puis relancez gsp-ui pour que l'autorisation soit prise en compte.</li>
      </ol>

      <p class="mb-6 rounded bg-mid-gray/10 p-3 text-xs leading-relaxed text-mid-gray">
        Cette version de l'application n'est pas signée avec un certificat Apple. macOS peut donc
        redemander l'autorisation après chaque mise à jour. En attendant, si gsp-ui n'apparaît pas
        dans la liste, ajoutez-la avec le bouton « + » et choisissez
        <code>/Applications/gsp-ui.app</code>.
      </p>

      <div class="flex flex-wrap justify-end gap-2">
        <button
          @click="$emit('close')"
          class="rounded px-4 py-2 font-medium text-mid-gray transition hover:bg-mid-gray/20"
        >
          Plus tard
        </button>
        <button
          @click="retry"
          :disabled="retrying"
          class="rounded bg-mid-gray/20 px-4 py-2 font-medium transition hover:bg-mid-gray/40 disabled:opacity-50"
        >
          {{ retrying ? 'Vérification…' : 'Réessayer' }}
        </button>
        <button
          @click="$emit('open-settings')"
          class="rounded bg-background-ui px-4 py-2 font-medium text-white transition hover:opacity-90"
        >
          Ouvrir les Réglages Système
        </button>
      </div>
    </div>
  </div>
</template>
