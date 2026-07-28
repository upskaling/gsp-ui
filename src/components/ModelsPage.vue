<script setup lang="ts">
import type { Model } from '@/types'

interface Props {
  models: Model[]
  loading: boolean
}

interface Emits {
  (e: 'delete', modelName: string): void
  (e: 'refresh'): void
}

withDefaults(defineProps<Props>(), {})
defineEmits<Emits>()
</script>

<template>
  <div class="p-4 border border-mid-gray/20 rounded-lg">
    <h3 class="font-semibold mb-4">Modèles de traduction</h3>

    <!-- Loading state -->
    <div v-if="loading" class="text-center py-8 text-mid-gray">
      ⏳ Chargement des modèles...
    </div>

    <!-- Empty state -->
    <div v-else-if="models.length === 0" class="text-center py-8 text-mid-gray">
      <p>Aucun modèle installé</p>
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
                class="px-3 py-1 text-sm text-red-500 hover:bg-red-500/10 rounded transition"
              >
                🗑️ Supprimer
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Refresh button -->
    <div class="mt-4 flex justify-end">
      <button
        @click="$emit('refresh')"
        :disabled="loading"
        class="px-4 py-2 text-sm rounded bg-mid-gray/10 hover:bg-background-ui hover:text-white disabled:opacity-50 transition font-medium"
      >
        🔄 Actualiser
      </button>
    </div>
  </div>
</template>
