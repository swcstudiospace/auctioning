import GarageLive from "@/components/garage/GarageLive";

export default async function GaragePage({ params }: { params: Promise<{ agent: string }> }) {
  const { agent } = await params;
  return <GarageLive handle={decodeURIComponent(agent)} />;
}
