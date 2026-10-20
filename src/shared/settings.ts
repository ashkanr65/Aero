import { DEFAULT_SHORTCUT_MAPPING, type ShortcutMapping } from '../main/keyboard-shortcuts';

export type ThemeMode = 'system' | 'light' | 'dark';
export type SearchEngine = 'duckduckgo' | 'brave' | 'bing' | 'google' | 'ecosia';
export type TrackingLevel = 'strict' | 'balanced' | 'off';

export interface BrowserSettings {
  appearance: {
    theme: ThemeMode;
    verticalTabs: boolean;
    compactTabs: boolean;
    sidebarOpen: boolean;
    aeroIdentity: boolean;
    showHomeButton: boolean;
    showFavoritesBar: boolean;
    disableGpu: boolean;
  };
  startup: {
    homePage: string;
    openHomeOnStartup: boolean;
  };
  search: {
    engine: SearchEngine;
    suggestions: boolean;
  };
  privacy: {
    trackingLevel: TrackingLevel;
    blockThirdPartyCookies: boolean;
    upgradeHttps: boolean;
    globalPrivacyControl: boolean;
    doNotTrack: boolean;
    fingerprintProtection: boolean;
    clearOnExit: boolean;
  };
  shields: {
    blockScripts: boolean;
    blockAdNetworks: boolean;
    blockAnalytics: boolean;
    blockSessionReplay: boolean;
    blockErrorMonitoring: boolean;
    blockBannerAds: boolean;
    blockYoutubeVideoAds: boolean;
    hideAdElements: boolean;
  };
  downloads: {
    directory: string;
    askWhereToSave: boolean;
  };
  performance: {
    sleepingTabs: boolean;
    sleepAfterMinutes: number;
    efficiencyMode: boolean;
  };
  content: {
    spellcheck: boolean;
  };
  permissions: {
    camera: boolean;
    microphone: boolean;
    geolocation: boolean;
    notifications: boolean;
    fullscreen: boolean;
    clipboard: boolean;
  };
  accessibility: {
    pageZoom: number;
  };
  layout: {
    leftSidebarWidth: number;
    rightSidebarWidth: number;
  };
  tabs: {
    location: 'top' | 'left';
    groups: boolean;
  };
  shortcuts: {
    mapping: ShortcutMapping;
    resetToDefault: boolean;
  };
}

export const DEFAULT_SETTINGS: BrowserSettings = {
  appearance: {
    theme: 'system',
    verticalTabs: true,
    compactTabs: false,
    sidebarOpen: true,
    aeroIdentity: true,
    showHomeButton: true,
    showFavoritesBar: false,
    disableGpu: false,
  },
  startup: {
    homePage: 'https://duckduckgo.com/',
    openHomeOnStartup: true,
  },
  search: {
    engine: 'duckduckgo',
    suggestions: true,
  },
  privacy: {
    trackingLevel: 'balanced',
    blockThirdPartyCookies: true,
    upgradeHttps: true,
    globalPrivacyControl: true,
    doNotTrack: true,
    fingerprintProtection: true,
    clearOnExit: false,
  },
  shields: {
    blockScripts: false,
    blockAdNetworks: true,
    blockAnalytics: true,
    blockSessionReplay: true,
    blockErrorMonitoring: true,
    blockBannerAds: true,
    blockYoutubeVideoAds: true,
    hideAdElements: true,
  },
  downloads: {
    directory: '',
    askWhereToSave: true,
  },
  performance: {
    sleepingTabs: true,
    sleepAfterMinutes: 15,
    efficiencyMode: true,
  },
  content: {
    spellcheck: true,
  },
  permissions: {
    camera: false,
    microphone: false,
    geolocation: false,
    notifications: false,
    fullscreen: true,
    clipboard: true,
  },
  accessibility: {
    pageZoom: 100,
  },
  layout: {
    leftSidebarWidth: 240,
    rightSidebarWidth: 310,
  },
  tabs: {
    location: 'left',
    groups: true,
  },
  shortcuts: {
    mapping: { ...DEFAULT_SHORTCUT_MAPPING } as ShortcutMapping,
    resetToDefault: false,
  },
};

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? value as Record<string, unknown> : {};
}

export function normalizeSettings(value: unknown): BrowserSettings {
  const root = objectValue(value);
  const result = structuredClone(DEFAULT_SETTINGS);
  for (const sectionName of Object.keys(result) as Array<keyof BrowserSettings>) {
    const incoming = objectValue(root[sectionName]);
    Object.assign(result[sectionName], incoming);
  }

  if (!['system', 'light', 'dark'].includes(result.appearance.theme)) result.appearance.theme = 'system';
  result.appearance.disableGpu = result.appearance.disableGpu === true;
  if (!['duckduckgo', 'brave', 'bing', 'google', 'ecosia'].includes(result.search.engine)) result.search.engine = 'duckduckgo';
  if (!['strict', 'balanced', 'off'].includes(result.privacy.trackingLevel)) result.privacy.trackingLevel = 'balanced';
  if (!/^https?:\/\//i.test(result.startup.homePage)) result.startup.homePage = DEFAULT_SETTINGS.startup.homePage;
  if (![5, 15, 30, 60].includes(result.performance.sleepAfterMinutes)) result.performance.sleepAfterMinutes = 15;
  result.accessibility.pageZoom = Math.min(200, Math.max(50, Math.round(result.accessibility.pageZoom / 10) * 10));
  result.layout.leftSidebarWidth = Math.min(400, Math.max(180, Math.round(Number(result.layout.leftSidebarWidth) || DEFAULT_SETTINGS.layout.leftSidebarWidth)));
  result.layout.rightSidebarWidth = Math.min(500, Math.max(230, Math.round(Number(result.layout.rightSidebarWidth) || DEFAULT_SETTINGS.layout.rightSidebarWidth)));
  return result;
}

export function searchUrl(engine: SearchEngine, query: string): string {
  const encoded = encodeURIComponent(query);
  if (engine === 'brave') return `https://search.brave.com/search?q=${encoded}`;
  if (engine === 'bing') return `https://www.bing.com/search?q=${encoded}`;
  if (engine === 'google') return `https://www.google.com/search?q=${encoded}`;
  if (engine === 'ecosia') return `https://www.ecosia.org/search?q=${encoded}`;
  return `https://duckduckgo.com/?q=${encoded}`;
}
