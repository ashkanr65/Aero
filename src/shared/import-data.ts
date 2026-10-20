export const HISTORY_LIMIT = 100_000;
interface Bookmark { id: string; title: string; url: string; }
interface History extends Bookmark { visitedAt: number; }
interface Incoming { title: string; url: string; visitedAt?: number; }
const safe = (value: string) => { try { return ['http:', 'https:'].includes(new URL(value).protocol); } catch { return false; } };
export function mergeBookmarks(current: Bookmark[], incoming: Incoming[], id: () => string) {
  const seen = new Set(current.map(entry => entry.url));
  const added: Bookmark[] = [];
  for (const entry of incoming) {
    if (!safe(entry.url) || seen.has(entry.url)) continue;
    seen.add(entry.url); added.push({ id: id(), title: entry.title || entry.url, url: entry.url });
  }
  return { entries: [...added, ...current], added: added.length };
}
export function mergeHistory(current: History[], incoming: Incoming[], id: () => string) {
  const seen = new Set(current.map(entry => entry.url));
  const added: History[] = [];
  for (const entry of incoming) {
    if (!safe(entry.url) || seen.has(entry.url) || !Number.isFinite(entry.visitedAt) || (entry.visitedAt ?? 0) <= 0) continue;
    seen.add(entry.url); added.push({ id: id(), title: entry.title || entry.url, url: entry.url, visitedAt: entry.visitedAt! });
  }
  const entries = [...added, ...current].sort((a,b) => b.visitedAt - a.visitedAt);
  // Never discard existing entries to make room for an import. Reject an
  // oversized combined result so the UI can explain it before writing.
  if (entries.length > HISTORY_LIMIT) throw new Error(`Aero can keep up to ${HISTORY_LIMIT.toLocaleString()} history pages. Import bookmarks now, or clear some Aero history before trying again.`);
  return { entries, added: added.length };
}
