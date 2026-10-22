import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { ExposedAPI } from '../preload/index';

const request = <T>(method: string, payload: unknown = null): Promise<T> => invoke<T>('aero_request', { method, payload }).catch(cause => { throw new Error(typeof cause === 'string' ? cause : 'The Aero operation failed.'); });
function subscribe<T>(event: string, listener: (value: T) => void) {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  void listen<T>(event, message => { if (!disposed) listener(message.payload); }).then(next => { if (disposed) next(); else unlisten = next; });
  return () => { disposed = true; unlisten?.(); };
}
let privateMode: boolean | undefined;
const privateListeners = new Set<(value: boolean) => void>();
const api = {
  getUpdateStatus: () => request('browser:update-status'),
  onUpdateStatus: (f: (v: unknown) => void) => subscribe('browser:update-status', f),
  clearBrowsingData: (payload: unknown) => request('browser:clear-data', payload),
  onBrowsingDataCleared: (f: (v: unknown) => void) => subscribe('browser:data-cleared', f),
  getState: async () => { const state = await request<{privateMode:boolean}>('browser:get-state'); privateMode = state.privateMode; privateListeners.forEach(f => f(privateMode!)); return state; }, getSettings: () => request('browser:get-settings'),
  setSettings: (settings: unknown) => request('browser:set-settings', settings), resetSettings: () => request('browser:reset-settings'),
  getDownloads: () => request('browser:get-downloads'), chooseDownloadDirectory: () => request('browser:choose-download-directory'),
  command: (command: string, payload?: unknown) => request('browser:command', { command, payload }),
  setBounds: (bounds: unknown) => { void request('browser:set-bounds', bounds); },
  getShortcutMapping: () => request('browser:get-shortcut-mapping'), setShortcutMapping: (mapping: unknown) => request('browser:set-shortcut-mapping', mapping), resetShortcutMapping: () => request('browser:reset-shortcut-mapping'),
  passwordManagerAvailability: () => request('password-manager:availability'), openPasswordManager: () => request('browser:open-password-manager'),
  fillSelectedCredential: () => request('password-manager:fill'), saveSubmittedCredential: () => request('password-manager:save-submission'), discardCredentialSubmission: () => request('password-manager:discard-submission'),
  startPasswordTransfer: (payload: unknown) => request('browser:start-password-transfer', payload),
  importSummary: (payload: unknown) => request('browser:import-from-source', payload),
  onState: (f: (v: unknown) => void) => subscribe('browser:state', f), onSettings: (f: (v: unknown) => void) => subscribe('browser:settings', f),
  onShortcut: (f: (v: unknown) => void) => subscribe('browser:shortcut', f), onOpenPasswordManager: (f: () => void) => subscribe('browser:command-open-password-manager', f),
  onSaveAllFavorites: (f: (v: unknown) => void) => subscribe('browser:save-all-favorites', f), onDownloads: (f: (v: unknown) => void) => subscribe('browser:downloads', f),
  onPrivateMode: (f: (v: boolean) => void) => { privateListeners.add(f); if(privateMode !== undefined)f(privateMode); const off=subscribe<boolean>('browser:private-mode', f); return () => { privateListeners.delete(f); off(); }; }, onShortcutMappingChange: (f: (v: unknown) => void) => subscribe('browser:shortcut-mapping-changed', f),
  onImportSummary: (f: (v: unknown) => void) => subscribe('browser:import-summary', f), onGpuSettingRestartRequired: (f: (v: unknown) => void) => subscribe('browser:gpu-setting-restart-required', f),
  onCredentialFillReady: (f: (origin: string) => void) => subscribe<{origin:string}>('password-manager:fill-ready', v => f(v.origin)),
  onCredentialFillOffer: (f: (v: unknown) => void) => subscribe('password-manager:save-offer', f), onPasswordManagerLocked: (f: () => void) => subscribe('password-manager:locked', f),
} as unknown as ExposedAPI;
window.aero = api;
window.aeroPasswordManager = {
  list: () => request('password-manager:list'), save: entry => request('password-manager:save', entry), remove: id => request('password-manager:delete', id),
  select: id => request('password-manager:select', id), reveal: () => request('password-manager:reveal'), copy: () => request('password-manager:copy'),
  selectForFill: id => request('password-manager:select-for-fill', id), backup: () => request('password-manager:backup'), restore: () => request('password-manager:restore'),
  transferStatus: () => request('password-manager:transfer-status'),
  cancelTransfer: () => request('password-manager:transfer-cancel'),
  importTransfer: (session, file) => request('password-manager:transfer-import', {session, file}),
  importCsv: () => request('password-manager:import-csv'), lock: () => request('password-manager:lock'), activity: () => { void request('password-manager:activity'); },
  onLock: f => subscribe('password-manager:lock', f), onFillReady: f => subscribe<{origin:string}>('password-manager:fill-ready', v => f(v.origin)),
};
// WebView2 uses the same app-region CSS; the explicit handler also works across Tauri versions.
document.addEventListener('mousedown', event => {
  if (event.button !== 0 || !(event.target instanceof Element) || !event.target.closest('.drag') || event.target.closest('.no-drag, button, input, a, [draggable="true"]')) return;
  void getCurrentWindow().startDragging();
});
