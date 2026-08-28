import type { Metadata } from "next";
import EnterForm from "@/components/enter/enter-form";

export const metadata: Metadata = {
  title: "Place a Bid -- auctioning.lol",
  description: "Add RP to enter the live race. Phantom and Whop are stubs only.",
};

export default function EnterPage() {
  return <EnterForm />;
}
