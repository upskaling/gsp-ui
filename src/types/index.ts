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
