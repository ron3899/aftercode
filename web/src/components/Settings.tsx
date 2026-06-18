import { useEffect, useState } from "react";
import { fetchSettings, saveSettings, verifySettings } from "../api";
import type { SettingsPatch, SettingsView, VerifyResult } from "../types";

const LLM = [
  { id: "anthropic", label: "Anthropic (Claude)" },
  { id: "openai", label: "OpenAI (GPT)" },
];
const TTS = [
  { id: "openai", label: "OpenAI" },
  { id: "elevenlabs", label: "ElevenLabs" },
];

function keyPlaceholder(set: boolean): string {
  return set ? "•••••••••••• (saved — leave blank to keep)" : "paste your API key";
}

export default function Settings({ onBack }: { onBack: () => void }) {
  const [view, setView] = useState<SettingsView | null>(null);
  const [error, setError] = useState<string | null>(null);

  // form state
  const [llm, setLlm] = useState("anthropic");
  const [tts, setTts] = useState("openai");
  const [anthropicKey, setAnthropicKey] = useState("");
  const [openaiKey, setOpenaiKey] = useState("");
  const [elevenKey, setElevenKey] = useState("");
  const [elevenHost, setElevenHost] = useState("");
  const [elevenExpert, setElevenExpert] = useState("");
  const [openaiModel, setOpenaiModel] = useState("");
  const [openaiHostVoice, setOpenaiHostVoice] = useState("");
  const [openaiExpertVoice, setOpenaiExpertVoice] = useState("");

  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const [verifyResult, setVerifyResult] = useState<VerifyResult | null>(null);

  useEffect(() => {
    fetchSettings()
      .then((s) => {
        setView(s);
        setLlm(s.llm_provider === "openai" ? "openai" : "anthropic");
        setTts(s.tts_provider === "elevenlabs" ? "elevenlabs" : "openai");
        setElevenHost(s.elevenlabs_host_voice_id ?? "");
        setElevenExpert(s.elevenlabs_expert_voice_id ?? "");
        setOpenaiModel(s.openai_tts_model ?? "tts-1");
        setOpenaiHostVoice(s.openai_tts_voice_host ?? "alloy");
        setOpenaiExpertVoice(s.openai_tts_voice_expert ?? "onyx");
      })
      .catch(() => setError("Couldn't load settings."));
  }, []);

  async function onSave() {
    setSaving(true);
    setSaved(false);
    setError(null);
    setVerifyResult(null);
    const patch: SettingsPatch = { llm_provider: llm, tts_provider: tts };
    if (anthropicKey.trim()) patch.anthropic_api_key = anthropicKey.trim();
    if (openaiKey.trim()) patch.openai_api_key = openaiKey.trim();
    if (tts === "elevenlabs") {
      if (elevenKey.trim()) patch.elevenlabs_api_key = elevenKey.trim();
      patch.elevenlabs_host_voice_id = elevenHost.trim();
      patch.elevenlabs_expert_voice_id = elevenExpert.trim();
    }
    if (tts === "openai") {
      patch.openai_tts_model = openaiModel.trim();
      patch.openai_tts_voice_host = openaiHostVoice.trim();
      patch.openai_tts_voice_expert = openaiExpertVoice.trim();
    }
    try {
      const updated = await saveSettings(patch);
      setView(updated);
      setAnthropicKey("");
      setOpenaiKey("");
      setElevenKey("");
      setSaved(true);
    } catch {
      setError("Save failed. Is the backend running?");
    } finally {
      setSaving(false);
    }
  }

  async function onVerify() {
    setVerifying(true);
    setVerifyResult(null);
    setError(null);
    try {
      setVerifyResult(await verifySettings());
    } catch {
      setError("Verify failed. Is the backend running?");
    } finally {
      setVerifying(false);
    }
  }

  if (error && !view) {
    return <p className="max-w-2xl mx-auto px-6 mt-10 text-ember font-mono text-sm">{error}</p>;
  }
  if (!view) {
    return <div className="max-w-2xl mx-auto px-6 mt-10 h-40 rounded-2xl bg-card/60 animate-pulse" />;
  }

  return (
    <div className="max-w-2xl mx-auto px-5 sm:px-6 pt-8 pb-32 relative z-10 animate-rise">
      <button onClick={onBack} className="font-mono text-xs text-muted hover:text-ember mb-6">
        ← back to library
      </button>

      <h1 className="font-display text-4xl mb-2">Providers</h1>
      <p className="text-muted mb-8 leading-relaxed">
        Add your own API keys to generate real episodes — no <code className="font-mono text-sm">.env</code>{" "}
        editing. Keys are stored on your backend and never shown again. Leave a side on{" "}
        <span className="font-mono">mock</span> to skip it.
      </p>

      {/* LLM */}
      <Section
        title="Script writer (LLM)"
        hint="Writes the two-speaker episode script from your coding session."
      >
        <Radio options={LLM} value={llm} onChange={setLlm} name="llm" />
        {llm === "anthropic" ? (
          <KeyField
            label="Anthropic API key"
            placeholder={keyPlaceholder(view.anthropic_key_set)}
            value={anthropicKey}
            onChange={setAnthropicKey}
            help="Get one at console.anthropic.com → Settings → API Keys"
            href="https://console.anthropic.com/settings/keys"
          />
        ) : (
          <KeyField
            label="OpenAI API key"
            placeholder={keyPlaceholder(view.openai_key_set)}
            value={openaiKey}
            onChange={setOpenaiKey}
            help="Get one at platform.openai.com → API keys"
            href="https://platform.openai.com/api-keys"
          />
        )}
      </Section>

      {/* TTS */}
      <Section title="Voice (TTS)" hint="Turns the script into audio with two voices.">
        <Radio options={TTS} value={tts} onChange={setTts} name="tts" />
        {tts === "openai" ? (
          <>
            <KeyField
              label="OpenAI API key"
              placeholder={keyPlaceholder(view.openai_key_set)}
              value={openaiKey}
              onChange={setOpenaiKey}
              help="Same OpenAI key works for both script and voice. platform.openai.com → API keys"
              href="https://platform.openai.com/api-keys"
            />
            <div className="grid sm:grid-cols-3 gap-3 mt-3">
              <Text label="Model" value={openaiModel} onChange={setOpenaiModel} placeholder="tts-1" />
              <Text label="Host voice" value={openaiHostVoice} onChange={setOpenaiHostVoice} placeholder="alloy" />
              <Text label="Expert voice" value={openaiExpertVoice} onChange={setOpenaiExpertVoice} placeholder="onyx" />
            </div>
          </>
        ) : (
          <>
            <KeyField
              label="ElevenLabs API key"
              placeholder={keyPlaceholder(view.elevenlabs_key_set)}
              value={elevenKey}
              onChange={setElevenKey}
              help="elevenlabs.io → your profile → API key. Copy voice IDs from Voice Lab."
              href="https://elevenlabs.io/app/settings/api-keys"
            />
            <div className="grid sm:grid-cols-2 gap-3 mt-3">
              <Text label="Host voice ID" value={elevenHost} onChange={setElevenHost} placeholder="e.g. 21m00Tcm…" />
              <Text label="Expert voice ID" value={elevenExpert} onChange={setElevenExpert} placeholder="e.g. AZnzlk1X…" />
            </div>
          </>
        )}
      </Section>

      <div className="flex items-center gap-3 mt-8">
        <button
          onClick={onSave}
          disabled={saving}
          className="font-sans font-semibold text-ink bg-ember hover:bg-emberdim disabled:opacity-50 transition-colors rounded-xl px-7 py-3"
        >
          {saving ? "Saving…" : "Save"}
        </button>
        <button
          onClick={onVerify}
          disabled={verifying}
          className="font-mono text-sm text-muted hover:text-ember border border-line rounded-xl px-5 py-3 disabled:opacity-50"
        >
          {verifying ? "Checking…" : "Test keys"}
        </button>
        {saved && <span className="font-mono text-xs text-ember">saved ✓</span>}
      </div>

      {error && <p className="text-ember font-mono text-sm mt-4">{error}</p>}
      {verifyResult && (
        <div className="mt-4 space-y-1 font-mono text-sm">
          <CheckLine label="LLM" check={verifyResult.llm} />
          <CheckLine label="TTS" check={verifyResult.tts} />
        </div>
      )}
    </div>
  );
}

