import React from 'react';
import { ShieldAlert } from 'lucide-react';
import type { BrowserState, ShieldMode } from '../types';

export const ShieldsMenu: React.FC<{ state: BrowserState; command(command: string, payload?: unknown): void }> = ({ state, command }) => {
  const saved = state.stats.bandwidthSaved < 1_000_000
    ? `${Math.round(state.stats.bandwidthSaved / 1000)} KB`
    : `${(state.stats.bandwidthSaved / 1_000_000).toFixed(1)} MB`;
  return (
    <div className="space-y-4 p-4">
      <div className="flex items-center gap-2"><ShieldAlert className="text-orange-500" size={19} /><div><h2 className="text-sm font-bold">Aero Shields</h2><p className="text-[10px] text-slate-500">Protection for this site</p></div></div>
      <div className="grid grid-cols-3 gap-2">
        <div className="stat-card"><strong>{state.stats.blocked}</strong><span>Blocked</span></div>
        <div className="stat-card"><strong>{state.stats.httpsUpgrades}</strong><span>Upgraded</span></div>
        <div className="stat-card"><strong>{saved}</strong><span>Saved</span></div>
      </div>
      <label className="block text-xs font-medium">Trackers and ads
        <select value={state.shieldMode} onChange={event => command('set-shield-mode', event.target.value as ShieldMode)} className="mt-1 w-full rounded-lg border border-slate-200 bg-white p-2 text-xs dark:border-zinc-700 dark:bg-zinc-800">
          <option value="aggressive">Aggressive</option><option value="standard">Standard</option><option value="disabled">Allow ads and tracking</option>
        </select>
      </label>
      <label className="flex items-center justify-between rounded-lg bg-slate-50 p-3 text-xs dark:bg-zinc-800">
        <span><strong className="block">Block scripts</strong><small className="text-slate-500">May break some sites</small></span>
        <input type="checkbox" checked={state.scriptBlocking} onChange={event => command('set-script-blocking', event.target.checked)} className="h-4 w-4 accent-orange-500" />
      </label>
      <div className="rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-xs text-emerald-800 dark:border-emerald-900 dark:bg-emerald-950 dark:text-emerald-200">
        GPC, HTTPS upgrades, cross-site cookie blocking, and fingerprint defenses are active.
      </div>
    </div>
  );
};
