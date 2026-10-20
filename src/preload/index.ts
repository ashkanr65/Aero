import type { ClearDataRequest, ClearDataResult } from '../shared/browsing-data';
import type { BrowserSettings } from '../shared/settings';
import type { BrowserState, DownloadRecord, ShortcutAction } from '../renderer/src/types';
import type { ImportSource, ImportSummary } from '../renderer/src/types-import';

export interface ExposedAPI {
  clearBrowsingData(request: ClearDataRequest): Promise<ClearDataResult>;
  onBrowsingDataCleared(listener: (result: ClearDataResult) => void): () => void;
  getState: () => Promise<BrowserState | null>;
  getSettings: () => Promise<BrowserSettings>;
  setSettings: (settings: BrowserSettings) => Promise<BrowserSettings>;
  resetSettings: () => Promise<BrowserSettings>;
  getDownloads: () => Promise<DownloadRecord[]>;
  chooseDownloadDirectory: () => Promise<string>;
  command: (command: string, payload?: unknown) => Promise<unknown>;
  setBounds: (bounds: { x: number; y: number; width: number; height: number }) => void;
  onState: (listener: (state: BrowserState) => void) => () => void;
  onShortcut: (listener: (action: ShortcutAction) => void) => () => void;
  onOpenPasswordManager: (listener: () => void) => () => void;
  onSaveAllFavorites: (listener: (entries: Array<{ title: string; url: string }>) => void) => () => void;
  onSettings: (listener: (settings: BrowserSettings) => void) => () => void;
  onDownloads: (listener: (downloads: DownloadRecord[]) => void) => () => void;
  onPrivateMode: (listener: (privateMode: boolean) => void) => () => void;
  onShortcutMappingChange: (listener: (mapping: Record<string, string>) => void) => () => void;
  importSummary: (payload: { source: ImportSource; action?: 'discover' | 'read' | 'browse' | 'password-settings'; profileId?: string; kinds?: Array<'bookmarks' | 'history' | 'passwords' | 'tabs'> }) => Promise<ImportSummary | null>;
  onImportSummary: (listener: (summary: ImportSummary) => void) => () => void;
  onGpuSettingRestartRequired: (listener: (disabled: boolean) => void) => () => void;
  getShortcutMapping: () => Promise<Record<string, string>>;
  setShortcutMapping: (mapping: Record<string, string>) => Promise<Record<string, string>>;
  resetShortcutMapping: () => Promise<Record<string, string>>;
  passwordManagerAvailability: () => Promise<{ available: boolean; message?: string }>;
  openPasswordManager: () => Promise<boolean>;
  fillSelectedCredential: () => Promise<boolean>;
  onCredentialFillReady: (listener: (origin: string) => void) => () => void;
  onCredentialFillOffer: (listener: (offer: { origin: string; username: string }) => void) => () => void;
  onPasswordManagerLocked: (listener: () => void) => () => void;
  saveSubmittedCredential: () => Promise<{ id: string } | false>;
  discardCredentialSubmission: () => Promise<boolean>;
}

