import { getJson, type GridSlot, type Project, type RaceEvent, type RaceWindow } from "@/lib/api";

export type ChampionshipStanding = {
  handle: string;
  display_name?: string | null;
  points: number;
  wins: number;
  best_finish?: number;
  rank: number;
};

export type FeaturedRace = {
  window_slug: string;
  window_name: string;
  score: number;
  because: string;
};

export type RaceCalendar = {
  windows?: RaceWindow[];
  active_card?: { title?: string; kind?: string; ends_at?: string } | null;
  featured?: FeaturedRace | null;
};

export type ContentItem = {
  slug: string;
  title: string;
  body_md: string;
  rp_reward: number;
};

export type NarrativePost = {
  event_id?: string;
  channel?: string;
  body: string;
  why_clauses?: string[];
  source?: string;
  generated_at?: string;
};

export type HoverMetrics = {
  handle: string;
  displayName: string;
  url?: string | null;
  blurb?: string | null;
  tag?: string | null;
  position: number;
  gap: number;
  pace: number;
  velocity: number;
  lastOvertake: string | null;
  raceRp: number;
  lifetimeRp: number;
  paidPct: number;
  communityPct: number;
  clicks: number;
  cpc: number | null;
  badge: string | null;
  footer: string;
};

const BADGES = new Set(["HOT", "REIGN", "DARK_HORSE", "PHOTO", "COOLING"]);

export function deriveBadge(p: Project, slot?: GridSlot | null, gapToNext?: number | null): string | null {
  if (slot?.badge && BADGES.has(slot.badge)) return slot.badge;
  if ((p.total_rp || 0) <= 0) return null;
  if (gapToNext != null && gapToNext >= 0 && gapToNext < 5) return "PHOTO";
  return null;
}

export function gapToNextOnPage(projects: Project[], index: number): number {
  const cur = projects[index]?.total_rp ?? 0;
  const next = projects[index + 1]?.total_rp;
  if (next == null) return 0;
  return Math.max(0, cur - next);
}

export function metricsFromProject(
  p: Project,
  slot?: GridSlot | null,
  extras?: { gapToNext?: number },
): HoverMetrics {
  const raceRp = slot?.race_rp ?? p.total_rp ?? 0;
  const paid = slot?.paid_rp ?? 0;
  const community = slot?.community_rp ?? Math.max(0, raceRp - paid);
  const denom = paid + community || raceRp;
  const paidPct = denom ? Math.round((paid / denom) * 100) : 0;
  const clicks = slot?.clicks ?? 0;
  const gap = extras?.gapToNext ?? slot?.gap_to_next ?? slot?.gap_to_leader ?? 0;
  const footer =
    slot?.hover_footer ||
    (raceRp > 0
      ? `P${p.rank} on ${raceRp.toLocaleString()} RP.`
      : "Waiting for fuel. Rank is catalog order until RP moves.");
  return {
    handle: p.handle,
    displayName: p.display_name || p.handle,
    url: p.url,
    blurb: p.blurb,
    tag: p.tags?.[0] || null,
    position: slot?.rank ?? p.rank,
    gap,
    pace: slot?.velocity ?? 0,
    velocity: slot?.velocity ?? 0,
    lastOvertake: slot?.last_overtake ?? null,
    raceRp,
    lifetimeRp: p.total_rp ?? raceRp,
    paidPct,
    communityPct: denom ? 100 - paidPct : 0,
    clicks,
    cpc: clicks > 0 ? raceRp / clicks : null,
    badge: deriveBadge(p, slot, gap),
    footer,
  };
}

export async function fetchCalendar(): Promise<RaceCalendar | null> {
  return getJson<RaceCalendar>("/v1/races/calendar");
}

export async function fetchChampionship(): Promise<ChampionshipStanding[]> {
  const res = await getJson<{ standings?: ChampionshipStanding[] }>("/v1/championship");
  return res?.standings || [];
}

export async function fetchContent(): Promise<ContentItem[]> {
  const res = await getJson<{ items?: ContentItem[] } | ContentItem[]>("/v1/content");
  if (!res) return [];
  if (Array.isArray(res)) return res;
  return res.items || [];
}

export async function fetchTape(slug: string): Promise<NarrativePost[]> {
  const res = await getJson<{ posts?: NarrativePost[] }>(`/v1/races/windows/${encodeURIComponent(slug)}/tape`);
  return res?.posts || [];
}

export async function fetchWindowEvents(slug: string): Promise<RaceEvent[]> {
  const res = await getJson<{ events?: RaceEvent[] }>(`/v1/races/windows/${encodeURIComponent(slug)}/events`);
  return res?.events || [];
}

export async function fetchWindows(): Promise<RaceWindow[]> {
  const res = await getJson<{ windows?: RaceWindow[] }>("/v1/races/windows");
  return res?.windows || [];
}
