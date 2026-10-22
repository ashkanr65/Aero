import React, { useEffect, useState } from 'react';
import { ArrowLeft, CheckCircle2, KeyRound, Loader2 } from 'lucide-react';
import type { ImportSource, ImportSummary, ImportSelection } from '../types-import';

interface Props {
  summary: ImportSummary | null;
  helpError: string;
  onStart(source: ImportSource, profileId?: string, manual?: boolean): void;
  onApply(selection: ImportSelection): void;
  onSourcePasswords(): void;
  onImportPasswords(): void;
  onTransferPasswords(): void;
  onReset(): void;
}
const SOURCES: Array<{id: ImportSource; name: string}> = [
  {id: 'edge', name: 'Microsoft Edge'}, {id: 'chrome', name: 'Google Chrome'},
  {id: 'firefox', name: 'Mozilla Firefox'}, {id: 'opera', name: 'Opera'},
];
export const ImportSection: React.FC<Props> = ({ summary, helpError, onStart, onApply, onSourcePasswords, onImportPasswords, onTransferPasswords, onReset }) => {
  const [selection, setSelection] = useState<ImportSelection>({ bookmarks: true, history: true });
  useEffect(() => setSelection({ bookmarks: true, history: true }), [summary?.source, summary?.profileId]);
  const name = SOURCES.find(s => s.id === summary?.source)?.name ?? 'Browser';
  const busy = summary?.stage === 'scanning';
  const ready = summary?.stage === 'ready';
  const done = summary?.stage === 'imported';
  const bookmarks = summary?.bookmarks?.length ?? 0;
  const history = summary?.history?.length ?? 0;
  return <div className="settings-stack" data-aero-import>
    <section className="settings-card">
      <h2>Import browser data</h2>
      <div className="space-y-4 p-5">
        {!summary ? <>
          <p className="text-sm text-slate-600 dark:text-slate-300">Choose a browser. Aero will find its profiles and read the available bookmarks and history. You do not need to find a folder or export these as files.</p>
          <div className="grid gap-3 sm:grid-cols-2">{SOURCES.map(source => <button key={source.id} data-import-browser={source.id} onClick={() => onStart(source.id)} className="secondary-button min-h-14 justify-start text-left">{source.name}</button>)}</div>
          <p className="text-xs text-slate-500 dark:text-slate-400">You choose what to add before Aero saves anything. Password transfer has separate steps below; open tabs, extensions, and signed-in sessions are not imported.</p>
        </> : <>
          <div className="flex items-center gap-3"><button className="icon-button" aria-label="Choose another browser" onClick={onReset}><ArrowLeft size={16}/></button><div className="min-w-0"><h3 className="font-semibold">{name}{summary.profileName ? ` · ${summary.profileName}` : ''}</h3>{summary.channel && <p className="text-xs text-slate-500">{summary.channel}</p>}</div></div>
          <p role="status" className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">{busy && <Loader2 size={16} className="shrink-0 animate-spin"/>}{done && <CheckCircle2 size={16} className="shrink-0 text-emerald-500"/>}{summary.message}</p>
          {summary.stage === 'choose-profile' && <div className="space-y-2">{summary.profiles?.map(profile => <button key={profile.id} data-import-profile={profile.name} className="flex w-full items-center justify-between gap-3 rounded-xl border border-slate-200 px-4 py-3 text-left hover:border-sky-400 dark:border-zinc-700" onClick={() => onStart(summary.source, profile.id)}><span><strong className="text-sm">{profile.name}</strong><span className="ml-2 text-xs text-slate-500">{profile.channel}</span></span>{profile.lastUsed && <span className="text-xs text-sky-600">Last used</span>}</button>)}</div>}
          {ready && <>
            <fieldset className="space-y-3 rounded-xl border border-slate-200 p-4 dark:border-zinc-700"><legend className="px-2 text-sm font-semibold">Choose what to import</legend>
              <label className="flex items-center gap-3 text-sm"><input type="checkbox" data-import-kind="bookmarks" checked={selection.bookmarks && bookmarks > 0} disabled={!bookmarks} onChange={e => setSelection(s => ({...s, bookmarks: e.target.checked}))}/>Bookmarks <span className="ml-auto text-slate-500">{bookmarks.toLocaleString()} found</span></label>
              <label className="flex items-center gap-3 text-sm"><input type="checkbox" data-import-kind="history" checked={selection.history && history > 0} disabled={!history} onChange={e => setSelection(s => ({...s, history: e.target.checked}))}/>Browsing history <span className="ml-auto text-slate-500">{history.toLocaleString()} pages found</span></label>
            </fieldset>
            <button className="primary-button" data-import-apply disabled={!(selection.bookmarks && bookmarks || selection.history && history)} onClick={() => onApply({ bookmarks: selection.bookmarks && bookmarks > 0, history: selection.history && history > 0 })}>Import selected data</button>
          </>}
          {!!summary.problems.length && <div className="space-y-2 rounded-xl border border-amber-300/60 p-4 text-sm dark:border-amber-700/60" role="alert">{summary.problems.map((problem,i) => <p key={`${problem.kind}-${i}`}>{problem.message}</p>)}</div>}
          {!busy && <div className="flex flex-wrap gap-2"><button className="secondary-button" data-import-retry onClick={() => onStart(summary.source, summary.profileName ? summary.profileId : undefined)}>Detect again / retry</button><button className="secondary-button" data-import-browse onClick={() => onStart(summary.source, undefined, true)}>Choose a custom profile folder…</button></div>}
          {!busy && summary.stage !== 'choose-profile' && <section className="space-y-3 rounded-xl border border-slate-200 p-4 dark:border-zinc-700" data-import-password-guide>
            <h3 className="flex items-center gap-2 font-semibold"><KeyRound size={16}/>Transfer saved passwords</h3>
            <p className="text-sm text-slate-600 dark:text-slate-300">{name} protects saved passwords separately. Aero can open its password settings and detect a new export in Downloads. You approve the export in the source browser and the import with Windows Hello.</p>
            <ol className="list-inside list-decimal space-y-2 text-sm text-slate-600 dark:text-slate-300"><li>Choose Transfer passwords below to open both password managers.</li><li>In {name}, choose Export passwords and approve its confirmation. Save a new CSV in Downloads.</li><li>Return to Aero Password Manager. Confirm the detected filename and approve Windows Hello. Aero handles the file transfer and encryption.</li></ol>
            <div className="flex flex-wrap gap-2"><button className="primary-button" data-import-transfer-passwords onClick={onTransferPasswords}>Transfer passwords from {name}</button><button className="secondary-button" data-import-open-passwords onClick={onSourcePasswords}>Open {name} password settings</button><button className="secondary-button" onClick={onImportPasswords}>Open Aero Password Manager</button></div>
            {helpError && <p role="alert" className="text-sm text-amber-700 dark:text-amber-400">{helpError}</p>}
            <p className="text-xs text-slate-500 dark:text-slate-400">The exported CSV contains readable passwords. Delete it after a successful import. Aero stores imported passwords in its encrypted local vault.</p>
          </section>}
        </>}
      </div>
    </section>
  </div>;
};
