pub mod assemble;
pub mod normalize;
pub mod rank;

use crate::providers::llm::{LlmProvider, ScriptOpts};
use crate::providers::tts::TtsProvider;
use crate::storage::blob::BlobStore;
use aftercode_core::audio::VoiceRole;
use aftercode_core::episode::{EpisodeScript, LearningTopic, Speaker};
use aftercode_core::session::SessionContext;
use assemble::{concat_with_gaps, encode_mp3, RenderedSegment};
use std::sync::Arc;

pub struct PipelineOutput {
    pub topics: Vec<LearningTopic>,
    pub script: EpisodeScript,
    pub audio_url: String,
    pub duration_seconds: i32,
    pub transcript: String,
}

/// Full generation: normalize -> topics -> rank -> script -> tts -> assemble -> store.
/// `on_status` is called as each stage starts so the worker can persist progress.
pub async fn run_pipeline(
    ctx: &SessionContext,
    episode_key: &str,
    llm: Arc<dyn LlmProvider>,
    tts: Arc<dyn TtsProvider>,
    blob: Arc<dyn BlobStore>,
    mut on_status: impl FnMut(&str),
) -> anyhow::Result<PipelineOutput> {
    on_status("extracting_topics");
    let norm = normalize::normalize(ctx);
    let topics = rank::rank(llm.extract_topics(&norm).await?, 3);

    on_status("writing_script");
    let script = llm
        .write_script(
            &topics,
            &ScriptOpts {
                language: ctx.language,
                minutes: ctx.episode_length_minutes,
            },
        )
        .await?;

    on_status("generating_audio");
    let mut rendered = Vec::new();
    for seg in &script.segments {
        let role = match seg.speaker {
            Speaker::Host => VoiceRole::Host,
            Speaker::Expert => VoiceRole::Expert,
        };
        let audio = tts.synthesize(&seg.text, role, ctx.language).await?;
        rendered.push(RenderedSegment {
            speaker: seg.speaker,
            audio,
        });
    }
    let full = concat_with_gaps(&rendered);
    let duration = full.duration_seconds() as i32;
    let mp3 = encode_mp3(&full)?;
    let url = blob
        .put(&format!("episodes/{episode_key}.mp3"), mp3, "audio/mpeg")
        .await?;

    let transcript = script
        .segments
        .iter()
        .map(|s| format!("{:?}: {}", s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(PipelineOutput {
        topics,
        script,
        audio_url: url,
        duration_seconds: duration,
        transcript,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::llm::MockLlm;
    use crate::providers::tts::MockTts;
    use crate::storage::blob::MockBlob;
    use aftercode_core::session::Language;

    #[tokio::test]
    async fn pipeline_produces_ready_episode_with_mocks() {
        let ctx = SessionContext {
            project_id: "p".into(),
            language: Language::En,
            episode_length_minutes: 10,
            collected_at: "t".into(),
            events: vec![],
            changed_files: vec!["migration.py".into()],
            git_diff_summary: Some("added CONCURRENTLY".into()),
            commit_messages: vec![],
            terminal_errors: vec![],
        };
        let mut statuses = Vec::new();
        let out = run_pipeline(
            &ctx,
            "ep_test",
            Arc::new(MockLlm),
            Arc::new(MockTts),
            Arc::new(MockBlob::default()),
            |s| statuses.push(s.to_string()),
        )
        .await
        .unwrap();
        assert!(out.audio_url.starts_with("mock://"));
        assert!(out.duration_seconds > 0);
        assert!(!out.script.segments.is_empty());
        assert_eq!(
            statuses,
            vec!["extracting_topics", "writing_script", "generating_audio"]
        );
    }
}
