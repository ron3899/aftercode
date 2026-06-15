import { clearToken, getToken } from "./auth";
import type { EpisodeDetail, EpisodeSummary } from "./types";

// Dev: VITE_API_BASE points at the backend. Prod: same origin (served by backend).
export const API_BASE: string =
  (import.meta.env.VITE_API_BASE as string | undefined)?.replace(/\/$/, "") ?? "";

export class AuthError extends Error {}

async function get<T>(path: string): Promise<T> {
  const token = getToken();
  const res = await fetch(`${API_BASE}${path}`, {
    headers: token ? { authorization: `Bearer ${token}` } : {},
  });
  if (res.status === 401) {
    clearToken();
    throw new AuthError("unauthorized");
  }
  if (!res.ok) throw new Error(`request failed: ${res.status}`);
  return (await res.json()) as T;
}

export async function fetchEpisodes(): Promise<EpisodeSummary[]> {
  const data = await get<{ episodes: EpisodeSummary[] }>("/episodes");
  return data.episodes;
}

export async function fetchEpisode(id: string): Promise<EpisodeDetail> {
  return get<EpisodeDetail>(`/episodes/${id}`);
}

/** Make a backend-relative audio_url absolute against the API base (dev). */
export function audioUrl(url: string | null): string | null {
  if (!url) return null;
  if (/^https?:\/\//.test(url) && !url.startsWith("http://localhost") && !API_BASE) return url;
  // When served same-origin, the stored absolute URL works as-is.
  return url;
}
