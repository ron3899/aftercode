use crate::db::queries;
use crate::pipeline::run_pipeline;
use crate::state::AppState;
use aftercode_core::session::SessionContext;
use uuid::Uuid;

/// Spawn generation for an already-inserted (queued) episode.
pub fn spawn(state: AppState, episode_id: Uuid, ctx: SessionContext) {
    tokio::spawn(async move {
        // Progress statuses flow through an ordered channel so the terminal
        // (ready/failed) write always lands last — fire-and-forget spawns
        // could otherwise clobber it.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let db_for_rx = state.db.clone();
        let drain = tokio::spawn(async move {
            while let Some(s) = rx.recv().await {
                let _ = queries::set_status(&db_for_rx, episode_id, &s).await;
            }
        });

        let result = run_pipeline(
            &ctx,
            &episode_id.to_string(),
            state.llm.clone(),
            state.tts.clone(),
            state.blob.clone(),
            |s| {
                let _ = tx.send(s.to_string());
            },
        )
        .await;

        // Closing the sender ends the drain task once all progress writes complete.
        drop(tx);
        let _ = drain.await;

        match result {
            Ok(out) => {
                let topics = serde_json::to_value(&out.topics).unwrap_or_default();
                let script = serde_json::to_value(&out.script).unwrap_or_default();
                let _ = sqlx::query(
                    "UPDATE podcast_episodes SET status='ready'::episode_status, title=$1,
                     audio_url=$2, duration_seconds=$3, topics_json=$4, script_json=$5,
                     transcript_text=$6, summary=$7, updated_at=now() WHERE id=$8",
                )
                .bind(&out.script.title)
                .bind(&out.audio_url)
                .bind(out.duration_seconds)
                .bind(topics)
                .bind(script)
                .bind(&out.transcript)
                .bind(out.script.summary_points.join(" "))
                .bind(episode_id)
                .execute(&state.db)
                .await;
            }
            Err(e) => {
                let _ = sqlx::query(
                    "UPDATE podcast_episodes SET status='failed'::episode_status, error=$1,
                     updated_at=now() WHERE id=$2",
                )
                .bind(e.to_string())
                .bind(episode_id)
                .execute(&state.db)
                .await;
            }
        }
    });
}
