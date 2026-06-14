use aftercode_core::episode::{EpisodeScript, LearningTopic};
use aftercode_core::session::Language;
use async_trait::async_trait;

pub struct NormalizedContext {
    pub text: String,
    pub language: Language,
    pub minutes: u8,
}

pub struct ScriptOpts {
    pub language: Language,
    pub minutes: u8,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn extract_topics(&self, ctx: &NormalizedContext) -> anyhow::Result<Vec<LearningTopic>>;
    async fn write_script(
        &self,
        topics: &[LearningTopic],
        opts: &ScriptOpts,
    ) -> anyhow::Result<EpisodeScript>;
}

pub struct MockLlm;

#[async_trait]
impl LlmProvider for MockLlm {
    async fn extract_topics(&self, _ctx: &NormalizedContext) -> anyhow::Result<Vec<LearningTopic>> {
        Ok(vec![LearningTopic {
            title: "Production-safe Postgres indexes".into(),
            summary: "Index built concurrently to avoid locking.".into(),
            evidence: vec!["postgresql_concurrently=True".into()],
            knowledge_gap: "Why CONCURRENTLY can't run in a txn.".into(),
            difficulty: "intermediate".into(),
            priority: "high".into(),
        }])
    }
    async fn write_script(
        &self,
        topics: &[LearningTopic],
        opts: &ScriptOpts,
    ) -> anyhow::Result<EpisodeScript> {
        use aftercode_core::episode::{Quiz, ScriptSegment, Speaker};
        Ok(EpisodeScript {
            title: format!("Why your migration matters: {}", topics[0].title),
            language: opts.language,
            segments: vec![
                ScriptSegment {
                    speaker: Speaker::Host,
                    text: "Today we unpack your session.".into(),
                },
                ScriptSegment {
                    speaker: Speaker::Expert,
                    text: "Index creation can lock tables.".into(),
                },
            ],
            summary_points: vec!["CONCURRENTLY reduces locking.".into()],
            quiz: Some(Quiz {
                question: "Why?".into(),
                answer: "Outside a transaction.".into(),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mock_llm_produces_topic_and_script() {
        let m = MockLlm;
        let ctx = NormalizedContext {
            text: "x".into(),
            language: Language::En,
            minutes: 10,
        };
        let topics = m.extract_topics(&ctx).await.unwrap();
        assert_eq!(topics.len(), 1);
        let script = m
            .write_script(
                &topics,
                &ScriptOpts {
                    language: Language::En,
                    minutes: 10,
                },
            )
            .await
            .unwrap();
        assert_eq!(script.segments.len(), 2);
    }
}
