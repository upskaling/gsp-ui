<script setup lang="ts">
import { computed } from 'vue'
import type { LanguageOption } from '@/composables/useLanguage'
import type { TtsModel } from '@/types'
import SelectField from './SelectField.vue'

interface Props {
  isSpeaking: boolean
  playbackSpeed: number
  speedOptions: number[]
  volume: number
  sourceLanguage: string
  ttsModels: TtsModel[]
  selectedTtsModel: string
  sourceLanguageOptions: LanguageOption[]
  ipaEnabled: boolean
}

interface Emits {
  (e: 'speak'): void
  (e: 'speak-ocr'): void
  (e: 'stop'): void
  (e: 'speed-change', speed: number): void
  (e: 'volume-change', volume: number): void
  (e: 'source-language-change', lang: string): void
  (e: 'tts-model-change', modelId: string): void
  (e: 'ipa-enabled-change', enabled: boolean): void
}

const props = withDefaults(defineProps<Props>(), {})
defineEmits<Emits>()

const speedSelectOptions = computed(() =>
  props.speedOptions.map((speed) => ({ value: String(speed), label: `${speed}x` })),
)

const languageSelectOptions = computed(() =>
  props.sourceLanguageOptions.map((lang) => ({ value: lang.code, label: lang.label })),
)

const modelSelectOptions = computed(() =>
  props.ttsModels.filter((model) => model.installed).map((model) => ({ value: model.id, label: model.label })),
)
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
        <SelectField
          id="speed-select"
          :model-value="String(playbackSpeed)"
          :options="speedSelectOptions"
          :disabled="isSpeaking"
          class="min-w-24"
          @update:model-value="(value) => $emit('speed-change', parseFloat(value))"
        />
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
        <SelectField
          id="source-lang"
          :model-value="sourceLanguage"
          :options="languageSelectOptions"
          :disabled="isSpeaking"
          class="min-w-32"
          @update:model-value="(value) => $emit('source-language-change', value)"
        />
      </div>

      <!-- TTS Model selection -->
      <div class="flex items-center gap-2">
        <label for="tts-model" class="text-sm font-medium">Parler:</label>
        <SelectField
          id="tts-model"
          :model-value="selectedTtsModel"
          :options="modelSelectOptions"
          :disabled="isSpeaking || modelSelectOptions.length === 0"
          placeholder="Aucun modèle installé"
          class="max-w-64"
          @update:model-value="(value) => $emit('tts-model-change', value)"
        />
      </div>
    </div>

    <!-- International Phonetic Alphabet support -->
    <label class="flex items-start gap-2 cursor-pointer text-sm">
      <input
        type="checkbox"
        :checked="ipaEnabled"
        @change="$emit('ipa-enabled-change', ($event.target as HTMLInputElement).checked)"
        class="w-4 h-4 mt-0.5"
      />
      <span>
        🔤 Prononcer l'alphabet phonétique international (API)
        <span class="block text-xs text-mid-gray">
          Les transcriptions comme <code>/bɔ̃ʒuʁ/</code> ou <code>[həˈloʊ]</code> sont lues à voix
          haute.
        </span>
      </span>
    </label>

  </div>
</template>
