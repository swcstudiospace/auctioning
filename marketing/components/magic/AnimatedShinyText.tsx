import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function AnimatedShinyText({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex animate-shine bg-[linear-gradient(110deg,#3E8E62,42%,#d1fae5,50%,#3E8E62)] bg-[length:200%_100%] bg-clip-text text-transparent motion-reduce:animate-none motion-reduce:text-forest",
        className,
      )}
    >
      {children}
    </span>
  );
}
