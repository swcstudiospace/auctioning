import Link from "next/link";
import { cn } from "@/lib/utils";

export function ShinyButton({
  href,
  children,
  className,
}: {
  href: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Link
      href={href}
      className={cn(
        "relative inline-flex items-center justify-center overflow-hidden rounded-full bg-forest px-5 py-2.5 text-sm font-semibold uppercase tracking-wide text-white shadow-sm",
        "bg-[linear-gradient(110deg,#3E8E62,45%,#9AE6B4,55%,#3E8E62)] bg-[length:200%_100%] animate-shine",
        className
      )}
    >
      {children}
    </Link>
  );
}
