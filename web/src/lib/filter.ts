import type { EpisodeSummary } from "../types";

export interface Filters {
  topic: string | null; // null = all
  language: string | null; // null = all
  query: string; // free text
}

export const EMPTY_FILTERS: Filters = { topic: null, language: null, query: "" };

/** All distinct topics across episodes, sorted, for the chip row. */
export function allTopics(episodes: EpisodeSummary[]): string[] {
  const set = new Set<string>();
  for (const e of episodes) for (const t of e.topics) set.add(t);
  return Array.from(set).sort((a, b) => a.localeCompare(b));
}

/** Pure: filter episodes by topic + language + free-text query. */
export function filterEpisodes(
  episodes: EpisodeSummary[],
  f: Filters,
): EpisodeSummary[] {
  const q = f.query.trim().toLowerCase();
  return episodes.filter((e) => {
    if (f.topic && !e.topics.includes(f.topic)) return false;
    if (f.language && e.language !== f.language) return false;
    if (q) {
      const hay = (e.title + " " + e.project_name + " " + e.topics.join(" ")).toLowerCase();
      if (!hay.includes(q)) return false;
    }
    return true;
  });
}
