export interface PasswordManagerEntry {
  id: string;
  origin: string;
  username: string;
  updated_at: number;
}

interface PasswordManagerAPI {
  lock(): Promise<boolean>;
  activity(): void;
  list(): Promise<PasswordManagerEntry[]>;
  importCsv(): Promise<{ imported: number; skipped: number; message: string }>;
  save(entry: { id?: string; origin: string; username: string; password: string }): Promise<{ id: string }>;
  remove(id: string): Promise<boolean>;
  copy(): Promise<boolean>;
  select(id: string): Promise<boolean>;
  reveal(): Promise<{ id: string; origin: string; username: string; password: string }>;
  onFillReady(listener: (origin: string) => void): () => void;
  onLock(listener: () => void): () => void;
  selectForFill(id: string): Promise<boolean>;
  backup(): Promise<boolean>;
  restore(): Promise<boolean>;
}


declare global { interface Window { aeroPasswordManager: PasswordManagerAPI; } }
