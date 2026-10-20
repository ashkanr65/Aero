import React, { useEffect, useMemo, useRef, useState } from 'react';
import { ChevronLeft, ChevronRight, Columns2, Flame, Home, Lock, MoreHorizontal, PanelRight, RefreshCw, Shield, ShieldAlert, Star } from 'lucide-react';
import type { BrowserState } from '../types';
import type { BrowserSettings } from '../../../shared/settings';

interface OmniboxProps {
  state: BrowserState;
  fireActive: boolean;
  onFire(): void;
  onOpenPrivacy(): void;
  onToggleSidebar(): void;
  onOpenSettings(): void;
  onAddFavorite(): void;
  settings: BrowserSettings;
  focusRequest: number;
  suggestions: Array<{ title: string; url: string }>;
  command(command: string, payload?: unknown): void;
}

export const Omnibox: React.FC<OmniboxProps> = ({ state, fireActive, onFire, onOpenPrivacy, onToggleSidebar, onOpenSettings, onAddFavorite, settings, focusRequest, suggestions, command }) => {
  const activeTab = state.tabs.find(tab => tab.id === state.activeTabId);
  const [address, setAddress] = useState(activeTab?.url ?? '');
  const [focused, setFocused] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const formRef = useRef<HTMLFormElement>(null);
  const blurTimer = useRef<number>();
  const selectOnFocus = useRef(false);
  const lastTab = useRef(state.activeTabId);
  const lastSuggestionRequest = useRef('');
  useEffect(() => {
    if (!focused || lastTab.current !== state.activeTabId) setAddress(activeTab?.url ?? '');
    lastTab.current = state.activeTabId;
  }, [activeTab?.url, state.activeTabId, focused]);
  useEffect(() => { if (focusRequest) { inputRef.current?.focus(); inputRef.current?.select(); } }, [focusRequest]);

  const dismiss = () => {
    window.clearTimeout(blurTimer.current);
    setFocused(false);
    inputRef.current?.blur();
    lastSuggestionRequest.current = '';
    command('hide-suggestions');
  };
  const submit = (value = address) => { dismiss(); command('navigate', value); };
  const completeDotCom = () => {
    const input = address.trim().replace(/^https?:\/\//i, '').replace(/^www\./i, '').replace(/\/$/, '');
    const completed = input.includes('.') || /\s/.test(input) ? address : `www.${input}.com`;
    setAddress(completed);
    submit(completed);
  };
  const matchingSuggestions = useMemo(() => settings.search.suggestions && focused
    ? suggestions.filter(item => `${item.title} ${item.url}`.toLowerCase().includes(address.toLowerCase())).filter((item, index, all) => all.findIndex(other => other.url === item.url) === index).slice(0, 5)
    : [], [address, focused, settings.search.suggestions, suggestions]);
  // Compare row content: browser state events can replace arrays without changing rows.
  const suggestionKey = JSON.stringify(matchingSuggestions);

  useEffect(() => {
    let frame = 0;
    const update = () => {
      const form = formRef.current;
      const items = JSON.parse(suggestionKey) as Array<{ title: string; url: string }>;
      if (!focused || !form || items.length === 0) {
        if (lastSuggestionRequest.current) command('hide-suggestions');
        lastSuggestionRequest.current = '';
        return;
      }
      const bounds = form.getBoundingClientRect();
      const payload = {
        bounds: { x: bounds.x, y: bounds.bottom + 5, width: bounds.width },
        items,
      };
      const request = JSON.stringify(payload);
      if (request !== lastSuggestionRequest.current) {
        lastSuggestionRequest.current = request;
        command('show-suggestions', payload);
      }
    };
    const schedule = () => { cancelAnimationFrame(frame); frame = requestAnimationFrame(update); };
    schedule();
    const observer = new ResizeObserver(schedule);
    if (formRef.current) observer.observe(formRef.current);
    window.addEventListener('resize', schedule);
    return () => { cancelAnimationFrame(frame); observer.disconnect(); window.removeEventListener('resize', schedule); };
  }, [command, focused, suggestionKey]);

  useEffect(() => () => { window.clearTimeout(blurTimer.current); command('hide-suggestions'); }, [command]);

  return (
    <div className="relative z-10 shrink-0">
    <div data-aero-toolbar className="flex h-12 items-center gap-1.5 border-b border-slate-200 bg-white/95 px-2 dark:border-zinc-800 dark:bg-[#25272c]">
      <button className="icon-button" disabled={!state.canGoBack} onClick={() => command('back')} title="Back"><ChevronLeft size={18} /></button>
      <button className="icon-button" disabled={!state.canGoForward} onClick={() => command('forward')} title="Forward"><ChevronRight size={18} /></button>
      <button className="icon-button" onClick={() => command('reload')} title="Reload"><RefreshCw size={16} className={activeTab?.loading ? 'animate-spin' : ''} /></button>
      {settings.appearance.showHomeButton && <button className="icon-button" onClick={() => command('home')} title="Home (Alt+Home)"><Home size={16} /></button>}

      <form ref={formRef} className="mx-1 flex min-w-0 flex-1 items-center rounded-full border border-slate-300 bg-slate-50 px-3 shadow-inner focus-within:border-sky-500 focus-within:ring-2 focus-within:ring-sky-500/20 dark:border-zinc-700 dark:bg-[#17191d]" onSubmit={event => { event.preventDefault(); submit(); }}>
        <button type="button" onClick={onOpenPrivacy} className="flex items-center gap-1 border-r border-slate-200 pr-2 text-emerald-600 dark:border-zinc-700" title="Privacy dashboard">
          <Shield size={15} /><span className="text-[10px] font-bold">A</span>
        </button>
        <Lock size={13} className="ml-2 shrink-0 text-slate-400" />
        <input ref={inputRef} value={address} onChange={event => setAddress(event.target.value)} onPointerDown={() => { selectOnFocus.current = document.activeElement !== inputRef.current; }} onFocus={event => { window.clearTimeout(blurTimer.current); setFocused(true); if (selectOnFocus.current) event.currentTarget.select(); selectOnFocus.current = false; }} onBlur={() => { window.clearTimeout(blurTimer.current); blurTimer.current = window.setTimeout(() => setFocused(false), 220); }} onKeyDown={event => { if (event.ctrlKey && event.key === 'Enter') { event.preventDefault(); completeDotCom(); } else if (event.key === 'Escape') { dismiss(); } }} className="h-8 min-w-0 flex-1 bg-transparent px-2 text-sm outline-none" spellCheck={false} aria-label="Address or search" />
        <button type="button" onClick={onAddFavorite} className="mr-2 text-slate-400 hover:text-amber-500" title="Add favorite (Ctrl+D)"><Star size={16} /></button>
        <button type="button" onClick={onOpenPrivacy} className="relative text-orange-500" title="Aero Shields">
          <ShieldAlert size={17} />
          {state.stats.blocked > 0 && <span className="absolute -right-2 -top-2 rounded-full bg-orange-500 px-1 text-[8px] font-bold text-white">{Math.min(99, state.stats.blocked)}</span>}
        </button>
      </form>

      <button className={`icon-button text-red-500 ${fireActive ? 'animate-pulse bg-red-100' : ''}`} onClick={onFire} title="Fire Button: clear browsing data"><Flame size={17} /></button>
      <button className={`icon-button ${state.splitTabId ? 'bg-sky-100 text-sky-700 dark:bg-sky-950' : ''}`} onClick={() => command('toggle-split')} title="Split screen"><Columns2 size={17} /></button>
      <button className="icon-button" onClick={onToggleSidebar} title="Aero Sidebar"><PanelRight size={17} /></button>
      <button className="icon-button" onClick={onOpenSettings} title="Settings and more (Alt+F)"><MoreHorizontal size={18} /></button>
    </div>
    </div>
  );
};
