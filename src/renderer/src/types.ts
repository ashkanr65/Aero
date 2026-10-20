export type ShieldMode = 'standard' | 'aggressive' | 'disabled';

export interface BrowserTab {
  id: number;
  title: string;
  url: string;
  favicon?: string;
  loading: boolean;
  audible: boolean;
  muted: boolean;
  discarded: boolean;
}

export interface BrowserState {
  tabs: BrowserTab[];
  activeTabId: number;
  splitTabId: number | null;
  leftTabId: number;
  rightTabId: number | null;
  splitRatio: number;
  canGoBack: boolean;
  canGoForward: boolean;
  shieldMode: ShieldMode;
  scriptBlocking: boolean;
  stats: { blocked: number; httpsUpgrades: number; bandwidthSaved: number };
  zoomPercent: number;
  fullScreen: boolean;
  memory: { discardedTabs: number; activeViews: number };
}

export const EMPTY_STATE: BrowserState = {
  tabs: [], activeTabId: 0, splitTabId: null, leftTabId: 0, rightTabId: null, splitRatio: 0.5,
  canGoBack: false, canGoForward: false,
  shieldMode: 'standard', scriptBlocking: false,
  stats: { blocked: 0, httpsUpgrades: 0, bandwidthSaved: 0 },
  zoomPercent: 100, fullScreen: false,
  memory: { discardedTabs: 0, activeViews: 0 },
};

export interface DownloadRecord {
  id: string;
  filename: string;
  state: 'progressing' | 'completed' | 'cancelled' | 'interrupted';
  receivedBytes: number;
  totalBytes: number;
  savePath: string;
}

export type ShortcutAction =
  | 'focus-address' | 'find' | 'find-next' | 'find-previous'
  | 'history' | 'downloads' | 'favorites' | 'add-favorite'
  | 'settings' | 'clear-data' | 'tab-search' | 'toggle-sidebar' | 'password-manager'
  | 'toggle-favorites-bar' | 'memory' | 'save-all-favorites' | 'focus-favorites' | 'focus-toolbar' | 'focus-browser';

declare global {
  interface Window {
    aero: import('../../preload/index').ExposedAPI;
  }
}

export type { ImportSource, ImportSummary } from './types-import';
