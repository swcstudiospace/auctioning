import RulesGrid from "../../components/RulesGrid";

export const metadata = {
  title: "Rules — auctioning.lol",
  description:
    "How the race works: fuel with RP, sixteen-slot grid, MagicBlock speed, and a featured homepage winner.",
};

export default function RulesPage() {
  return (
    <main className="ui-page">
      <RulesGrid />
    </main>
  );
}
