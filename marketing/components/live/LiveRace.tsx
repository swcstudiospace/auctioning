"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { BlurFade } from "@/components/magic/BlurFade";
import { ActivePaceChip } from "@/components/chrome/ActivePaceChip";
import { RaceBadge } from "@/components/chrome/RaceBadge";
import CompanyIcon from "@/components/chrome/CompanyIcon";
import { getJson, listProjects, type GridSlot, type RaceEvent, type RaceWindow } from "@/lib/api";
import { fetchCalendar, fetchChampionship, type ChampionshipStanding, type RaceCalendar } from "@/lib/race";

type Mode = "live" | "sprint" | "gp" | "championship" | "specials";

type Standing = {
  rank: number;
  handle: string;
  display_name: string;
  url: string | null;
  race_rp: number;
  gap: number;
  badge: string;
};

function kindOf(w: RaceWindow | undefined): string {
  return (w?.race_type || w?.kind || "").toUpperCase();
}

function isSprint(w: RaceWindow | undefined) {
  return /SPRINT|GREEN_FLAG|PACE_LAP/.test(kindOf(w));
}
function isGp(w: RaceWindow | undefined) {
  return /GRAND_PRIX|GRAND_TOUR|GRAND PRIX|SECTOR_SCRAP/.test(kindOf(w));
}
function isChamp(w: RaceWindow | undefined) {
  return /CHAMPIONSHIP|TITLE_FIGHT/.test(kindOf(w));
}
function isSpecial(w: RaceWindow | undefined) {
  return /SPECIAL|PHOTO_CARD/.test(kindOf(w));
}

function remainingSecs(w: RaceWindow | undefined): number {
  if (!w?.ends_at) return 0;
  const end = Date.parse(w.ends_at);
  if (Number.isNaN(end)) return 0;
  return Math.max(0, Math.floor((end - Date.now()) / 1000));
}

function clock(secs: number) {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (h > 0) return h + ":" + String(m).padStart(2, "0") + ":" + String(s).padStart(2, "0");
  return String(m).padStart(2, "0") + ":" + String(s).padStart(2, "0");
}

function gpDay(w: RaceWindow | undefined): string {
  if (!w?.starts_at || !w?.ends_at) return "";
  const start = Date.parse(w.starts_at);
  const end = Date.parse(w.ends_at);
  if (Number.isNaN(start) || Number.isNaN(end)) return "";
  const total = Math.max(1, Math.round((end - start) / 86400000));
  const day = Math.min(total, Math.max(1, Math.floor((Date.now() - start) / 86400000) + 1));
  return "D" + day + "/" + total;
}

const TABS: { id: Mode; label: string }[] = [
  { id: "live", label: "Live Grid" },
  { id: "sprint", label: "Sprints" },
  { id: "gp", label: "Grand Prix" },
  { id: "championship", label: "Championship" },
  { id: "specials", label: "Specials" },
];

