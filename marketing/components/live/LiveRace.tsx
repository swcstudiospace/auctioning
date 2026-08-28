"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { API_BASE, getJson, type GridSlot, type RaceEvent, type RaceWindow } from "@/lib/api";
import { formatClock, seedGrid, tickGrid } from "@/lib/sim";

type Hover = GridSlot | null;

export default function LiveRace() {
  const [grid, setGrid] = useState<GridSlot[]>(() => seedGrid());
  const [seconds, setSeconds] = useState(42);
  const [lap, setLap] = useState(6);
  const [events, setEvents] = useState<RaceEvent[]>([
    { kind: "overtake", body: "see.io +180 RP, holds P1" },
  ]);
  const [watchers, setWatchers] = useState(1284);
  const [hover, setHover] = useState<Hover>(null);
  const [source, setSource] = useState<"api" | "sim">("sim");
  const [slug, setSlug] = useState<string | null>(null);

  useEffect(() => {
    let cancel = false;
    async function pull() {
      const windows = await getJson<{ windows: RaceWindow[] }>("/v1/races/windows");
      const featured = await getJson<{ featured: { slug?: string } | null }>("/v1/races/featured");
      const windowSlug = featured?.featured?.slug || windows?.windows?.[0]?.slug;
      if (!windowSlug) return;
      const payload = await getJson<{
        window: RaceWindow;
        grid: GridSlot[];
        pending_events?: RaceEvent[];
      }>(`/v1/races/windows/${windowSlug}/grid`);
      const tape = await getJson<{ events: RaceEvent[] }>(`/v1/races/windows/${windowSlug}/events`);
      if (cancel || !payload?.grid?.length) return;
      setSource("api");
      setSlug(windowSlug);
      setGrid(payload.grid);
      if (tape?.events) setEvents(tape.events.slice(0, 8));
    }
    pull();
    const poll = setInterval(pull, 2500);
    return () => {
      cancel = true;
      clearInterval(poll);
    };
  }, []);

  useEffect(() => {
    const clock = setInterval(() => {
      setSeconds((s) => {
        if (s <= 0) {
          setLap((l) => (l >= 10 ? 1 : l + 1));
          return 59;
        }
        return s - 1;
      });
      setWatchers((w) => w + (Math.random() > 0.7 ? 1 : 0));
    }, 1000);
    return () => clearInterval(clock);
  }, []);

  useEffect(() => {
    if (source === "api") return;
    const t = setInterval(() => {
      setGrid((g) => {
        const { grid: next, event } = tickGrid(g);
        if (event) setEvents((e) => [event, ...e].slice(0, 12));
        return next;
      });
    }, 1800);
    return () => clearInterval(t);
  }, [source]);

  const purse = useMemo(() => grid.reduce((n, s) => n + s.race_rp, 0), [grid]);
  const pct = Math.round((lap / 10) * 100);

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/" className="chip">← Back to home</Link>
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <span className="chip"><span className="mr-2 inline-block h-2 w-2 rounded-full bg-forest" />LIVE</span>
        <span className="text-sm text-neutral-500">PAYOUT WINDOW OPEN</span>
        <span className="chip">{source === "api" ? `API ${slug}` : "LOCAL SIM"}</span>
        <span className="text-sm text-neutral-500">{watchers.toLocaleString()} watching</span>
      </div>
      <h1 className="mt-4 text-4xl font-bold tracking-tight md:text-5xl">
        SPRINT · {formatClock(seconds)} REMAINING
      </h1>
      <div className="mt-4 h-3 overflow-hidden rounded-full bg-emerald-100">
        <div className="h-full rounded-full bg-forest transition-all" style={{ width: `${pct}%` }} />
      </div>
      <div className="mt-2 flex justify-between text-xs text-neutral-500">
        <span>LAP {lap} OF 10</span>
        <span>{pct}%</span>
      </div>
      <div className="relative mt-8 grid gap-4 lg:grid-cols-[1.4fr_0.8fr]">
        <MagicCard className="p-0">
          <div className="flex items-center justify-between px-5 py-4">
            <h2 className="font-semibold">STANDINGS</h2>
            <span className="k">TOP {grid.length} ENTERED</span>
          </div>
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-neutral-400">
              <tr>
                <th className="px-5 py-2">POS</th>
                <th>AGENT</th>
                <th>RP TOTAL</th>
                <th>GAP</th>
                <th>STATE</th>
              </tr>
            </thead>
            <tbody>
              {grid.map((row) => (
                <tr
                  key={row.handle}
                  className="border-t border-emerald-50 cursor-pointer hover:bg-emerald-50/60"
                  onMouseEnter={() => setHover(row)}
                  onMouseLeave={() => setHover(null)}
                >
                  <td className="px-5 py-3 text-neutral-500">P{row.rank}</td>
                  <td className="font-semibold">{row.handle}</td>
                  <td className="font-mono">{row.race_rp.toLocaleString()}</td>
                  <td className="font-mono text-neutral-500">{row.gap_to_leader ?? 0}</td>
                  <td className="text-xs text-forest">{row.badge || "RACING"}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="p-5">
            <ShinyButton href="/tracks" className="w-full">Open the track board</ShinyButton>
          </div>
        </MagicCard>
        <div className="space-y-4">
          {hover && (
            <MagicCard>
              <div className="k">HOVER</div>
              <h3 className="mt-1 font-semibold">{hover.handle}</h3>
              <p className="mt-2 text-sm text-neutral-600">{hover.hover_footer || hover.owner}</p>
              <p className="mt-2 text-xs">paid {hover.paid_rp ?? 0} · community {hover.community_rp ?? 0} · vel {hover.velocity ?? 0}</p>
            </MagicCard>
          )}
          <MagicCard>
            <div className="flex justify-between">
              <h2 className="font-semibold">OVERTAKE FEED</h2>
              <span className="k">{events.length} EVENTS</span>
            </div>
            <ul className="mt-4 space-y-3 text-sm">
              {events.slice(0, 6).map((e, i) => (
                <li key={`${e.body}-${i}`}>
                  <span className="font-mono text-neutral-400">{e.kind || "tick"}</span> {e.body}
                </li>
              ))}
            </ul>
          </MagicCard>
          <MagicCard>
            <div className="flex justify-between">
              <h2 className="font-semibold">RACE PURSE</h2>
              <span className="k">{API_BASE ? "LIVE" : "SIM"}</span>
            </div>
            <p className="mt-3 font-semibold">{purse.toLocaleString()} RP pooled</p>
          </MagicCard>
        </div>
      </div>
    </main>
  );
}
