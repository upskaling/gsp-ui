<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

export interface SelectOption {
  value: string
  label: string
  disabled?: boolean
}

interface Props {
  id: string
  modelValue: string
  options: SelectOption[]
  disabled?: boolean
  placeholder?: string
}

const props = withDefaults(defineProps<Props>(), {
  disabled: false,
  placeholder: 'Sélectionner…',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const rootEl = ref<HTMLElement | null>(null)
const triggerEl = ref<HTMLButtonElement | null>(null)
const listEl = ref<HTMLUListElement | null>(null)
const isOpen = ref(false)
const highlightedIndex = ref(-1)

const listboxId = computed(() => `${props.id}-listbox`)
const selectedIndex = computed(() => props.options.findIndex((option) => option.value === props.modelValue))
const selectedOption = computed(() => props.options.find((option) => option.value === props.modelValue))
const displayLabel = computed(() => selectedOption.value?.label ?? props.placeholder)
const activeDescendant = computed(() =>
  isOpen.value && highlightedIndex.value >= 0 ? optionId(highlightedIndex.value) : undefined,
)

function optionId(index: number): string {
  return `${props.id}-option-${index}`
}

function isEnabled(index: number): boolean {
  return index >= 0 && index < props.options.length && !props.options[index].disabled
}

function firstEnabledIndex(): number {
  return props.options.findIndex((option) => !option.disabled)
}

function lastEnabledIndex(): number {
  for (let index = props.options.length - 1; index >= 0; index--) {
    if (!props.options[index].disabled) return index
  }
  return -1
}

function scrollHighlightedIntoView(): void {
  const list = listEl.value
  if (!list) return
  const item = list.children[highlightedIndex.value] as HTMLElement | undefined
  item?.scrollIntoView({ block: 'nearest' })
}

function openMenu(): void {
  if (props.disabled || isOpen.value) return
  isOpen.value = true
  highlightedIndex.value = selectedIndex.value >= 0 ? selectedIndex.value : firstEnabledIndex()
  void nextTick(scrollHighlightedIntoView)
}

function closeMenu(restoreFocus = false): void {
  if (!isOpen.value) return
  isOpen.value = false
  highlightedIndex.value = -1
  if (restoreFocus) triggerEl.value?.focus()
}

function toggleMenu(): void {
  if (isOpen.value) {
    closeMenu()
  } else {
    openMenu()
  }
}

function selectOption(option: SelectOption): void {
  if (option.disabled) return
  emit('update:modelValue', option.value)
  closeMenu(true)
}

function moveHighlight(direction: 1 | -1): void {
  const count = props.options.length
  if (count === 0) return

  let index = highlightedIndex.value
  for (let step = 0; step < count; step++) {
    index = (index + direction + count) % count
    if (!props.options[index].disabled) {
      highlightedIndex.value = index
      break
    }
  }
  void nextTick(scrollHighlightedIntoView)
}

function handleKeydown(event: KeyboardEvent): void {
  if (props.disabled) return

  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      if (isOpen.value) {
        moveHighlight(1)
      } else {
        openMenu()
      }
      break
    case 'ArrowUp':
      event.preventDefault()
      if (isOpen.value) {
        moveHighlight(-1)
      } else {
        openMenu()
      }
      break
    case 'Home':
      if (!isOpen.value) return
      event.preventDefault()
      highlightedIndex.value = firstEnabledIndex()
      void nextTick(scrollHighlightedIntoView)
      break
    case 'End':
      if (!isOpen.value) return
      event.preventDefault()
      highlightedIndex.value = lastEnabledIndex()
      void nextTick(scrollHighlightedIntoView)
      break
    case 'Enter':
    case ' ':
      event.preventDefault()
      if (!isOpen.value) {
        openMenu()
      } else if (isEnabled(highlightedIndex.value)) {
        selectOption(props.options[highlightedIndex.value])
      }
      break
    case 'Escape':
      if (!isOpen.value) return
      event.preventDefault()
      closeMenu(true)
      break
    case 'Tab':
      closeMenu()
      break
  }
}

function handlePointerDown(event: PointerEvent): void {
  if (!isOpen.value || rootEl.value?.contains(event.target as Node)) return
  closeMenu()
}

onMounted(() => {
  document.addEventListener('pointerdown', handlePointerDown, true)
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', handlePointerDown, true)
})
</script>

<template>
  <div ref="rootEl" class="select-wrapper relative inline-flex items-center" @keydown="handleKeydown">
    <button
      :id="id"
      ref="triggerEl"
      type="button"
      role="combobox"
      aria-haspopup="listbox"
      :aria-expanded="isOpen"
      :aria-controls="listboxId"
      :aria-activedescendant="activeDescendant"
      :disabled="disabled"
      class="select-field select-trigger"
      @click="toggleMenu"
    >
      <span class="select-trigger-label" :class="{ 'select-trigger-label--placeholder': !selectedOption }">
        {{ displayLabel }}
      </span>
    </button>

    <span class="select-chevron" :class="{ 'select-chevron--open': isOpen }" aria-hidden="true">
      <svg
        class="w-4 h-4"
        viewBox="0 0 20 20"
        fill="none"
        stroke="currentColor"
        stroke-width="1.75"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M6 8l4 4 4-4" />
      </svg>
    </span>

    <ul
      v-if="isOpen"
      :id="listboxId"
      ref="listEl"
      class="select-panel"
      role="listbox"
      :aria-labelledby="id"
      tabindex="-1"
    >
      <li
        v-for="(option, index) in options"
        :id="optionId(index)"
        :key="option.value"
        role="option"
        class="select-option"
        :aria-selected="option.value === modelValue"
        :aria-disabled="option.disabled || undefined"
        :data-highlighted="index === highlightedIndex"
        :data-selected="option.value === modelValue"
        @mousedown.prevent
        @mousemove="!option.disabled && (highlightedIndex = index)"
        @click="selectOption(option)"
      >
        <span class="select-option-label">{{ option.label }}</span>
        <span v-if="option.value === modelValue" class="select-option-check" aria-hidden="true">✓</span>
      </li>
    </ul>
  </div>
</template>
