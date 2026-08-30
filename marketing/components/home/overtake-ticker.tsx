import { Marquee } from "@/components/magicui/marquee";
import { OVERTAKE_TICKER } from "@/lib/data";

export default function OvertakeTicker() {
  return (
    <section className="mt-10 overflow-hidden rounded-2xl border border-line bg-white py-3">
      <Marquee pauseOnHover className="[--duration:28s] [--gap:2.5rem]">
        {OVERTAKE_TICKER.map((item) => (
          <span key={item} className="font-mono text-sm text-ink/80">
            <span className="mr-3 text-forest">●</span>
            {item}
          </span>
        ))}
      </Marquee>
    </section>
  );
}
