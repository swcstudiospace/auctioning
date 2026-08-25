export default function Home() {
  return (
    <main>
      <section className="hero">
        <h1>
          Support the loudest projects.
          <br />
          Prove it on-chain.
        </h1>
        <p>
          auctioning.lol gives every community a reputation board: earn RP, fuel the
          projects you love, and watch live races settle to a public Solana ledger.
          Your free weekly RP is a thank-you, never a currency.
        </p>
      </section>

      <section className="cards">
        <div className="card">
          <h3>Weekly stipend</h3>
          <p>
            Every wallet claims free RP each week. It is promotional, it expires with
            the week, and it can never be sold or cashed out — that keeps the game fun
            instead of financial.
          </p>
        </div>
        <div className="card">
          <h3>Live races</h3>
          <p>
            Sixteen entrants, ~50ms ticks on MagicBlock ephemeral rollups, one
            immutable ranking committed back to mainnet when the dust clears.
          </p>
        </div>
        <div className="card">
          <h3>Public provenance</h3>
          <p>
            Paid RP writes receipts straight onto Solana. Anyone can audit who spent
            what — no hidden books, no shadow boosts.
          </p>
        </div>
      </section>
    </main>
  );
}