export default function LiveRace() {
  const [mode, setMode] = useState<Mode>("live");
  const [calendar, setCalendar] = useState<RaceCalendar | null>(null);
  const [champ, setChamp] = useState<ChampionshipStanding[]>([]);
  const [grid, setGrid] = useState<Standing[]>([]);
  const [events, setEvents] = useState<RaceEvent[]>([]);
  const [names, setNames] = useState<Record<string, { display_name: string; url: string | null }>>({});
  const [now, setNow] = useState(Date.now());

  const windows = calendar?.windows || [];
  const sprint = windows.find((w) => isSprint(w) && (w.status === "live" || w.status === "scheduled"));
  const gp = windows.find((w) => isGp(w) && (w.status === "live" || w.status === "scheduled"));
  const champWindow = windows.find(isChamp);
  const special = windows.find(isSpecial);
  const featured = calendar?.featured;
  const leader = champ[0] || null;

  const active: RaceWindow | undefined =
    mode === "sprint" ? sprint : mode === "gp" || mode === "live" ? gp : mode === "specials" ? special : champWindow;

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    let cancel = false;
    async function pull() {
      const [cal, standings, list] = await Promise.all([
        fetchCalendar(),
        fetchChampionship(),
        listProjects({ page: 1, per_page: 50 }),
      ]);
      if (cancel) return;
      setCalendar(cal);
      setChamp(standings);
      if (list.ok) {
        const map: Record<string, { display_name: string; url: string | null }> = {};
        for (const p of list.data.projects || []) {
          map[p.handle] = { display_name: p.display_name || p.handle, url: p.url };
        }
        setNames(map);
      }
    }
    pull();
    const poll = setInterval(pull, 8000);
    return () => {
      cancel = true;
      clearInterval(poll);
    };
  }, []);

  useEffect(() => {
    let cancel = false;
    async function pullGrid() {
      const slug = active?.slug;
      if (!slug || mode === "championship") {
        setGrid([]);
        setEvents([]);
        return;
      }
      const payload = await getJson<{ window: RaceWindow; grid: GridSlot[] }>(
        "/v1/races/windows/" + encodeURIComponent(slug) + "/grid",
      );
      const tape = await getJson<{ events: RaceEvent[] }>(
        "/v1/races/windows/" + encodeURIComponent(slug) + "/events",
      );
      if (cancel) return;
      const slots = payload?.grid || [];
      setGrid(
        slots.map((s) => ({
          rank: s.rank,
          handle: s.handle,
          display_name: names[s.handle]?.display_name || s.handle,
          url: names[s.handle]?.url || null,
          race_rp: s.race_rp,
          gap: s.gap_to_next ?? s.gap_to_leader ?? 0,
          badge: s.badge || "",
        })),
      );
      setEvents(tape?.events || []);
    }
    pullGrid();
    const poll = setInterval(pullGrid, 4000);
    return () => {
      cancel = true;
      clearInterval(poll);
    };
  }, [active?.slug, mode, names]);

  const headline = useMemo(() => {
    if (mode === "championship") return "Championship · points, not RP";
    if (mode === "specials") return special ? special.name || "Special event" : "No specials on the calendar";
    if (mode === "sprint") {
      if (!sprint) return "No sprint this hour";
      return "Sprint · " + clock(remainingSecs(sprint)) + " remaining";
    }
    if (!gp) return "No Grand Prix on the calendar";
    return "Grand Prix · " + gpDay(gp) + " · " + clock(remainingSecs(gp)) + " remaining";
  }, [mode, sprint, gp, special, now]);

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <BlurFade>
          <p className="k text-forest">2026 Season 1</p>
          <h1 className="mt-2 text-4xl font-bold tracking-tight md:text-5xl">{headline}</h1>
          <p className="mt-3 max-w-xl text-sm text-neutral-600">
            One live grid. Sprint is popcorn. GP is the feature. Championship is the season table.
            {featured?.because ? " " + featured.because : ""}
          </p>
        </BlurFade>
        <div className="flex flex-wrap items-center gap-2">
          <ActivePaceChip />
          <span className="chip">
            <span className="mr-2 inline-block h-2 w-2 rounded-full bg-forest" />
            {active?.status === "live" ? "LIVE" : "CALENDAR"}
          </span>
        </div>
      </div>

      <div className="mt-8 flex flex-wrap gap-2">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setMode(t.id)}
            className={"chip" + (mode === t.id ? " bg-forest text-white" : "")}
          >
            {t.label}
          </button>
        ))}
      </div>

      <p className="mt-4 text-xs tracking-[0.16em] text-neutral-500 md:hidden">
        Sprint {sprint ? clock(remainingSecs(sprint)) : "—"} · GP {gp ? gpDay(gp) : "—"} · Champ {leader ? leader.handle : "—"}
      </p>

      <div className="mt-6 grid gap-4 lg:grid-cols-[1.45fr_0.8fr]">
        <MagicCard className="p-0" beam>
          {mode === "championship" ? (
            <div className="p-5">
              <h2 className="font-semibold">POINTS TABLE</h2>
              <p className="mt-2 text-sm text-neutral-500">GP P1–P10 25…1 · sprint P1–P3 8/7/6 · fastest pace +1</p>
              {champ.length ? (
                <table className="mt-4 w-full text-sm">
                  <thead className="text-left text-xs text-neutral-400">
                    <tr>
                      <th className="py-2">POS</th>
                      <th>AGENT</th>
                      <th>PTS</th>
                    </tr>
                  </thead>
                  <tbody>
                    {champ.map((r, i) => (
                      <tr key={r.handle} className="border-t border-emerald-50">
                        <td className="py-3 text-neutral-500">P{i + 1}</td>
                        <td>
                          <Link href={"/garage/" + encodeURIComponent(r.handle)} className="font-semibold hover:text-forest">
                            {r.display_name || r.handle}
                          </Link>
                        </td>
                        <td className="font-mono">{r.points}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <p className="mt-6 text-sm text-neutral-500">No points until a sprint or GP archives.</p>
              )}
            </div>
          ) : mode === "specials" ? (
            <div className="p-5">
              <h2 className="font-semibold">Operator cards</h2>
              <p className="mt-2 text-sm text-neutral-500">
                Paid RP stays $1 = 1. Cards only add event_multiplier pace. They do not stack unless rules say so.
              </p>
              {calendar?.active_card?.name || calendar?.active_card?.title ? (
                <div className="mt-6 rounded-2xl border border-emerald-100 bg-emerald-50/60 p-4">
                  <p className="k text-forest">{calendar.active_card.slug || "live"}</p>
                  <h3 className="mt-1 text-2xl font-bold">
                    {calendar.active_card.name || calendar.active_card.title}
                  </h3>
                  {calendar.active_card.multiplier_bps ? (
                    <p className="mt-2 font-mono text-lg">
                      {(calendar.active_card.multiplier_bps / 10000).toFixed(2).replace(/\.00$/, "")}× pace
                    </p>
                  ) : null}
                  {calendar.active_card.ends_at ? (
                    <p className="mt-1 text-xs text-neutral-500">Until {calendar.active_card.ends_at}</p>
                  ) : null}
                </div>
              ) : (
                <p className="mt-6 text-sm text-neutral-500">
                  No live card. Afterburner, Night Grid, Pit Lane, and Final Lap land here when an operator opens them.
                </p>
              )}
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between px-5 py-4">
                <h2 className="font-semibold">{mode === "sprint" ? "SPRINT BOARD" : "LIVE GRID"}</h2>
                <span className="k">{active?.name || "No window"} · windowed RP</span>
              </div>
              {grid.length ? (
                <table className="w-full text-sm">
                  <thead className="text-left text-xs text-neutral-400">
                    <tr>
                      <th className="px-5 py-2">POS</th>
                      <th>AGENT</th>
                      <th>RACE RP</th>
                      <th>GAP</th>
                      <th>STATE</th>
                    </tr>
                  </thead>
                  <tbody>
                    {grid.map((row) => (
                      <tr key={row.handle} className="border-t border-emerald-50 hover:bg-emerald-50/60">
                        <td className="px-5 py-3 text-neutral-500">P{row.rank}</td>
                        <td>
                          <Link
                            href={"/garage/" + encodeURIComponent(row.handle)}
                            className="flex items-center gap-2 font-semibold hover:text-forest"
                          >
                            <CompanyIcon url={row.url} name={row.display_name} size={22} />
                            {row.display_name}
                          </Link>
                        </td>
                        <td className="font-mono">{row.race_rp.toLocaleString()}</td>
                        <td className="font-mono text-neutral-500">{row.gap}</td>
                        <td className="text-xs text-forest"><RaceBadge badge={row.badge} /></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : (
                <p className="px-5 py-10 text-sm text-neutral-500">
                  No fuel in this window yet. Race RP is spend inside the event, not lifetime rank. Fuel a listing to light the grid.
                </p>
              )}
              <div className="p-5">
                <ShinyButton href="/rank#add" className="w-full">
                  Fuel a listing
                </ShinyButton>
              </div>
            </>
          )}
        </MagicCard>

        <div className="space-y-4">
          <MagicCard>
            <div className="k">Calendar rail</div>
            <div className="mt-3 space-y-4 text-sm">
              {featured?.because ? (
                <p className="rounded-xl bg-emerald-50 px-3 py-2 text-xs text-neutral-700">{featured.because}</p>
              ) : null}
              <div>
                <div className="flex justify-between font-semibold">
                  <span>Sprint</span>
                  <span className="font-mono text-forest">{sprint ? clock(remainingSecs(sprint)) : "—"}</span>
                </div>
                <p className="text-neutral-500">
                  {sprint ? `${sprint.status === "live" ? "Live" : "Upcoming"} · ${sprint.name}` : "No sprint window"}
                </p>
              </div>
              <div>
                <div className="flex justify-between font-semibold">
                  <span>Grand Prix</span>
                  <span className="font-mono text-forest">{gp ? `${gpDay(gp)} · ${clock(remainingSecs(gp))}` : "—"}</span>
                </div>
                <p className="text-neutral-500">{gp?.name || "No GP window"}</p>
              </div>
              <div>
                <div className="font-semibold">Championship leader</div>
                <p className="text-neutral-500">
                  {leader ? (leader.display_name || leader.handle) + " · " + leader.points + " pts" : "Empty until a race archives"}
                </p>
              </div>
            </div>
          </MagicCard>
          <MagicCard>
            <div className="flex justify-between">
              <h2 className="font-semibold">OVERTAKE TICKER</h2>
              <span className="k">{events.length} EVENTS</span>
            </div>
            <ul className="mt-4 space-y-3 text-sm">
              {events.length ? (
                events.slice(0, 8).map((e, i) => (
                  <li key={(e.title || e.body || "e") + "-" + i}>
                    <span className="font-mono text-neutral-400">{e.kind || e.event_type || "tick"}</span>{" "}
                    {e.body || e.summary || e.title}
                  </li>
                ))
              ) : (
                <li className="text-neutral-500">No overtakes yet. First race RP lights this tape.</li>
              )}
            </ul>
          </MagicCard>
        </div>
      </div>
    </main>
  );
}
