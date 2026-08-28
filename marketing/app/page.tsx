import LandingHero from "@/components/home/landing-hero";
import HowItWorks from "@/components/home/how-it-works";
import OvertakeTicker from "@/components/home/overtake-ticker";
import FinalCta from "@/components/home/final-cta";

export default function Home() {
  return (
    <main className="mx-auto max-w-6xl px-4 py-10 sm:px-6">
      <LandingHero />
      <OvertakeTicker />
      <HowItWorks />
      <FinalCta />
    </main>
  );
}
