import type { Metadata } from "next";
import { IBM_Plex_Mono, Inter } from "next/font/google";
import "./globals.css";
import { brand } from "@/lib/brand";
import SiteNav from "@/components/chrome/site-nav";
import SiteFooter from "@/components/chrome/site-footer";

const inter = Inter({ subsets: ["latin"], variable: "--font-inter" });
const ibm = IBM_Plex_Mono({ subsets: ["latin"], weight: ["400", "500", "600", "700"], variable: "--font-ibm" });

export const metadata: Metadata = {
  metadataBase: new URL("https://auctioning.lol"),
  title: "auctioning.lol -- pay to race",
  description: "Pay to race. Rank becomes news. Business attention, raced live. $1 = 1 paid RP. Community RP is promotional and non-cashable.",
  openGraph: { title: "auctioning.lol -- pay to race", description: brand.tagline, siteName: brand.name, type: "website" },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={`${inter.variable} ${ibm.variable} bg-mint font-sans text-ink antialiased`}>
        <SiteNav />
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
