import React, { useEffect } from 'react';
import { createPortal } from 'react-dom';
import type { BrowserState } from '../types';

export const TAB_DRAG_TYPE = 'application/x-aero-tab-id';
export function draggedTab(event: React.DragEvent): number | null {
  const id = Number(event.dataTransfer.getData(TAB_DRAG_TYPE));
  return Number.isInteger(id) && id > 0 ? id : null;
}

export const TabContextMenu: React.FC<{
  tab: { id: number; x: number; y: number } | null;
  state: BrowserState;
  command(name: string, payload?: unknown): void;
  close(): void;
}> = ({ tab, state, command, close }) => {
  useEffect(() => {
    if (!tab) return;
    const dismiss = (event: KeyboardEvent) => { if (event.key === 'Escape') close(); };
    window.addEventListener('keydown', dismiss);
    return () => window.removeEventListener('keydown', dismiss);
  }, [tab, close]);
  if (!tab) return null;
  const act = (name: string, payload?: unknown) => { command(name, payload); close(); };
  const button = 'w-full rounded px-3 py-2 text-left text-xs hover:bg-slate-100 dark:hover:bg-zinc-800';
  return createPortal(<div className="no-drag fixed inset-x-0 bottom-0 top-9 z-[100]" onMouseDown={close}>
    <div role="menu" className="fixed min-w-52 rounded-lg border border-slate-200 bg-white p-1 shadow-xl dark:border-zinc-700 dark:bg-zinc-900" style={{ left: Math.max(8, Math.min(tab.x, window.innerWidth - 224)), top: Math.max(36, Math.min(tab.y, window.innerHeight - 190)) }} onMouseDown={event => event.stopPropagation()}>
      <button role="menuitem" className={button} onClick={() => act('activate-tab', tab.id)}>Open tab</button>
      {tab.id !== state.leftTabId && <button role="menuitem" className={button} onClick={() => act('move-tab-to-pane', { id: tab.id, pane: 'left' })}>{tab.id === state.rightTabId ? 'Move to left pane' : 'Open in left pane'}</button>}
      {tab.id !== state.rightTabId && <button role="menuitem" className={button} onClick={() => act('move-tab-to-pane', { id: tab.id, pane: 'right' })}>{tab.id === state.leftTabId && state.splitTabId ? 'Move to right pane' : 'Open in right pane'}</button>}
      <button role="menuitem" className={button} onClick={() => act('toggle-mute', tab.id)}>{state.tabs.find(item => item.id === tab.id)?.muted ? 'Unmute tab' : 'Mute tab'}</button>
      <button role="menuitem" className={button} onClick={() => act('close-tab', tab.id)}>Close tab</button>
    </div>
  </div>, document.body);
};
