import { cn } from "@/lib/utils";

export function MagicCard({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "card relative overflow-hidden p-5 transition-shadow hover:shadow-md",
        className
      )}
    >
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_right,rgba(62,142,98,0.12),transparent_45%)]" />
      <div className="relative">{children}</div>
    </div>
  );
}
