import React from 'react';
import { Heart } from 'lucide-react';
import type { FavoriteEntry } from './BrowserPanel';

export const FavoritesBar = React.forwardRef<HTMLDivElement, { favorites: FavoriteEntry[]; command(command: string, payload?: unknown): void }>(({ favorites, command }, ref) => <div ref={ref} className="flex h-8 shrink-0 items-center gap-1 overflow-x-auto border-b border-slate-200 bg-white/90 px-3 dark:border-zinc-800 dark:bg-[#202228]">
  {!favorites.length && <span className="flex items-center gap-1 text-[11px] text-slate-400"><Heart size={12} />Press Ctrl+D to add this page</span>}
  {favorites.map(item => <button key={item.id} className="flex h-6 max-w-44 items-center gap-1.5 rounded-md px-2 text-[11px] hover:bg-slate-100 dark:hover:bg-zinc-700" onClick={() => command('navigate', item.url)}><Heart size={11} className="fill-amber-400 text-amber-400" /><span className="truncate">{item.title}</span></button>)}
</div>);
FavoritesBar.displayName = 'FavoritesBar';
