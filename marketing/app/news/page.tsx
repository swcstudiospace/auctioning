import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { news } from "@/lib/data";

export default function NewsPage() {
  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/rules" className="chip">← Back to rules</Link>
      <p className="mt-6 k text-forest">S1 2026</p>
      <h1 className="mt-2 text-4xl font-bold">NEWS & CASE STUDIES</h1>
      <p className="mt-3 text-neutral-600">Proof the grid pays: wins, burn rates, and bragging rights from Season 1.</p>
      <div className="mt-8 grid gap-4 md:grid-cols-3">
        {news.map((n) => (
          <MagicCard key={n.id}>
            <div className="flex items-start justify-between">
              <span className="grid h-8 w-8 place-items-center rounded-full bg-forest text-xs text-white">{n.id}</span>
              <span className="chip">{n.kind}</span>
            </div>
            <h3 className="mt-4 text-lg font-bold">{n.stat}</h3>
            <p className="text-sm text-neutral-500">{n.detail}</p>
            <p className="mt-3 text-sm italic text-neutral-600">“{n.quote}”</p>
            <div className="mt-4 border-t border-forest pt-2 text-xs tracking-[0.16em]">{n.company}</div>
          </MagicCard>
        ))}
      </div>
      <MagicCard className="mt-10 flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold">Ready to race?</h2>
          <p className="text-sm text-neutral-600">New case studies drop every week — your podium is waiting.</p>
        </div>
        <ShinyButton href="/enter">Start bidding →</ShinyButton>
      </MagicCard>
    </main>
  );
}
