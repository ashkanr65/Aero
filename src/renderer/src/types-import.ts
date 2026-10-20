export type ImportSource = 'edge' | 'chrome' | 'firefox' | 'opera';

export interface ImportedBookmark { title: string; url: string; }
export interface ImportedHistoryEntry { url: string; title: string; visitedAt: number; }
export interface ImportedTab { url: string; title?: string; }
export interface ImportProfile { id: string; name: string; channel: string; lastUsed: boolean; }
export interface ImportSelection { bookmarks: boolean; history: boolean; }

export interface ImportSummary {
  source: ImportSource;
  found: boolean;
  profilePath?: string;
  stage: 'choose-profile' | 'scanning' | 'ready' | 'imported' | 'failed';
  profileId?: string;
  profileName?: string;
  channel?: string;
  profiles?: ImportProfile[];
  message: string;
  bookmarksImported?: number;
  historyImported?: number;
  passwordsImported?: number;
  tabsImported?: number;
  bookmarks?: ImportedBookmark[];
  history?: ImportedHistoryEntry[];
  tabs?: ImportedTab[];
  problems: Array<{ kind: string; message: string }>;
}

export interface ImportSummaryRenderer {
  source: ImportSource;
  found: boolean;
  profilePath?: string;
  problems: Array<{ kind: string; message: string }>;
}

export interface ImportState {
  active: boolean;
  summary: ImportSummary | null;
}

export type ImportPayload =
  | { type: 'start'; source: ImportSource }
  | { type: 'summary'; summary: ImportSummary }
  | { type: 'reset' };
