import Link from "next/link";
import { cn } from "@/lib/utils";

export function Mark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 32 32" className={cn("h-7 w-7", className)} aria-hidden>
      <polygon fill="currentColor" points="16,3 21,3 11,22 6,22" />
      <rect fill="currentColor" x="6" y="24" width="6" height="6" />
      <polygon fill="currentColor" points="21,3 26,3 30,29 24,29" />
    </svg>
  );
}

export function Logo({ href = "/", dark = false }: { href?: string; dark?: boolean }) {
  return (
    <Link
      href={href}
      className={cn(
        "flex items-center gap-2 font-semibold tracking-tight",
        dark ? "text-[#EDEAE2]" : "text-ink",
      )}
    >
      <span className={cn(dark ? "text-[#3e8e62]" : "text-forest")}>
        <Mark />
      </span>
      <span>
        auctioning<span className="text-forest">.lol</span>
      </span>
    </Link>
  );
}
