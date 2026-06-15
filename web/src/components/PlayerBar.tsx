import { useEffect, useRef, useState, type ReactNode } from "react";
import type { EpisodeSummary } from "../types";
import { API_BASE } from "../api";
import { fmtTime } from "../lib/format";

interface Props {
  current: EpisodeSummary | null;
  playing: boolean;
  setPlaying: (p: boolean) => void;
  onPrev: () => void;
  onNext: () => void;
  hasPrev: boolean;
  hasNext: boolean;
}

function srcFor(id: string): string {
  return `${API_BASE}/static/episodes/${id}.mp3`;
}

export default function PlayerBar({
  current,
  playing,
  setPlaying,
  onPrev,
  onNext,
  hasPrev,
  hasNext,
}: Props) {
  const audio = useRef<HTMLAudioElement>(null);
  const [time, setTime] = useState(0);
  const [dur, setDur] = useState(0);
  const [err, setErr] = useState(false);

  // Load a new source when the current episode changes.
  useEffect(() => {
    const a = audio.current;
    if (!a || !current) return;
    setErr(false);
    a.src = srcFor(current.id);
    a.load();
    if (playing) a.play().catch(() => setErr(true));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current?.id]);

  // React to play/pause intent.
  useEffect(() => {
    const a = audio.current;
    if (!a || !current) return;
    if (playing) a.play().catch(() => setErr(true));
    else a.pause();
  }, [playing, current]);

  if (!current) return null;

  return (
    <div className="fixed bottom-0 inset-x-0 z-20 border-t border-line bg-ink/95 backdrop-blur">
      <audio
        ref={audio}
        onTimeUpdate={(e) => setTime(e.currentTarget.currentTime)}
        onLoadedMetadata={(e) => setDur(e.currentTarget.duration)}
        onEnded={() => (hasNext ? onNext() : setPlaying(false))}
        onError={() => setErr(true)}
      />
      <div className="max-w-5xl mx-auto px-4 sm:px-6 py-3 flex items-center gap-3 sm:gap-5">
        <div className="min-w-0 flex-1">
          <div className="truncate font-display text-base leading-tight">{current.title}</div>
          <div className="truncate font-mono text-[11px] text-muted">
            {err ? "audio unavailable" : current.project_name}
          </div>
        </div>

        <div className="flex items-center gap-1.5">
          <Ctrl onClick={onPrev} disabled={!hasPrev} label="Previous"><PrevIcon /></Ctrl>
          <button
            onClick={() => setPlaying(!playing)}
            aria-label={playing ? "Pause" : "Play"}
            className="grid place-items-center w-12 h-12 rounded-full bg-ember text-ink hover:bg-emberdim transition-colors"
          >
            {playing ? <PauseIcon /> : <PlayIcon />}
          </button>
          <Ctrl onClick={onNext} disabled={!hasNext} label="Next"><NextIcon /></Ctrl>
        </div>

        <div className="hidden sm:flex items-center gap-3 flex-1 min-w-0">
          <span className="font-mono text-xs text-muted tabular-nums">{fmtTime(time)}</span>
          <input
            type="range"
            min={0}
            max={dur || 0}
            value={time}
            onChange={(e) => {
              const a = audio.current;
              if (a) {
                a.currentTime = Number(e.target.value);
                setTime(a.currentTime);
              }
            }}
            className="flex-1"
          />
          <span className="font-mono text-xs text-muted tabular-nums">{fmtTime(dur)}</span>
        </div>
      </div>
    </div>
  );
}

function Ctrl({
  children,
  onClick,
  disabled,
  label,
}: {
  children: ReactNode;
  onClick: () => void;
  disabled: boolean;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      className="grid place-items-center w-10 h-10 rounded-full text-paper hover:text-ember disabled:opacity-30 disabled:hover:text-paper transition-colors"
    >
      {children}
    </button>
  );
}

function PlayIcon() {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor"><path d="M4 2.5v11l9-5.5z" /></svg>;
}
function PauseIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
      <rect x="3.5" y="2.5" width="3" height="11" rx="1" />
      <rect x="9.5" y="2.5" width="3" height="11" rx="1" />
    </svg>
  );
}
function PrevIcon() {
  return <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M5 3v10H3.5V3zM13 3v10l-7-5z" /></svg>;
}
function NextIcon() {
  return <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M11 3v10h1.5V3zM3 3v10l7-5z" /></svg>;
}
