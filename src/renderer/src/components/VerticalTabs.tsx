import React, { useState } from 'react';
import { Globe, Leaf, PanelLeft, Plus, Volume2, VolumeX, X } from 'lucide-react';
import type { BrowserState } from '../types';
import { useEffect } from 'react';
import { TabContextMenu } from './TabContextMenu';

interface VerticalTabsProps {
  state: BrowserState;
  collapsed: boolean;
  width: number;
  onToggleCollapsed(): void;
  command(command: string, payload?: unknown): void;
  onDragChange(dragging: boolean): void;
  onMenuChange(open: boolean): void;
}

export const VerticalTabs: React.FC<VerticalTabsProps> = ({ state, collapsed, width, onToggleCollapsed, command, onDragChange, onMenuChange }) => {
  const [contextMenu, setContextMenu] = useState<{ id: number; x: number; y: number } | null>(null);

  const beginDrag = (event: React.DragEvent, id: number) => {
    event.dataTransfer.setData('application/x-aero-tab-id', String(id));
    event.dataTransfer.effectAllowed = 'move';
    window.setTimeout(() => onDragChange(true), 0);
  };
  useEffect(() => { onMenuChange(contextMenu !== null); return () => onMenuChange(false); }, [contextMenu, onMenuChange]);
  const dragToPane = (event: React.DragEvent) => {
    if (!event.dataTransfer.types.includes('application/x-aero-tab-id')) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  };
  const dropOnTabs = (event: React.DragEvent) => {
    event.preventDefault();
    const id = Number(event.dataTransfer.getData('application/x-aero-tab-id'));
    if (!Number.isInteger(id) || id <= 0) return;
    command('reorder-tab', { id, beforeId: null });
    onDragChange(false);
  };
  const dropBeforeTab = (event: React.DragEvent, beforeId: number) => {
    event.preventDefault();
    event.stopPropagation();
    const id = Number(event.dataTransfer.getData('application/x-aero-tab-id'));
    if (Number.isInteger(id) && id > 0) command('reorder-tab', { id, beforeId });
    onDragChange(false);
  };

  return (
    <aside style={{ width: collapsed ? 58 : width }} className="relative flex h-full shrink-0 flex-col overflow-hidden border-r border-slate-200 bg-slate-50/95 p-2 dark:border-zinc-800 dark:bg-[#1e2025]" onDragOver={dragToPane} onDrop={dropOnTabs}>
      <div className="mb-2 flex gap-1">
        <button className="primary-button flex-1" onClick={() => command('new-tab')} title="New tab"><Plus size={15} /><span className={collapsed ? 'hidden' : ''}>New tab</span></button>
        {!collapsed && <button className="icon-button" onClick={onToggleCollapsed} title="Collapse tabs"><PanelLeft size={15} /></button>}
      </div>
      <div className="flex-1 space-y-1 overflow-y-auto">
        {state.tabs.map(tab => (
          <button key={tab.id} onClick={() => command('activate-tab', tab.id)}
            onContextMenu={event => { event.preventDefault(); setContextMenu({ id: tab.id, x: event.clientX, y: event.clientY }); }}
            onDragOver={dragToPane}
            onDrop={event => dropBeforeTab(event, tab.id)}
            draggable onDragStart={event => beginDrag(event, tab.id)} onDragEnd={() => onDragChange(false)}
            title={`${tab.title}${tab.audible ? ' — Playing audio' : ''}${tab.muted ? ' — Muted' : ''}`}
            className={`group flex h-10 w-full items-center gap-2 rounded-lg px-2 text-left text-xs ${tab.id === state.activeTabId ? 'bg-white shadow-sm ring-1 ring-slate-200 dark:bg-zinc-700 dark:ring-zinc-600' : 'hover:bg-white/80 dark:hover:bg-zinc-800'}`}>
            {!(collapsed && (tab.audible || tab.muted)) && (tab.favicon ? <img src={tab.favicon} className="h-4 w-4 shrink-0" /> : <Globe size={16} className="shrink-0 text-sky-500" />)}
            {!collapsed && <span className="min-w-0 flex-1 truncate">{tab.title}</span>}
            {!collapsed && tab.discarded && <Leaf size={13} className="shrink-0 text-emerald-500" />}
            {(tab.audible || tab.muted) && <span role="button" tabIndex={0} aria-label={tab.muted ? `Unmute ${tab.title}` : `Mute ${tab.title}`} onClick={event => { event.stopPropagation(); command('toggle-mute', tab.id); }} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); event.stopPropagation(); command('toggle-mute', tab.id); } }} className="rounded p-1 hover:bg-slate-200 dark:hover:bg-zinc-600">{tab.muted ? <VolumeX size={13} /> : <Volume2 size={13} />}</span>}
            {!collapsed && <span onClick={event => { event.stopPropagation(); command('close-tab', tab.id); }} className="rounded p-1 opacity-0 hover:bg-slate-200 group-hover:opacity-100 dark:hover:bg-zinc-600"><X size={13} /></span>}
          </button>
        ))}
      </div>
      {collapsed && <button className="icon-button mx-auto mt-2" onClick={onToggleCollapsed} title="Expand tabs"><PanelLeft size={15} /></button>}
      <TabContextMenu tab={contextMenu} state={state} command={command} close={() => setContextMenu(null)} />
    </aside>
  );
};
