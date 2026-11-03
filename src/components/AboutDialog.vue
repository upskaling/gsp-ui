<script setup lang="ts">
interface Props {
  version: string
}

withDefaults(defineProps<Props>(), {})
</script>

<template>
  <div class="p-6 border border-mid-gray/20 rounded-lg bg-background-ui/5">
    <div class="flex items-center gap-3 mb-4">
      <img src="/logo.svg" alt="Logo gsp-ui" class="w-12 h-12 rounded-xl shrink-0" />
      <div>
        <p class="text-xl font-bold leading-tight">gsp-ui</p>
        <p class="text-sm text-mid-gray">Version {{ version }}</p>
      </div>
    </div>

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
