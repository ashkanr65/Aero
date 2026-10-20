export type BrowserShortcutAction =
  | 'reopen-tab' | 'new-private-window' | 'close-window' | 'duplicate-tab'
  | 'hard-reload' | 'clear-data' | 'tab-search' | 'favorites' | 'toggle-favorites-bar'
  | 'paste-and-go' | 'devtools' | 'toggle-sidebar' | 'capture-page' | 'read-aloud'  | 'collections' | 'print-system' | 'save-all-favorites' | 'paste-plain' | 'open-password-manager'
  | 'focus-settings' | 'focus-toolbar' | 'focus-favorites' | 'focus-browser'
  | 'new-tab' | 'new-window' | 'close-tab' | 'focus-address' | 'next-tab' | 'previous-tab'
  | 'select-tab' | 'reload' | 'back' | 'forward' | 'home' | 'find' | 'find-next'
  | 'find-previous' | 'history' | 'downloads' | 'add-favorite' | 'print' | 'save-page'
  | 'open-file' | 'view-source' | 'zoom-in' | 'zoom-out' | 'zoom-reset' | 'mute-tab'
  | 'settings' | 'open-url-new-tab' | 'fullscreen' | 'reader' | 'stop' | 'help'
  | 'import-edge' | 'import-chrome' | 'import-firefox';

export type ShortcutMapping = Record<string, string>;

export const DEFAULT_SHORTCUT_MAPPING: ShortcutMapping = {
  'reopen-tab': 'Ctrl+Shift+T',
  'new-private-window': 'Ctrl+Shift+N',
  'close-window': 'Ctrl+Shift+W',
  'duplicate-tab': 'Ctrl+Shift+K',
  'hard-reload': 'Ctrl+Shift+R',
  'clear-data': 'Ctrl+Shift+Delete',
  'tab-search': 'Ctrl+Shift+A',
  favorites: 'Ctrl+Shift+O',
  'toggle-favorites-bar': 'Ctrl+B',
  'paste-and-go': 'Ctrl+Shift+L',
  devtools: 'Ctrl+Shift+I',
  'toggle-sidebar': 'Ctrl+E',
  'capture-page': 'Ctrl+Shift+S',
  'read-aloud': 'Ctrl+Shift+U',
  collections: 'Ctrl+Shift+Y',
  'print-system': 'Ctrl+P',
  'save-all-favorites': 'Ctrl+Shift+D',
  'paste-plain': 'Ctrl+V',
  'focus-settings': 'Alt+Shift+I',
  'focus-toolbar': 'Alt+Shift+T',
  'focus-favorites': 'Alt+Shift+B',
  'focus-browser': 'Ctrl+F6',
  'open-password-manager': 'Ctrl+Shift+P',
  'new-tab': 'Ctrl+T',
  'new-window': 'Ctrl+N',
  'close-tab': 'Ctrl+W',
  'focus-address': 'Ctrl+L',
  'next-tab': 'Ctrl+Tab',
  'previous-tab': 'Ctrl+Shift+Tab',
  'select-tab': 'Ctrl+1',
  reload: 'Ctrl+R',
  back: 'Alt+Left',
  forward: 'Alt+Right',
  home: 'Alt+Home',
  find: 'Ctrl+F',
  'find-next': 'Ctrl+G',
  'find-previous': 'Ctrl+Shift+G',
  history: 'Ctrl+H',
  downloads: 'Ctrl+J',
  'add-favorite': 'Ctrl+D',
  print: 'Ctrl+P',
  'save-page': 'Ctrl+S',
  'open-file': 'Ctrl+O',
  'view-source': 'Ctrl+U',
  'zoom-in': 'Ctrl+Plus',
  'zoom-out': 'Ctrl+Minus',
  'zoom-reset': 'Ctrl+0',
  'mute-tab': 'Ctrl+M',
  settings: 'Alt+F',
  'open-url-new-tab': 'Alt+Enter',
  fullscreen: 'F11',
  reader: 'F9',
  stop: 'Escape',
  help: 'F1',
};

export interface ShortcutInput {
  key: string;
  control?: boolean;
  meta?: boolean;
  shift?: boolean;
  alt?: boolean;
}

