import { getKeyFromCode } from '@/utils/keyMapping'
import type { useShortcuts } from './useShortcuts'

type ShortcutsReturn = ReturnType<typeof useShortcuts>

export function useKeyRecording(shortcuts: ShortcutsReturn) {
  const heldModifiers = {
    ctrl: false,
    shift: false,
    alt: false,
    meta: false,
  }

  const handleKeydown = async (event: KeyboardEvent) => {
    if (event.key === 'Control') heldModifiers.ctrl = event.ctrlKey
    if (event.key === 'Shift') heldModifiers.shift = event.shiftKey
    if (event.key === 'Alt') heldModifiers.alt = event.altKey
    if (event.key === 'Meta') heldModifiers.meta = event.metaKey

    if (!shortcuts.recordingKey.value && !shortcuts.recordingOCRKey.value) {
      const { ctrl, shift, alt, meta, key } = shortcuts.shortcutConfig.value
      const keyMatches =
        ctrl === event.ctrlKey &&
        shift === event.shiftKey &&
        alt === event.altKey &&
        meta === event.metaKey &&
        key === getKeyFromCode(event.code).toLowerCase()

      if (keyMatches) {
        event.preventDefault()
      }
    }
  }

  const handleKeyup = async (event: KeyboardEvent) => {
    if (event.key === 'Control') heldModifiers.ctrl = event.ctrlKey
    if (event.key === 'Shift') heldModifiers.shift = event.shiftKey
    if (event.key === 'Alt') heldModifiers.alt = event.altKey
    if (event.key === 'Meta') heldModifiers.meta = event.metaKey

    if (shortcuts.recordingKey.value) {
      event.preventDefault()

      if (['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) {
        return
      }

      const keyCode = getKeyFromCode(event.code)
      shortcuts.shortcutConfig.value.key = keyCode
      shortcuts.recordingKey.value = false

      if (shortcuts.recordingBindingId.value) {
        await shortcuts.resumeBindingAfterRecording(shortcuts.recordingBindingId.value)
        shortcuts.recordingBindingId.value = null
      }
      return
    }

    if (shortcuts.recordingOCRKey.value) {
      event.preventDefault()

      if (['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) {
        return
      }

      const keyCode = getKeyFromCode(event.code)
      shortcuts.ocrShortcutConfig.value.key = keyCode
      shortcuts.recordingOCRKey.value = false

      if (shortcuts.recordingOCRBindingId.value) {
        await shortcuts.resumeBindingAfterRecording(shortcuts.recordingOCRBindingId.value)
        shortcuts.recordingOCRBindingId.value = null
      }
      return
    }
  }

  return {
    handleKeydown,
    handleKeyup,
  }
}
