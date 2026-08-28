import { Droplets, LayoutGrid, Newspaper } from "lucide-react";
import { BentoCard, BentoGrid } from "@/components/magicui/bento-grid";

const features = [
  {
    Icon: Droplets,
    name: "Fuel",
    description: "Paid RP is a purchase: $1 = 1 paid RP. Community RP is promotional and non-cashable — it never cashes out.",
    href: "/rules/",
    cta: "Read the house rules",
    className: "md:col-span-1",
    background: <div className="absolute inset-0 bg-[radial-gradient(circle_at_top_right,rgba(62,142,98,0.12),transparent_55%)]" />,
  },
  {
    Icon: LayoutGrid,
    name: "Grid",
    description: "Six live slots. Highest standing bid holds the place. Overtakes settle on the clock, not in a back-room.",
    href: "/live/",
    cta: "Open the live race",
    className: "md:col-span-1",
    background: <div className="absolute inset-0 bg-[radial-gradient(circle_at_bottom_left,rgba(69,160,115,0.12),transparent_55%)]" />,
  },
  {
    Icon: Newspaper,
    name: "News",
    description: "Rank is the product. Pole, burn, and form become case studies — business attention, raced live.",
    href: "/news/",
    cta: "See case studies",
    className: "md:col-span-1",
    background: <div className="absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(62,142,98,0.10),transparent_55%)]" />,
  },
];

export default function HowItWorks() {
  return (
    <section className="mt-16">
      <p className="text-[11px] font-semibold tracking-[0.18em] text-forest">HOW IT WORKS</p>
      <h2 className="mt-2 text-3xl font-bold tracking-tight text-ink">Fuel the grid. Own the news.</h2>
      <div className="mt-8">
        <BentoGrid>
          {features.map((feature) => (
            <BentoCard key={feature.name} {...feature} />
          ))}
        </BentoGrid>
      </div>
    </section>
  );
}
