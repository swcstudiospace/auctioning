"use client";

import { cn } from "@/lib/utils";

export function BorderBeam({
  className,
  size = 72,
  duration = 8,
  colorFrom = "#3E8E62",
  colorTo = "#9AE6B4",
  delay = 0,
}: {
  className?: string;
  size?: number;
  duration?: number;
  colorFrom?: string;
  colorTo?: string;
  delay?: number;
}) {
  return (
    <div
      className={cn("pointer-events-none absolute inset-0 rounded-[inherit]", className)}
      aria-hidden
    >
      <div
        className="absolute aspect-square rounded-full opacity-80 motion-reduce:animate-none"
        style={{
          width: size,
          height: size,
          background: `linear-gradient(to left, ${colorFrom}, ${colorTo}, transparent)`,
          offsetPath: `rect(0 auto auto 0 round ${Math.round(size / 3)}px)`,
          offsetAnchor: "100% 50%",
          animation: `border-beam ${duration}s linear ${delay}s infinite`,
        }}
      />
    </div>
  );
}
