import type { Metadata } from "next";
import { IBM_Plex_Mono } from "next/font/google";
import "./globals.css";
import { brand } from "../lib/brand";
import SiteNav from "../components/chrome/SiteNav";
import SiteFooter from "../components/chrome/SiteFooter";

const ibmPlexMono = IBM_Plex_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
});

export const metadata: Metadata = {
  metadataBase: new URL("https://auctioning.lol"),
  title: "auctioning.lol — support the loudest projects",
  description:
    "Reputation points, weekly stipends, and live races for the projects people actually care about. Free RP is always non-cashable.",
  openGraph: {
    title: "auctioning.lol — support the loudest projects",
    description:
      "Reputation points, weekly stipends, and live races for the projects people actually care about. Free RP is always non-cashable.",
    siteName: brand.name,
    type: "website",
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={ibmPlexMono.className}>
        <SiteNav />
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
