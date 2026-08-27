import type { Metadata } from "next";
import Link from "next/link";
import { IBM_Plex_Mono } from "next/font/google";
import "./globals.css";

const ibmPlexMono = IBM_Plex_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
});

export const metadata: Metadata = {
  title: "auctioning.lol — support the loudest projects",
  description:
    "Reputation points, weekly stipends, and live races for the projects people actually care about. Free RP is always non-cashable.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={ibmPlexMono.className}>
        <nav className="nav">
          <Link href="/" className="brand">
            auctioning<span>.lol</span>
          </Link>
          <div>
            <Link href="/live">Live</Link>
            <Link href="/tracks">Tracks</Link>
            <Link href="/championship">Championship</Link>
            <Link href="/news">News</Link>
            <Link href="/enter">Enter</Link>
            <Link href="/rules">Rules</Link>
            <a href="/tos/">Terms</a>
            <a href="/privacy/">Privacy</a>
            <a href="/legal/">Legal</a>
            <Link href="/enter" className="ui-btn-gradient">
              Enter Race
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
