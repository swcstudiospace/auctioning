"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import GarageCard, { statsFromProject, type GarageStats } from "@/components/garage/GarageCard";
import { getJson, recordClick, type GridSlot, type Project } from "@/lib/api";
import { fetchChampionship } from "@/lib/race";

export default function GarageLive({ handle }: { handle: string }) {
  const [stats, setStats] = useState<GarageStats | null>(null);
  const [points, setPoints] = useState<number | null>(null);
  const [wins, setWins] = useState<number | null>(null);

  useEffect(() => {
    let cancel = false;
    async function load() {
      const [project, grid, standings] = await Promise.all([
        getJson<Partial<Project>>(`/v1/projects/${encodeURIComponent(handle)}`),
        getJson<{ grid: GridSlot[] }>("/v1/grid"),
        fetchChampionship(),
      ]);
      if (cancel) return;
      const slot = grid?.grid?.find((s) => s.handle === handle) || null;
      const standing = standings.find((s) => s.handle === handle) || null;
      setPoints(standing ? standing.points : null);
      setWins(standing ? standing.wins : null);
      const base: Project = {
        handle,
        owner_wallet: project?.owner_wallet ?? null,
        source: project?.source || "manual",
        source_ref: project?.source_ref ?? null,
        display_name: project?.display_name || handle,
        blurb: project?.blurb ?? null,
        stable_id: project?.stable_id ?? null,
        url: project?.url ?? null,
        tags: project?.tags || [],
        total_rp: project?.total_rp ?? slot?.race_rp ?? 0,
        rank: project?.rank ?? slot?.rank ?? 0,
        clicks: project?.clicks ?? slot?.clicks ?? 0,
      };
      setStats(statsFromProject(base, slot));
    }
    load();
    return () => {
      cancel = true;
    };
  }, [handle]);

  if (!stats) {
    return (
      <main className="mx-auto max-w-6xl px-6 py-10">
        <p className="text-sm text-neutral-500">Loading garage…</p>
      </main>
    );
  }

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/rank" className="chip">
        ← Back to board
      </Link>
      <div className="mt-6">
        <GarageCard stats={stats} />
      </div>
      <div className="mt-4 flex flex-wrap items-center gap-3 text-sm">
        {points != null ? (
          <span className="chip">
            {points} pts · {wins ?? 0} wins
          </span>
        ) : (
          <span className="chip">No championship points yet</span>
        )}
        {stats.url ? (
          <a
            href={stats.url}
            target="_blank"
            rel="noreferrer"
            className="text-forest"
            onClick={() => {
              void recordClick(stats.handle);
            }}
          >
            Visit site
          </a>
        ) : null}
        <Link href={"/rank?q=" + encodeURIComponent(stats.handle)} className="text-forest">
          Support on the board →
        </Link>
      </div>
    </main>
  );
}
