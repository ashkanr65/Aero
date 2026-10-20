import React, { useEffect, useMemo, useState } from 'react';
import { Download, Gauge, Keyboard, Palette, RotateCcw, Search, Settings, Shield, X, Import } from 'lucide-react';
import type { BrowserSettings } from '../../../shared/settings';
import { DEFAULT_SHORTCUT_MAPPING, type BrowserShortcutAction } from '../../../main/keyboard-shortcuts';

export type SettingsSection = 'general' | 'appearance' | 'privacy' | 'performance' | 'import' | 'downloads' | 'shortcuts';

import { ClearBrowsingData } from './ClearBrowsingData';
import type { ClearDataRequest } from '../../../shared/browsing-data';

interface SettingsPanelProps {
  section: SettingsSection;
  onSectionChange(section: SettingsSection): void;
  importContent: React.ReactNode;
  clearRequest: number;
  privateMode: boolean;
  onClearBrowsingData(request: ClearDataRequest): Promise<void>;
  settings: BrowserSettings;
  onChange(settings: BrowserSettings): void;
  onReset(): void;
  onChooseDownloadDirectory(): void;
  onOpenImport(): void;
  onClose(): void;
}

const shortcutGroups = [
  { title: 'Tabs and windows', shortcuts: [
    { id: 'new-tab', keys: DEFAULT_SHORTCUT_MAPPING['new-tab'], description: 'New tab' },
    { id: 'close-tab', keys: DEFAULT_SHORTCUT_MAPPING['close-tab'], description: 'Close tab' },
    { id: 'next-tab', keys: DEFAULT_SHORTCUT_MAPPING['next-tab'], description: 'Next tab' },
    { id: 'previous-tab', keys: DEFAULT_SHORTCUT_MAPPING['previous-tab'], description: 'Previous tab' },
    { id: 'select-tab', keys: DEFAULT_SHORTCUT_MAPPING['select-tab'], description: 'Select tab' },
    { id: 'reopen-tab', keys: DEFAULT_SHORTCUT_MAPPING['reopen-tab'], description: 'Reopen closed tab' },
    { id: 'duplicate-tab', keys: DEFAULT_SHORTCUT_MAPPING['duplicate-tab'], description: 'Duplicate tab' },
    { id: 'new-window', keys: DEFAULT_SHORTCUT_MAPPING['new-window'], description: 'New window' },
    { id: 'new-private-window', keys: DEFAULT_SHORTCUT_MAPPING['new-private-window'], description: 'New private window' },
  ]},
  { title: 'Address and navigation', shortcuts: [
    { id: 'focus-address', keys: DEFAULT_SHORTCUT_MAPPING['focus-address'], description: 'Address bar' },
    { id: 'reload', keys: DEFAULT_SHORTCUT_MAPPING['reload'], description: 'Reload' },
    { id: 'hard-reload', keys: DEFAULT_SHORTCUT_MAPPING['hard-reload'], description: 'Hard reload' },
    { id: 'stop', keys: DEFAULT_SHORTCUT_MAPPING['stop'], description: 'Stop loading' },
  ]},
  { title: 'Page and tools', shortcuts: [
    { id: 'find', keys: DEFAULT_SHORTCUT_MAPPING['find'], description: 'Find on page' },
    { id: 'find-next', keys: DEFAULT_SHORTCUT_MAPPING['find-next'], description: 'Next match' },
    { id: 'find-previous', keys: DEFAULT_SHORTCUT_MAPPING['find-previous'], description: 'Previous match' },
    { id: 'add-favorite', keys: DEFAULT_SHORTCUT_MAPPING['add-favorite'], description: 'Add favorite' },
    { id: 'favorites', keys: DEFAULT_SHORTCUT_MAPPING['favorites'], description: 'Favorites' },
    { id: 'history', keys: DEFAULT_SHORTCUT_MAPPING['history'], description: 'History' },
    { id: 'downloads', keys: DEFAULT_SHORTCUT_MAPPING['downloads'], description: 'Downloads' },
    { id: 'print', keys: DEFAULT_SHORTCUT_MAPPING['print'], description: 'Print' },
    { id: 'save-page', keys: DEFAULT_SHORTCUT_MAPPING['save-page'], description: 'Save page' },
    { id: 'open-file', keys: DEFAULT_SHORTCUT_MAPPING['open-file'], description: 'Open file' },
    { id: 'view-source', keys: DEFAULT_SHORTCUT_MAPPING['view-source'], description: 'View source' },
    { id: 'read-aloud', keys: DEFAULT_SHORTCUT_MAPPING['read-aloud'], description: 'Read aloud' },
  ]},
  { title: 'View and browser', shortcuts: [
    { id: 'zoom-in', keys: DEFAULT_SHORTCUT_MAPPING['zoom-in'], description: 'Zoom in' },
    { id: 'zoom-out', keys: DEFAULT_SHORTCUT_MAPPING['zoom-out'], description: 'Zoom out' },
    { id: 'zoom-reset', keys: DEFAULT_SHORTCUT_MAPPING['zoom-reset'], description: 'Reset zoom' },
    { id: 'reader', keys: DEFAULT_SHORTCUT_MAPPING['reader'], description: 'Immersive reader' },
    { id: 'fullscreen', keys: DEFAULT_SHORTCUT_MAPPING['fullscreen'], description: 'Full screen' },
    { id: 'devtools', keys: DEFAULT_SHORTCUT_MAPPING['devtools'], description: 'Developer tools' },
    { id: 'mute-tab', keys: DEFAULT_SHORTCUT_MAPPING['mute-tab'], description: 'Mute tab' },
    { id: 'clear-data', keys: DEFAULT_SHORTCUT_MAPPING['clear-data'], description: 'Clear data' },
    { id: 'tab-search', keys: DEFAULT_SHORTCUT_MAPPING['tab-search'], description: 'Search tabs' },
    { id: 'toggle-sidebar', keys: DEFAULT_SHORTCUT_MAPPING['toggle-sidebar'], description: 'Toggle sidebar' },
    { id: 'toggle-favorites-bar', keys: DEFAULT_SHORTCUT_MAPPING['toggle-favorites-bar'], description: 'Toggle favorites bar' },
    { id: 'settings', keys: DEFAULT_SHORTCUT_MAPPING['settings'], description: 'Settings' },
  ]},
];

