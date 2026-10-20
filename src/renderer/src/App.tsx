import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { BrowserPanel, type BrowserPanelType, type FavoriteEntry, type HistoryEntry } from './components/BrowserPanel';
import { FavoritesBar } from './components/FavoritesBar';
import { FindBar } from './components/FindBar';
import { Header } from './components/Header';
import { Omnibox } from './components/Omnibox';
import { SettingsPanel, type SettingsSection } from './components/SettingsPanel';
import { Sidebar, type SidebarTool } from './components/Sidebar';
import { SplitView } from './components/SplitView';
import { VerticalTabs } from './components/VerticalTabs';
import { DEFAULT_SETTINGS, type BrowserSettings } from '../../shared/settings';
import { EMPTY_STATE, type BrowserState, type DownloadRecord, type ShortcutAction } from './types';
import type { ImportSource, ImportSummary, ImportSelection } from './types-import';
import { ImportSection } from './components/ImportSection';
import { retainOutsideRange, type ClearDataRequest } from '../../shared/browsing-data';
import { HISTORY_LIMIT, mergeBookmarks, mergeHistory } from '../../shared/import-data';

function readLocal<T>(key: string, fallback: T): T {
  try { return JSON.parse(localStorage.getItem(key) ?? '') as T; } catch { return fallback; }
}

