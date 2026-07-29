export interface ClipboardShortcut {
  ctrl: boolean
  shift: boolean
  alt: boolean
  meta: boolean
  key: string
}

export interface Model {
  name: string
  size_mb: number
}

export interface PiperModel {
  name: string
  size_mb: number
}

export interface AvailablePiperModel {
  id: string
  name: string
  language: string
  quality: string
}
