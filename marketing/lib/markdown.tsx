import Link from "next/link";
import type { ReactNode } from "react";

function inline(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const re = /(\[([^\]]+)\]\(([^)]+)\)|\*\*([^*]+)\*\*)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let i = 0;
  while ((m = re.exec(text))) {
    if (m.index > last) nodes.push(text.slice(last, m.index));
    if (m[2] && m[3]) {
      const href = m[3];
      const label = m[2];
      if (href.startsWith("/")) {
        nodes.push(
          <Link key={`l${i++}`} href={href} className="text-forest underline">
            {label}
          </Link>,
        );
      } else {
        nodes.push(
          <a key={`a${i++}`} href={href} className="text-forest underline">
            {label}
          </a>,
        );
      }
    } else if (m[4]) {
      nodes.push(<strong key={`b${i++}`}>{m[4]}</strong>);
    }
    last = m.index + m[0].length;
  }
  if (last < text.length) nodes.push(text.slice(last));
  return nodes;
}

export function Markdown({ source }: { source: string }) {
  const blocks = source.trim().split(/\n\n+/);
  return (
    <div className="space-y-6 text-[17px] leading-8 text-neutral-700">
      {blocks.map((block, i) => {
        const lines = block.split("\n");
        if (lines[0].startsWith("## ")) {
          return (
            <h2 key={i} className="pt-6 text-2xl font-bold tracking-tight text-neutral-950">
              {lines[0].slice(3)}
            </h2>
          );
        }
        if (lines[0].startsWith("# ")) {
          return (
            <h1 key={i} className="text-3xl font-bold">
              {lines[0].slice(2)}
            </h1>
          );
        }
        if (lines.every((l) => l.startsWith("- "))) {
          return (
            <ul key={i} className="list-disc space-y-2 pl-5">
              {lines.map((l, j) => (
                <li key={j}>{inline(l.slice(2))}</li>
              ))}
            </ul>
          );
        }
        return (
          <p key={i} className={i === 0 ? "text-lg leading-8 text-neutral-800" : undefined}>
            {inline(block.replace(/\n/g, " "))}
          </p>
        );
      })}
    </div>
  );
}
