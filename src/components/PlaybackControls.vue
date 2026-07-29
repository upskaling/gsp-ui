<script setup lang="ts">
import type { LanguageOption } from '@/composables/useLanguage'
import type { TtsModel } from '@/types'

interface Props {
  isSpeaking: boolean
  playbackSpeed: number
  speedOptions: number[]
  volume: number
  sourceLanguage: string
  ttsModels: TtsModel[]
  selectedTtsModel: string
  sourceLanguageOptions: LanguageOption[]
}

interface Emits {
  (e: 'speak'): void
  (e: 'speak-ocr'): void
  (e: 'stop'): void
  (e: 'speed-change', speed: number): void
  (e: 'volume-change', volume: number): void
  (e: 'source-language-change', lang: string): void
  (e: 'tts-model-change', modelId: string): void
}

withDefaults(defineProps<Props>(), {})
defineEmits<Emits>()
</script>

<template>
  <div class="flex flex-col gap-4">
    <!-- Main action buttons -->
    <div class="flex gap-2 flex-wrap">
      <button
        @click="$emit('speak')"
        :disabled="isSpeaking"
        class="flex items-center gap-2 px-4 py-2 bg-background-ui text-white border-2 border-background-ui rounded-lg hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition font-medium"
      >
        <span>🔊</span>
        {{ isSpeaking ? 'Lecture en cours...' : 'Lire' }}
      </button>

      <button
        @click="$emit('speak-ocr')"
        :disabled="isSpeaking"
        class="flex items-center gap-2 px-4 py-2 bg-background-ui text-white border-2 border-background-ui rounded-lg hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition font-medium"
      >
        <span>📸</span>
        {{ isSpeaking ? 'Capture en cours...' : 'OCR' }}
      </button>

      <button
        v-if="isSpeaking"
        @click="$emit('stop')"
        class="flex items-center gap-2 px-4 py-2 bg-red-500 text-white border-2 border-red-500 rounded-lg hover:bg-red-600 hover:border-red-600 transition font-medium"
      >
        <span>⏹️</span>
        Arrêter
      </button>
    </div>

    <!-- Controls row -->
    <div class="flex gap-4 flex-wrap items-center">
      <!-- Speed control -->
      <div class="flex items-center gap-2">
        <label for="speed-select" class="text-sm font-medium">Vitesse:</label>
        <select
          id="speed-select"
          :value="playbackSpeed"
          @change="(e) => $emit('speed-change', parseFloat((e.target as HTMLSelectElement).value))"
          :disabled="isSpeaking"
          class="px-3 py-1 rounded border border-mid-gray/20 bg-background text-text disabled:opacity-50"
        >
          <option v-for="speed in speedOptions" :key="speed" :value="speed">
            {{ speed }}x
          </option>
        </select>
      </div>

      <!-- Volume control -->
      <div class="flex items-center gap-2">
        <label for="volume-slider" class="text-sm font-medium">🔊 Volume:</label>
        <input
          id="volume-slider"
          type="range"
          min="0"
          max="1"
          step="0.1"
          :value="volume"
          @input="(e) => $emit('volume-change', parseFloat((e.target as HTMLInputElement).value))"
          class="w-24"
        />
        <span class="text-sm text-mid-gray w-12">{{ Math.round(volume * 100) }}%</span>
      </div>

      <!-- Source language -->
      <div class="flex items-center gap-2">
        <label for="source-lang" class="text-sm font-medium">Source:</label>
        <select
          id="source-lang"
          :value="sourceLanguage"
          @change="(e) => $emit('source-language-change', (e.target as HTMLSelectElement).value)"
          :disabled="isSpeaking"
          class="px-3 py-1 rounded border border-mid-gray/20 bg-background text-text disabled:opacity-50"
        >
          <option v-for="lang in sourceLanguageOptions" :key="lang.code" :value="lang.code">
            {{ lang.label }}
          </option>
        </select>
      </div>

      <!-- TTS Model selection -->
      <div class="flex items-center gap-2">
        <label for="tts-model" class="text-sm font-medium">Parler:</label>
        <select
          id="tts-model"
          :value="selectedTtsModel"
          @change="(e) => $emit('tts-model-change', (e.target as HTMLSelectElement).value)"
          :disabled="isSpeaking || ttsModels.length === 0"
          class="px-3 py-1 rounded border border-mid-gray/20 bg-background text-text disabled:opacity-50"
        >
          <option v-for="model in ttsModels.filter((m) => m.installed)" :key="model.id" :value="model.id">
            {{ model.label }}
          </option>
          <option v-if="ttsModels.filter((m) => m.installed).length === 0" disabled>
            Aucun modèle installé
          </option>
        </select>
      </div>
    </div>

  </div>
</template>
