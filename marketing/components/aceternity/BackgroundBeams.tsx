export function BackgroundBeams() {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden">
      <div className="absolute left-8 top-0 h-full w-px bg-gradient-to-b from-transparent via-forest/40 to-transparent animate-beam" />
      <div className="absolute left-1/3 top-0 h-full w-px bg-gradient-to-b from-transparent via-emerald-300/50 to-transparent animate-beam" />
      <div className="absolute right-1/3 top-0 h-full w-px bg-gradient-to-b from-transparent via-forest/30 to-transparent animate-beam" />
      <div className="absolute right-8 top-0 h-full w-px bg-gradient-to-b from-transparent via-emerald-400/40 to-transparent animate-beam" />
    </div>
  );
}
