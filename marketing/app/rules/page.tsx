import type { Metadata } from "next";
import Link from "next/link";
import { BackLink } from "@/components/chrome/back-link";
import { HOUSE_RULES, PILLARS } from "@/lib/data";

export const metadata: Metadata = {
  title: "Race Rules -- auctioning.lol",
  description: "House playbook: fuel, grid, speed, featured.",
};

export default function RulesPage() {
  return (
    <main className="mx-auto max-w-6xl px-4 py-8 sm:px-6">
      <BackLink href="/garage/" label="BACK TO GARAGE" />
      <p className="text-[11px] font-semibold tracking-[0.18em] text-forest">HOUSE PLAYBOOK</p>
      <h1 className="mt-2 text-4xl font-bold tracking-tight sm:text-5xl">Race Rules</h1>
      <p className="mt-3 max-w-2xl text-muted">Every race runs on the same house mechanics: auction the grid, settle every overtake on the clock, pay everything out at the flag.</p>
      <div className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {PILLARS.map((pillar) => (
          <article key={pillar.title} className="rounded-3xl bg-white p-5 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
            <span className="inline-flex rounded-full bg-mint px-3 py-1 text-[11px] font-semibold tracking-[0.14em] text-forest">{pillar.title}</span>
            <p className="mt-4 text-sm leading-relaxed text-ink">{pillar.body}</p>
          </article>
        ))}
      </div>
      <div className="mt-6 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {HOUSE_RULES.map((rule, i) => (
          <article key={rule} className="rounded-3xl bg-white p-6 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
            <p className="font-mono text-3xl font-semibold text-forest/35">{String(i + 1).padStart(2, "0")}</p>
            <p className="mt-3 text-sm leading-relaxed text-ink">{rule}</p>
          </article>
        ))}
      </div>
      <section className="mt-6 rounded-3xl bg-white p-6 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
        <p className="inline-flex items-center gap-2 text-[11px] font-semibold tracking-[0.16em] text-forest">
          <span className="h-2 w-2 rounded-full bg-forest" />
          <span className="h-2 w-2 rounded-full bg-forest" />
          WORKED EXAMPLE
        </p>
        <p className="mt-3 text-lg text-ink">
          Pole costs <span className="font-mono font-semibold text-forest">410 RP</span>
          {" "}· losers split refunds <span className="font-mono font-semibold text-forest">270 RP</span>
          {" "}· net burn holding P3 to flag = <span className="font-mono font-semibold text-forest">140 RP</span>
        </p>
      </section>
      <div className="mt-8 flex justify-center">
        <Link href="/news/" className="rounded-full bg-forest px-6 py-3 text-[12px] font-semibold tracking-[0.12em] text-white hover:bg-forest-bright">SEE CASE STUDIES</Link>
      </div>
    </main>
  );
}
