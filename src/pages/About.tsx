import { PageHeader } from "../components/shell/PageHeader";
import { useRpc } from "../lib/store";
import type { AppInfo } from "../lib/types";

const FACTS: [string, string][] = [
  ["Platform", "Ubuntu Linux"],
  ["Built with", "Tauri 2, React and Rust"],
  ["Storage", "SQLite, on this computer only"],
  ["License", "MIT"],
  ["Author", "Muhammad Ammar Qaisar"],
];

export function About() {
  const { data: info } = useRpc<AppInfo>("app.info");
  return (
    <div className="animate-page mx-auto max-w-[560px]">
      <PageHeader title="About" />
      <div className="surface flex flex-col items-center px-6 py-10 text-center">
        <img src="/logo.svg" alt="" className="size-20" />
        <h2 className="mt-4 text-page font-[650]">Dayweave</h2>
        <p className="mt-1 text-heading text-ink-2">Build better days, one habit at a time.</p>
        <p className="tabular mt-3 text-label text-ink-2">Version {info?.version}</p>
      </div>
      <dl className="surface row-list mt-4 overflow-hidden">
        {FACTS.map(([label, value]) => (
          <div key={label} className="flex justify-between gap-4 px-4 py-2.5 text-body">
            <dt className="text-ink-2">{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
