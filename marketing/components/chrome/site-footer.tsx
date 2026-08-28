import Link from "next/link";
import { FOOTER_LINKS } from "@/lib/data";
import { SiteLogo } from "./logo";

export default function SiteFooter() {
  return (
    <footer className="mt-16 border-t border-line/80">
      <div className="mx-auto flex max-w-6xl flex-col gap-5 px-4 py-8 sm:px-6">
        <div className="flex flex-col items-start justify-between gap-4 sm:flex-row sm:items-center">
          <SiteLogo />
          <nav className="flex flex-wrap gap-x-6 gap-y-2 text-[11px] font-semibold tracking-[0.14em] text-ink/70" aria-label="Footer">
            {FOOTER_LINKS.map((item) => (
              <Link key={item.href} href={item.href} className="hover:text-ink">
                {item.label}
              </Link>
            ))}
          </nav>
        </div>
        <div className="flex flex-col justify-between gap-2 text-[11px] tracking-wide text-muted sm:flex-row">
          <p className="font-mono">© 2026 auctioning.lol — pay to race. All RP figures illustrative.</p>
          <p className="font-mono">Built on Solana · Whop-powered entries</p>
        </div>
        <nav className="flex gap-4 text-[11px] text-muted" aria-label="Legal">
          <Link href="/tos/">Terms</Link>
          <Link href="/privacy/">Privacy</Link>
          <Link href="/legal/">Legal</Link>
        </nav>
      </div>
    </footer>
  );
}
