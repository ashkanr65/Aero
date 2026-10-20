import React, { useState } from 'react';
import { Trash2 } from 'lucide-react';
import { CLEAR_DATA_TYPES, CLEAR_TIME_RANGES, clearDataRequest, type ClearDataRequest, type ClearDataType, type ClearTimeRange } from '../../../shared/browsing-data';

export const ClearBrowsingData: React.FC<{ onClear(request: ClearDataRequest): Promise<void> }> = ({ onClear }) => {
  const [timeRange, setTimeRange] = useState<ClearTimeRange>('today');
  const [selected, setSelected] = useState<ClearDataType[]>(['history', 'cookies', 'cache', 'site-data']);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const allSelected = selected.length === CLEAR_DATA_TYPES.length;
  const clear = async () => {
    if (busy || selected.length === 0) return;
    setBusy(true); setMessage(''); setError('');
    try {
      await onClear(clearDataRequest(timeRange, selected));
      setMessage('Selected browsing data cleared.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Browsing data could not be cleared. Please try again.');
    } finally { setBusy(false); }
  };
  return <div data-aero-clear-data className="space-y-4 p-5">
    <label className="flex items-center justify-between gap-4 text-sm"><span className="font-medium">Time range</span>
      <select aria-label="Clear browsing data time range" disabled={busy} value={timeRange} onChange={event => { setTimeRange(event.target.value as ClearTimeRange); setMessage(''); }} className="h-9 rounded-lg border border-slate-300 bg-white px-3 dark:border-zinc-700 dark:bg-zinc-900">
        {CLEAR_TIME_RANGES.map(range => <option key={range.value} value={range.value}>{range.label}</option>)}
      </select>
    </label>
    <fieldset disabled={busy} className="space-y-2">
      <legend className="sr-only">Choose browsing data to clear</legend>
      <label className="flex cursor-pointer items-center gap-3 border-b border-slate-100 pb-3 text-sm font-medium dark:border-zinc-800">
        <input type="checkbox" aria-label="Select all browsing data types" checked={allSelected} onChange={event => setSelected(event.target.checked ? CLEAR_DATA_TYPES.map(type => type.id) : [])} className="h-4 w-4 shrink-0 accent-sky-600" />Select all
      </label>
      {CLEAR_DATA_TYPES.map(type => <label key={type.id} className="flex cursor-pointer items-start gap-3 rounded-lg py-2 text-sm">
        <input type="checkbox" data-aero-clear-kind={type.id} checked={selected.includes(type.id)} onChange={event => { setSelected(previous => event.target.checked ? [...previous, type.id] : previous.filter(value => value !== type.id)); setMessage(''); }} className="mt-0.5 h-4 w-4 shrink-0 accent-sky-600" />
        <span><strong className="block font-medium">{type.label}</strong><small className="mt-0.5 block text-xs leading-relaxed text-slate-500 dark:text-slate-400">{type.description}</small></span>
      </label>)}
    </fieldset>
    <p className="text-xs leading-relaxed text-slate-500 dark:text-slate-400">Bookmarks and saved vault passwords are kept. Manage saved passwords in Password Manager.</p>
    {error && <p role="alert" className="text-sm text-red-600 dark:text-red-400">{error}</p>}
    {message && <p role="status" className="text-sm text-emerald-700 dark:text-emerald-400">{message}</p>}
    <div className="flex justify-end"><button className="primary-button !bg-red-600 disabled:cursor-not-allowed disabled:opacity-50" disabled={busy || selected.length === 0} onClick={() => void clear()}><Trash2 size={15} />{busy ? 'Clearing…' : 'Clear selected data'}</button></div>
  </div>;
};
