import HowItWorks from "../components/HowItWorks";
import LandingHero from "../components/LandingHero";

export default function Home() {
  return (
    <main className="ui-page">
      <LandingHero />
      <HowItWorks />
    </main>
  );
}
