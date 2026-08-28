export function Marquee({ items }: { items: string[] }) {
  const line = items.join("  ·  ") + "  ·  ";
  return (
    <div className="overflow-hidden rounded-full border border-emerald-100 bg-white py-2">
      <div className="flex w-max animate-marquee">
        <span className="px-4 text-sm text-neutral-600">{line}</span>
        <span className="px-4 text-sm text-neutral-600">{line}</span>
      </div>
    </div>
  );
}