const Toggle: React.FC<{ checked: boolean; onChange(value: boolean): void; label: string; description?: string }> = ({ checked, onChange, label, description }) => (
  <label className="setting-row cursor-pointer">
    <span><strong>{label}</strong>{description && <small>{description}</small>}</span>
    <input type="checkbox" checked={checked} onChange={event => onChange(event.target.checked)} className="h-4 w-4 accent-sky-600" />
  </label>
);

type ShortcutRow = { id: BrowserShortcutAction; keys: string; description: string };

export const SettingsPanel: React.FC<SettingsPanelProps> = ({ settings, onChange, onReset, onChooseDownloadDirectory, onOpenImport, onClose, section, onSectionChange: setSection, importContent, clearRequest, privateMode, onClearBrowsingData }) => {
  useEffect(() => {
    if (section === 'privacy' && clearRequest > 0) document.getElementById('clear-browsing-data')?.scrollIntoView({ block: 'start' });
  }, [section, clearRequest]);
  const [filter, setFilter] = useState('');
  const [editingId, setEditingId] = useState<BrowserShortcutAction | null>(null);
  const update = <Section extends keyof BrowserSettings>(name: Section, patch: Partial<BrowserSettings[Section]>) => {
    onChange({ ...settings, [name]: { ...settings[name], ...patch } });
  };
  const shortcuts: ShortcutRow[] = [
    { id: 'new-tab', keys: settings.shortcuts.mapping['new-tab'] ?? 'Ctrl+T', description: 'New tab' },
    { id: 'close-tab', keys: settings.shortcuts.mapping['close-tab'] ?? 'Ctrl+W', description: 'Close tab' },
    { id: 'next-tab', keys: settings.shortcuts.mapping['next-tab'] ?? 'Ctrl+Tab', description: 'Next tab' },
    { id: 'previous-tab', keys: settings.shortcuts.mapping['previous-tab'] ?? 'Ctrl+Shift+Tab', description: 'Previous tab' },
    { id: 'select-tab', keys: settings.shortcuts.mapping['select-tab'] ?? 'Ctrl+1', description: 'Select tab by number' },
    { id: 'reopen-tab', keys: settings.shortcuts.mapping['reopen-tab'] ?? 'Ctrl+Shift+T', description: 'Reopen closed tab' },
    { id: 'duplicate-tab', keys: settings.shortcuts.mapping['duplicate-tab'] ?? 'Ctrl+Shift+K', description: 'Duplicate tab' },
    { id: 'new-window', keys: settings.shortcuts.mapping['new-window'] ?? 'Ctrl+N', description: 'New window' },
    { id: 'new-private-window', keys: settings.shortcuts.mapping['new-private-window'] ?? 'Ctrl+Shift+N', description: 'New private window' },
    { id: 'focus-address', keys: settings.shortcuts.mapping['focus-address'] ?? 'Ctrl+L', description: 'Address bar' },
    { id: 'reload', keys: settings.shortcuts.mapping['reload'] ?? 'Ctrl+R', description: 'Reload' },
    { id: 'hard-reload', keys: settings.shortcuts.mapping['hard-reload'] ?? 'Ctrl+Shift+R', description: 'Hard reload' },
    { id: 'find', keys: settings.shortcuts.mapping['find'] ?? 'Ctrl+F', description: 'Find on page' },
    { id: 'find-next', keys: settings.shortcuts.mapping['find-next'] ?? 'Ctrl+G', description: 'Next match' },
    { id: 'find-previous', keys: settings.shortcuts.mapping['find-previous'] ?? 'Ctrl+Shift+G', description: 'Previous match' },
    { id: 'add-favorite', keys: settings.shortcuts.mapping['add-favorite'] ?? 'Ctrl+D', description: 'Add favorite' },
    { id: 'favorites', keys: settings.shortcuts.mapping['favorites'] ?? 'Ctrl+Shift+O', description: 'Favorites' },
    { id: 'history', keys: settings.shortcuts.mapping['history'] ?? 'Ctrl+H', description: 'History' },
    { id: 'downloads', keys: settings.shortcuts.mapping['downloads'] ?? 'Ctrl+J', description: 'Downloads' },
    { id: 'save-page', keys: settings.shortcuts.mapping['save-page'] ?? 'Ctrl+S', description: 'Save page' },
    { id: 'devtools', keys: settings.shortcuts.mapping['devtools'] ?? 'Ctrl+Shift+I', description: 'Developer tools' },
    { id: 'toggle-sidebar', keys: settings.shortcuts.mapping['toggle-sidebar'] ?? 'Ctrl+E', description: 'Toggle sidebar' },
    { id: 'toggle-favorites-bar', keys: settings.shortcuts.mapping['toggle-favorites-bar'] ?? 'Ctrl+B', description: 'Toggle favorites bar' },
    { id: 'settings', keys: settings.shortcuts.mapping['settings'] ?? 'Alt+F', description: 'Settings' },
    { id: 'clear-data', keys: settings.shortcuts.mapping['clear-data'] ?? 'Ctrl+Shift+Delete', description: 'Clear data' },
    { id: 'tab-search', keys: settings.shortcuts.mapping['tab-search'] ?? 'Ctrl+Shift+A', description: 'Search tabs' },
  ];
  const filteredShortcuts = useMemo(() => shortcuts.filter(row => row.description.toLowerCase().includes(filter.toLowerCase()) || row.keys.toLowerCase().includes(filter.toLowerCase())), [filter]);

  return (
    <div className="absolute inset-x-0 bottom-0 top-9 z-40 flex bg-white/90 backdrop-blur-xl dark:bg-[#17191d]/95">
      <nav className="w-64 shrink-0 border-r border-slate-200 p-4 dark:border-zinc-800">
        <div className="mb-6 flex items-center gap-2 px-2"><Settings size={20} className="text-sky-600" /><strong>Aero Settings</strong></div>
        {([
          ['general', Search, 'Startup & search'], ['appearance', Palette, 'Appearance'], ['privacy', Shield, 'Privacy & shields'],
          ['performance', Gauge, 'Performance'], ['import', Import, 'Import browser data'], ['downloads', Download, 'Downloads'], ['shortcuts', Keyboard, 'Shortcuts'],
        ] as const).map(([id, Icon, label]) => <button key={id} aria-current={section === id ? 'page' : undefined} className={`settings-nav ${section === id ? 'active' : ''}`} onClick={() => setSection(id)}><Icon size={16} />{label}</button>)}
      </nav>

      <section data-aero-settings-section={section} className="min-w-0 flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-3xl">
          <div className="mb-7 flex items-center justify-between"><h1 className="text-2xl font-semibold">{section === 'shortcuts' ? 'Keyboard shortcuts' : section === 'import' ? 'Import browser data' : 'Browser settings'}</h1><button className="icon-button" aria-label="Close settings" onClick={onClose}><X size={18} /></button></div>

          {section === 'general' && <div className="settings-stack">
            <SettingsCard title="Startup and home">
              <Toggle checked={settings.startup.openHomeOnStartup} onChange={value => update('startup', { openHomeOnStartup: value })} label="Open the home page on startup" />
              <label className="setting-field"><span>Home page</span><input value={settings.startup.homePage} onChange={event => update('startup', { homePage: event.target.value })} /></label>
              <Toggle checked={settings.appearance.showHomeButton} onChange={value => update('appearance', { showHomeButton: value })} label="Show Home button" />
            </SettingsCard>
            <SettingsCard title="Address bar and search">
              <label className="setting-field"><span>Search engine</span><select value={settings.search.engine} onChange={event => update('search', { engine: event.target.value as BrowserSettings['search']['engine'] })}><option value="duckduckgo">DuckDuckGo</option><option value="brave">Brave Search</option><option value="bing">Bing</option><option value="google">Google</option><option value="ecosia">Ecosia</option></select></label>
              <Toggle checked={settings.search.suggestions} onChange={value => update('search', { suggestions: value })} label="Address bar suggestions" description="Suggest matching local favorites and history without sending keystrokes to a provider." />
            </SettingsCard>
          </div>}

          {section === 'appearance' && <div className="settings-stack">
            <SettingsCard title="Theme and layout">
              <label className="setting-field"><span>Theme</span><select value={settings.appearance.theme} onChange={event => update('appearance', { theme: event.target.value as BrowserSettings['appearance']['theme'] })}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label>
              <Toggle checked={settings.appearance.verticalTabs} onChange={value => update('appearance', { verticalTabs: value })} label="Use vertical tabs" />
              <Toggle checked={settings.tabs.groups} onChange={value => update('tabs', { groups: value })} label="Tab groups" description="Same as Edge tab groups: name a group and give it a color; grouped tabs open together." />
              <Toggle checked={settings.appearance.compactTabs} onChange={value => update('appearance', { compactTabs: value })} label="Compact tab spacing" />
              <Toggle checked={settings.appearance.aeroIdentity} onChange={value => update('appearance', { aeroIdentity: value })} label="Identify browser as Aero" description="Adds the Aero product name to standard web requests. Chromium engine hints are still reported by Chromium." />
              <Toggle checked={settings.appearance.sidebarOpen} onChange={value => update('appearance', { sidebarOpen: value })} label="Show Aero sidebar by default" />
              <Toggle checked={settings.appearance.showFavoritesBar} onChange={value => update('appearance', { showFavoritesBar: value })} label="Show favorites bar" />
            </SettingsCard>
          </div>}

          {section === 'privacy' && <div className="settings-stack">
            <SettingsCard title="Password Manager">
              <div data-aero-password-row className="setting-row">
                <span className="flex-1"><small className="!mt-0 leading-relaxed">Keep credentials in an encrypted local vault. Windows Hello is required for access. Available in regular windows.</small></span>
                <button className="secondary-button shrink-0 whitespace-nowrap disabled:opacity-50" disabled={privateMode} onClick={() => void window.aero.openPasswordManager()}>Open Password Manager</button>
              </div>
            </SettingsCard>
            <div id="clear-browsing-data" className="scroll-mt-4"><SettingsCard title="Clear browsing data"><ClearBrowsingData onClear={onClearBrowsingData} /></SettingsCard></div>
            <SettingsCard title="Tracking prevention">
              <label className="setting-field"><span>Protection level</span><select value={settings.privacy.trackingLevel} onChange={event => update('privacy', { trackingLevel: event.target.value as BrowserSettings['privacy']['trackingLevel'] })}><option value="strict">Strict (Brave-style aggressive)</option><option value="balanced">Balanced</option><option value="off">Off for all sites</option></select></label>
              <Toggle checked={settings.privacy.blockThirdPartyCookies} onChange={value => update('privacy', { blockThirdPartyCookies: value })} label="Block third-party cookies" />
              <Toggle checked={settings.privacy.upgradeHttps} onChange={value => update('privacy', { upgradeHttps: value })} label="Always upgrade connections to HTTPS" />
              <Toggle checked={settings.privacy.fingerprintProtection} onChange={value => update('privacy', { fingerprintProtection: value })} label="Fingerprint protection" description="Applies light canvas, WebGL and audio noise; sophisticated fingerprinting can still identify browser/device traits." />
              <p className="px-1 text-xs text-slate-500">WebRTC is restricted to the public network interface to reduce private/local IP exposure. BrowserLeaks can report exposed data, but cannot guarantee anonymity or eliminate all fingerprinting.</p>
              <Toggle checked={settings.shields.blockAdNetworks} onChange={value => update('shields', { blockAdNetworks: value })} label="Block ad networks" description="Blocks ad requests matched by Aero’s built-in community filter lists." />
              <Toggle checked={settings.shields.blockYoutubeVideoAds} onChange={value => update('shields', { blockYoutubeVideoAds: value })} label="Block YouTube video ads" description="Best-effort: skips detected skippable ads and blocks identifiable ad requests. Ads inserted into shared video streams may still play." />
              <Toggle checked={settings.shields.blockAnalytics} onChange={value => update('shields', { blockAnalytics: value })} label="Block analytics and metrics" description="Blocks Google Analytics, Google Tag Manager, and Yandex Metrica requests." />
              <Toggle checked={settings.shields.blockSessionReplay} onChange={value => update('shields', { blockSessionReplay: value })} label="Block session replay and heatmaps" description="Blocks session recording services such as Hotjar." />
              <Toggle checked={settings.shields.blockErrorMonitoring} onChange={value => update('shields', { blockErrorMonitoring: value })} label="Block error monitoring" description="Blocks reporting services such as Sentry and Bugsnag." />
              <Toggle checked={settings.shields.blockBannerAds} onChange={value => update('shields', { blockBannerAds: value })} label="Block banner ads and legacy plugins" description="Blocks ad-identified banner files and unsupported Flash content." />
              <Toggle checked={settings.shields.hideAdElements} onChange={value => update('shields', { hideAdElements: value })} label="Hide common ad containers" description="Hides standard ad slots and detected ad frames; broad page cosmetic filtering is not included." />
            </SettingsCard>
            <SettingsCard title="Privacy signals and cleanup">
              <Toggle checked={settings.privacy.globalPrivacyControl} onChange={value => update('privacy', { globalPrivacyControl: value })} label="Global Privacy Control" />
              <Toggle checked={settings.privacy.doNotTrack} onChange={value => update('privacy', { doNotTrack: value })} label="Send Do Not Track" />
              <Toggle checked={settings.privacy.clearOnExit} onChange={value => update('privacy', { clearOnExit: value })} label="Clear browsing data when Aero closes" />
              <Toggle checked={settings.shields.blockScripts} onChange={value => update('shields', { blockScripts: value })} label="Block JavaScript on every site" description="Strong protection that can break websites." />
            </SettingsCard>
            <SettingsCard title="Site permissions">
              <Toggle checked={settings.permissions.camera} onChange={value => update('permissions', { camera: value })} label="Allow camera requests" />
              <Toggle checked={settings.permissions.microphone} onChange={value => update('permissions', { microphone: value })} label="Allow microphone requests" />
              <Toggle checked={settings.permissions.geolocation} onChange={value => update('permissions', { geolocation: value })} label="Allow location requests" />
              <Toggle checked={settings.permissions.notifications} onChange={value => update('permissions', { notifications: value })} label="Allow website notifications" />
              <Toggle checked={settings.permissions.fullscreen} onChange={value => update('permissions', { fullscreen: value })} label="Allow full-screen requests" />
              <Toggle checked={settings.permissions.clipboard} onChange={value => update('permissions', { clipboard: value })} label="Allow websites to write to the clipboard" description="Webpage clipboard reading is blocked to protect copied passwords. You can still paste with Ctrl+V." />
            </SettingsCard>
            <SettingsCard title="Restore defaults">
              <div className="setting-row"><span className="flex-1"><strong>Reset browser settings</strong><small>Restore Aero's default preferences.</small></span><button className="secondary-button shrink-0" onClick={onReset}><RotateCcw size={14} />Restore defaults</button></div>
            </SettingsCard>
          </div>}

          {section === 'performance' && <div className="settings-stack">
            <SettingsCard title="Aero RAM Manager">
              <Toggle checked={settings.performance.sleepingTabs} onChange={value => update('performance', { sleepingTabs: value })} label="Put inactive tabs to sleep" description="Discarded tabs release their Chromium renderer and reload when selected." />
              <label className="setting-field"><span>Sleep inactive tabs after</span><select disabled={!settings.performance.sleepingTabs} value={settings.performance.sleepAfterMinutes} onChange={event => update('performance', { sleepAfterMinutes: Number(event.target.value) })}><option value={5}>5 minutes</option><option value={15}>15 minutes</option><option value={30}>30 minutes</option><option value={60}>1 hour</option></select></label>
              <Toggle checked={settings.performance.efficiencyMode} onChange={value => update('performance', { efficiencyMode: value })} label="Efficiency mode" description="Throttles timers and animation in background tabs." />
              <button className="secondary-button" onClick={() => void window.aero.command('restart')}>Restart Aero to apply graphics changes</button>
              <Toggle checked={settings.appearance.disableGpu} onChange={value => update('appearance', { disableGpu: value })} label="Disable GPU acceleration" description={`Disable GPU acceleration for every site and window, on any computer running this Aero profile. ${settings.appearance.disableGpu ? 'Turn off to re-enable GPU acceleration.' : 'Leave off to keep GPU acceleration enabled.'} Restart Aero to apply the change.`} />
              <Toggle checked={settings.content.spellcheck} onChange={value => update('content', { spellcheck: value })} label="Spell checking" description="Applied to newly opened or restored tabs." />
              <label className="setting-field"><span>Default page zoom</span><select value={settings.accessibility.pageZoom} onChange={event => update('accessibility', { pageZoom: Number(event.target.value) })}>{[50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200].map(value => <option key={value} value={value}>{value}%</option>)}</select></label>
            </SettingsCard>
            <SettingsCard title="Import browser data"><div className="setting-row"><span className="flex-1"><strong>Bring your browser data to Aero</strong><small>Import bookmarks and history from another browser.</small></span><button className="secondary-button shrink-0" onClick={onOpenImport}><Import size={14} />Open import</button></div></SettingsCard>
          </div>}

          {section === 'import' && importContent}

          {section === 'downloads' && <div className="settings-stack">
            <SettingsCard title="Downloads">
              <Toggle checked={settings.downloads.askWhereToSave} onChange={value => update('downloads', { askWhereToSave: value })} label="Ask where to save each file" />
              <div className="setting-row"><span><strong>Download location</strong><small>{settings.downloads.directory || 'Windows Downloads folder'}</small></span><button className="secondary-button" onClick={onChooseDownloadDirectory}>Change</button></div>
            </SettingsCard>
          </div>}

          {section === 'shortcuts' && <div>
            <div className="flex gap-3 mb-6">
              <input value={filter} onChange={event => setFilter(event.target.value)} placeholder="Search shortcuts" className="flex-1 h-10 rounded-xl border border-slate-200 bg-white px-4 outline-none focus:border-sky-500 dark:border-zinc-700 dark:bg-zinc-900" />
              {settings.shortcuts.resetToDefault ? (
                <button className="secondary-button" onClick={() => update('shortcuts', { resetToDefault: false, mapping: { ...DEFAULT_SHORTCUT_MAPPING } })}>
                  Reset to default
                </button>
              ) : (
                <button className="secondary-button" onClick={() => update('shortcuts', { resetToDefault: true, mapping: {} })}>
                  Reset to default
                </button>
              )}
            </div>
            <div className="settings-stack">
              {filteredShortcuts.map(row => (
                <SettingsCard key={row.id} title={row.description}>
                  <div className="setting-row">
                    <kbd className={editingId === row.id ? 'editing' : ''}>{row.keys}</kbd>
                    {editingId === row.id ? (
                      <input
                        autoFocus
                        defaultValue={row.keys}
                        className="flex-1 h-10 rounded border border-slate-300 bg-white px-3 text-sm outline-none focus:border-sky-500 dark:border-zinc-600 dark:bg-zinc-800"
                        onKeyDown={e => {
                          if (e.key === 'Enter') {
                            const next = { ...settings.shortcuts.mapping };
                            next[row.id] = e.currentTarget.value;
                            update('shortcuts', { mapping: next });
                            setEditingId(null);
                          }
                          if (e.key === 'Escape') setEditingId(null);
                        }}
                        onBlur={e => {
                          const next = { ...settings.shortcuts.mapping };
                          next[row.id] = e.currentTarget.value;
                          update('shortcuts', { mapping: next });
                          setEditingId(null);
                        }}
                      />
                    ) : (
                      <button className="ml-2 text-xs text-sky-600 hover:underline" onClick={() => setEditingId(row.id)}>
                        Change
                      </button>
                    )}
                  </div>
                </SettingsCard>
              ))}
            </div>
            {settings.shortcuts.resetToDefault && (
              <p className="mt-4 text-sm text-amber-700 dark:text-amber-400">
                Shortcuts are reset to default. Click "Reset to default" again to confirm.
              </p>
            )}
          </div>}
        </div>
      </section>
    </div>
  );
};

const SettingsCard: React.FC<React.PropsWithChildren<{ title: string }>> = ({ title, children }) => <div className="settings-card"><h2>{title}</h2><div>{children}</div></div>;
