// A running total of text sent to online services in this session, so nothing is a surprise. Held in memory only:
// it is gone when Soundstage closes and is never written to disk.
const sent = new Map<string, number>();

export function addSent(provider: string, chars: number): void {
  if (!Number.isFinite(chars) || chars <= 0) return;
  sent.set(provider, (sent.get(provider) ?? 0) + Math.round(chars));
}

export function sentBy(provider: string): number {
  return sent.get(provider) ?? 0;
}

export function sentTotals(): [string, number][] {
  return [...sent.entries()].sort((a, b) => b[1] - a[1]);
}

export function resetSent(): void {
  sent.clear();
}
