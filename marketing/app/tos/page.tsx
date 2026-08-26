export const metadata = {
  title: "Terms of service — auctioning.lol",
  description:
    "Plain-English terms for auctioning.lol, including refunds under the Australian Consumer Law. Not legal advice.",
};

export default function Terms() {
  return (
    <main className="prose">
      <h1>Terms of service</h1>
      <p>
        This page is plain-English information, not legal advice. It describes how
        auctioning.lol works and what you can expect from us. If you need advice
        about your own situation, talk to a qualified Australian lawyer.
      </p>

      <h2>The service</h2>
      <p>
        auctioning.lol is a community game. You earn or buy reputation points (RP)
        to support projects and enter live races. Nothing here is an investment
        product, a managed investment scheme, or a promise of returns.
      </p>

      <h2>Free RP has zero cash value</h2>
      <ul>
        <li>
          Free RP is promotional. It cannot be bought, sold, transferred, or
          cashed out.
        </li>
        <li>
          It lives entirely off-chain in our private ledger, expires weekly, and
          carries no redemption right of any kind.
        </li>
        <li>
          RP has zero cash value. It is not money, not a currency, and not
          redeemable for cash or any other consideration.
        </li>
        <li>
          It does not confer equity, revenue share, governance rights, or a stake
          in anything.
        </li>
      </ul>

      <h2>Paid RP is consumable gameplay, not a token</h2>
      <ul>
        <li>
          Paid RP buys gameplay utility: race entries, boosts, cosmetics. It is
          consumed when you use it.
        </li>
        <li>
          We publish paid purchases as receipts on Solana mainnet (
          <code>log_paid_rp</code>) for transparency. That receipt records spend
          provenance — it is not a token, a security, or a claim on future
          returns.
        </li>
        <li>
          There is no yield, no investment, and no secondary market for RP. None
          is provided by us, and we make no representation that RP has any resale
          value.
        </li>
      </ul>

      <h2>Australia — Corporations Act and consumer law</h2>
      <ul>
        <li>
          We designed the economy to sit outside the definition of a financial
          product under the Corporations Act 2001 (Cth): no schemes, no managed
          investment products, no promises of returns.
        </li>
        <li>
          Free RP never touches a market; paid RP is consumed in-game. Neither is
          offered as, or intended to be, a financial product or advice.
        </li>
        <li>
          The Australian Consumer Law (ACL, Schedule 2 of the Competition and
          Consumer Act 2010) still applies to the service we supply. Consumer
          guarantees for faulty service are not excluded.
        </li>
      </ul>

      <h2>Refunds (ACL)</h2>
      <p>
        If a paid-RP purchase fails and the ledger is not credited, that is
        faulty service. Contact us via the community channels linked from the app
        within 14 days of that failed purchase. We will restore the service
        (credit the RP) or refund the Whop charge — for faulty service only.
      </p>
      <p>
        Race outcomes are not refundable. Losing a race, disliking a ranking, or
        changing your mind after RP has been credited and consumed is not a
        fault in the service.
      </p>

      <h2>Disputes</h2>
      <p>
        These terms are governed by the laws of Australia. Raise a dispute via
        the community channels first; we would rather fix it than argue about it.
        Serious legal enquiries can request our formal contact address there.
      </p>
    </main>
  );
}
