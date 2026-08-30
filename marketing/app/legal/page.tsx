import Link from "next/link";

export default function LegalPage() {
  return (
    <main className="mx-auto max-w-3xl px-6 py-16">
      <h1 className="text-4xl font-bold">Legal</h1>
      <ul className="mt-6 space-y-2 text-forest">
        <li><Link href="/privacy">Privacy</Link></li>
        <li><Link href="/tos">Terms</Link></li>
      </ul>
    </main>
  );
}
