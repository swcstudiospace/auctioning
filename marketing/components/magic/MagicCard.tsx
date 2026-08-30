"use client";

import { useCallback, useRef, type MouseEvent, type ReactNode } from "react";
import { cn } from "@/lib/utils";
import { BorderBeam } from "./BorderBeam";

export function MagicCard({
  children,
  className,
  tone = "light",
  beam = false,
}: {
  children: ReactNode;
  className?: string;
  tone?: "light" | "dark";
  beam?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);

  const onMove = useCallback((e: MouseEvent<HTMLDivElement>) => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    el.style.setProperty("--mx", `${e.clientX - r.left}px`);
    el.style.setProperty("--my", `${e.clientY - r.top}px`);
  }, []);

  const spot =
    tone === "dark"
      ? "rgba(158,227,180,0.18)"
      : "rgba(62,142,98,0.18)";

  return (
    <div
      ref={ref}
      onMouseMove={onMove}
      className={cn(
        "group/magic relative overflow-hidden rounded-2xl p-5 transition-[box-shadow,border-color] duration-300",
        tone === "dark"
          ? "border border-white/10 bg-[#111] hover:border-forest/50 hover:shadow-[0_0_40px_rgba(62,142,98,0.14)]"
          : "card hover:shadow-md",
        className,
      )}
      style={{ ["--mx" as string]: "70%", ["--my" as string]: "0%" }}
    >
      <div
        className="pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-300 group-hover/magic:opacity-100"
        style={{
          background: `radial-gradient(280px circle at var(--mx) var(--my), ${spot}, transparent 42%)`,
        }}
      />
      {beam ? <BorderBeam /> : null}
      <div className="relative z-[1]">{children}</div>
    </div>
  );
}
