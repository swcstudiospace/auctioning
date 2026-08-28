"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { getJson, type GridSlot } from "@/lib/api";
import { seedGrid, tickGrid } from "@/lib/sim";

export default function GarageLive({ handle }: { handle: string }) {
  const [slot, setSlot] = useState<GridSlot | null>(null);
  const [clock, setClock] = useState(0);

  useEffect(() => {
    let grid = seedGrid();
    const apply = (g: GridSlot[]) => {
      setSlot(g.find((s) => s.handle === handle) || g[0]);
    };
    apply(grid);
    getJson<{ grid: GridSlot[] }>("/v1/grid").then((res) => {
      if (res?.grid?.length) {
        grid = res.grid;
        apply(grid);
      }
    });
    const id = setInterval(() => {
      const ticked = tickGrid(grid);
      grid = ticked.grid;
      apply(grid);
      setClock((c) => c + 1);
    }, 2000);
    return () => clearInterval(id);
  }, [handle]);

  const s = slot;
  if (!s) return null;
  const paidPct = s.race_rp ? Math.round(((s.paid_rp ?? 0) / s.race_rp) * 100) : 68;

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/championship" className="chip">← Back to championship</Link>
      <div className="mt-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-4xl font-bold">{handle} GARAGE</h1>
          <p className="mt-2 max-w-xl text-neutral-600">
            Live bid telemetry — pay-per-RP racing, replays, and pace deltas.
          </p>
        </div>
        <span className="chip">TELEMETRY LIVE</span>
      </div>
      <div className="mt-8 grid gap-4 md:grid-cols-4">
        <MagicCard>
          <div className="k">RACE RP</div>
          <div className="mt-2 font-mono text-3xl">{s.race_rp.toLocaleString()}</div>
        </MagicCard>
        <MagicCard>
          <div className="k">LIFETIME RP</div>
          <div className="mt-2 font-mono text-3xl">{(s.race_rp * 3 + 380).toLocaleString()}</div>
        </MagicCard>
        <MagicCard>
          <div className="k">PACE</div>
          <div className="mt-2 font-mono text-3xl">{s.velocity ?? 0} RP/MIN</div>
        </MagicCard>
        <MagicCard>
          <div className="k">VELOCITY</div>
          <div className="mt-2 font-mono text-3xl">{s.velocity ?? 0}</div>
        </MagicCard>
      </div>
      <div className="mt-4 grid gap-4 md:grid-cols-[1.4fr_0.6fr]">
        <MagicCard>
          <div className="k">PAID VS COMMUNITY</div>
          <div className="mt-4 font-mono text-xl">{paidPct}% PAID</div>
          <div className="mt-3 h-3 overflow-hidden rounded-full bg-neutral-200">
            <div className="h-full bg-forest transition-all" style={{ width: `${paidPct}%` }} />
          </div>
        </MagicCard>
        <MagicCard>
          <div className="k">CLICKS</div>
          <div className="font-mono text-3xl">{(s.clicks ?? 0) + clock}</div>
        </MagicCard>
      </div>
      <MagicCard className="mt-4">
        <h2 className="font-semibold">HOW THEY DID IT</h2>
        <p className="mt-2 text-sm text-neutral-600">
          Last overtake: {s.last_overtake || "none this window"}. Badge: {s.badge || "RACING"}.
        </p>
        <div className="mt-4 flex justify-end">
          <ShinyButton href="/rules">How scoring works →</ShinyButton>
        </div>
      </MagicCard>
    </main>
  );
}
