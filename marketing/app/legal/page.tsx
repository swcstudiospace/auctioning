export const metadata = {
  title: "Legal — auctioning.lol",
  description: "Plain-English legal posture for auctioning.lol, written for an Australian audience.",
};

export default function Legal() {
  return (
    <main className="prose">
      <h1>Legal posture</h1>
      <p>
        This page is plain-English information, not legal advice. We built
        auctioning.lol to be a community game, and we have deliberately made design
        choices so it stays one.
      </p>

      <h2>Free RP is not a financial product</h2>
      <ul>
        <li>Free RP is promotional. It cannot be bought, sold, transferred, or cashed out.</li>
        <li>It lives entirely off-chain in our private ledger, expires weekly, and carries no redemption right of any kind.</li>
        <li>It does not confer equity, revenue share, governance rights, or a stake in anything.</li>
      </ul>

      <h2>Paid RP is a purchase, not an investment</h2>
      <ul>
        <li>Paid RP buys gameplay utility: race entries, boosts, cosmetics.</li>
        <li>We publish paid purchases as receipts on Solana mainnet for transparency, but the receipt records spend provenance — it is not a token, a security, or a claim on future returns.</li>
        <li>There is no secondary market for RP, none is provided by us, and we make no representation that RP has any resale value.</li>
      </ul>

      <h2>Australia-specific notes</h2>
      <ul>
        <li>
          We designed the economy to sit outside the definition of a financial product
          under the Corporations Act 2001 (Cth): no schemes, no managed investment
          products, no promises of returns.
        </li>
        <li>
          Free RP never touches a market; paid RP is consumed in-game. Neither is
          offered as, or intended to be, a financial product or advice.
        </li>
        <li>
          If you are in Australia and have concerns about how consumer law applies to
          your purchase, contact us before purchasing — we would rather fix it than
          argue about it.
        </li>
      </ul>

      <h2>Data</h2>
      <ul>
        <li>The private ledger stores your wallet address, RP balances, and gameplay events. No names, emails, or PII are required to play.</li>
        <li>Solana transactions are public by nature; anything you sign on-chain is visible to everyone forever.</li>
      </ul>

      <p>
        Questions? Reach out via the community channels linked from the app. Serious
        legal enquiries can request our formal contact address there.
      </p>
    </main>
  );
}
