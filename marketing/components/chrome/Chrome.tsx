"use client";
import { useEffect } from "react";
import { usePathname } from "next/navigation";
import MarketingNav from "@/components/chrome/MarketingNav";
import SiteNav from "@/components/chrome/SiteNav";
import SiteFooter from "@/components/chrome/SiteFooter";

export default function Chrome({ children }: { children: React.ReactNode }) {
  const path = usePathname();
  const marketing = path === "/";

  useEffect(() => {
    document.documentElement.classList.toggle("marketing", marketing);
  }, [marketing]);

  return (
    <>
      {marketing ? <MarketingNav /> : <SiteNav />}
      {children}
      {marketing ? null : <SiteFooter />}
    </>
  );
}
