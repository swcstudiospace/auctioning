import type { GridSlot, RaceEvent } from "./api";
import { standings } from "./data";

export function seedGrid(): GridSlot[] {
  return standings.map((row, i) => ({
    handle: row.agent,
    owner: row.owner,
    rank: i + 1,
    race_rp: row.rp,
    velocity: 40 + (5 - i) * 8,
    gap_to_leader: row.rp - standings[0].rp,
    paid_rp: Math.round(row.rp * 0.68),
    community_rp: Math.round(row.rp * 0.32),
    badge: i === 0 ? "REIGN" : i === 4 ? "DARK_HORSE" : null,
    clicks: 8000 + i * 120,
    hover_footer: `${row.owner} · windowed RP`,
  }));
}

export function sortGrid(grid: GridSlot[]): GridSlot[] {
  const sorted = [...grid].sort((a, b) => b.race_rp - a.race_rp);
  const leader = sorted[0]?.race_rp ?? 0;
  return sorted.map((slot, i) => ({
    ...slot,
    rank: i + 1,
    gap_to_leader: leader - slot.race_rp,
    gap_to_next: i === 0 ? 0 : sorted[i - 1].race_rp - slot.race_rp,
  }));
}

export function tickGrid(grid: GridSlot[]): { grid: GridSlot[]; event: RaceEvent | null } {
  const next = grid.map((s) => ({ ...s }));
  const i = Math.floor(Math.random() * next.length);
  const bump = 10 + Math.floor(Math.random() * 18) * 10;
  const before = sortGrid(next);
  const fromRank = before.find((s) => s.handle === next[i].handle)?.rank ?? next[i].rank;
  next[i].race_rp += bump;
  next[i].velocity = (next[i].velocity ?? 20) + 3;
  next[i].paid_rp = (next[i].paid_rp ?? 0) + Math.round(bump * 0.7);
  const after = sortGrid(next);
  const toRank = after.find((s) => s.handle === next[i].handle)?.rank ?? next[i].rank;
  let event: RaceEvent | null = null;
  if (toRank < fromRank) {
    const passed = after.find((s) => s.rank === toRank + 1);
    event = {
      kind: toRank === 1 ? "lead_change" : "overtake",
      handle: next[i].handle,
      victim: passed?.handle,
      body: `${next[i].handle} +${bump} RP${passed ? `, passed ${passed.handle}` : ""}`,
    };
    next[i].last_overtake = passed?.handle ?? null;
    next[i].badge = toRank === 1 ? "HOT" : next[i].badge;
  }
  return { grid: after, event };
}

export function predictRank(grid: GridSlot[], bid: number): number {
  const higher = grid.filter((s) => s.race_rp >= bid).length;
  return Math.min(grid.length + 1, higher + 1);
}

export function formatClock(total: number): string {
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
