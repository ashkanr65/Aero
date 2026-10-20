import React, { useRef } from 'react';
import { ArrowLeftRight, X } from 'lucide-react';
import { draggedTab, TAB_DRAG_TYPE } from './TabContextMenu';
import type { BrowserState } from '../types';

interface SplitViewProps {
  state: BrowserState;
  dragging: boolean;
  onDragChange(dragging: boolean): void;
  command(name: string, payload?: unknown): void;
}

export const SplitView = React.forwardRef<HTMLDivElement, SplitViewProps>(({ state, dragging, command, onDragChange }, ref) => {
  const wrapper = useRef<HTMLDivElement>(null);
  const active = Boolean(state.splitTabId);
  const ratio = state.splitRatio ?? 0.5;
  const drop = (event: React.DragEvent, pane: 'left' | 'right') => {
    event.preventDefault(); event.stopPropagation();
    const id = draggedTab(event);
    if (id !== null) command('move-tab-to-pane', { id, pane });
    onDragChange(false);
  };
  const over = (event: React.DragEvent) => {
    if (event.dataTransfer.types.includes(TAB_DRAG_TYPE)) { event.preventDefault(); event.dataTransfer.dropEffect = 'move'; }
  };
  const resize = (event: React.PointerEvent<HTMLDivElement>) => {
    event.preventDefault(); event.currentTarget.setPointerCapture(event.pointerId);
  };
  const move = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!event.currentTarget.hasPointerCapture(event.pointerId) || !wrapper.current) return;
    const rect = wrapper.current.getBoundingClientRect();
    command('set-split-ratio', (event.clientX - rect.left) / rect.width);
  };
  return <div ref={wrapper} className="flex min-h-0 flex-1 flex-col">
    {(active || dragging) && <div className="no-drag flex h-9 shrink-0 items-stretch border-b border-slate-200 bg-slate-50 text-xs dark:border-zinc-800 dark:bg-zinc-900">
      {(['left', 'right'] as const).map(pane => {
        const id = pane === 'left' ? state.leftTabId : state.rightTabId;
        const selected = state.activeTabId === id;
        return <button key={pane} className={`min-w-0 flex-1 truncate px-3 text-left ${selected ? 'font-semibold text-sky-600' : ''}`} onClick={() => command('activate-pane', pane)} onDragOver={over} onDrop={event => drop(event, pane)} title={`Drop a tab here to open it in the ${pane} pane`}>
          {pane === 'left' ? 'Left' : 'Right'} · {dragging ? 'Drop tab here' : state.tabs.find(tab => tab.id === id)?.title || 'Open a tab here'}
        </button>;
      })}
      {active && <><button className="icon-button" aria-label="Swap split panes" onClick={() => command('swap-panes')}><ArrowLeftRight size={15} /></button><button className="icon-button" aria-label="Close split screen" onClick={() => command('toggle-split')}><X size={15} /></button></>}
    </div>}
    <div ref={ref} className="relative min-h-0 flex-1 bg-slate-100 dark:bg-zinc-950">
      {active && <div role="separator" tabIndex={0} aria-label="Resize split panes" aria-orientation="vertical" aria-valuemin={20} aria-valuemax={80} aria-valuenow={Math.round(ratio * 100)} onPointerDown={resize} onPointerMove={move} onPointerUp={event => event.currentTarget.releasePointerCapture(event.pointerId)} onKeyDown={event => { if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); command('set-split-ratio', ratio + (event.key === 'ArrowLeft' ? -0.02 : 0.02)); } }} onDoubleClick={() => command('set-split-ratio', 0.5)} className="absolute inset-y-0 z-20 w-3 cursor-col-resize touch-none bg-slate-200 hover:bg-sky-300 focus:bg-sky-300 dark:bg-zinc-800" style={{ left: `calc((100% - 12px) * ${ratio})` }} />}
      {dragging && <div className="absolute inset-0 z-30 flex gap-3 bg-slate-100/95 p-3 dark:bg-zinc-950/95">{(['left', 'right'] as const).map(pane => <div key={pane} onDragOver={over} onDrop={event => drop(event, pane)} className="grid flex-1 place-items-center rounded-xl border-2 border-dashed border-sky-400 text-sm text-sky-600">Drop in {pane} pane</div>)}</div>}
    </div>
  </div>;
});
SplitView.displayName = 'SplitView';
