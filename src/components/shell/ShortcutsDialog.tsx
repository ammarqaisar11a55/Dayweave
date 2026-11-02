import { showShortcuts } from "../../lib/ui";
import { Dialog } from "../ui/Dialog";

const GROUPS: { title: string; items: [string, string][] }[] = [
  {
    title: "Anywhere",
    items: [
      ["Ctrl+K", "Open the command palette"],
      ["/", "Search"],
      ["Ctrl+N", "New task"],
      ["Ctrl+Shift+N", "New habit"],
      ["Ctrl+B", "Collapse the sidebar"],
      ["Ctrl+1 to 7", "Go to a page"],
      ["Ctrl+,", "Settings"],
      ["F1", "Show these shortcuts"],
      ["Ctrl+W", "Close the window"],
      ["Ctrl+Q", "Quit Dayweave"],
    ],
  },
  {
    title: "Task lists",
    items: [
      ["J or K", "Next or previous task"],
      ["X", "Complete the selected task"],
      ["Enter", "Open the selected task"],
      ["Esc", "Close the details panel"],
    ],
  },
  {
    title: "New task",
    items: [
      ["Enter", "Add the task"],
      ["Ctrl+Enter", "Add it and start another"],
    ],
  },
];

export function ShortcutsDialog() {
  return (
    <Dialog title="Keyboard shortcuts" onClose={() => showShortcuts(false)} width="max-w-md">
      <div className="flex flex-col gap-5 pb-5">
        {GROUPS.map((group) => (
          <section key={group.title} aria-label={group.title}>
            <h3 className="mb-1.5 text-label font-semibold">{group.title}</h3>
            <dl>
              {group.items.map(([keys, description]) => (
                <div key={keys} className="flex items-center justify-between py-1">
                  <dd className="text-body text-ink-2">{description}</dd>
                  <dt className="tabular rounded-[5px] bg-sunken px-1.5 py-0.5 text-meta">{keys}</dt>
                </div>
              ))}
            </dl>
          </section>
        ))}
      </div>
    </Dialog>
  );
}
