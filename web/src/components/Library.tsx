import type { EpisodeSummary } from "../types";
import { allTopics, filterEpisodes, type Filters } from "../lib/filter";
import { langLabel } from "../lib/format";
import TopicFilter from "./TopicFilter";
import EpisodeCard from "./EpisodeCard";

interface Props {
  episodes: EpisodeSummary[];
  filters: Filters;
  setFilters: (f: Filters) => void;
  currentId: string | null;
  playing: boolean;
  onPlay: (id: string) => void;
  onOpen: (id: string) => void;
}

export default function Library({
  episodes,
  filters,
  setFilters,
  currentId,
  playing,
  onPlay,
  onOpen,
}: Props) {
  const topics = allTopics(episodes);
  const langs = Array.from(new Set(episodes.map((e) => e.language)));
  const shown = filterEpisodes(episodes, filters);

  return (
    <div className="max-w-5xl mx-auto px-5 sm:px-6 pt-8 pb-40 relative z-10">
      <header className="mb-8 animate-rise">
        <p className="font-mono text-xs tracking-[0.3em] text-ember uppercase mb-2">Aftercode · Library</p>
        <h1 className="font-display text-4xl sm:text-5xl">Your episodes</h1>
        <p className="text-muted mt-2">
          {episodes.length} {episodes.length === 1 ? "episode" : "episodes"} from your coding sessions.
        </p>
      </header>

      <div className="flex flex-col gap-4 mb-8">
        <div className="flex flex-col sm:flex-row gap-3">
          <input
            value={filters.query}
            onChange={(e) => setFilters({ ...filters, query: e.target.value })}
            placeholder="Search episodes…"
            className="flex-1 bg-card border border-line rounded-xl px-4 py-2.5 text-paper placeholder:text-muted/60 focus:outline-none focus:border-ember"
          />
          {langs.length > 1 && (
            <select
              value={filters.language ?? ""}
              onChange={(e) => setFilters({ ...filters, language: e.target.value || null })}
              className="bg-card border border-line rounded-xl px-4 py-2.5 text-paper focus:outline-none focus:border-ember"
            >
              <option value="">All languages</option>
              {langs.map((l) => (
                <option key={l} value={l}>
                  {langLabel(l)}
                </option>
              ))}
            </select>
          )}
        </div>
        <TopicFilter topics={topics} active={filters.topic} onPick={(t) => setFilters({ ...filters, topic: t })} />
      </div>

      {shown.length === 0 ? (
        <div className="text-center py-20 text-muted">
          <p className="font-display text-2xl mb-2">No episodes match</p>
          <p className="text-sm">Try clearing a filter, or run <span className="font-mono text-ember">aftercode episode</span>.</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {shown.map((ep) => (
            <EpisodeCard
              key={ep.id}
              ep={ep}
              isPlaying={currentId === ep.id && playing}
              onPlay={() => onPlay(ep.id)}
              onOpen={() => onOpen(ep.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
