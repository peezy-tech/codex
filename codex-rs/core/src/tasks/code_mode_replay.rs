use std::sync::Arc;

use codex_git_utils::get_git_repo_root;
use codex_protocol::models::ContentItem;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::AgentMessageEvent;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::TurnStartedEvent;
use codex_protocol::user_input::UserInput;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::state::TaskKind;
use crate::tools::code_mode::execute_code_mode_source;
use crate::turn_diff_tracker::TurnDiffTracker;

use super::SessionTask;
use super::SessionTaskContext;

#[derive(Clone)]
pub(crate) struct CodeModeReplayTask {
    source: String,
}

impl CodeModeReplayTask {
    pub(crate) fn new(source: String) -> Self {
        Self { source }
    }
}

impl SessionTask for CodeModeReplayTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }

    fn span_name(&self) -> &'static str {
        "session_task.code_mode_replay"
    }

    async fn run(
        self: Arc<Self>,
        session: Arc<SessionTaskContext>,
        turn_context: Arc<TurnContext>,
        _input: Vec<UserInput>,
        _cancellation_token: CancellationToken,
    ) -> Option<String> {
        execute_code_mode_replay(session.clone_session(), turn_context, self.source.clone()).await
    }
}

async fn execute_code_mode_replay(
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    source: String,
) -> Option<String> {
    session
        .send_event(
            turn_context.as_ref(),
            EventMsg::TurnStarted(TurnStartedEvent {
                turn_id: turn_context.sub_id.clone(),
                started_at: turn_context.turn_timing_state.started_at_unix_secs().await,
                model_context_window: turn_context.model_context_window(),
                collaboration_mode_kind: turn_context.collaboration_mode.mode,
            }),
        )
        .await;

    let display_root = get_git_repo_root(turn_context.cwd.as_path())
        .unwrap_or_else(|| turn_context.cwd.clone().into_path_buf());
    let tracker = Arc::new(Mutex::new(TurnDiffTracker::with_display_root(display_root)));
    let call_id = format!("code-mode-replay-{}", Uuid::new_v4());
    let output_text = match execute_code_mode_source(
        Arc::clone(&session),
        Arc::clone(&turn_context),
        tracker,
        call_id,
        source,
    )
    .await
    {
        Ok(output) => output.into_text(),
        Err(error) => format!("Code Mode replay failed:\n{error}"),
    };
    let output_text = if output_text.trim().is_empty() {
        "Code Mode replay completed with no text output.".to_string()
    } else {
        output_text
    };

    let response_item = ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: output_text.clone(),
        }],
        phase: Some(MessagePhase::FinalAnswer),
    };
    session
        .record_conversation_items(turn_context.as_ref(), std::slice::from_ref(&response_item))
        .await;
    session
        .send_event(
            turn_context.as_ref(),
            EventMsg::AgentMessage(AgentMessageEvent {
                message: output_text.clone(),
                phase: Some(MessagePhase::FinalAnswer),
                memory_citation: None,
            }),
        )
        .await;
    session.ensure_rollout_materialized().await;

    Some(output_text)
}
