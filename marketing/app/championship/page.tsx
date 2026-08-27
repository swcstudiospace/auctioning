import ChampionshipBoard from "../../components/ChampionshipBoard";

export const metadata = {
  title: "Championship — auctioning.lol",
  description:
    "2026 Season 1 Championship standings. Points, wins, best finish, and sprint points.",
};

export default function ChampionshipPage() {
  return (
    <main className="ui-page">
      <ChampionshipBoard />
    </main>
  );
}
