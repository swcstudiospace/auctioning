import Link from "next/link";
import { ShinyButton } from "@/components/magicui/shiny-button";

export default function FinalCta() {
  return (
    <section className="mt-16 overflow-hidden rounded-[28px] bg-forest px-6 py-12 text-white sm:px-10">
      <div className="flex flex-col items-start justify-between gap-6 sm:flex-row sm:items-center">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Ready to race?</h2>
          <p className="mt-2 max-w-md text-white/80">
            $1 = 1 paid RP. Community RP is promotional. Phantom and Whop stubs on the next screen — no live checkout here.
          </p>
        </div>
        <div className="flex flex-col gap-3 sm:flex-row">
          <Link href="/enter/">
            <ShinyButton className="rounded-full border-white/20 bg-white px-7 py-3 text-forest">
              Place a bid
            </ShinyButton>
          </Link>
          <Link href="/live/" className="inline-flex items-center justify-center rounded-full border border-white/30 px-6 py-3 text-sm font-semibold">
            Watch live
          </Link>
        </div>
      </div>
    </section>
  );
}
