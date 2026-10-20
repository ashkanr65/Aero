import { ClearBrowsingData } from './ClearBrowsingData';
import type { ClearDataRequest } from '../../../shared/browsing-data';
import React, { useMemo, useState } from 'react';
import { Clock3, Download, Flame, FolderOpen, Heart, MemoryStick, NotebookTabs, Search, Trash2, X } from 'lucide-react';
import type { BrowserState, DownloadRecord } from '../types';

export type BrowserPanelType = 'history' | 'downloads' | 'favorites' | 'tab-search' | 'memory' | 'clear-data';
export interface HistoryEntry { id: string; title: string; url: string; visitedAt: number; }
export interface FavoriteEntry { id: string; title: string; url: string; }

interface BrowserPanelProps {
  type: BrowserPanelType;
  state: BrowserState;
  history: HistoryEntry[];
  favorites: FavoriteEntry[];
  downloads: DownloadRecord[];
  onRemoveHistory(id: string): void;
  onClearHistory(): void;
  onClearBrowsingData(request: ClearDataRequest): Promise<void>;
  onRemoveFavorite(id: string): void;
  onClose(): void;
  command(command: string, payload?: unknown): void;
}

const titles: Record<BrowserPanelType, string> = {
  history: 'History', downloads: 'Downloads', favorites: 'Favorites',
  'tab-search': 'Search tabs', memory: 'Aero RAM Manager', 'clear-data': 'Clear browsing data',
};

