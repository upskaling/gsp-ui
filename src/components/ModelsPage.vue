<script setup lang="ts">
import { ref } from 'vue'
import type { AvailablePiperModel, Model, PiperModel, TtsModel } from '@/types'

type TabType = 'tts' | 'translation'

interface Props {
  models: Model[]
  piperModels: PiperModel[]
  loading: boolean
  availablePiperModels: AvailablePiperModel[]
  downloadingModel: string | null
  availableTtsModels?: TtsModel[]
  selectedTtsModel?: string
}

interface Emits {
  (e: 'delete', modelName: string): void
  (e: 'delete-piper', modelName: string): void
  (e: 'download-piper', modelId: string): void
  (e: 'refresh'): void
  (e: 'tts-model-change', modelId: string): void
}

withDefaults(defineProps<Props>(), {
  availableTtsModels: () => [],
  selectedTtsModel: 'piper:fr_FR-tom-medium'
})
defineEmits<Emits>()

const activeTab = ref<TabType>('tts')

function isModelInstalled(modelId: string, installedModels: PiperModel[]) {
  return installedModels.some(m => m.name === modelId)
}
</script>

<template>
  <div class="space-y-6">
    <!-- Tabs -->
    <div class="flex gap-2 border-b border-mid-gray/20">
      <button
        @click="activeTab = 'tts'"
        :class="[
          'px-4 py-2 text-sm font-medium transition-colors',
          activeTab === 'tts'
            ? 'border-b-2 border-logo-primary text-logo-primary'
            : 'text-mid-gray hover:text-text/80'
        ]"
      >
        🔊 Modèles TTS
      </button>
      <button
        @click="activeTab = 'translation'"
        :class="[
          'px-4 py-2 text-sm font-medium transition-colors',
          activeTab === 'translation'
            ? 'border-b-2 border-logo-primary text-logo-primary'
            : 'text-mid-gray hover:text-text/80'
        ]"
      >
        🌐 Modèles de traduction
      </button>
    </div>

    <!-- TTS Models Tab -->
    <div v-if="activeTab === 'tts'" class="space-y-6">
      <!-- Sélection du moteur TTS -->
      <div class="p-4 border border-mid-gray/20 rounded-lg">
        <h3 class="font-semibold mb-4">Moteur TTS</h3>
        <div class="space-y-3">
          <div v-for="model in availableTtsModels" :key="model.id" class="flex items-center gap-3 p-3 border border-mid-gray/20 rounded cursor-pointer hover:bg-mid-gray/5 transition"
            :class="{ 'border-logo-primary/50 bg-logo-primary/5': selectedTtsModel === model.id }"
            @click="$emit('tts-model-change', model.id)">
            <input
              type="radio"
              :id="`tts-${model.id}`"
              :value="model.id"
              :checked="selectedTtsModel === model.id"
              @change="$emit('tts-model-change', model.id)"
              class="cursor-pointer"
            />
            <label :for="`tts-${model.id}`" class="flex-1 cursor-pointer">
              <p class="font-medium text-sm">{{ model.label }}</p>
              <p class="text-xs text-mid-gray">{{ model.engine }}</p>
              <p v-if="model.installed" class="text-xs text-green-500/80">✓ Installé</p>
              <p v-else class="text-xs text-mid-gray">Non disponible</p>
            </label>
          </div>
        </div>
      </div>

      <!-- Modèles TTS installés -->
      <div class="p-4 border border-mid-gray/20 rounded-lg">
        <div class="flex items-center justify-between mb-4">
          <h3 class="font-semibold">Modèles TTS installés</h3>
          <button
            @click="$emit('refresh')"
            :disabled="loading"
            class="px-2 py-1 text-xs rounded bg-mid-gray/10 hover:bg-mid-gray/20 disabled:opacity-50 transition"
            title="Actualiser la liste des modèles"
          >
            🔄
          </button>
        </div>

        <!-- Loading state -->
        <div v-if="loading" class="text-center py-8 text-mid-gray text-sm">
          ⏳ Chargement des modèles...
        </div>

        <!-- Empty state -->
        <div v-else-if="piperModels.length === 0" class="text-center py-8 text-mid-gray text-sm">
          <p>Aucun modèle TTS installé</p>
        </div>

        <!-- Installed models list -->
        <div v-else class="space-y-2">
          <div v-for="model in piperModels" :key="model.name" class="flex items-center justify-between p-3 bg-green-500/5 border border-green-500/20 rounded">
            <div class="flex-1">
              <p class="font-medium text-sm">{{ model.name }}</p>
              <p class="text-xs text-mid-gray">{{ model.size_mb.toFixed(2) }} MB</p>
            </div>
            <button
              @click="$emit('delete-piper', model.name)"
              class="px-3 py-1 text-xs text-red-500 hover:bg-red-500/10 rounded transition font-medium"
            >
              🗑️
            </button>
          </div>
        </div>
      </div>

      <!-- Modèles TTS disponibles -->
      <div v-if="availablePiperModels.some(m => !isModelInstalled(m.id, piperModels))" class="p-4 border border-mid-gray/20 rounded-lg">
        <h3 class="font-semibold mb-4">Modèles TTS disponibles</h3>

        <!-- Available models grid -->
        <div class="space-y-2">
          <div v-for="model in availablePiperModels.filter(m => !isModelInstalled(m.id, piperModels))" :key="model.id" class="flex items-center justify-between p-3 bg-mid-gray/5 border border-mid-gray/10 rounded">
            <div class="flex-1 min-w-0">
              <p class="font-medium text-sm">{{ model.name }}</p>
              <p class="text-xs text-mid-gray">{{ model.language }} • {{ model.quality }}</p>
            </div>
            <div class="flex items-center gap-2 flex-shrink-0">
              <button
                @click="$emit('download-piper', model.id)"
                :disabled="loading || downloadingModel === model.id"
                class="px-3 py-1 text-xs text-mid-gray hover:bg-mid-gray/10 hover:text-white disabled:opacity-50 rounded transition font-medium whitespace-nowrap"
              >
                {{ downloadingModel === model.id ? '⬇️ ...' : '⬇️' }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Translation Models Tab -->
    <div v-if="activeTab === 'translation'" class="p-4 border border-mid-gray/20 rounded-lg">
      <div class="flex items-center justify-between mb-4">
        <h3 class="font-semibold">Modèles de traduction</h3>
        <button
          @click="$emit('refresh')"
          :disabled="loading"
          class="px-2 py-1 text-xs rounded bg-mid-gray/10 hover:bg-mid-gray/20 disabled:opacity-50 transition"
          title="Actualiser la liste des modèles"
        >
          🔄
        </button>
      </div>

      <!-- Loading state -->
      <div v-if="loading" class="text-center py-8 text-mid-gray text-sm">
        ⏳ Chargement des modèles...
      </div>

      <!-- Empty state -->
      <div v-else-if="models.length === 0" class="text-center py-8 text-mid-gray text-sm">
        <p>Aucun modèle de traduction installé</p>
      </div>

      <!-- Models table -->
      <div v-else class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-mid-gray/20">
              <th class="text-left py-2 px-4">Nom</th>
              <th class="text-left py-2 px-4">Taille</th>
              <th class="text-left py-2 px-4">Action</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="model in models" :key="model.name" class="border-b border-mid-gray/10">
              <td class="py-3 px-4">{{ model.name }}</td>
              <td class="py-3 px-4">{{ model.size_mb.toFixed(2) }} MB</td>
              <td class="py-3 px-4">
                <button
                  @click="$emit('delete', model.name)"
                  class="px-3 py-1 text-xs text-red-500 hover:bg-red-500/10 rounded transition"
                >
                  🗑️ Supprimer
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
