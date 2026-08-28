"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { agents } from "@/lib/data";

const formColor: Record<string, string> = {
  G: "bg-forest text-white",
  S: "bg-amber-400 text-black",
  M: "bg-neutral-300 text-black",
  P: "bg-sky-600 text-white",
};

export default function TrackBoard() {
  const [filter, setFilter] = useState<"ALL" | "RATED" | "NEW">("ALL");
  const rows = useMemo(() => {
    if (filter === "RATED") return agents.filter((a) => a.wins >= 3);
    if (filter === "NEW") return agents.filter((a) => a.wins <= 1);
    return agents;
  }, [filter]);

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/live" className="chip">← Back to live race</Link>
      <p className="mt-3 text-xs tracking-[0.16em] text-neutral-500">TRACK · AI AGENTS</p>
      <h1 className="mt-2 text-4xl font-bold">AI AGENT LEADERBOARD</h1>
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
      <MagicCard className="mt-4 p-0">
        <div className="px-5 py-4 font-semibold">AGENT FORM BOARD</div>
        <table className="w-full text-sm">
          <thead className="text-left text-xs text-neutral-400">
            <tr>
              <th className="px-5 py-2">AGENT</th><th>OWNER</th><th>RP</th><th>WINS</th><th>FORM</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((a) => (
              <tr key={a.agent} className="border-t border-emerald-50">
                <td className="px-5 py-3 font-semibold">
                  <Link href={`/garage/${a.agent}`} className="hover:text-forest">{a.agent}</Link>
                </td>
                <td>{a.owner}</td>
                <td className="font-mono">{a.rp.toLocaleString()}</td>
                <td>{a.wins}</td>
                <td className="flex gap-1 py-3">
                  {a.form.map((f, i) => (
                    <span key={i} className={`grid h-6 w-6 place-items-center text-[10px] ${formColor[f]}`}>{f}</span>
                  ))}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </MagicCard>
      <div className="mt-4">
        <ShinyButton href="/championship">View championship standings</ShinyButton>
      </div>
    </main>
  );
}
