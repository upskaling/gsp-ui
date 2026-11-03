export function getKeyFromCode(code: string): string {
  if (code.startsWith('Digit')) {
    return code.replace('Digit', '')
  }
  if (code.startsWith('Key')) {
    return code.replace('Key', '').toLowerCase()
  }
  if (code.startsWith('Numpad')) {
    return code.replace('Numpad', 'numpad').toLowerCase()
  }

  const specialKeys: Record<string, string> = {
    Space: 'space',
    Enter: 'return',
    Tab: 'tab',
    Escape: 'escape',
    Backspace: 'backspace',
    Delete: 'delete',
    Insert: 'insert',
    Home: 'home',
    End: 'end',
    PageUp: 'pageup',
    PageDown: 'pagedown',
    ArrowUp: 'up',
    ArrowDown: 'down',
    ArrowLeft: 'left',
    ArrowRight: 'right',
    Minus: 'minus',
    Equal: 'equal',
    BracketLeft: 'bracketleft',
    BracketRight: 'bracketright',
    Backslash: 'backslash',
    Semicolon: 'semicolon',
    Quote: 'quote',
    Comma: 'comma',
    Period: 'period',
    Slash: 'slash',
    Backquote: 'backquote',
  }

  return specialKeys[code] || code.toLowerCase()
}
