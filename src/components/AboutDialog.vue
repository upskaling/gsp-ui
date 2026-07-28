<script setup lang="ts">
interface Props {
  version: string
}

withDefaults(defineProps<Props>(), {})
</script>

<template>
  <div class="p-6 border border-mid-gray/20 rounded-lg bg-background-ui/5">
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
  </div>
</template>
