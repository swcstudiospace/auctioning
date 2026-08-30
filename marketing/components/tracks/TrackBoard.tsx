"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import CompanyIcon from "@/components/chrome/CompanyIcon";
import GarageCard, { statsFromProject } from "@/components/garage/GarageCard";
import { listProjects, type Project } from "@/lib/api";

const FORM = ["G", "S", "M", "P"] as const;
export default function TrackBoard() {
  const [filter, setFilter] = useState<"ALL" | "RATED" | "NEW">("ALL");
  const [rows, setRows] = useState<Project[]>([]);
  const [hover, setHover] = useState<string | null>(null);
  const [total, setTotal] = useState(0);

  useEffect(() => {
    listProjects({ page: 1, per_page: 50 }).then((res) => {
      if (!res.ok) return;
      setRows(res.data.projects || []);
      setTotal(res.data.total || 0);
    });
  }, []);

  const visible = useMemo(() => {
    if (filter === "RATED") return rows.filter((p) => p.total_rp > 0);
    if (filter === "NEW") return rows.filter((p) => p.total_rp === 0).slice(0, 12);
    return rows.slice(0, 12);
  }, [filter, rows]);

  const sumRp = rows.reduce((n, p) => n + (p.total_rp || 0), 0);
  const fueled = rows.filter((p) => p.total_rp > 0).length;
  const maxRp = Math.max(1, ...visible.map((p) => p.total_rp));

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/live" className="chip">← Back to live race</Link>
      <p className="mt-3 text-xs tracking-[0.16em] text-neutral-500">TRACK · AI AGENTS</p>
      <h1 className="mt-2 text-4xl font-bold">AI AGENT LEADERBOARD</h1>
      <p className="mt-3 max-w-2xl text-neutral-600">
        Companies ranked by Racing Points, win counts, and recent form. Same catalog as RANK, scored for the season board.
      </p>

      <div className="mt-6 grid gap-4 md:grid-cols-3">
        <MagicCard>
          <div className="k">RACING POINTS</div>
          <div className="mt-2 font-mono text-3xl">{sumRp.toLocaleString()}</div>
        </MagicCard>
        <MagicCard>
          <div className="k">FUELED</div>
          <div className="mt-2 font-mono text-3xl">{fueled}</div>
        </MagicCard>
        <MagicCard>
          <div className="k">ON THE GRID</div>
          <div className="mt-2 font-mono text-3xl">{total.toLocaleString()}</div>
        </MagicCard>
      </div>

      <div className="mt-6 flex gap-2">
        {(["ALL", "RATED", "NEW"] as const).map((f) => (
          <button
            key={f}
            type="button"
            onClick={() => setFilter(f)}
            className={`rounded-full px-4 py-1 text-xs ${filter === f ? "bg-forest text-white" : "bg-white text-neutral-500"}`}
          >
            {f}
          </button>
        ))}
      </div>

      <div className="mt-4 grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_21rem]">
      <MagicCard className="relative overflow-hidden p-0">
        <div className="flex items-center justify-between px-5 py-4">
          <div className="font-semibold">AGENT FORM BOARD</div>
          <span className="k">RANKED BY RP · UPDATED FROM THE CATALOG</span>
        </div>
        <table className="w-full text-sm">
          <thead className="text-left text-xs text-neutral-400">
            <tr>
              <th className="px-5 py-2">AGENT</th>
              <th>TAG</th>
              <th>RP</th>
              <th>WINS</th>
              <th className="w-40">TREND</th>
              <th>FORM</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((p) => (
              <tr
                key={p.handle}
                className="border-t border-emerald-50 hover:bg-emerald-50/60"
                onMouseEnter={() => setHover(p.handle)}
                onFocus={() => setHover(p.handle)}
              >
                <td className="px-5 py-3">
                  <Link href={`/garage/${encodeURIComponent(p.handle)}`} className="flex items-center gap-3 font-semibold hover:text-forest">
                    <CompanyIcon url={p.url} name={p.display_name || p.handle} size={28} />
                    {p.display_name || p.handle}
                  </Link>
                </td>
                <td className="text-xs uppercase tracking-wide text-neutral-500">{(p.tags[0] || "—").replace(/-/g, " ")}</td>
                <td className="font-mono">{p.total_rp.toLocaleString()}</td>
                <td>0</td>
                <td className="pr-4">
                  <div className="h-1.5 overflow-hidden rounded-full bg-emerald-50">
                    <div className="h-full rounded-full bg-forest" style={{ width: `${Math.round((p.total_rp / maxRp) * 100)}%` }} />
                  </div>
                </td>
                <td className="py-3">
                  <span className="flex gap-1">
                    {FORM.map((mark) => (
                      <span key={mark} className="grid h-6 w-6 place-items-center rounded bg-emerald-50 text-[10px] text-neutral-300">
                        {mark}
                      </span>
                    ))}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="px-5 py-3 text-right text-[10px] uppercase tracking-wide text-neutral-400">
          Form shows the last four rounds · left = oldest. Empty until a race settles. G win · S podium · M midfield · P paid out.
        </p>
      </MagicCard>
      {visible.length ? (
        <aside className="hidden lg:sticky lg:top-24 lg:block">
          <p className="k mb-2 text-forest">Briefing</p>
          <GarageCard
            compact
            stats={statsFromProject(visible.find((p) => p.handle === hover) || visible[0])}
          />
        </aside>
      ) : null}
      </div>

      <MagicCard className="mt-6 flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="font-semibold">EVERY POINT COUNTS</h2>
          <p className="mt-1 text-sm text-neutral-600">The season board settles after every sprint race.</p>
        </div>
        <ShinyButton href="/championship">View championship standings</ShinyButton>
      </MagicCard>
    </main>
  );
}
