<script setup lang="ts">
import type { ClipboardShortcut } from '@/types'

interface Props {
  title: string
  shortcutConfig: ClipboardShortcut
  recordingKey: boolean
  bindingId: string
}

interface Emits {
  (e: 'update:shortcutConfig', config: ClipboardShortcut): void
  (e: 'toggle-recording', bindingId: string): void
  (e: 'reset-to-default'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const updateModifier = (key: 'ctrl' | 'shift' | 'alt' | 'meta') => {
  emit('update:shortcutConfig', {
    ...props.shortcutConfig,
    [key]: !props.shortcutConfig[key],
  })
}
</script>

<template>
  <div class="p-4 border border-mid-gray/20 rounded-lg bg-background-ui/5">
    <h3 class="font-semibold mb-4">{{ title }}</h3>

    <!-- Modifiers grid -->
    <div class="grid grid-cols-4 gap-2 mb-4">
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          :checked="shortcutConfig.ctrl"
          @change="updateModifier('ctrl')"
          class="w-4 h-4"
        />
        <span class="text-sm">Ctrl</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          :checked="shortcutConfig.shift"
          @change="updateModifier('shift')"
          class="w-4 h-4"
        />
        <span class="text-sm">Shift</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          :checked="shortcutConfig.alt"
          @change="updateModifier('alt')"
          class="w-4 h-4"
        />
        <span class="text-sm">Alt</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          :checked="shortcutConfig.meta"
          @change="updateModifier('meta')"
          class="w-4 h-4"
        />
        <span class="text-sm">Super</span>
      </label>
    </div>

    <!-- Key recording button -->
    <button
      @click="emit('toggle-recording', bindingId)"
      :class="{ 'bg-background-ui text-white': recordingKey }"
      class="w-full px-4 py-2 rounded border border-mid-gray/20 hover:bg-mid-gray/10 transition text-sm font-medium"
    >
      {{
        recordingKey
          ? '🎹 Appuyez sur une touche...'
          : `⌨️ Touche: ${shortcutConfig.key.toUpperCase()}`
      }}
    </button>

    <!-- Reset button (optional, shown on parent level) -->
  </div>
</template>
