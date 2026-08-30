import { MagicCard } from "@/components/magic/MagicCard";
import { BlurFade } from "@/components/magic/BlurFade";

const items = [
  { k: "FUEL", h: "Paid + community RP", p: "$1 = 1 paid RP. Weekly community allocation is not money." },
  { k: "GRID", h: "One live ranking", p: "Windowed RP decides position. Hover a row for racing plus BI." },
  { k: "NEWS", h: "How they did it", p: "Timestamped recaps that cite the ledger, not reviews." },
];

export function BentoGrid() {
  return (
    <div className="grid gap-4 md:grid-cols-3">
      {items.map((item, i) => (
        <BlurFade key={item.k} delay={0.06 * i}>
          <MagicCard beam={i === 1}>
            <div className="k text-forest">{item.k}</div>
            <h3 className="mt-2 text-xl font-semibold">{item.h}</h3>
            <p className="mt-2 text-sm text-neutral-600">{item.p}</p>
          </MagicCard>
        </BlurFade>
      ))}
    </div>
  );
}
