export interface PasswordManagerEntry {
  id: string;
  origin: string;
  username: string;
  updated_at: number;
}

export interface PasswordTransferStatus {
  active: boolean;
  id?: string;
  source?: string;
  folder?: string;
  files?: Array<{ id: string; name: string; bytes: number }>;
  message?: string;
}
interface PasswordManagerAPI {
  transferStatus(): Promise<PasswordTransferStatus>;
  cancelTransfer(): Promise<boolean>;
  importTransfer(session: string, file: string): Promise<{ imported: number; skipped: number; message: string }>;
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
