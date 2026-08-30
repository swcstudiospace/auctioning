"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { brand } from "@/lib/brand";
import { Logo } from "@/components/chrome/Logo";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { ActivePaceChip } from "@/components/chrome/ActivePaceChip";
import { cn } from "@/lib/utils";

export default function SiteNav() {
  const path = usePathname();
  return (
    <header className="sticky top-0 z-40 border-b border-emerald-100 bg-white/90 backdrop-blur">
      <div className="mx-auto flex max-w-6xl items-center gap-6 px-6 py-4">
        <Logo href="/" />
        <nav className="hidden flex-1 items-center justify-center gap-5 text-xs tracking-[0.16em] md:flex">
          {brand.nav.map((item) => {
            const active = item.href === "/rank" ? path === "/rank" : path.startsWith(item.href);
            return (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  "pb-1",
                  active ? "border-b-2 border-forest text-ink" : "text-neutral-500",
                )}
              >
                {item.label}
              </Link>
            );
          })}
        </nav>
        <ActivePaceChip />
        <ShinyButton href="/rank#claim">Claim #1</ShinyButton>
      </div>
    </header>
  );
}
