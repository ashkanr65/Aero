export const CLEAR_DATA_TYPES = [
  { id: 'history', label: 'Browsing history', description: 'Visited pages and recently closed tabs.' },
  { id: 'downloads', label: 'Download history', description: 'Completed download records. Downloaded files stay on your computer.' },
  { id: 'cookies', label: 'Cookies', description: 'Website sign-ins and cookie preferences. You may be signed out.' },
  { id: 'cache', label: 'Cached images and files', description: 'Temporary files used to load websites faster.' },
  { id: 'site-data', label: 'Website storage', description: 'Local storage, databases, and offline website data.' },
  { id: 'autofill', label: 'Autofill form data', description: 'Saved form information, excluding passwords in the encrypted vault.' },
] as const;
export type ClearDataType = typeof CLEAR_DATA_TYPES[number]['id'];
export type ClearTimeRange = 'today' | 'last-week' | 'last-month' | 'last-year' | 'all-time';
export const CLEAR_TIME_RANGES: Array<{ value: ClearTimeRange; label: string }> = [
  { value: 'today', label: 'Today' },
  { value: 'last-week', label: 'Last week (7 days)' },
  { value: 'last-month', label: 'Last month (30 days)' },
  { value: 'last-year', label: 'Last year (365 days)' },
  { value: 'all-time', label: 'All time' },
];
export interface ClearDataRequest {
  timeRange: ClearTimeRange;
  startTime: number;
  endTime: number;
  types: ClearDataType[];
}
export type ClearDataResult = ClearDataRequest;

export function clearDataRequest(timeRange: ClearTimeRange, types: ClearDataType[], now = Date.now()): ClearDataRequest {
  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  const days = { 'last-week': 7, 'last-month': 30, 'last-year': 365 };
  const startTime = timeRange === 'all-time' ? 0 : timeRange === 'today' ? today.getTime() : now - days[timeRange] * 86400000;
  return { timeRange, startTime: Math.max(0, startTime), endTime: now, types: [...new Set(types)] };
}
export function inClearRange(timestamp: number, range: Pick<ClearDataRequest, 'startTime' | 'endTime'>): boolean {
  return Number.isFinite(timestamp) && timestamp >= range.startTime && timestamp < range.endTime;
}
export function retainOutsideRange<T>(entries: T[], timestamp: (entry: T) => number, range: ClearDataRequest): T[] {
  return entries.filter(entry => !inClearRange(timestamp(entry), range));
}
