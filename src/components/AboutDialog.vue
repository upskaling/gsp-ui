<script setup lang="ts">
interface Props {
  version: string
}

interface Emits {
  (e: 'close'): void
}

withDefaults(defineProps<Props>(), {})
defineEmits<Emits>()
</script>

<template>
  <div class="p-6 border border-mid-gray/20 rounded-lg bg-background-ui/5">
    <h2 class="text-2xl font-bold mb-4">À propos de gsp-ui</h2>

    <p class="text-lg font-semibold text-background-ui mb-2">Version {{ version }}</p>

    <p class="text-text mb-4">
      Une application de lecture d'écran pour les utilisateurs dyslexiques.
    </p>

    <p class="mb-6">
      <a
        href="https://github.com/upskaling/gsp-ui"
        target="_blank"
        rel="noopener noreferrer"
        class="text-background-ui hover:underline flex items-center gap-2"
      >
        🔗 Consulter le code source sur GitHub
      </a>
    </p>

    <button
      @click="$emit('close')"
      class="px-6 py-2 rounded bg-background-ui text-white font-medium hover:opacity-90 transition"
    >
      ✕ Fermer
    </button>
  </div>
</template>
