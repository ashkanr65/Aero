import React, { useEffect, useRef, useState } from 'react';
import { ChevronDown, ChevronUp, X } from 'lucide-react';

export const FindBar: React.FC<{ requestDirection: 0 | 1 | -1; onClose(): void; command(command: string, payload?: unknown): void }> = ({ requestDirection, onClose, command }) => {
  const [text, setText] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);
  const find = (forward = true) => command('find', { text, forward });
  useEffect(() => { inputRef.current?.focus(); inputRef.current?.select(); }, []);
  useEffect(() => { if (requestDirection) find(requestDirection > 0); }, [requestDirection]);
  return <div className="flex h-11 shrink-0 items-center justify-end gap-1 border-b border-slate-200 bg-white px-4 dark:border-zinc-700 dark:bg-zinc-800">
    <input ref={inputRef} value={text} onChange={event => { setText(event.target.value); command('find', { text: event.target.value, forward: true }); }} onKeyDown={event => { if (event.key === 'Enter') find(!event.shiftKey); if (event.key === 'Escape') { command('find', { text: '' }); onClose(); } }} placeholder="Find on page" className="h-7 w-60 bg-transparent px-2 text-sm outline-none" />
    <button className="icon-button" onClick={() => find(false)}><ChevronUp size={15} /></button><button className="icon-button" onClick={() => find(true)}><ChevronDown size={15} /></button><button className="icon-button" onClick={() => { command('find', { text: '' }); onClose(); }}><X size={15} /></button>
  </div>;
};
