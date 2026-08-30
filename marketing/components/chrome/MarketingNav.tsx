"use client";
import Link from "next/link";
import { Logo } from "@/components/chrome/Logo";
import { ShinyButton } from "@/components/magic/ShinyButton";

const links = [
  { href: "/live", label: "Live race" },
  { href: "#how", label: "How" },
  { href: "/news", label: "News" },
  { href: "#faq", label: "FAQ" },
];

export default function MarketingNav() {
  return (
    <header className="sticky top-0 z-40 border-b border-white/10 bg-[#0a0a0a]/80 backdrop-blur">
      <div className="mx-auto flex max-w-6xl items-center gap-6 px-6 py-4">
        <Logo dark />
        <nav className="hidden flex-1 items-center justify-center gap-6 text-xs tracking-[0.16em] text-white/55 md:flex">
          {links.map((l) => (
            <Link key={l.href} href={l.href} className="hover:text-[#EDEAE2]">
              {l.label}
            </Link>
          ))}
        </nav>
        <div className="ml-auto flex items-center gap-3">
          <Link
            href="/rank"
            className="hidden rounded-full border border-white/20 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-[#EDEAE2] hover:border-forest sm:inline-flex"
          >
            Enter the grid
          </Link>
          <ShinyButton href="/enter">Claim 50 RP</ShinyButton>
        </div>
      </div>
    </header>
  );
}
