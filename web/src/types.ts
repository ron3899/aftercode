export type EpisodeStatus =
  | "queued"
  | "extracting_topics"
  | "writing_script"
  | "generating_audio"
  | "ready"
  | "failed";

export interface EpisodeSummary {
  id: string;
  title: string;
  language: string;
  status: EpisodeStatus;
  duration_seconds: number | null;
  topics: string[];
  project_name: string;
  created_at: string;
}

export interface LearningTopic {
  title: string;
  summary: string;
  evidence: string[];
  knowledge_gap: string;
  difficulty: string;
  priority: string;
}

export interface ScriptSegment {
  speaker: "host" | "expert";
  text: string;
}

export interface EpisodeScript {
  title: string;
  language: string;
  segments: ScriptSegment[];
  summary_points: string[];
  quiz?: { question: string; answer: string } | null;
}

export interface EpisodeDetail {
  id: string;
  title: string;
  language: string;
  status: EpisodeStatus;
  audio_url: string | null;
  duration_seconds: number | null;
  summary: string | null;
  transcript_text: string | null;
  topics: LearningTopic[] | null;
  script: EpisodeScript | null;
  error: string | null;
  created_at: string;
}
