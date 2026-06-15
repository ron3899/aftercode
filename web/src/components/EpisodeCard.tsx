import type { EpisodeSummary } from "../types";
import { fmtDate, fmtTime, langLabel } from "../lib/format";

interface Props {
  ep: EpisodeSummary;
  isPlaying: boolean;
  onPlay: () => void;
  onOpen: () => void;
}

export default function EpisodeCard({ ep, isPlaying, onPlay, onOpen }: Props) {
  const ready = ep.status === "ready";
  return (
    <div
      className={
        "group relative rounded-2xl border bg-card/80 p-5 transition-all hover:-translate-y-0.5 " +
        (isPlaying ? "border-ember shadow-[0_0_0_1px_rgba(224,121,75,0.4)]" : "border-line hover:border-line/80")
      }
    >
      <div className="flex items-start justify-between gap-3">
        <button onClick={onOpen} className="text-left">
          <h3 className="font-display text-xl leading-snug group-hover:text-ember transition-colors">
            {ep.title || "Untitled episode"}
          </h3>
        </button>
        <PlayButton ready={ready} playing={isPlaying} onClick={onPlay} />
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-xs text-muted">
        <span>{ep.project_name}</span>
        <span className="text-line">·</span>
        <span>{langLabel(ep.language)}</span>
        {ready && ep.duration_seconds != null && (
          <>
            <span className="text-line">·</span>
            <span>{fmtTime(ep.duration_seconds)}</span>
          </>
        )}
        <span className="text-line">·</span>
        <span>{fmtDate(ep.created_at)}</span>
      </div>

      {ep.topics.length > 0 && (
        <div className="mt-4 flex flex-wrap gap-1.5">
          {ep.topics.slice(0, 4).map((t) => (
            <span key={t} className="text-xs px-2 py-0.5 rounded-full bg-ink border border-line text-muted">
              {t}
            </span>
          ))}
        </div>
      )}

      {!ready && <StatusNote status={ep.status} />}
    </div>
  );
}

function PlayButton({ ready, playing, onClick }: { ready: boolean; playing: boolean; onClick: () => void }) {
  if (!ready) return null;
  return (
    <button
      onClick={onClick}
      aria-label={playing ? "Pause" : "Play"}
      className={
        "shrink-0 grid place-items-center w-11 h-11 rounded-full transition-colors " +
        (playing ? "bg-ember text-ink" : "bg-ink border border-line text-paper hover:border-ember hover:text-ember")
      }
    >
      {playing ? <PauseIcon /> : <PlayIcon />}
    </button>
  );
}

function StatusNote({ status }: { status: string }) {
  const failed = status === "failed";
  return (
    <div className={"mt-3 text-xs font-mono " + (failed ? "text-ember" : "text-muted")}>
      {failed ? "● generation failed" : "○ " + status.replace(/_/g, " ") + "…"}
    </div>
  );
}

function PlayIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
      <path d="M4 2.5v11l9-5.5z" />
    </svg>
  );
}
function PauseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
      <rect x="3.5" y="2.5" width="3" height="11" rx="1" />
      <rect x="9.5" y="2.5" width="3" height="11" rx="1" />
    </svg>
  );
}
