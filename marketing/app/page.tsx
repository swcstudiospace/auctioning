import LandingHero from "../components/LandingHero";
import ProductScreens from "../components/home/ProductScreens";
import HowItWorks from "../components/HowItWorks";
import FinalCta from "../components/home/FinalCta";

export default function Home() {
  return (
    <main className="ui-page">
      <LandingHero />
      <ProductScreens />
      <HowItWorks />
      <FinalCta />
    </main>
  );
}