function Section({
  title,
  hint,
  children,
}: {
  title: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-2xl border border-line bg-card/50 p-5 sm:p-6 mb-5">
      <h2 className="font-display text-2xl">{title}</h2>
      <p className="text-muted text-sm mb-4">{hint}</p>
      {children}
    </div>
  );
}

function Radio({
  options,
  value,
  onChange,
  name,
}: {
  options: { id: string; label: string }[];
  value: string;
  onChange: (v: string) => void;
  name: string;
}) {
  return (
    <div className="flex flex-wrap gap-2 mb-4">
      {options.map((o) => (
        <button
          key={o.id}
          type="button"
          aria-pressed={value === o.id}
          onClick={() => onChange(o.id)}
          name={name}
          className={`font-mono text-sm rounded-lg px-4 py-2 border transition-colors ${
            value === o.id
              ? "border-ember text-ember bg-ember/10"
              : "border-line text-muted hover:text-paper"
          }`}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

function KeyField({
  label,
  placeholder,
  value,
  onChange,
  help,
  href,
}: {
  label: string;
  placeholder: string;
  value: string;
  onChange: (v: string) => void;
  help: string;
  href: string;
}) {
  return (
    <label className="block">
      <span className="font-mono text-xs text-muted">{label}</span>
      <input
        type="password"
        autoComplete="off"
        spellCheck={false}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className="mt-1 w-full bg-ink/60 border border-line rounded-lg px-3 py-2.5 font-mono text-sm focus:border-ember outline-none"
      />
      <a
        href={href}
        target="_blank"
        rel="noreferrer"
        className="inline-block mt-1.5 text-xs text-muted hover:text-ember underline decoration-dotted"
      >
        {help} ↗
      </a>
    </label>
  );
}

function Text({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
}) {
  return (
    <label className="block">
      <span className="font-mono text-xs text-muted">{label}</span>
      <input
        type="text"
        spellCheck={false}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className="mt-1 w-full bg-ink/60 border border-line rounded-lg px-3 py-2.5 font-mono text-sm focus:border-ember outline-none"
      />
    </label>
  );
}

function CheckLine({ label, check }: { label: string; check: { provider: string; ok: boolean; error?: string } }) {
  return (
    <p className={check.ok ? "text-ember" : "text-red-400"}>
      {label} · {check.provider}: {check.ok ? "ok ✓" : `failed — ${check.error ?? "error"}`}
    </p>
  );
}
