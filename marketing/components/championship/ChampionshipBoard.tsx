"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { fetchChampionship, type ChampionshipStanding } from "@/lib/race";

export default function ChampionshipBoard() {
  const [rows, setRows] = useState<ChampionshipStanding[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    fetchChampionship().then((s) => {
      setRows(s);
      setLoaded(true);
    });
  }, []);

  const max = Math.max(1, ...rows.map((r) => r.points));

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/tracks" className="chip">← Back to track</Link>
      <div className="mt-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-xs tracking-[0.16em] text-neutral-500">PAY-TO-RACE SERIES · SEASON ONE</p>
          <h1 className="mt-2 text-4xl font-bold">CHAMPIONSHIP S1 2026</h1>
          <p className="mt-3 max-w-xl text-neutral-600">
            Points, not catalog RP. GPs and sprints write this table when a window archives.
          </p>
        </div>
        <MagicCard className="min-w-[16rem]">
          <div className="k">SCORING</div>
          <p className="mt-2 text-sm">GP P1–P10: 25 18 15 12 10 8 6 4 2 1</p>
          <p className="mt-1 text-sm">Sprint P1–P3: 8 7 6 · fastest pace +1</p>
        </MagicCard>
      </div>

      <MagicCard className="mt-8 p-0">
        <div className="flex items-center justify-between px-5 py-4">
          <div className="font-semibold">SEASON STANDINGS / DRIVER CHAMPIONSHIP</div>
          <span className="k">POINTS</span>
        </div>
        {!loaded ? (
          <p className="px-5 pb-5 text-sm text-neutral-500">Loading standings…</p>
        ) : rows.length === 0 ? (
          <p className="px-5 pb-5 text-sm text-neutral-500">
            No finished GPs or sprints yet. This stays empty until a race window archives. Catalog RP does not count.
          </p>
        ) : (
          <div className="space-y-5 px-5 pb-5">
            {rows.map((r) => (
              <div key={r.handle}>
                <div className="mb-1 flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2">
                    <span className="w-8 font-mono text-sm text-neutral-500">P{r.rank}</span>
                    <Link href={`/garage/${encodeURIComponent(r.handle)}`} className="text-sm font-semibold hover:text-forest">
                      {r.display_name || r.handle}
                    </Link>
                    <span className="text-[10px] tracking-[0.16em] text-neutral-400">{r.wins} WINS</span>
                  </div>
                  <span className="font-mono text-sm">{r.points.toLocaleString()} PTS</span>
                </div>
                <div className="h-2 overflow-hidden rounded-full bg-emerald-50">
                  <div className="h-full rounded-full bg-forest" style={{ width: `${Math.round((r.points / max) * 100)}%` }} />
                </div>
              </div>
            ))}
          </div>
        )}
      </MagicCard>

      <div className="mt-8 flex justify-center">
        <ShinyButton href="/garage">View garage telemetry</ShinyButton>
      </div>
    </main>
  );
}
