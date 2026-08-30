"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/** Aceternity-style tracing beam: a left rail that fills as you read. */
export function TracingBeam({ children, className }: { children: ReactNode; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pct, setPct] = useState(0);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onScroll = () => {
      const rect = el.getBoundingClientRect();
      const vh = window.innerHeight || 1;
      const total = rect.height - vh * 0.35;
      const seen = Math.min(Math.max(-rect.top + vh * 0.2, 0), Math.max(total, 1));
      setPct(Math.min(100, Math.max(4, (seen / Math.max(total, 1)) * 100)));
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <div ref={ref} className={cn("relative", className)}>
      <div className="pointer-events-none absolute bottom-0 left-0 top-0 hidden w-px bg-emerald-100 md:block" aria-hidden>
        <div
          className="absolute left-1/2 top-0 w-[3px] -translate-x-1/2 rounded-full bg-gradient-to-b from-forest via-emerald-300 to-forest"
          style={{ height: `${pct}%` }}
        />
        <span className="absolute left-1/2 top-0 h-2 w-2 -translate-x-1/2 rounded-full bg-forest shadow-[0_0_12px_#3E8E62]" />
      </div>
      <div className="md:pl-8">{children}</div>
    </div>
  );
}
