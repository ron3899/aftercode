import { useEffect, useMemo, useState } from "react";
import { AuthError, fetchEpisodes } from "./api";
import { captureTokenFromHash, clearToken, getToken } from "./auth";
import type { EpisodeSummary } from "./types";
import { EMPTY_FILTERS, filterEpisodes, type Filters } from "./lib/filter";
import SignIn from "./components/SignIn";
import Library from "./components/Library";
import EpisodeDetail from "./components/EpisodeDetail";
import PlayerBar from "./components/PlayerBar";

export default function App() {
  // Capture a token returned in the URL fragment before first render decisions.
  const [token, setTokenState] = useState<string | null>(() => {
    const captured = captureTokenFromHash();
    return captured ?? getToken();
  });

  const [episodes, setEpisodes] = useState<EpisodeSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filters, setFilters] = useState<Filters>(EMPTY_FILTERS);
  const [detailId, setDetailId] = useState<string | null>(null);

  // Player state
  const [currentId, setCurrentId] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);

  useEffect(() => {
    if (!token) return;
    setLoading(true);
    setError(null);
    fetchEpisodes()
      .then(setEpisodes)
      .catch((e) => {
        if (e instanceof AuthError) {
          setTokenState(null);
        } else {
          setError("Can't reach the backend. Is the server running?");
        }
      })
      .finally(() => setLoading(false));
  }, [token]);

  // The queue is whatever the filters currently show.
  const queue = useMemo(() => filterEpisodes(episodes, filters), [episodes, filters]);
  const current = useMemo(
    () => episodes.find((e) => e.id === currentId) ?? null,
    [episodes, currentId],
  );
  const qIndex = current ? queue.findIndex((e) => e.id === current.id) : -1;

  function play(id: string) {
    if (currentId === id) {
      setPlaying((p) => !p);
    } else {
      setCurrentId(id);
      setPlaying(true);
    }
  }

  function signOut() {
    clearToken();
    setTokenState(null);
    setEpisodes([]);
    setCurrentId(null);
    setPlaying(false);
  }

  if (!token) return <SignIn />;

  return (
    <div className="min-h-screen relative">
      <TopBar onSignOut={signOut} />

      {error && (
        <p className="max-w-5xl mx-auto px-6 mt-6 text-ember font-mono text-sm relative z-10">{error}</p>
      )}
      {loading && episodes.length === 0 && (
        <div className="max-w-5xl mx-auto px-6 pt-10 grid md:grid-cols-2 gap-4 relative z-10">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="h-32 rounded-2xl bg-card/60 animate-pulse" />
          ))}
        </div>
      )}

      {!loading || episodes.length > 0 ? (
        detailId ? (
          <EpisodeDetail
            id={detailId}
            onBack={() => setDetailId(null)}
            onPlay={play}
            playingId={playing ? currentId : null}
          />
        ) : (
          <Library
            episodes={episodes}
            filters={filters}
            setFilters={setFilters}
            currentId={currentId}
            playing={playing}
            onPlay={play}
            onOpen={setDetailId}
          />
        )
      ) : null}

      <PlayerBar
        current={current}
        playing={playing}
        setPlaying={setPlaying}
        onPrev={() => qIndex > 0 && play(queue[qIndex - 1].id)}
        onNext={() => qIndex >= 0 && qIndex < queue.length - 1 && play(queue[qIndex + 1].id)}
        hasPrev={qIndex > 0}
        hasNext={qIndex >= 0 && qIndex < queue.length - 1}
      />
    </div>
  );
}

function TopBar({ onSignOut }: { onSignOut: () => void }) {
  return (
    <div className="sticky top-0 z-20 border-b border-line bg-ink/85 backdrop-blur">
      <div className="max-w-5xl mx-auto px-5 sm:px-6 h-14 flex items-center justify-between">
        <span className="font-display text-xl">
          after<span className="text-ember">code</span>
        </span>
        <button onClick={onSignOut} className="font-mono text-xs text-muted hover:text-ember">
          sign out
        </button>
      </div>
    </div>
  );
}