function createId(): string { return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`; }

export const App: React.FC = () => {
  const [state, setState] = useState<BrowserState>(EMPTY_STATE);
  const [settings, setSettings] = useState<BrowserSettings>(DEFAULT_SETTINGS);
  const [tabsCollapsed, setTabsCollapsed] = useState(false);
  const [sidebarWidths, setSidebarWidths] = useState(DEFAULT_SETTINGS.layout);
  const [resizeSide, setResizeSide] = useState<'left' | 'right' | null>(null);
  const resizeStart = useRef<{ side: 'left' | 'right'; x: number; width: number } | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(DEFAULT_SETTINGS.appearance.sidebarOpen);
  const [sidebarTool, setSidebarTool] = useState<SidebarTool>('privacy');
  const [fireActive, setFireActive] = useState(false);
  const [settingsSection, setSettingsSection] = useState<SettingsSection>('general');
  const [clearRequest, setClearRequest] = useState(0);
  const [importSummary, setImportSummary] = useState<ImportSummary | null>(null);
  const importRequest = useRef(0);
  const [importHelpError, setImportHelpError] = useState('');

  const [tabDragging, setTabDragging] = useState(false);
  const [tabMenuOpen, setTabMenuOpen] = useState(false);
  useEffect(() => {
    const end = () => setTabDragging(false);
    window.addEventListener('dragend', end); window.addEventListener('drop', end);
    return () => { window.removeEventListener('dragend', end); window.removeEventListener('drop', end); };
  }, []);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [panel, setPanel] = useState<BrowserPanelType | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  const [findDirection, setFindDirection] = useState<0 | 1 | -1>(0);
  const [focusRequest, setFocusRequest] = useState(0);
  const [privateMode, setPrivateMode] = useState(false);
  const [credentialSaveOffer, setCredentialSaveOffer] = useState<{ origin: string; username: string } | null>(null);
  const [credentialFillOffer, setCredentialFillOffer] = useState<string | null>(null);
  const favoritesBarRef = useRef<HTMLDivElement>(null);
  const [downloads, setDownloads] = useState<DownloadRecord[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>(() => readLocal('aero.history', []));
  const [favorites, setFavorites] = useState<FavoriteEntry[]>(() => readLocal('aero.favorites', []));
  const addressSuggestions = useMemo(() => [...favorites, ...history], [favorites, history]);
  const viewportRef = useRef<HTMLDivElement>(null);
  const privateModeRef = useRef<boolean | undefined>(undefined);
  const lastHistoryVisit = useRef<string | null>(null);
  const stateRef = useRef(state);
  const settingsRef = useRef(settings);

  useEffect(() => { stateRef.current = state; }, [state]);
  useEffect(() => { settingsRef.current = settings; }, [settings]);

  const command = useCallback((name: string, payload?: unknown) => {
    if (window.aero) void window.aero.command(name, payload);
  }, []);

  const saveSettings = useCallback((next: BrowserSettings) => {
    settingsRef.current = next;
    setSettings(next);
    void window.aero?.setSettings(next);
  }, []);

  const toggleFavoritesBar = useCallback(() => {
    const current = settingsRef.current;
    saveSettings({ ...current, appearance: { ...current.appearance, showFavoritesBar: !current.appearance.showFavoritesBar } });
  }, [saveSettings]);

  const addFavorite = useCallback(() => {
    const current = stateRef.current;
    const active = current.tabs.find(tab => tab.id === current.activeTabId);
    if (!active?.url) return;
    setFavorites(previous => {
      if (previous.some(item => item.url === active.url)) return previous;
      const next = [{ id: createId(), title: active.title || active.url, url: active.url }, ...previous];
      localStorage.setItem('aero.favorites', JSON.stringify(next));
      return next;
    });
  }, []);

  const openClearData = useCallback(() => { setPanel(null); setSettingsSection('privacy'); setClearRequest(value => value + 1); setSettingsOpen(true); }, []);
  const clearBrowsingData = useCallback(async (request: ClearDataRequest) => {
    setFireActive(true);
    try { await window.aero.clearBrowsingData(request); }
    finally { setFireActive(false); }
  }, []);

  const handleShortcut = useCallback((action: ShortcutAction) => {
    if (action === 'password-manager') command('open-password-manager');
    else if (action === 'focus-address') setFocusRequest(value => value + 1);
    else if (action === 'find') { setFindOpen(true); setFindDirection(0); }
    else if (action === 'find-next') { setFindOpen(true); setFindDirection(value => value === 1 ? 0 : 1); window.setTimeout(() => setFindDirection(1), 0); }
    else if (action === 'find-previous') { setFindOpen(true); setFindDirection(value => value === -1 ? 0 : -1); window.setTimeout(() => setFindDirection(-1), 0); }
    else if (action === 'clear-data') openClearData();
    else if (action === 'settings') { setPanel(null); setSettingsOpen(true); }
    else if (action === 'toggle-sidebar') setSidebarOpen(value => !value);
    else if (action === 'toggle-favorites-bar') toggleFavoritesBar();
    else if (action === 'add-favorite') addFavorite();
    else if (action === 'save-all-favorites') {
      setFavorites(previous => {
        const additions = stateRef.current.tabs.filter(tab => /^https?:/i.test(tab.url) && !previous.some(item => item.url === tab.url))
          .map(tab => ({ id: createId(), title: tab.title || tab.url, url: tab.url }));
        const next = [...additions, ...previous];
        localStorage.setItem('aero.favorites', JSON.stringify(next));
        return next;
      });
    } else if (action === 'focus-favorites') favoritesBarRef.current?.querySelector('button')?.focus();
    else if (action === 'focus-toolbar') document.querySelector<HTMLElement>('[data-aero-toolbar] button')?.focus();
    else setPanel(action as BrowserPanelType);
  }, [addFavorite, command, toggleFavoritesBar, openClearData]);

  useEffect(() => {
    if (!window.aero) return;
    let mounted = true;
    void Promise.all([window.aero.getState(), window.aero.getSettings(), window.aero.getDownloads()]).then(([nextState, nextSettings, nextDownloads]) => {
      if (!mounted) return;
      if (nextState) setState(nextState);
      if (nextSettings) { setSettings(nextSettings); setSidebarWidths(nextSettings.layout); setSidebarOpen(nextSettings.appearance.sidebarOpen); }
      setDownloads(nextDownloads);
    });
    const unsubscribers = [
      window.aero.onState(nextState => { if (mounted) setState(nextState); }),
      window.aero.onSettings(nextSettings => { if (mounted) setSettings(nextSettings); }),
      window.aero.onShortcut(action => { if (mounted) handleShortcut(action); }),
      window.aero.onOpenPasswordManager(() => { if (mounted) void window.aero.openPasswordManager(); }),
      window.aero.onShortcutMappingChange(mapping => {
        if (mounted) setSettings(previous => {
          const next = { ...previous, shortcuts: { ...previous.shortcuts, mapping } };
          settingsRef.current = next;
          return next;
        });
      }),
      window.aero.onImportSummary(summary => {
        if (mounted) { setImportSummary(summary); setSettingsSection('import'); setSettingsOpen(true); }
      }),
      window.aero.onGpuSettingRestartRequired(disabled => {
        if (mounted) window.alert(`GPU acceleration ${disabled ? 'disabled' : 'enabled'} for the next launch. Restart Aero to apply this change.`);
      }),
      window.aero.onSaveAllFavorites(entries => {
        setFavorites(previous => {
          const additions = entries.filter(entry => !previous.some(item => item.url === entry.url)).map(entry => ({ id: createId(), ...entry }));
          const next = [...additions, ...previous];
          localStorage.setItem('aero.favorites', JSON.stringify(next));
          return next;
        });
      }),
      window.aero.onDownloads(nextDownloads => { if (mounted) setDownloads(nextDownloads); }),
      window.aero.onPrivateMode(value => { if (mounted) { privateModeRef.current = value; setPrivateMode(value); } }),
      window.aero.onBrowsingDataCleared(result => {
        if (!mounted || !result.types.includes('history')) return;
        setHistory(previous => {
          const next = retainOutsideRange(readLocal<HistoryEntry[]>('aero.history', previous), entry => entry.visitedAt, result);
          localStorage.setItem('aero.history', JSON.stringify(next));
          return next;
        });
      }),
      window.aero.onCredentialFillOffer(offer => { if (mounted) setCredentialSaveOffer(offer); }),
      window.aero.onCredentialFillReady(origin => { if (mounted) setCredentialFillOffer(origin); }),
      window.aero.onPasswordManagerLocked(() => {
        if (mounted) { setCredentialSaveOffer(null); setCredentialFillOffer(null); }
      }),
    ];
    return () => { mounted = false; unsubscribers.forEach(unsubscribe => unsubscribe()); };
  }, [handleShortcut]);

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => {
      const dark = settings.appearance.theme === 'dark' || (settings.appearance.theme === 'system' && media.matches);
      document.documentElement.classList.toggle('dark', dark);
      document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
    };
    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, [settings.appearance.theme]);

  useEffect(() => {
    if (privateModeRef.current !== false) return;
    const active = state.tabs.find(tab => tab.id === state.activeTabId);
    if (!active?.url || !/^https?:/i.test(active.url)) return;
    const visitKey = active.id + ':' + active.url;
    const newVisit = lastHistoryVisit.current !== visitKey;
    lastHistoryVisit.current = visitKey;
    setHistory(previous => {
      if (!newVisit) {
        const first = previous[0];
        if (!first || first.url !== active.url || first.title === active.title) return previous;
        const next = [{ ...first, title: active.title || active.url }, ...previous.slice(1)];
        localStorage.setItem('aero.history', JSON.stringify(next));
        return next;
      }
      const next = [{ id: createId(), title: active.title || active.url, url: active.url, visitedAt: Date.now() }, ...previous].slice(0, HISTORY_LIMIT);
      localStorage.setItem('aero.history', JSON.stringify(next));
      return next;
    });
  }, [state.activeTabId, state.tabs]);

  useEffect(() => {
    if (!resizeSide) return;
    const start = resizeStart.current;
    if (!start) return;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    const onMove = (event: MouseEvent) => {
      const direction = start.side === 'left' ? 1 : -1;
      const minimum = start.side === 'left' ? 180 : 230;
      const maximum = start.side === 'left' ? 400 : 500;
      setSidebarWidths(previous => ({ ...previous, [start.side === 'left' ? 'leftSidebarWidth' : 'rightSidebarWidth']: Math.min(maximum, Math.max(minimum, start.width + (event.clientX - start.x) * direction)) }));
    };
    const onUp = () => {
      const current = settingsRef.current;
      setSidebarWidths(widths => {
        const next = { ...current, layout: widths };
        settingsRef.current = next;
        void window.aero?.setSettings(next);
        return widths;
      });
      resizeStart.current = null;
      setResizeSide(null);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp, { once: true });
    return () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [resizeSide]);

  const beginResize = (side: 'left' | 'right') => (event: React.MouseEvent) => {
    event.preventDefault();
    resizeStart.current = { side, x: event.clientX, width: side === 'left' ? sidebarWidths.leftSidebarWidth : sidebarWidths.rightSidebarWidth };
    setResizeSide(side);
  };

  const applyImport = useCallback((selection: ImportSelection) => {
    if (!importSummary || importSummary.stage !== 'ready') return;
    const errors: Array<{kind: string; message: string}> = [];
    let bookmarksImported = 0, historyImported = 0;
    if (selection.bookmarks) {
      try {
        const result = mergeBookmarks(readLocal<FavoriteEntry[]>('aero.favorites', favorites), importSummary.bookmarks ?? [], createId);
        localStorage.setItem('aero.favorites', JSON.stringify(result.entries));
        setFavorites(result.entries); bookmarksImported = result.added;
      } catch { errors.push({kind: 'save-bookmarks', message: 'Bookmarks could not be saved. Aero’s local storage may be full. Existing bookmarks were kept.'}); }
    }
    if (selection.history) {
      try {
        const result = mergeHistory(readLocal<HistoryEntry[]>('aero.history', history), importSummary.history ?? [], createId);
        localStorage.setItem('aero.history', JSON.stringify(result.entries));
        setHistory(result.entries); historyImported = result.added;
      } catch (cause) { errors.push({kind: 'save-history', message: cause instanceof Error && cause.message.startsWith('Aero can keep') ? cause.message : 'History could not be saved. Aero’s local storage may be full. Existing history was kept.'}); }
    }
    setImportSummary({ ...importSummary, stage: errors.length ? 'ready' : 'imported', bookmarksImported, historyImported,
      message: errors.length ? `Added ${bookmarksImported} bookmarks and ${historyImported} history pages. Some selected data could not be saved; see the details below.` : `Added ${bookmarksImported} bookmarks and ${historyImported} history pages. Duplicates already in Aero were kept once.`,
      problems: [...importSummary.problems.filter(p => !p.kind.startsWith('save-')), ...errors] });
  }, [importSummary, favorites, history]);

  const openPasswordImport = useCallback(async () => {
    try { await window.aero.openPasswordManager(); }
    catch (cause) { window.alert(cause instanceof Error ? cause.message : 'Password Manager could not be opened.'); }
  }, []);

  const saveSubmittedCredential = async () => {
    try {
      const result = await window.aero.saveSubmittedCredential();
      if (result) window.alert('Password saved to the encrypted Aero vault.');
    } catch (cause) {
      window.alert(cause instanceof Error ? cause.message : 'The password could not be saved.');
    } finally {
      setCredentialSaveOffer(null);
      try { await window.aero.discardCredentialSubmission(); } catch { /* Navigation may have destroyed the shell while the prompt was open. */ }
    }
  };

  const fillSelectedCredential = async () => {
    try { await window.aero.fillSelectedCredential(); }
    catch (cause) { window.alert(cause instanceof Error ? cause.message : 'The selected credential could not be filled.'); }
    finally { setCredentialFillOffer(null); }
  };

  const overlayOpen = settingsOpen || Boolean(panel) || Boolean(credentialSaveOffer) || Boolean(credentialFillOffer);
  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport || !window.aero) return;
    const update = () => {
      const bounds = viewport.getBoundingClientRect();
      window.aero.setBounds({ x: bounds.x, y: bounds.y, width: bounds.width, height: overlayOpen || tabDragging || tabMenuOpen ? 0 : bounds.height });
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(viewport);
    window.addEventListener('resize', update);
    const frame = requestAnimationFrame(update);
    return () => { cancelAnimationFrame(frame); observer.disconnect(); window.removeEventListener('resize', update); };
  }, [settings.appearance.verticalTabs, tabsCollapsed, sidebarOpen, settings.appearance.showFavoritesBar, findOpen, overlayOpen, tabDragging, tabMenuOpen, state.splitTabId, sidebarWidths]);

  const openPrivacy = useCallback(() => { setSidebarTool('privacy'); setSidebarOpen(true); }, []);
  const fire = openClearData;

  const removeHistory = (id: string) => setHistory(previous => { const next = previous.filter(item => item.id !== id); localStorage.setItem('aero.history', JSON.stringify(next)); return next; });
  const clearHistory = openClearData;
  const removeFavorite = (id: string) => setFavorites(previous => { const next = previous.filter(item => item.id !== id); localStorage.setItem('aero.favorites', JSON.stringify(next)); return next; });
  const chooseDownloadDirectory = async () => {
    const directory = await window.aero.chooseDownloadDirectory();
    if (directory) saveSettings({ ...settings, downloads: { ...settings.downloads, directory } });
  };

  const startImport = useCallback(async (source: ImportSource, profileId?: string, manual = false) => {
    const requestId = ++importRequest.current;
    setSettingsSection('import');
    setSettingsOpen(true);
    setImportHelpError('');
    setImportSummary({ source, found: false, stage: 'scanning', message: profileId || manual ? 'Reading the selected profile…' : 'Detecting browser profiles and reading available data…', problems: [] });
    if (!window.aero?.importSummary) return;
    try {
      const summary = await window.aero.importSummary({ source, action: manual ? 'browse' : profileId ? 'read' : 'discover', profileId });
      if (requestId === importRequest.current) setImportSummary(summary);
    } catch (cause) {
      if (requestId === importRequest.current) setImportSummary({ source, profileId, found: false, stage: 'failed', message: cause instanceof Error ? cause.message : 'The profile could not be read.', problems: [] });
    }
  }, []);
  const openSourcePasswords = useCallback(async () => {
    if (!importSummary) return;
    setImportHelpError('');
    try { await window.aero.importSummary({ source: importSummary.source, action: 'password-settings', profileId: importSummary.profileName ? importSummary.profileId : undefined }); }
    catch (cause) { setImportHelpError(cause instanceof Error ? cause.message : 'Open the source browser’s Passwords settings manually.'); }
  }, [importSummary]);

  const resetImport = useCallback(() => {
    ++importRequest.current;
    setImportSummary(null);
    setImportHelpError('');
  }, []);

  const openImport = useCallback(() => {
    setSettingsSection('import');
    setImportSummary(null);
    setSettingsOpen(true);
  }, []);

  return (
    <div className="relative flex h-screen w-screen flex-col overflow-hidden bg-[#f4f7fb] text-slate-800 dark:bg-[#17191d] dark:text-slate-100">
      <Header state={state} verticalTabs={settings.appearance.verticalTabs} onToggleVerticalTabs={() => saveSettings({ ...settings, appearance: { ...settings.appearance, verticalTabs: !settings.appearance.verticalTabs } })} command={command} onError={message => window.alert(message)} onDragChange={setTabDragging} onMenuChange={setTabMenuOpen} />
      {privateMode && <div className="pointer-events-none absolute left-1/2 top-1 z-[111] -translate-x-1/2 rounded-full bg-violet-600 px-3 py-1 text-[10px] font-semibold text-white">PRIVATE WINDOW</div>}
      {credentialSaveOffer && !privateMode && <div role="status" className="absolute right-4 top-12 z-50 max-w-md rounded-xl border border-sky-200 bg-white p-4 shadow-2xl dark:border-sky-900 dark:bg-zinc-900"><p className="text-sm font-semibold">Save or update password for {credentialSaveOffer.origin}?</p><p className="mt-1 truncate text-xs text-slate-500">Username: {credentialSaveOffer.username}</p><div className="mt-3 flex justify-end gap-2"><button className="secondary-button" onClick={() => { setCredentialSaveOffer(null); void window.aero.discardCredentialSubmission(); }}>Not now</button><button className="primary-button" onClick={() => void saveSubmittedCredential()}>Save password</button></div></div>}
      {credentialFillOffer && !privateMode && <div role="status" className="absolute right-4 top-12 z-50 max-w-md rounded-xl border border-sky-200 bg-white p-4 shadow-2xl dark:border-sky-900 dark:bg-zinc-900"><p className="text-sm font-semibold">Credential selected for {credentialFillOffer}</p><p className="mt-1 text-xs text-slate-500">Aero will request Windows Hello, then fill only the matching secure page.</p><div className="mt-3 flex justify-end gap-2"><button className="secondary-button" onClick={() => setCredentialFillOffer(null)}>Cancel</button><button className="primary-button" onClick={() => void fillSelectedCredential()}>Authenticate &amp; fill</button></div></div>}
      <div className="flex min-h-0 flex-1">
        {settings.appearance.verticalTabs && <div className="relative flex shrink-0" style={{ width: tabsCollapsed ? 58 : sidebarWidths.leftSidebarWidth }}><VerticalTabs state={state} collapsed={tabsCollapsed} onToggleCollapsed={() => setTabsCollapsed(value => !value)} width={sidebarWidths.leftSidebarWidth} command={command} onDragChange={setTabDragging} onMenuChange={setTabMenuOpen} />{!tabsCollapsed && <div role="separator" aria-label="Resize tab sidebar" aria-orientation="vertical" onMouseDown={beginResize('left')} className="absolute right-[-3px] top-0 z-30 h-full w-1.5 cursor-col-resize hover:bg-sky-500/50" />}</div>}
        <main className="flex min-w-0 flex-1 flex-col">
          <Omnibox state={state} settings={settings} focusRequest={focusRequest} suggestions={addressSuggestions} fireActive={fireActive} onFire={fire} onOpenPrivacy={openPrivacy} onToggleSidebar={() => setSidebarOpen(value => !value)} onOpenSettings={() => setSettingsOpen(true)} onAddFavorite={addFavorite} command={command} />
          {settings.appearance.showFavoritesBar && <FavoritesBar ref={favoritesBarRef} favorites={favorites} command={command} />}
          {findOpen && <FindBar requestDirection={findDirection} onClose={() => setFindOpen(false)} command={command} />}
          <SplitView ref={viewportRef} state={state} dragging={tabDragging} command={command} onDragChange={setTabDragging} />
        </main>
        <Sidebar state={state} open={sidebarOpen && !overlayOpen} width={sidebarWidths.rightSidebarWidth} onResizeStart={beginResize('right')} tool={sidebarTool} onTool={setSidebarTool} onClose={() => setSidebarOpen(false)} command={command} />
      </div>

      {settingsOpen && <SettingsPanel section={settingsSection} onSectionChange={setSettingsSection} clearRequest={clearRequest} privateMode={privateMode} onClearBrowsingData={clearBrowsingData} importContent={<ImportSection summary={importSummary} helpError={importHelpError} onStart={startImport} onReset={resetImport} onApply={applyImport} onSourcePasswords={() => void openSourcePasswords()} onImportPasswords={() => void openPasswordImport()} />} settings={settings} onChange={saveSettings} onReset={() => void window.aero.resetSettings().then(next => { setSettings(next); setSidebarWidths(next.layout); setSidebarOpen(next.appearance.sidebarOpen); })} onChooseDownloadDirectory={() => void chooseDownloadDirectory()} onOpenImport={openImport} onClose={() => setSettingsOpen(false)} />}
      {panel && <BrowserPanel type={panel} state={state} history={history} favorites={favorites} downloads={downloads} onRemoveHistory={removeHistory} onClearHistory={clearHistory} onClearBrowsingData={clearBrowsingData} onRemoveFavorite={removeFavorite} onClose={() => setPanel(null)} command={command} />}
    </div>
  );
};

export default App;
