import { useEffect, useState, type ReactNode } from "react";
import { fetchEpisode } from "../api";
import type { EpisodeDetail as Detail } from "../types";
import { fmtDate, fmtTime, langLabel } from "../lib/format";

interface Props {
  id: string;
  onBack: () => void;
  onPlay: (id: string) => void;
  playingId: string | null;
}

export default function EpisodeDetail({ id, onBack, onPlay, playingId }: Props) {
  const [ep, setEp] = useState<Detail | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [quizOpen, setQuizOpen] = useState(false);

  useEffect(() => {
    setEp(null);
    setErr(null);
    fetchEpisode(id)
      .then(setEp)
      .catch((e) => setErr(String(e)));
  }, [id]);

  return (
    <div className="max-w-3xl mx-auto px-5 sm:px-6 pt-8 pb-40 relative z-10 animate-rise">
      <button onClick={onBack} className="font-mono text-xs text-muted hover:text-ember mb-8">
        ← back to library
      </button>

      {err && <p className="text-ember font-mono text-sm">{err}</p>}
      {!ep && !err && <div className="h-40 rounded-2xl bg-card/60 animate-pulse" />}

      {ep && (
        <>
          <p className="font-mono text-xs tracking-widest text-ember uppercase mb-3">
            {langLabel(ep.language)} · {fmtDate(ep.created_at)}
            {ep.duration_seconds != null && <> · {fmtTime(ep.duration_seconds)}</>}
          </p>
          <h1 className="font-display text-4xl sm:text-5xl leading-[1.02] mb-6">{ep.title}</h1>

          {ep.status === "ready" ? (
            <button
              onClick={() => onPlay(ep.id)}
              className="inline-flex items-center gap-2 font-semibold text-ink bg-ember hover:bg-emberdim rounded-xl px-6 py-3 mb-10"
            >
              {playingId === ep.id ? "Now playing" : "Play episode"} ▸
            </button>
          ) : (
            <p className="font-mono text-sm text-muted mb-10">
              {ep.status === "failed" ? `generation failed: ${ep.error ?? "unknown"}` : `${ep.status.replace(/_/g, " ")}…`}
            </p>
          )}

          {ep.topics && ep.topics.length > 0 && (
            <Section title="Topics">
              <div className="flex flex-wrap gap-2">
                {ep.topics.map((t) => (
                  <span key={t.title} className="px-3 py-1 rounded-full bg-card border border-line text-sm">
                    {t.title}
                  </span>
                ))}
              </div>
            </Section>
          )}

          {ep.script?.summary_points && ep.script.summary_points.length > 0 && (
            <Section title="Key takeaways">
              <ul className="space-y-2">
                {ep.script.summary_points.map((p, i) => (
                  <li key={i} className="flex gap-3">
                    <span className="text-ember font-mono">{String(i + 1).padStart(2, "0")}</span>
                    <span className="text-paper/90">{p}</span>
                  </li>
                ))}
              </ul>
            </Section>
          )}

          {ep.script?.quiz && (
            <Section title="Quiz">
              <p className="text-paper/90 mb-3">{ep.script.quiz.question}</p>
              {quizOpen ? (
                <p className="text-ember">{ep.script.quiz.answer}</p>
              ) : (
                <button onClick={() => setQuizOpen(true)} className="font-mono text-xs text-muted hover:text-ember">
                  reveal answer →
                </button>
              )}
            </Section>
          )}

          {ep.script?.segments && ep.script.segments.length > 0 && (
            <Section title="Transcript">
              <div className="space-y-4">
                {ep.script.segments.map((s, i) => (
                  <div key={i}>
                    <span
                      className={
                        "font-mono text-xs uppercase tracking-wider " +
                        (s.speaker === "host" ? "text-ember" : "text-muted")
                      }
                    >
                      {s.speaker}
                    </span>
                    <p className="text-paper/90 mt-1 leading-relaxed">{s.text}</p>
                  </div>
                ))}
              </div>
            </Section>
          )}
        </>
      )}
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-10">
      <h2 className="font-mono text-xs tracking-widest text-muted uppercase mb-4 border-b border-line pb-2">
        {title}
      </h2>
      {children}
    </section>
  );
}
