import NewsCases from "../../components/NewsCases";

export const metadata = {
  title: "News — auctioning.lol",
  description:
    "How they did it — six recent boards with RP burst and paid versus community mix.",
};

export default function NewsPage() {
  return (
    <main className="ui-page">
      <NewsCases />
    </main>
  );
}
