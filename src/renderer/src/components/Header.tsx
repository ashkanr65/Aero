import React, { useEffect, useRef, useState } from 'react';
import { Globe, PanelLeft, Plus, X, Volume2, VolumeX } from 'lucide-react';
import type { BrowserState } from '../types';
import { TabContextMenu, TAB_DRAG_TYPE } from './TabContextMenu';

interface HeaderProps {
  state: BrowserState;
  verticalTabs: boolean;
  onToggleVerticalTabs(): void;
  command(command: string, payload?: unknown): void;
  onError(message: string): void;
  onDragChange(dragging: boolean): void;
  onMenuChange(open: boolean): void;
}

export const Header: React.FC<HeaderProps> = ({ state, verticalTabs, onToggleVerticalTabs, command, onDragChange, onMenuChange }) => {
  const [contextTabId, setContextTabId] = useState<{ id: number; x: number; y: number } | null>(null);
  const tabsAreaRef = useRef<HTMLDivElement>(null);
  const [tabsAreaWidth, setTabsAreaWidth] = useState(0);
  useEffect(() => {
    const element = tabsAreaRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setTabsAreaWidth(element.clientWidth));
    observer.observe(element);
    setTabsAreaWidth(element.clientWidth);
    return () => observer.disconnect();
  }, [verticalTabs]);
  useEffect(() => { onMenuChange(contextTabId !== null); return () => onMenuChange(false); }, [contextTabId, onMenuChange]);
  const tabWidth = Math.min(240, Math.max(0, (tabsAreaWidth - 36) / Math.max(1, state.tabs.length)));
  const startDrag = (event: React.DragEvent, id: number) => {
    event.dataTransfer.setData(TAB_DRAG_TYPE, String(id));
    event.dataTransfer.effectAllowed = 'move';
    // Delay detaching native views until the browser captures the dragged tab.
    window.setTimeout(() => onDragChange(true), 0);
  };
  const dropTab = (event: React.DragEvent, beforeId: number | null) => {
    event.preventDefault(); event.stopPropagation();
    const id = Number(event.dataTransfer.getData(TAB_DRAG_TYPE));
    if (Number.isInteger(id) && id > 0) command('reorder-tab', { id, beforeId });
    onDragChange(false);
  };

  return <header className="drag relative z-[110] flex h-9 shrink-0 items-center border-b border-slate-200/80 bg-white/85 pl-2 dark:border-zinc-800 dark:bg-[#17191d]/95">
    <div className="flex items-center gap-2 pr-3">
      <button className="no-drag icon-button" onClick={onToggleVerticalTabs} title="Toggle vertical tabs">
        <PanelLeft size={16} />
      </button>
      <div className="flex items-center gap-2 font-semibold tracking-[0.18em] text-sky-600">
        <img src="./icon.png" alt="" className="h-5 w-5 rounded-md object-cover" draggable={false} />
        <span className="text-[11px]">AERO</span>
      </div>
    </div>

    {!verticalTabs && (
      <div ref={tabsAreaRef} className="drag relative flex min-w-0 flex-1 self-stretch overflow-hidden pt-1" onDragOver={event => { if (event.dataTransfer.types.includes('application/x-aero-tab-id')) event.preventDefault(); }} onDrop={event => dropTab(event, null)}>
        {state.tabs.map(tab => (
          <button
            key={tab.id}
            title={`${tab.title}${tab.audible ? ' — Playing audio' : ''}${tab.muted ? ' — Muted' : ''}`}
            style={{ flex: `0 0 ${tabWidth}px`, width: tabWidth, minWidth: 0, borderBottom: tab.id === state.leftTabId ? '2px solid #0ea5e9' : tab.id === state.rightTabId ? '2px solid #a855f7' : undefined }}
            onClick={() => command('activate-tab', tab.id)}
            onContextMenu={event => { event.preventDefault(); setContextTabId({ id: tab.id, x: event.clientX, y: event.clientY }); }}
            onDragStart={event => startDrag(event, tab.id)}
            onDragEnd={() => onDragChange(false)}
            onDragOver={event => { if (event.dataTransfer.types.includes('application/x-aero-tab-id')) event.preventDefault(); }}
            onDrop={event => dropTab(event, tab.id)}
            draggable
            className={`no-drag group flex items-center gap-1 overflow-hidden rounded-t-lg px-1 text-left text-xs ${tab.id === state.activeTabId ? 'bg-slate-100 dark:bg-zinc-800' : 'hover:bg-slate-100/70 dark:hover:bg-zinc-800/60'}`}
          >
            {(tab.audible || tab.muted) ? <span role="button" tabIndex={0} aria-label={tab.muted ? `Unmute ${tab.title}` : `Mute ${tab.title}`} onClick={event => { event.stopPropagation(); command('toggle-mute', tab.id); }} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); event.stopPropagation(); command('toggle-mute', tab.id); } }} className="mx-auto flex h-5 w-5 shrink-0 items-center justify-center rounded hover:bg-slate-200 dark:hover:bg-zinc-700">{tab.muted ? <VolumeX size={12} /> : <Volume2 size={12} />}</span> : tab.favicon ? <img src={tab.favicon} className="mx-auto h-4 max-w-full shrink object-contain" /> : <Globe size={14} className="mx-auto min-w-0 shrink" />}
            {tabWidth >= 72 && <span className="min-w-0 flex-1 truncate">{tab.title}</span>}
            {tabWidth >= 92 && <span onClick={event => { event.stopPropagation(); command('close-tab', tab.id); }} className="shrink-0 rounded p-0.5 opacity-0 hover:bg-slate-200 group-hover:opacity-100 dark:hover:bg-zinc-700"><X size={12} /></span>}
          </button>
        ))}
        <TabContextMenu tab={contextTabId} state={state} command={command} close={() => setContextTabId(null)} />

        <button className="no-drag icon-button mt-0.5 shrink-0" onClick={() => command('new-tab')} title="New tab"><Plus size={15} /></button>
      </div>
    )}

    <div className={`drag h-full min-w-[64px] ${verticalTabs ? 'flex-1' : 'w-16 shrink-0'}`} title="Drag to move Aero" />
    <div className="no-drag ml-auto flex h-full items-stretch text-slate-600 dark:text-zinc-300">
      <button className="window-button" onClick={() => command('minimize')} aria-label="Minimize">&#8212;</button>
      <button className="window-button" onClick={() => command('maximize')} aria-label="Maximize">&#9633;</button>
      <button className="window-button hover:!bg-red-500 hover:!text-white" onClick={() => command('close-window')} aria-label="Close">&#10005;</button>
    </div>
  </header>;
};
