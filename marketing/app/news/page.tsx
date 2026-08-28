import type { Metadata } from "next";
import Link from "next/link";
import { BackLink } from "@/components/chrome/back-link";
import { NEWS_CASES } from "@/lib/data";
import { cn } from "@/lib/utils";

export const metadata: Metadata = {
  title: "News and Case Studies -- auctioning.lol",
  description: "Proof the grid pays: wins, burn rates, and bragging rights from Season 1.",
};

export default function NewsPage() {
  return (
    <main className="mx-auto max-w-6xl px-4 py-8 sm:px-6">
      <BackLink href="/rules/" label="BACK TO RULES" />
      <p className="text-[11px] font-semibold tracking-[0.18em] text-muted">-- S1 2026</p>
      <h1 className="mt-2 text-4xl font-bold tracking-tight sm:text-5xl">NEWS & CASE STUDIES</h1>
      <p className="mt-3 max-w-2xl text-muted">Proof the grid pays: wins, burn rates, and bragging rights from Season 1.</p>
      <div className="mt-8 grid gap-5 md:grid-cols-2 lg:grid-cols-3">
        {NEWS_CASES.map((item) => (
          <article key={item.brand} className="flex flex-col rounded-3xl bg-white p-5 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
            <div className="mb-4 flex items-center justify-between">
              <span className="inline-flex h-8 w-8 items-center justify-center rounded-md bg-forest text-sm font-bold text-white">{item.letter}</span>
              <span className={cn("rounded-full px-2.5 py-1 text-[10px] font-semibold tracking-[0.12em]", item.tag === "CASE STUDY" ? "bg-forest text-white" : "bg-line text-ink/70")}>{item.tag}</span>
            </div>
            <h2 className="text-lg font-bold leading-snug">{item.headline}</h2>
            <p className="mt-1 text-sm text-muted">{item.sub}</p>
            <p className="mt-4 flex-1 text-sm italic text-ink/70">&ldquo;{item.quote}&rdquo;</p>
            <div className="mt-5 border-t border-forest/40 pt-3 text-[11px] font-semibold tracking-[0.14em]">{item.brand}</div>
          </article>
        ))}
      </div>
      <section className="mt-6 flex flex-col items-start justify-between gap-4 rounded-[28px] bg-white p-6 sm:flex-row sm:items-center">
        <div>
          <h3 className="text-xl font-bold">Ready to race?</h3>
          <p className="mt-1 text-sm text-muted">New case studies drop every week — your podium is waiting.</p>
        </div>
        <Link href="/enter/" className="rounded-full bg-forest px-6 py-3 text-[12px] font-semibold tracking-[0.12em] text-white hover:bg-forest-bright">START BIDDING →</Link>
      </section>
    </main>
  );
}
