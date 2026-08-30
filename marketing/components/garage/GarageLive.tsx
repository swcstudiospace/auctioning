"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import GarageCard, { statsFromProject, type GarageStats } from "@/components/garage/GarageCard";
import { getJson, type GridSlot, type Project } from "@/lib/api";

export default function GarageLive({ handle }: { handle: string }) {
  const [stats, setStats] = useState<GarageStats | null>(null);

  useEffect(() => {
    let cancel = false;
    async function load() {
      const [project, grid] = await Promise.all([
        getJson<Project>(`/v1/projects/${encodeURIComponent(handle)}`),
        getJson<{ grid: GridSlot[] }>("/v1/grid"),
      ]);
      if (cancel) return;
      const slot = grid?.grid?.find((s) => s.handle === handle) || null;
      if (project) {
        setStats(statsFromProject(project, slot));
        return;
      }
      setStats(
        statsFromProject(
          {
            handle,
            owner_wallet: null,
            source: "manual",
            source_ref: null,
            display_name: handle,
            blurb: null,
            stable_id: null,
            url: null,
            tags: [],
            total_rp: slot?.race_rp ?? 0,
            rank: slot?.rank ?? 0,
          },
          slot,
        ),
      );
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
      <Link href="/rank" className="chip">← Back to board</Link>
      <div className="mt-6">
        <GarageCard stats={stats} />
      </div>
    </main>
  );
}