export function matchBrowserShortcut(input: ShortcutInput): { action: BrowserShortcutAction; value?: number } | null {
  const key = input.key.toLowerCase();
  const primary = Boolean(input.control || input.meta);
  const shift = Boolean(input.shift);
  const alt = Boolean(input.alt);
  if (primary && shift && key === 'p') return { action: 'open-password-manager' };
  if (primary && shift) {
    const actions: Record<string, BrowserShortcutAction> = {
      t: 'reopen-tab', n: 'new-private-window', w: 'close-window', k: 'duplicate-tab',
      r: 'hard-reload', delete: 'clear-data', a: 'tab-search', o: 'favorites',
      b: 'toggle-favorites-bar', l: 'paste-and-go', i: 'devtools', e: 'toggle-sidebar',
      s: 'capture-page', u: 'read-aloud', y: 'collections',
      d: 'save-all-favorites', v: 'paste-plain',
    };
    if (actions[key]) return { action: actions[key] };
    if (key === 'f4') return { action: 'close-tab' };
  }
  if (alt && shift) {
    if (key === 'i') return { action: 'focus-settings' };
    if (key === 't') return { action: 'focus-toolbar' };
    if (key === 'b') return { action: 'focus-favorites' };
  }
  if (primary && shift && key === 'f6') return null;
  if (primary && key === 'f6') return { action: 'focus-browser' };
  if (primary && key === 'tab') return { action: shift ? 'previous-tab' : 'next-tab' };
  if (primary && (key === 'pagedown' || key === 'pageup')) return { action: key === 'pagedown' ? 'next-tab' : 'previous-tab' };
  if (primary && /^[1-9]$/.test(key)) return { action: 'select-tab', value: Number(key) };
  if (primary && (key === 'w' || key === 'f4')) return { action: 'close-tab' };
  if (primary && key === 't') return { action: 'new-tab' };
  if (primary && key === 'n') return { action: 'new-window' };
  if (primary && ['l', 'k', 'e'].includes(key)) return { action: 'focus-address' };
  if (primary && (key === 'r' || key === 'f5')) return { action: 'reload' };
  if (alt && key === 'left') return { action: 'back' };
  if (alt && key === 'right') return { action: 'forward' };
  if (alt && key === 'home') return { action: 'home' };
  if (alt && key === 'enter') return { action: 'open-url-new-tab' };
  if (alt && key === 'f4') return { action: 'close-window' };
  if (alt && key === 'd') return { action: 'focus-address' };
  if (primary && key === 'enter') return { action: 'focus-address' };
  if (primary && key === 'f') return { action: 'find' };
  if (key === 'f3') return { action: shift ? 'find-previous' : 'find-next' };
  if (primary && key === 'g') return { action: shift ? 'find-previous' : 'find-next' };
  if (primary && key === 'h') return { action: 'history' };
  if (primary && key === 'j') return { action: 'downloads' };
  if (primary && key === 'd') return { action: 'add-favorite' };
  if (primary && key === 'p') return { action: 'print' };
  if (primary && (key === 's' || key === 'S')) return { action: 'save-page' };
  if (primary && key === 'o') return { action: 'open-file' };
  if (primary && key === 'u') return { action: 'view-source' };
  if (primary && (key === '+' || key === '=')) return { action: 'zoom-in' };
  if (primary && key === '-' || (key === 'm' && shift)) return { action: 'zoom-out' };
  if (primary && key === '0') return { action: 'zoom-reset' };
  if (primary && key === 'm') return { action: 'mute-tab' };
  if (alt && key === 'f') return { action: 'settings' };
  if (key === 'escape') return { action: 'stop' };
  if (key === 'f1') return { action: 'help' };
  if (key === 'f9') return { action: 'reader' };
  if (key === 'f10') return { action: 'settings' };
  if (key === 'f6') return { action: 'focus-address' };
  if (key === 'f4') return { action: 'focus-address' };
  if (shift && key === 'f5') return { action: 'hard-reload' };
  if (key === 'f1') return { action: 'help' };
  if (key === 'f9') return { action: 'reader' };
  if (key === 'f11') return { action: 'fullscreen' };
  if (key === 'f12') return { action: 'devtools' };
  if (key === 'escape') return { action: 'stop' };
  if (key === 'f10') return { action: 'settings' };
  if (key === 'f6') return { action: 'focus-address' };
  if (key === 'f4') return { action: 'focus-address' };
  if (shift && key === 'f5') return { action: 'hard-reload' };
  return null;
}

export function normalizeShortcutKeys(): ShortcutMapping {
  return { ...DEFAULT_SHORTCUT_MAPPING };
}
