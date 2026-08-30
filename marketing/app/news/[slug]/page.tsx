import NewsPost from "@/components/news/NewsPost";

export default async function NewsSlugPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  return <NewsPost slug={slug} />;
}