export const BrowserPanel: React.FC<BrowserPanelProps> = props => {
  const [filter, setFilter] = useState('');
  const normalizedFilter = filter.toLowerCase();
  const history = useMemo(() => props.history.filter(item => `${item.title} ${item.url}`.toLowerCase().includes(normalizedFilter)), [props.history, normalizedFilter]);
  const favorites = useMemo(() => props.favorites.filter(item => `${item.title} ${item.url}`.toLowerCase().includes(normalizedFilter)), [props.favorites, normalizedFilter]);
  const tabs = useMemo(() => props.state.tabs.filter(item => `${item.title} ${item.url}`.toLowerCase().includes(normalizedFilter)), [props.state.tabs, normalizedFilter]);
  const open = (url: string) => { props.command('new-tab', url); props.onClose(); };

  return <div className="absolute inset-x-0 bottom-0 top-9 z-30 bg-slate-950/20 p-8 backdrop-blur-sm" onMouseDown={event => { if (event.currentTarget === event.target) props.onClose(); }}>
    <section className="mx-auto flex max-h-full w-full max-w-3xl flex-col overflow-hidden rounded-2xl border border-white/40 bg-white shadow-2xl dark:border-zinc-700 dark:bg-[#202228]">
      <header className="flex h-14 shrink-0 items-center gap-3 border-b border-slate-200 px-5 dark:border-zinc-800">
        <PanelIcon type={props.type} /><h1 className="font-semibold">{titles[props.type]}</h1><button className="icon-button ml-auto" onClick={props.onClose}><X size={18} /></button>
      </header>

      {!['memory', 'clear-data'].includes(props.type) && <div className="border-b border-slate-200 p-4 dark:border-zinc-800"><label className="flex items-center gap-2 rounded-xl bg-slate-100 px-3 dark:bg-zinc-800"><Search size={16} className="text-slate-400" /><input autoFocus value={filter} onChange={event => setFilter(event.target.value)} className="h-9 min-w-0 flex-1 bg-transparent text-sm outline-none" placeholder={`Search ${titles[props.type].toLowerCase()}`} /></label></div>}

      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {props.type === 'history' && <>
          <div className="mb-3 flex justify-end"><button className="secondary-button" onClick={props.onClearHistory}><Trash2 size={14} />Clear history</button></div>
          {history.map(item => <div key={item.id} className="browser-list-row"><button className="min-w-0 flex-1 text-left" onClick={() => open(item.url)}><strong>{item.title}</strong><small>{item.url}</small></button><time>{new Date(item.visitedAt).toLocaleString()}</time><button className="icon-button" onClick={() => props.onRemoveHistory(item.id)}><X size={14} /></button></div>)}
          {!history.length && <Empty label="No matching history" />}
        </>}
        {props.type === 'favorites' && <>
          {favorites.map(item => <div key={item.id} className="browser-list-row"><Heart size={16} className="fill-amber-400 text-amber-400" /><button className="min-w-0 flex-1 text-left" onClick={() => open(item.url)}><strong>{item.title}</strong><small>{item.url}</small></button><button className="icon-button" onClick={() => props.onRemoveFavorite(item.id)}><X size={14} /></button></div>)}
          {!favorites.length && <Empty label="Press Ctrl+D to save the current page" />}
        </>}
        {props.type === 'downloads' && <>
          <div className="mb-3 flex justify-end"><button className="secondary-button" onClick={() => props.command('open-downloads-folder')}><FolderOpen size={14} />Open downloads folder</button></div>
          {props.downloads.filter(item => item.filename.toLowerCase().includes(normalizedFilter)).map(item => <div key={item.id} className="browser-list-row"><Download size={17} className="text-sky-600" /><div className="min-w-0 flex-1"><strong>{item.filename}</strong><small>{item.state}{item.totalBytes > 0 ? ` · ${Math.round(item.receivedBytes / item.totalBytes * 100)}%` : ''}</small></div></div>)}
          {!props.downloads.length && <Empty label="Downloaded files will appear here" />}
        </>}
        {props.type === 'tab-search' && <>
          {tabs.map(tab => <button key={tab.id} className="browser-list-row w-full text-left" onClick={() => { props.command('activate-tab', tab.id); props.onClose(); }}><NotebookTabs size={16} className="text-sky-600" /><span className="min-w-0 flex-1"><strong>{tab.title}</strong><small>{tab.url}</small></span>{tab.discarded && <em>Sleeping</em>}</button>)}
        </>}
        {props.type === 'memory' && <div className="space-y-4">
          <div className="grid grid-cols-2 gap-3"><div className="memory-stat"><strong>{props.state.memory.activeViews}</strong><span>Live renderer processes</span></div><div className="memory-stat"><strong>{props.state.memory.discardedTabs}</strong><span>Sleeping tabs</span></div></div>
          <p className="rounded-xl bg-emerald-50 p-4 text-sm text-emerald-800 dark:bg-emerald-950 dark:text-emerald-200">Sleeping a tab closes its Chromium renderer to release RAM. Selecting it restores the page automatically.</p>
          <button className="primary-button" onClick={() => props.command('discard-other-tabs')}><MemoryStick size={15} />Sleep all inactive tabs now</button>
          {props.state.tabs.filter(tab => tab.id !== props.state.activeTabId).map(tab => <div key={tab.id} className="browser-list-row"><span className="min-w-0 flex-1"><strong>{tab.title}</strong><small>{tab.discarded ? 'Sleeping — RAM released' : 'Active in memory'}</small></span>{!tab.discarded && <button className="secondary-button" onClick={() => props.command('discard-tab', tab.id)}>Sleep</button>}</div>)}
        </div>}
        {props.type === 'clear-data' && <ClearBrowsingData onClear={props.onClearBrowsingData} />}
      </div>
    </section>
  </div>;
};

const Empty: React.FC<{ label: string }> = ({ label }) => <div className="py-16 text-center text-sm text-slate-400">{label}</div>;
const PanelIcon: React.FC<{ type: BrowserPanelType }> = ({ type }) => type === 'history' ? <Clock3 size={19} /> : type === 'downloads' ? <Download size={19} /> : type === 'favorites' ? <Heart size={19} /> : type === 'memory' ? <MemoryStick size={19} /> : type === 'clear-data' ? <Flame size={19} /> : <NotebookTabs size={19} />;
