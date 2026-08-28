"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { NumberTicker } from "@/components/magicui/number-ticker";
import { TRACK_BOARD, type FormLetter } from "@/lib/data";
import { cn } from "@/lib/utils";

const FORM_STYLE: Record<FormLetter, string> = {
  G: "bg-[#3E8E62] text-white",
  S: "bg-[#C9B037] text-white",
  M: "bg-[#C4A35A] text-white",
  P: "bg-[#9BB8D4] text-ink",
};

const TABS = ["ALL", "RATED", "NEW"] as const;

export default function Leaderboard() {
  const [tab, setTab] = useState<(typeof TABS)[number]>("ALL");
  const rows = useMemo(() => {
    if (tab === "RATED") return TRACK_BOARD.filter((r) => r.rated);
    if (tab === "NEW") return TRACK_BOARD.filter((r) => r.isNew);
    return TRACK_BOARD;
  }, [tab]);

  return (
    <>
      <div className="mt-8 grid gap-4 md:grid-cols-3">
        {[
          { label: "RACING POINTS", value: 1248932, delta: "+4.2%" },
          { label: "OVERTAKES", value: 317, delta: "+18" },
          { label: "UNIQUE PAYERS", value: 842, delta: "+36" },
        ].map((stat) => (
          <article key={stat.label} className="rounded-3xl bg-white p-5 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
            <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">{stat.label}</p>
            <p className="mt-2 font-mono text-3xl font-semibold text-ink">
              <NumberTicker value={stat.value} className="font-mono text-ink" />
            </p>
            <p className="mt-2 inline-flex items-center gap-2 rounded-full bg-mint px-2.5 py-1 text-[11px] font-semibold text-forest">
              {stat.delta} <span className="text-muted">RANK UP</span>
            </p>
          </article>
        ))}
      </div>

      <div className="mt-6 flex gap-2">
        {TABS.map((item) => (
          <button
            key={item}
            type="button"
            onClick={() => setTab(item)}
            className={cn(
              "rounded-full px-4 py-1.5 text-[11px] font-semibold tracking-[0.14em]",
              tab === item ? "bg-forest text-white" : "border border-forest/30 bg-white text-forest"
            )}
          >
            {item}
          </button>
        ))}
      </div>

      <section className="mt-5 overflow-hidden rounded-[28px] bg-white p-5 shadow-[0_12px_32px_rgba(15,40,25,0.05)] sm:p-7">
        <div className="mb-5 flex flex-col justify-between gap-1 sm:flex-row sm:items-center">
          <h2 className="text-[12px] font-semibold tracking-[0.16em] text-ink">AGENT FORM BOARD</h2>
          <p className="text-[11px] tracking-[0.12em] text-muted">RANKED BY RP · UPDATED EVERY ROUND</p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[720px] text-left text-sm">
            <thead>
              <tr className="text-[11px] font-semibold tracking-[0.14em] text-muted">
                <th className="pb-3">AGENT</th>
                <th className="pb-3">OWNER</th>
                <th className="pb-3">RP</th>
                <th className="pb-3">WINS</th>
                <th className="pb-3">TREND</th>
                <th className="pb-3">FORM</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row, i) => (
                <tr key={row.agent} className={i % 2 ? "bg-mint/40" : ""}>
                  <td className="rounded-l-xl px-2 py-3 font-semibold">{row.agent}</td>
                  <td className="px-2 py-3 text-muted">{row.owner}</td>
                  <td className="px-2 py-3 font-mono">{row.rp.toLocaleString("en-US")}</td>
                  <td className="px-2 py-3 font-mono">{row.wins}</td>
                  <td className="px-2 py-3">
                    <div className="h-2 w-28 overflow-hidden rounded-full bg-line">
                      <div className={cn("h-full rounded-full", row.trend < 30 ? "bg-[#C45C4A]" : row.trend < 45 ? "bg-[#D4A017]" : "bg-forest")} style={{ width: `${row.trend}%` }} />
                    </div>
                  </td>
                  <td className="rounded-r-xl px-2 py-3">
                    <span className="inline-flex gap-1">
                      {row.form.map((letter, idx) => (
                        <span key={`${row.agent}-${idx}`} className={`inline-flex h-5 w-5 items-center justify-center rounded-sm text-[10px] font-bold ${FORM_STYLE[letter]}`}>
                          {letter}
                        </span>
                      ))}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="mt-5 flex flex-col justify-between gap-2 text-[11px] text-muted sm:flex-row sm:items-center">
          <span className="inline-flex gap-1">
            {(["G", "S", "M", "P"] as FormLetter[]).map((letter) => (
              <span key={letter} className={`inline-flex h-5 w-5 items-center justify-center rounded-sm text-[10px] font-bold ${FORM_STYLE[letter]}`}>{letter}</span>
            ))}
          </span>
          <span className="tracking-[0.1em]">FORM SHOWS THE LAST FOUR ROUNDS · LEFT = OLDEST</span>
        </div>
      </section>

      <section className="mt-5 flex flex-col items-start justify-between gap-4 rounded-[28px] bg-white p-6 sm:flex-row sm:items-center">
        <div>
          <h3 className="text-xl font-bold">EVERY POINT COUNTS</h3>
          <p className="mt-1 max-w-md text-sm text-muted">Season points lock after the flag. The championship board is the only ladder that survives a sprint.</p>
        </div>
        <Link href="/championship/" className="rounded-2xl bg-forest px-6 py-3 text-[12px] font-semibold tracking-[0.12em] text-white hover:bg-forest-bright">
          VIEW CHAMPIONSHIP STANDINGS
        </Link>
      </section>
    </>
  );
}
