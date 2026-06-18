import { useEffect, useMemo, useState } from "react";
import { AuthError, fetchEpisodes, fetchSettings } from "./api";
import { captureTokenFromHash, clearToken, getToken } from "./auth";
import { isConfigured, type EpisodeSummary, type SettingsView } from "./types";
import { EMPTY_FILTERS, filterEpisodes, type Filters } from "./lib/filter";
import SignIn from "./components/SignIn";
import Library from "./components/Library";
import EpisodeDetail from "./components/EpisodeDetail";
import PlayerBar from "./components/PlayerBar";
import Settings from "./components/Settings";

type View = "library" | "settings";

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
  const [view, setView] = useState<View>("library");
  const [settings, setSettings] = useState<SettingsView | null>(null);

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
    // Settings drive the "set up providers" banner. Refetched when we leave the
    // settings page so the banner reflects a fresh save.
    fetchSettings()
      .then(setSettings)
      .catch(() => setSettings(null));
  }, [token, view]);

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
    setView("library");
  }

  if (!token) return <SignIn />;

  const needsSetup = settings != null && !isConfigured(settings);

  return (
    <div className="min-h-screen relative">
      <TopBar view={view} onNav={setView} onSignOut={signOut} />

      {view === "settings" ? (
        <Settings onBack={() => setView("library")} />
      ) : (
        <>
          {needsSetup && !detailId && <SetupBanner onSetup={() => setView("settings")} />}

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
        </>
      )}

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

function SetupBanner({ onSetup }: { onSetup: () => void }) {
  return (
    <div className="max-w-5xl mx-auto px-5 sm:px-6 mt-6 relative z-10">
      <div className="rounded-2xl border border-ember/40 bg-ember/10 px-5 py-4 flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm">
          <span className="font-semibold text-ember">Running in mock mode.</span>{" "}
          <span className="text-muted">Add an API key to generate real episodes.</span>
        </p>
        <button
          onClick={onSetup}
          className="font-sans font-semibold text-ink bg-ember hover:bg-emberdim transition-colors rounded-lg px-4 py-2 text-sm"
        >
          Set up providers →
        </button>
      </div>
    </div>
  );
}

function TopBar({
  view,
  onNav,
  onSignOut,
}: {
  view: View;
  onNav: (v: View) => void;
  onSignOut: () => void;
}) {
  return (
    <div className="sticky top-0 z-20 border-b border-line bg-ink/85 backdrop-blur">
      <div className="max-w-5xl mx-auto px-5 sm:px-6 h-14 flex items-center justify-between">
        <button onClick={() => onNav("library")} className="font-display text-xl">
          after<span className="text-ember">code</span>
        </button>
        <div className="flex items-center gap-5">
          <button
            onClick={() => onNav(view === "settings" ? "library" : "settings")}
            className={`font-mono text-xs hover:text-ember ${
              view === "settings" ? "text-ember" : "text-muted"
            }`}
          >
            settings
          </button>
          <button onClick={onSignOut} className="font-mono text-xs text-muted hover:text-ember">
            sign out
          </button>
        </div>
      </div>
    </div>
  );
}
