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

export interface SettingsView {
  llm_provider: string;
  tts_provider: string;
  anthropic_key_set: boolean;
  openai_key_set: boolean;
  elevenlabs_key_set: boolean;
  elevenlabs_host_voice_id: string | null;
  elevenlabs_expert_voice_id: string | null;
  openai_tts_model: string | null;
  openai_tts_voice_host: string | null;
  openai_tts_voice_expert: string | null;
}

export interface SettingsPatch {
  llm_provider?: string;
  anthropic_api_key?: string;
  openai_api_key?: string;
  tts_provider?: string;
  elevenlabs_api_key?: string;
  elevenlabs_host_voice_id?: string;
  elevenlabs_expert_voice_id?: string;
  openai_tts_model?: string;
  openai_tts_voice_host?: string;
  openai_tts_voice_expert?: string;
}

export interface ProviderCheck {
  provider: string;
  ok: boolean;
  error?: string;
}

export interface VerifyResult {
  llm: ProviderCheck;
  tts: ProviderCheck;
}

/** True once at least one non-mock provider has its key set (real episodes possible). */
export function isConfigured(s: SettingsView): boolean {
  const llmReal =
    s.llm_provider !== "mock" &&
    (s.llm_provider === "openai" ? s.openai_key_set : s.anthropic_key_set);
  const ttsReal =
    s.tts_provider !== "mock" &&
    (s.tts_provider === "openai" ? s.openai_key_set : s.elevenlabs_key_set);
  return llmReal || ttsReal;
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
