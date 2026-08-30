import type { Metadata } from "next";
import { IBM_Plex_Mono } from "next/font/google";
import "./globals.css";
import { brand } from "@/lib/brand";
import Chrome from "@/components/chrome/Chrome";

const ibm = IBM_Plex_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  variable: "--font-ibm",
});

export const metadata: Metadata = {
  metadataBase: new URL("https://auctioning.lol"),
  title: "auctioning.lol — play to rank",
  description: brand.tagline,
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={`${ibm.variable} ${ibm.className} min-h-screen bg-mint`}>
        <Chrome>{children}</Chrome>
      </body>
    </html>
  );
}
