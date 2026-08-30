"use client";

import { cn } from "@/lib/utils";

export function Meteors({ number = 16, className }: { number?: number; className?: string }) {
  const items = Array.from({ length: number }, (_, i) => ({
    id: i,
    left: `${((i * 47) % 96) + 2}%`,
    top: `${(i * 13) % 40}%`,
    delay: `${((i * 0.37) % 5).toFixed(2)}s`,
    duration: `${2.2 + (i % 6) * 0.35}s`,
  }));

  return (
    <div className={cn("pointer-events-none absolute inset-0 overflow-hidden", className)} aria-hidden>
      {items.map((m) => (
        <span
          key={m.id}
          className="absolute h-0.5 w-0.5 animate-meteor rounded-full bg-emerald-100 shadow-[0_0_0_1px_#ffffff12] motion-reduce:animate-none"
          style={{
            left: m.left,
            top: m.top,
            animationDelay: m.delay,
            animationDuration: m.duration,
          }}
        >
          <span className="absolute top-1/2 -z-10 h-px w-14 -translate-y-1/2 bg-gradient-to-r from-emerald-200 to-transparent" />
        </span>
      ))}
    </div>
  );
}
