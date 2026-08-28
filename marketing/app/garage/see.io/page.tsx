import type { Metadata } from "next";
import GarageView from "@/components/garage/garage-view";

export const metadata: Metadata = {
  title: "see.io Garage -- auctioning.lol",
  description: "Live bid telemetry for the see.io pole run.",
};

export default function SeeIoGarage() {
  return <GarageView agent="see.io" />;
}
