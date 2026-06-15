import { describe, it, expect } from "vitest";
import { allTopics, filterEpisodes, EMPTY_FILTERS } from "./filter";
import type { EpisodeSummary } from "../types";

const ep = (over: Partial<EpisodeSummary>): EpisodeSummary => ({
  id: "1",
  title: "T",
  language: "en",
  status: "ready",
  duration_seconds: 100,
  topics: [],
  project_name: "p",
  created_at: "2026-06-15",
  ...over,
});

const data: EpisodeSummary[] = [
  ep({ id: "a", title: "Redis caching", topics: ["Redis", "Caching"], language: "en" }),
  ep({ id: "b", title: "RabbitMQ queues", topics: ["RabbitMQ"], language: "he" }),
  ep({ id: "c", title: "Postgres indexes", topics: ["Postgres", "Redis"], language: "en" }),
];

describe("allTopics", () => {
  it("returns sorted distinct topics", () => {
    expect(allTopics(data)).toEqual(["Caching", "Postgres", "RabbitMQ", "Redis"]);
  });
});

describe("filterEpisodes", () => {
  it("no filters returns all", () => {
    expect(filterEpisodes(data, EMPTY_FILTERS)).toHaveLength(3);
  });
  it("filters by topic", () => {
    const out = filterEpisodes(data, { ...EMPTY_FILTERS, topic: "Redis" });
    expect(out.map((e) => e.id)).toEqual(["a", "c"]);
  });
  it("filters by language", () => {
    const out = filterEpisodes(data, { ...EMPTY_FILTERS, language: "he" });
    expect(out.map((e) => e.id)).toEqual(["b"]);
  });
  it("filters by query across title/topics/project", () => {
    expect(filterEpisodes(data, { ...EMPTY_FILTERS, query: "postgres" }).map((e) => e.id)).toEqual(["c"]);
    expect(filterEpisodes(data, { ...EMPTY_FILTERS, query: "rabbit" }).map((e) => e.id)).toEqual(["b"]);
  });
  it("combines topic + language", () => {
    const out = filterEpisodes(data, { ...EMPTY_FILTERS, topic: "Redis", language: "en" });
    expect(out.map((e) => e.id)).toEqual(["a", "c"]);
  });
  it("empty when nothing matches", () => {
    expect(filterEpisodes(data, { ...EMPTY_FILTERS, query: "zzz" })).toHaveLength(0);
  });
});
