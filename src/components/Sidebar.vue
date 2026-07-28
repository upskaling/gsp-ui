<script setup lang="ts">
import { computed } from 'vue'

export type SidebarSection = 'playback' | 'shortcuts' | 'models' | 'about'

interface SidebarProps {
  activeSection: SidebarSection
}

defineProps<SidebarProps>()
defineEmits<{
  'section-change': [section: SidebarSection]
}>()

const sections = computed(() => [
  {
    id: 'playback' as SidebarSection,
    label: 'Lecture',
    icon: '▶️',
  },
  {
    id: 'shortcuts' as SidebarSection,
    label: 'Raccourcis',
    icon: '⌨️',
  },
  {
    id: 'models' as SidebarSection,
    label: 'Modèles',
    icon: '🧠',
  },
  {
    id: 'about' as SidebarSection,
    label: 'À propos',
    icon: 'ℹ️',
  },
])
</script>

<template>
  <div class="flex flex-col w-48 h-screen border-r border-mid-gray/20 items-center px-2 py-4 bg-background">
    <!-- Logo -->
    <div class="mb-6 text-2xl font-bold">🎧 gsp-ui</div>

    <!-- Navigation sections -->
    <div class="flex flex-col w-full gap-1">
      <button
        v-for="section in sections"
        :key="section.id"
        :aria-current="activeSection === section.id"
        @click="$emit('section-change', section.id)"
        class="flex gap-3 items-center p-3 w-full rounded-lg cursor-pointer transition-colors text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500"
        :class="{
          'bg-background-ui/80 text-white font-medium': activeSection === section.id,
          'hover:bg-mid-gray/20 opacity-85 hover:opacity-100': activeSection !== section.id,
        }"
      >
        <span class="text-xl shrink-0">{{ section.icon }}</span>
        <span class="text-sm font-medium truncate">{{ section.label }}</span>
      </button>
    </div>
  </div>
</template>
