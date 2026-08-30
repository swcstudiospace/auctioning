"use client";

import { useCallback, useRef, type MouseEvent, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export function Spotlight({
  children,
  className,
}: {
  children?: ReactNode;
  className?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);

  const onMove = useCallback((e: MouseEvent<HTMLDivElement>) => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    el.style.setProperty("--sx", `${e.clientX - r.left}px`);
    el.style.setProperty("--sy", `${e.clientY - r.top}px`);
  }, []);

  return (
    <div
      ref={ref}
      onMouseMove={onMove}
      className={cn("relative", className)}
      style={{ ["--sx" as string]: "42%", ["--sy" as string]: "18%" }}
    >
      <div
        className="pointer-events-none absolute inset-0 overflow-hidden"
        aria-hidden
        style={{
          background:
            "radial-gradient(560px circle at var(--sx) var(--sy), rgba(62,142,98,0.24), transparent 58%)",
        }}
      />
      {children}
    </div>
  );
}
