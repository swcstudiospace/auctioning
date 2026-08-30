import { cn } from "@/lib/utils";

export function Marquee({
  items,
  className,
  reverse = false,
}: {
  items: string[];
  className?: string;
  reverse?: boolean;
}) {
  const line = items.join("  ·  ") + "  ·  ";
  return (
    <div
      className={cn(
        "group relative overflow-hidden rounded-full border border-white/10 bg-white/5 py-2",
        className,
      )}
    >
      <div
        className={cn(
          "flex w-max",
          reverse ? "animate-marquee-reverse" : "animate-marquee",
          "group-hover:[animation-play-state:paused] motion-reduce:animate-none",
        )}
      >
        <span className="px-4 text-sm text-white/60">{line}</span>
        <span className="px-4 text-sm text-white/60" aria-hidden>
          {line}
        </span>
      </div>
    </div>
  );
}
