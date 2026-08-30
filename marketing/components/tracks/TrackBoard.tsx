"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import CompanyIcon from "@/components/chrome/CompanyIcon";
import { RaceBadge } from "@/components/chrome/RaceBadge";
import GarageCard, { statsFromProject } from "@/components/garage/GarageCard";
import { listProjects, getJson, type GridSlot, type Project } from "@/lib/api";
import { fetchChampionship, type ChampionshipStanding } from "@/lib/race";

export default function TrackBoard() {
  const [tag, setTag] = useState<string>("");
  const [tags, setTags] = useState<string[]>([]);
  const [rows, setRows] = useState<Project[]>([]);
  const [total, setTotal] = useState(0);
  const [gridByHandle, setGridByHandle] = useState<Record<string, GridSlot>>({});
  const [champ, setChamp] = useState<Record<string, ChampionshipStanding>>({});
  const [hover, setHover] = useState<string | null>(null);
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const scheduleHover = useCallback((handle: string) => {
    if (hoverTimer.current) clearTimeout(hoverTimer.current);
    hoverTimer.current = setTimeout(() => setHover(handle), 250);
  }, []);

  useEffect(() => {
    listProjects({ page: 1, per_page: 50, tag: tag || undefined }).then((res) => {
      if (!res.ok) return;
      setRows(res.data.projects || []);
      setTotal(res.data.total || 0);
      if (!tag && res.data.tags?.length) setTags(res.data.tags);
    });
  }, [tag]);

  useEffect(() => {
    getJson<{ grid: GridSlot[] }>("/v1/grid").then((res) => {
      const map: Record<string, GridSlot> = {};
      for (const s of res?.grid || []) map[s.handle] = s;
      setGridByHandle(map);
    });
    fetchChampionship().then((standings) => {
      const map: Record<string, ChampionshipStanding> = {};
      for (const s of standings) map[s.handle] = s;
      setChamp(map);
    });
  }, []);

  const visible = rows.slice(0, 24);
  const sumRp = rows.reduce((n, p) => n + (p.total_rp || 0), 0);
  const fueled = rows.filter((p) => p.total_rp > 0).length;
  const brief = visible.find((p) => p.handle === hover) || visible[0];
  const title = tag ? tag.replace(/-/g, " ") : "Open grid";

  const chips = useMemo(() => ["", ...tags.slice(0, 12)], [tags]);

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/live" className="chip">
        ← Back to live race
      </Link>
      <p className="mt-3 text-xs tracking-[0.16em] text-neutral-500">TRACK · SECTOR SCRAP</p>
      <h1 className="mt-2 text-4xl font-bold capitalize">{title}</h1>
      <p className="mt-3 max-w-2xl text-neutral-600">
        One tag, one board. Rank is catalog RP. Wins and points come from archived sprints and GPs, not invented form.
      </p>

      <div className="mt-6 grid gap-4 md:grid-cols-3">
        <MagicCard>
          <div className="k">Racing points</div>
          <div className="mt-2 font-mono text-3xl">{sumRp.toLocaleString()}</div>
        </MagicCard>
        <MagicCard>
          <div className="k">Fueled</div>
          <div className="mt-2 font-mono text-3xl">{fueled}</div>
        </MagicCard>
        <MagicCard>
          <div className="k">On this track</div>
          <div className="mt-2 font-mono text-3xl">{total.toLocaleString()}</div>
        </MagicCard>
      </div>

      <div className="mt-6 flex flex-wrap gap-2">
        {chips.map((t) => (
          <button
            key={t || "all"}
            type="button"
            onClick={() => setTag(t)}
            className={"chip" + (tag === t ? " bg-forest text-white" : "")}
          >
            {t ? t.replace(/-/g, " ") : "All tags"}
          </button>
        ))}
      </div>

      <div className="mt-4 grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_21rem]">
        <MagicCard className="relative overflow-hidden p-0">
          <div className="flex items-center justify-between px-5 py-4">
            <div className="font-semibold">Sector board</div>
            <span className="k">Ranked by RP · badges from /v1/grid</span>
          </div>
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-neutral-400">
              <tr>
                <th className="px-5 py-2">Pos</th>
                <th>Agent</th>
                <th>RP</th>
                <th>Pts</th>
                <th>Wins</th>
                <th>State</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((p) => {
                const standing = champ[p.handle];
                const badge = gridByHandle[p.handle]?.badge;
                return (
                  <tr
                    key={p.handle}
                    className="border-t border-emerald-50 hover:bg-emerald-50/60"
                    onMouseEnter={() => scheduleHover(p.handle)}
                    onFocus={() => setHover(p.handle)}
                  >
                    <td className="px-5 py-3 text-neutral-500">P{p.rank}</td>
                    <td>
                      <Link
                        href={"/garage/" + encodeURIComponent(p.handle)}
                        className="flex items-center gap-3 font-semibold hover:text-forest"
                      >
                        <CompanyIcon url={p.url} name={p.display_name || p.handle} size={28} />
                        {p.display_name || p.handle}
                      </Link>
                    </td>
                    <td className="font-mono">{p.total_rp.toLocaleString()}</td>
                    <td className="font-mono">{standing?.points ?? 0}</td>
                    <td className="font-mono">{standing?.wins ?? 0}</td>
                    <td>
                      <RaceBadge badge={badge} />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {!visible.length ? (
            <p className="px-5 py-8 text-sm text-neutral-500">No listings on this tag yet.</p>
          ) : null}
        </MagicCard>
        {brief ? (
          <aside className="hidden lg:sticky lg:top-24 lg:block">
            <p className="k mb-2 text-forest">Briefing</p>
            <GarageCard compact stats={statsFromProject(brief, gridByHandle[brief.handle] || null)} />
          </aside>
        ) : null}
      </div>

      <MagicCard className="mt-6 flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="font-semibold">Every point counts</h2>
          <p className="mt-1 text-sm text-neutral-600">Championship points land after a sprint or GP archives.</p>
        </div>
        <ShinyButton href="/championship">View championship standings</ShinyButton>
      </MagicCard>
    </main>
  );
}
