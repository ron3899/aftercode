interface Props {
  topics: string[];
  active: string | null;
  onPick: (t: string | null) => void;
}

export default function TopicFilter({ topics, active, onPick }: Props) {
  if (topics.length === 0) return null;
  return (
    <div className="flex flex-wrap gap-2">
      <Chip label="All topics" on={active === null} onClick={() => onPick(null)} />
      {topics.map((t) => (
        <Chip key={t} label={t} on={active === t} onClick={() => onPick(active === t ? null : t)} />
      ))}
    </div>
  );
}

function Chip({ label, on, onClick }: { label: string; on: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={
        "px-3 py-1.5 rounded-full text-sm transition-colors border " +
        (on
          ? "bg-ember text-ink border-ember font-semibold"
          : "bg-transparent text-muted border-line hover:border-ember hover:text-paper")
      }
    >
      {label}
    </button>
  );
}
