import Link from "next/link";
import { ArrowLeft } from "lucide-react";

export function BackLink({ href, label }: { href: string; label: string }) {
  return (
    <Link
      href={href}
      className="mb-6 inline-flex items-center gap-2 rounded-xl border border-line bg-white/80 px-3 py-1.5 text-[11px] font-semibold tracking-[0.12em] text-ink/70 shadow-sm transition hover:border-forest/40 hover:text-ink"
    >
      <ArrowLeft className="h-3.5 w-3.5" />
      {label}
    </Link>
  );
}
