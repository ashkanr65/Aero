import React, { useMemo, useState } from 'react';
import { Calculator, Gauge, Globe, Shield, StickyNote, X } from 'lucide-react';
import type { BrowserState } from '../types';
import { ShieldsMenu } from './ShieldsMenu';

export type SidebarTool = 'privacy' | 'notes' | 'tools';

interface SidebarProps {
  state: BrowserState;
  open: boolean;
  width: number;
  onResizeStart(event: React.MouseEvent): void;
  tool: SidebarTool;
  onTool(tool: SidebarTool): void;
  onClose(): void;
  command(command: string, payload?: unknown): void;
}

export const Sidebar: React.FC<SidebarProps> = ({ state, open, width, onResizeStart, tool, onTool, onClose, command }) => {
  const [notes, setNotes] = useState(() => localStorage.getItem('aero.notes') ?? '');
  const [calculation, setCalculation] = useState('12 * 8');
  const result = useMemo(() => {
    const match = calculation.match(/^\s*(-?\d+(?:\.\d+)?)\s*([+\-*/])\s*(-?\d+(?:\.\d+)?)\s*$/);
    if (!match) return '—';
    const left = Number(match[1]); const right = Number(match[3]);
    if (match[2] === '+') return String(left + right);
    if (match[2] === '-') return String(left - right);
    if (match[2] === '*') return String(left * right);
    return right === 0 ? '—' : String(left / right);
  }, [calculation]);

  if (!open) return null;
  return (
    <aside style={{ width }} className="relative z-20 flex shrink-0 isolate border-l border-slate-200 bg-white dark:border-zinc-800 dark:bg-[#1e2025]">
      <div role="separator" aria-label="Resize Aero sidebar" aria-orientation="vertical" onMouseDown={onResizeStart} className="absolute left-[-3px] top-0 z-30 h-full w-1.5 cursor-col-resize hover:bg-sky-500/50" />
      <div className="sidebar-tool-rail">
        <button type="button" className={`sidebar-tool-button ${tool === 'privacy' ? 'active' : ''}`} onClick={() => onTool('privacy')} aria-label="Privacy"><Shield size={17} aria-hidden="true" /></button>
        <button type="button" className={`sidebar-tool-button ${tool === 'notes' ? 'active' : ''}`} onClick={() => onTool('notes')} aria-label="Notes"><StickyNote size={17} aria-hidden="true" /></button>
        <button type="button" className={`sidebar-tool-button ${tool === 'tools' ? 'active' : ''}`} onClick={() => onTool('tools')} aria-label="Quick tools"><Calculator size={17} aria-hidden="true" /></button>
        <button type="button" className="sidebar-tool-button mt-auto" onClick={() => command('new-tab', 'https://duckduckgo.com')} aria-label="Open DuckDuckGo"><Globe size={17} aria-hidden="true" /></button>
      </div>
      <div className="min-w-0 flex-1 overflow-y-auto">
        <div className="flex h-11 items-center justify-between border-b border-slate-200 px-4 dark:border-zinc-800"><strong className="text-xs">Aero Hub</strong><button className="icon-button" onClick={onClose}><X size={15} /></button></div>
        {tool === 'privacy' && <ShieldsMenu state={state} command={command} />}
        {tool === 'notes' && <div className="space-y-3 p-4"><h2 className="text-sm font-bold">Aero Notes</h2><textarea value={notes} onChange={event => { setNotes(event.target.value); localStorage.setItem('aero.notes', event.target.value); }} className="h-64 w-full resize-none rounded-xl border border-slate-200 bg-amber-50 p-3 text-sm outline-none focus:border-sky-500 dark:border-zinc-700 dark:bg-zinc-800" placeholder="Private notes stored on this device…" /></div>}
        {tool === 'tools' && <div className="space-y-4 p-4"><h2 className="text-sm font-bold">Quick Tools</h2><div className="rounded-xl bg-slate-50 p-3 dark:bg-zinc-800"><label className="text-[10px] font-semibold uppercase text-slate-500">Calculator</label><input value={calculation} onChange={event => setCalculation(event.target.value)} className="mt-2 w-full rounded-lg border border-slate-200 bg-white p-2 text-sm dark:border-zinc-700 dark:bg-zinc-900" /><div className="mt-2 text-right text-xl font-bold text-sky-600">{result}</div></div><button className="secondary-button w-full" onClick={() => command('new-tab', 'https://fast.com')}><Gauge size={15} />Open speed test</button></div>}
      </div>
    </aside>
  );
};
