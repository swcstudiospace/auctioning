import Landing from "@/components/marketing/Landing";
import { listProjects } from "@/lib/api";

export default async function HomePage() {
  const res = await listProjects({ page: 1, per_page: 1 });
  const catalogTotal = res.ok ? res.data.total : 0;
  return <Landing catalogTotal={catalogTotal} />;
}
