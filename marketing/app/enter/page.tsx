import EnterRace from "../../components/EnterRace";

export const metadata = {
  title: "Enter Race — auctioning.lol",
  description:
    "Add RP to enter the live race. $1 = 1 paid RP. Community RP is not money. RP is non-refundable and for race use only.",
};

export default function EnterPage() {
  return (
    <main className="ui-page">
      <EnterRace />
    </main>
  );
}
