import Link from "next/link";
import { brand } from "@/lib/brand";

const WHOP = process.env.NEXT_PUBLIC_WHOP_CHECKOUT_URL || "";

const links = [
  { href: "/rank", label: "RANK" },
  { href: "/tracks", label: "TRACK BOARD" },
  { href: "/championship", label: "CHAMPIONSHIP" },
  { href: "/rules", label: "RACE RULES" },
  { href: "/news", label: "NEWS" },
  { href: "/legal", label: "LEGAL" },
  { href: "/privacy", label: "PRIVACY" },
  { href: "/tos", label: "TERMS" },
];

export default function SiteFooter() {
  return (
    <footer className="mt-16 border-t border-emerald-100 bg-white">
      <div className="mx-auto flex max-w-6xl flex-col gap-6 px-6 py-8">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <span className="font-semibold">{brand.name}</span>
          <div className="flex flex-wrap gap-4 text-xs tracking-[0.14em] text-neutral-500">
            {links.map((l) => (
              <Link key={l.href} href={l.href}>{l.label}</Link>
            ))}
            {WHOP ? <a href={WHOP}>WHOP</a> : null}
          </div>
        </div>
        <div className="flex flex-wrap justify-between gap-2 text-xs text-neutral-500">
          <span>© 2026 auctioning.lol — rank is fueled with RP, never USD.</span>
          <span>Built on Solana</span>
        </div>
      </div>
    </footer>
  );
}
