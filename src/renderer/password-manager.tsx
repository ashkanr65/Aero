import React, { useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Clipboard, Eye, EyeOff, KeyRound, LockKeyhole, Plus, RefreshCw, Save, ShieldCheck, Trash2, Upload, X } from 'lucide-react';
import type { PasswordManagerEntry, PasswordTransferStatus } from '../preload/password-manager';
import './src/index.css';

const api = window.aeroPasswordManager;

const PasswordManager: React.FC = () => {
  const [entries, setEntries] = useState<PasswordManagerEntry[]>([]);
  const [filter, setFilter] = useState('');
  const [error, setError] = useState('');
  const [status, setStatus] = useState('Vault locked. Authenticate with Windows Hello to continue.');
  const [busy, setBusy] = useState(false);
  const [edit, setEdit] = useState<{ id?: string; origin: string; username: string; password: string } | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<{ id: string; password: string } | null>(null);
  const [importStatus, setImportStatus] = useState('');
  const [importing, setImporting] = useState(false);
  const [transfer, setTransfer] = useState<PasswordTransferStatus | null>(null);
  const [transferError, setTransferError] = useState('');

  const refresh = async () => {
    setBusy(true);
    setError('');
    setStatus('Waiting for Windows Hello…');
    try {
      const next = await api.list();
      setEntries(next);
      setStatus(`Unlocked for this operation. ${next.length} saved credential${next.length === 1 ? '' : 's'}.`);
      setRevealed(null);
    } catch (cause) {
      setError(messageOf(cause));
      setStatus('Locked. No vault data was returned.');
      setEntries([]);
    } finally { setBusy(false); }
  };

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => {
      document.documentElement.classList.toggle('dark', media.matches);
      document.documentElement.style.colorScheme = media.matches ? 'dark' : 'light';
    };
    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, []);

  useEffect(() => {
    const unsubscribe = api.onLock(() => {
      setEntries([]);
      setRevealed(null);
      setEdit(null);
      setStatus('Locked. Authenticate again to access vault metadata.');
    });
    let lastActivity = 0;
    const activity = (event: Event) => {
      if (event.isTrusted && Date.now() - lastActivity > 10_000) { lastActivity = Date.now(); api.activity(); }
    };
    document.addEventListener('pointerdown', activity); document.addEventListener('keydown', activity);
    return () => { unsubscribe(); document.removeEventListener('pointerdown', activity); document.removeEventListener('keydown', activity); };
  }, []);
  useEffect(() => {
    if (!revealed) return;
    const timer = window.setTimeout(() => setRevealed(null), 30_000);
    return () => window.clearTimeout(timer);
  }, [revealed]);

  useEffect(() => {
    let stopped = false;
    let timer: number;
    const poll = async () => {
      try {
        const next = await api.transferStatus();
        if (!stopped) { setTransfer(next.active ? next : null); setTransferError(''); if (next.message) setImportStatus(next.message); }
      } catch (cause) { if (!stopped) setTransferError(messageOf(cause)); }
      if (!stopped) timer = window.setTimeout(() => void poll(), 1000);
    };
    void poll();
    return () => { stopped = true; window.clearTimeout(timer); };
  }, []);

  const importDetected = async (file: string) => {
    if (!transfer?.id) return;
    setImporting(true); setError(''); setImportStatus('Waiting for Windows Hello…');
    try {
      const result = await api.importTransfer(transfer.id, file);
      setTransfer(null); setEntries([]); setRevealed(null);
      setImportStatus(`${result.message}${result.skipped ? ` ${result.skipped} unsupported rows were skipped.` : ''}`);
      setStatus('Import complete. Use Unlock / refresh to review the saved credentials.');
    } catch (cause) { setError(messageOf(cause)); setImportStatus('No import was confirmed. You can retry or cancel.'); }
    finally { setImporting(false); }
  };
  const cancelTransfer = async () => {
    try { await api.cancelTransfer(); setTransfer(null); setImportStatus('Password transfer cancelled. No exported file was deleted.'); }
    catch (cause) { setTransferError(messageOf(cause)); }
  };

  const visibleEntries = useMemo(() => entries.filter(entry => `${entry.origin} ${entry.username}`.toLowerCase().includes(filter.trim().toLowerCase())), [entries, filter]);

  const guarded = async (operation: () => Promise<unknown>, success: string) => {
    setBusy(true);
    setError('');
    setStatus('Waiting for Windows Hello…');
    try {
      await operation();
      setStatus(success);
      return true;
    } catch (cause) {
      setError(messageOf(cause));
      setStatus('Locked. Windows authentication did not authorize the operation.');
      return false;
    } finally { setBusy(false); }
  };

  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!edit) return;
    const ok = await guarded(() => api.save(edit), 'Credential saved.');
    if (ok) { setEdit(null); await refresh(); }
  };

  const remove = async (entry: PasswordManagerEntry) => {
    if (!window.confirm(`Delete the saved credential for ${entry.origin}?`)) return;
    const ok = await guarded(() => api.remove(entry.id), 'Credential deleted.');
    if (ok) await refresh();
  };

  const reveal = async () => {
    if (!selected) return;
    if (!await guarded(() => api.select(selected), 'Credential selected.')) return;
    try {
      setBusy(true);
      setStatus('Authenticate with Windows Hello to reveal the selected password…');
      const credential = await api.reveal();
      setRevealed({ id: credential.id, password: credential.password });
      credential.password = '';
      setStatus('Password revealed in this isolated manager window. It hides automatically after 30 seconds; Lock closes this window.');
    } catch (cause) { setError(messageOf(cause)); setStatus('Locked. The reveal was denied.'); }
    finally { setBusy(false); }
  };

  const copy = async () => {
    if (!selected) return;
    const selectedOk = await guarded(() => api.select(selected), 'Credential selected.');
    if (!selectedOk) return;
    await guarded(() => api.copy(), 'Password copied. Aero will clear it after 30 seconds if the clipboard is unchanged.');
  };

  const fill = async () => {
    if (!selected) return;
    await guarded(() => api.selectForFill(selected), 'Credential selected for the current HTTPS origin.');
  };

  const backup = async () => { await guarded(() => api.backup(), 'Encrypted backup saved.'); };
  const importCsv = async () => {
    setImporting(true);
    setImportStatus('');
    setError('');
    try {
      const result = await api.importCsv();
      setImportStatus(`${result.message}${result.skipped ? ` ${result.skipped} invalid or unsupported rows were skipped.` : ''}`);
      if (result.imported > 0) { setEntries([]); setRevealed(null); setStatus('Import complete. Use Unlock / refresh to review the saved credentials.'); }
    } catch (cause) {
      setError(messageOf(cause));
    } finally { setImporting(false); }
  };
  const restore = async () => {
    if (!window.confirm('Restore will replace the current vault only after Windows authentication and successful decryption. Continue?')) return;
    if (await guarded(() => api.restore(), 'Encrypted vault restored.')) await refresh();
  };

  return <main className="min-h-screen overflow-y-auto bg-[#f4f7fb] p-6 text-slate-800 dark:bg-[#17191d] dark:text-slate-100">
    <div className="mx-auto max-w-3xl space-y-5">
      <header className="flex items-start gap-3">
        <div className="grid h-11 w-11 place-items-center rounded-2xl bg-sky-100 text-sky-700 dark:bg-sky-950 dark:text-sky-300"><KeyRound size={21} /></div>
        <div className="min-w-0 flex-1"><h1 className="text-xl font-semibold">Aero Password Manager</h1><p className="mt-1 text-xs text-slate-500">Local-only storage · Windows Hello required · Only selected credentials reach approved forms</p></div>
        <button className="secondary-button" onClick={() => void api.lock()}><LockKeyhole size={14} />Lock</button>
        <button className="secondary-button" disabled={busy || importing} onClick={() => void refresh()}><RefreshCw size={14} />Unlock / refresh</button>
      </header>

      <details className="rounded-2xl border border-sky-200 bg-sky-50 p-4 text-sm text-sky-950 dark:border-sky-900 dark:bg-sky-950/40 dark:text-sky-100">
        <summary className="flex cursor-pointer items-center gap-2 font-semibold"><ShieldCheck size={17} />How Aero protects passwords</summary>
        <p className="mt-2 text-xs leading-relaxed">DPAPI protects the random vault key for this Windows user. Windows Hello approval is an Aero access gate, not extra DPAPI encryption; malware running as your account can still capture credentials or control an authenticated session. Password strings shown here cannot be reliably erased from JavaScript memory.</p>
      </details>

      {(error || status) && <div className={`rounded-xl border p-3 text-xs ${error ? 'border-red-300 bg-red-50 text-red-800 dark:border-red-900 dark:bg-red-950/40 dark:text-red-200' : 'border-slate-200 bg-white text-slate-600 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-300'}`} role={error ? 'alert' : 'status'}>{error || status}{error && /Windows|Hello|authenticat/i.test(error) && <p className="mt-1">Enable Windows Hello in Windows Settings → Accounts → Sign-in options. Aero never asks for your Windows password.</p>}</div>}

      {(transfer || transferError) && <section className="space-y-3 rounded-2xl border border-sky-300 bg-white p-5 dark:border-sky-800 dark:bg-zinc-900" data-password-transfer>
        <h2 className="font-semibold">Transfer passwords{transfer?.source ? ` from ${transfer.source}` : ''}</h2>
        <p className="text-sm leading-relaxed">In the source browser, choose Export passwords and approve its confirmation. Save a new CSV in Downloads, then return here. Confirm the filename below and approve Windows Hello to encrypt the passwords in Aero.</p>
        {transfer?.folder && <p className="break-all text-xs text-slate-500">Watching for new CSV files in {transfer.folder} for up to 10 minutes. Existing files are ignored.</p>}
        {!transfer?.files?.length && !transferError && <p role="status" className="text-sm text-sky-600 dark:text-sky-300">Waiting for your new export…</p>}
        {transfer?.files?.map(file => <div key={file.id} className="flex items-center gap-3 rounded-xl border border-slate-200 p-3 dark:border-zinc-700"><div className="min-w-0 flex-1"><strong className="block break-all text-sm">{file.name}</strong><small className="text-slate-500">{Math.ceil(file.bytes / 1024)} KB · Confirm this is the password file you just exported.</small></div><button className="primary-button" data-transfer-file={file.id} disabled={busy || importing} onClick={() => void importDetected(file.id)}>{importing ? 'Importing…' : 'Confirm & import'}</button></div>)}
        {transferError && <p role="alert" className="text-sm text-red-600 dark:text-red-300">{transferError}</p>}
        <div className="flex flex-wrap gap-2"><button className="secondary-button" disabled={busy || importing} onClick={() => void importCsv()}>Choose exported CSV…</button><button className="secondary-button" onClick={() => void cancelTransfer()}>Cancel transfer</button></div>
        <p className="text-xs leading-relaxed text-slate-500">The source browser still controls password export. Aero does not read its protected password database. Delete the readable CSV after a successful import; Aero never deletes your files automatically.</p>
      </section>}

      {importStatus && <p role="status" className="rounded-xl border border-slate-200 p-3 text-sm dark:border-zinc-700" data-password-import-status>{importStatus}</p>}

      <section className="settings-card">
        <div className="flex items-center gap-2 border-b border-slate-200 p-4 dark:border-zinc-800">
          <input className="h-9 min-w-0 flex-1 rounded-lg border border-slate-200 bg-white px-3 text-sm dark:border-zinc-700 dark:bg-zinc-900" value={filter} onChange={event => setFilter(event.target.value)} placeholder="Search saved site or username" aria-label="Search credentials" />
          <button className="secondary-button" onClick={() => setEdit({ origin: '', username: '', password: '' })}><Plus size={14} />Add</button>
        </div>
        {visibleEntries.map(entry => <article className="browser-list-row border-b border-slate-100 last:border-0 dark:border-zinc-800" key={entry.id}>
          <button className="min-w-0 flex-1 text-left" onClick={() => setSelected(entry.id)} aria-pressed={selected === entry.id}>
            <strong>{entry.origin}</strong><small>{entry.username} · password ••••••••••••</small>
          </button>
          {selected === entry.id && revealed?.id === entry.id && <span className="max-w-44 truncate rounded bg-amber-50 px-2 py-1 font-mono text-xs dark:bg-amber-950">{revealed.password}</span>}
          <button className="icon-button" aria-label="Select credential" title="Select" onClick={() => setSelected(entry.id)}><KeyRound size={15} /></button>
          <button className="icon-button" disabled={busy || importing} aria-label="Edit credential" title="Edit" onClick={() => { setSelected(entry.id); setEdit({ id: entry.id, origin: entry.origin, username: entry.username, password: '' }); }}><Save size={15} /></button>
          <button className="icon-button" disabled={busy || selected !== entry.id} aria-label="Reveal selected password" title="Reveal selected" onClick={() => void reveal()}><Eye size={15} /></button>
          <button className="icon-button" disabled={busy || selected !== entry.id} aria-label="Copy selected password" title="Copy selected" onClick={() => void copy()}><Clipboard size={15} /></button>
          <button className="icon-button" disabled={busy || selected !== entry.id} aria-label="Fill selected credential in the active secure page" title="Fill on current secure site" onClick={() => void fill()}><Upload size={15} /></button>
          <button className="icon-button text-red-600" disabled={busy || importing} aria-label="Delete credential" title="Delete" onClick={() => void remove(entry)}><Trash2 size={15} /></button>
        </article>)}
        {!visibleEntries.length && <div className="p-10 text-center text-sm text-slate-500">{entries.length ? 'No matching credentials.' : 'No credentials loaded. Unlock with Windows Hello, then add a synthetic test credential or wait for a confirmed save offer.'}</div>}
      </section>

      {edit && <div className="fixed inset-0 z-50 grid place-items-center bg-slate-950/40 p-5" role="presentation"><form className="w-full max-w-lg space-y-4 rounded-2xl border border-slate-200 bg-white p-5 shadow-2xl dark:border-zinc-700 dark:bg-[#202228]" onSubmit={save}>
        <header className="flex items-center justify-between"><h2 className="text-lg font-semibold">{edit.id ? 'Update credential' : 'Add credential'}</h2><button type="button" className="icon-button" onClick={() => setEdit(null)} aria-label="Cancel"><X size={17} /></button></header>
        <label className="block text-xs font-medium">HTTPS site origin<input required autoFocus type="url" value={edit.origin} onChange={event => setEdit({ ...edit, origin: event.target.value })} placeholder="https://example.test" className="mt-1 h-10 w-full rounded-lg border border-slate-200 bg-white px-3 dark:border-zinc-700 dark:bg-zinc-900" /></label>
        <label className="block text-xs font-medium">Username<input required autoComplete="off" value={edit.username} onChange={event => setEdit({ ...edit, username: event.target.value })} className="mt-1 h-10 w-full rounded-lg border border-slate-200 bg-white px-3 dark:border-zinc-700 dark:bg-zinc-900" /></label>
        <label className="block text-xs font-medium">{edit.id ? 'New password (required to update)' : 'Password'}<input required type="password" autoComplete="new-password" value={edit.password} onChange={event => setEdit({ ...edit, password: event.target.value })} className="mt-1 h-10 w-full rounded-lg border border-slate-200 bg-white px-3 dark:border-zinc-700 dark:bg-zinc-900" /></label>
        <p className="text-[11px] text-slate-500">Aero will request fresh Windows authentication before writing this credential. Only HTTPS origins are accepted.</p>
        <div className="flex justify-end gap-2"><button type="button" className="secondary-button" onClick={() => setEdit(null)}>Cancel</button><button className="primary-button" disabled={busy || importing}><Save size={14} />Save after authentication</button></div>
      </form></div>}

      <footer className="flex flex-wrap gap-2">
        <button className="secondary-button" disabled={busy || importing} onClick={() => void importCsv()}><Upload size={14} />{importing ? 'Importing…' : 'Import CSV passwords'}</button>
        <button className="secondary-button" disabled={busy || importing} onClick={() => void backup()}><DownloadIcon />Encrypted backup</button>
        <button className="secondary-button" disabled={busy || importing} onClick={() => void restore()}><Upload size={14} />Restore encrypted backup</button>
        <span className="basis-full text-[10px] text-slate-500">Import supports CSV exports from Chrome, Edge, Firefox, and Opera. Aero reads the chosen CSV locally and creates no extra plaintext copy. Delete your source export after checking the import.</span>
        <span className="ml-auto flex items-center gap-1 text-[10px] text-slate-500"><LockKeyhole size={12} />Backups need this Windows account’s DPAPI keys</span>
      </footer>
    </div>
  </main>;
};

const DownloadIcon: React.FC = () => <EyeOff size={14} aria-hidden="true" />;
const messageOf = (cause: unknown) => cause instanceof Error ? cause.message : 'The vault operation failed safely.';

createRoot(document.getElementById('password-manager-root')!).render(<PasswordManager />);
