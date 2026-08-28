"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { getJson } from "@/lib/api";
import { championship as seed } from "@/lib/data";

type Row = { handle: string; points: number; wins?: number; best_finish?: number; rank: number };

export default function ChampionshipBoard() {
  const [rows, setRows] = useState<Row[]>(
    seed.map((r) => ({ handle: r.name, points: r.pts, rank: r.rank }))
  );

  useEffect(() => {
    async function pull() {
      const res = await getJson<{ standings: Row[] }>("/v1/championship");
      if (res?.standings?.length) setRows(res.standings);
    }
    pull();
    const id = setInterval(pull, 4000);
    return () => clearInterval(id);
  }, []);

  const max = Math.max(1, ...rows.map((r) => r.points));

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/tracks" className="chip">← Back to track</Link>
      <div className="mt-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="k">PAY-TO-RACE SERIES · SEASON ONE</p>
          <h1 className="mt-2 text-4xl font-bold">CHAMPIONSHIP S1 2026</h1>
        </div>
        <div className="chip">PRIZE POOL 50,000 RP · paid to top 3 at season close</div>
      </div>
      <MagicCard className="mt-8">
        <div className="flex justify-between">
          <div>
            <h2 className="font-semibold">SEASON STANDINGS</h2>
            <p className="k mt-1">DRIVER CHAMPIONSHIP</p>
          </div>
          <p className="max-w-xs text-right text-xs text-neutral-500">Bars scale from the leader. Polls /v1/championship when the API is set.</p>
        </div>
        <div className="mt-6 space-y-4">
          {rows.map((row) => (
            <div key={row.handle} className="flex items-center gap-3 text-sm">
              <span className="w-6 text-neutral-400">{row.rank}</span>
              <span className="w-32 font-semibold">{row.handle}</span>
              <div className="h-3 flex-1 overflow-hidden rounded-full bg-emerald-50">
                <div className="h-full rounded-full bg-forest transition-all duration-700" style={{ width: `${(row.points / max) * 100}%` }} />
              </div>
              <span className="w-10 font-mono">{row.points}</span>
              {row.rank === 1 ? (
                <Link href={`/garage/${row.handle}`} className="text-xs text-forest">VIEW GARAGE ↗</Link>
              ) : (
                <span className="w-24" />
              )}
            </div>
          ))}
        </div>
      </MagicCard>
    </main>
  );
}
