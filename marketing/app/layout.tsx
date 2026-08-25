import Link from "next/link";
import "./globals.css";

export const metadata = {
  title: "auctioning.lol — support the loudest projects",
  description:
    "Reputation points, weekly stipends, and live races for the projects people actually care about. Free RP is always non-cashable.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <nav className="nav">
          <Link href="/" className="brand">
            auctioning<span>.lol</span>
          </Link>
          <div>
            <a href="/legal/">Legal</a>
            <Link href="/" className="cta">
              Launch app
            </Link>
          </div>
        </nav>
        {children}
        <footer>
          auctioning.lol is a community project. Free RP is promotional, non-cashable and
          off-chain; paid RP provenance settles on Solana mainnet. Not financial advice, not
          an investment product.
        </footer>
      </body>
    </html>
  );
}
