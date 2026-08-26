export const metadata = {
  title: "Privacy — auctioning.lol",
  description:
    "Plain-English privacy information for auctioning.lol under the Privacy Act 1988 (Cth). Not legal advice.",
};

export default function Privacy() {
  return (
    <main className="prose">
      <h1>Privacy</h1>
      <p>
        This page is plain-English information, not legal advice. It explains
        what auctioning.lol collects and why, written for an Australian audience
        under the Privacy Act 1988 (Cth).
      </p>

      <h2>What we collect</h2>
      <ul>
        <li>
          We collect your wallet address and gameplay events (RP balances,
          race activity, paid-RP spend provenance). That is enough to run the
          game.
        </li>
        <li>
          No names, emails, or other personal identifiers are required to play.
          We do not ask for them as a condition of using the service.
        </li>
      </ul>

      <h2>Solana is public</h2>
      <p>
        Solana transactions are public by nature. Anything you sign on-chain —
        including paid-RP receipts — is visible to everyone forever. We cannot
        make those records private.
      </p>

      <h2>We do not sell your data</h2>
      <p>
        We do not sell wallet addresses, gameplay events, or any other
        information we hold. We use what we collect to operate the game, keep
        the ledger honest, and meet our legal obligations.
      </p>

      <h2>Cookies</h2>
      <p>
        The marketing site may use essential cookies only — the minimum needed
        for the site to work. We do not use advertising or analytics cookies on
        the marketing origin.
      </p>

      <h2>Questions</h2>
      <p>
        Reach out via the community channels linked from the app. Serious
        privacy enquiries can request our formal contact address there.
      </p>
    </main>
  );
}
