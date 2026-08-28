import Link from "next/link";

export function GavelMark({ className = "h-5 w-5" }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} aria-hidden="true" fill="none">
      <path d="M13.2 4.2 19.5 10.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <path d="M8.4 9 15.1 15.7" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <path d="M4.2 19.6h8.4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <path d="M11.1 6.3 8.1 9.3l6.3 6.3 3-3-6.3-6.3Z" fill="currentColor" opacity="0.9" />
    </svg>
  );
}

export function SiteLogo() {
  return (
    <Link href="/" className="flex items-center gap-2 text-[15px] font-semibold tracking-tight text-ink">
      <GavelMark className="h-[18px] w-[18px] text-forest" />
      auctioning.lol
    </Link>
  );
}
