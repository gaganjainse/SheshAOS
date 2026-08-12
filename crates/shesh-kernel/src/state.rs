use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::task::{TaskId, TaskRequest};

/// Roles a model can fill
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[non_exhaustive]
pub enum ModelRole {
    Planner,
    Coder,
    Vision,
    Reviewer,
}

/// Task lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TaskState {
    Received,
    Classified,
    Planned,
    AwaitingConfirmation,
    Executing,
    Blocked,
    Failed,
    RolledBack,
    Completed,
    Archived,
}

impl TaskState {
    /// Returns true if this state is terminal and cannot transition to non-terminal states.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Failed | Self::RolledBack | Self::Completed | Self::Archived)
    }

    /// Returns the valid states this state can transition into.
    pub fn valid_transitions(&self) -> Vec<TaskState> {
        match self {
            Self::Received => vec![Self::Classified],
            Self::Classified => vec![Self::Planned, Self::Failed],
            Self::Planned => vec![Self::AwaitingConfirmation, Self::Executing, Self::Failed],
            Self::AwaitingConfirmation => vec![Self::Executing, Self::Failed],
            Self::Executing => vec![Self::Completed, Self::Failed, Self::Blocked],
            Self::Blocked => vec![Self::Executing, Self::Failed],
            Self::Failed => vec![Self::RolledBack, Self::Archived],
            Self::Completed => vec![Self::Archived],
            Self::RolledBack => vec![Self::Archived],
            Self::Archived => vec![],
        }
    }

    /// Returns true if transitioning to `target` is valid from the current state.
    pub fn can_transition_to(&self, target: &TaskState) -> bool {
        self.valid_transitions().contains(target)
    }
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Received => "Received",
            Self::Classified => "Classified",
            Self::Planned => "Planned",
            Self::AwaitingConfirmation => "AwaitingConfirmation",
            Self::Executing => "Executing",
            Self::Blocked => "Blocked",
            Self::Failed => "Failed",
            Self::RolledBack => "RolledBack",
            Self::Completed => "Completed",
            Self::Archived => "Archived",
        };
        write!(f, "{}", name)
    }
}

/// Record of a task's current state with its full history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: TaskId,
    pub request: TaskRequest,
    pub current_state: TaskState,
    pub assigned_role: Option<ModelRole>,
    pub state_history: Vec<(TaskState, DateTime<Utc>)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        // Validate all allowed transitions according to the specification
        assert!(TaskState::Received.can_transition_to(&TaskState::Classified));

        assert!(TaskState::Classified.can_transition_to(&TaskState::Planned));
        assert!(TaskState::Classified.can_transition_to(&TaskState::Failed));

        assert!(TaskState::Planned.can_transition_to(&TaskState::AwaitingConfirmation));
        assert!(TaskState::Planned.can_transition_to(&TaskState::Executing));
        assert!(TaskState::Planned.can_transition_to(&TaskState::Failed));

        assert!(TaskState::AwaitingConfirmation.can_transition_to(&TaskState::Executing));
        assert!(TaskState::AwaitingConfirmation.can_transition_to(&TaskState::Failed));

        assert!(TaskState::Executing.can_transition_to(&TaskState::Completed));
        assert!(TaskState::Executing.can_transition_to(&TaskState::Failed));
        assert!(TaskState::Executing.can_transition_to(&TaskState::Blocked));

        assert!(TaskState::Blocked.can_transition_to(&TaskState::Executing));
        assert!(TaskState::Blocked.can_transition_to(&TaskState::Failed));

        assert!(TaskState::Failed.can_transition_to(&TaskState::RolledBack));
        assert!(TaskState::Failed.can_transition_to(&TaskState::Archived));

        assert!(TaskState::Completed.can_transition_to(&TaskState::Archived));

        assert!(TaskState::RolledBack.can_transition_to(&TaskState::Archived));
    }

    #[test]
    fn test_invalid_transitions() {
        // Assert some random invalid transitions to be robust
        assert!(!TaskState::Received.can_transition_to(&TaskState::Planned));
        assert!(!TaskState::Received.can_transition_to(&TaskState::Received)); // Self-transition not explicitly allowed

        assert!(!TaskState::Completed.can_transition_to(&TaskState::Executing));
        assert!(!TaskState::Archived.can_transition_to(&TaskState::Received));

        assert!(!TaskState::Failed.can_transition_to(&TaskState::Completed));

        // Archived can go nowhere
        assert!(TaskState::Archived.valid_transitions().is_empty());
    }

    #[test]
    fn test_is_terminal() {
        assert!(TaskState::Failed.is_terminal());
        assert!(TaskState::RolledBack.is_terminal());
        assert!(TaskState::Completed.is_terminal());
        assert!(TaskState::Archived.is_terminal());

        // Non-terminal
        assert!(!TaskState::Received.is_terminal());
        assert!(!TaskState::Classified.is_terminal());
        assert!(!TaskState::Planned.is_terminal());
        assert!(!TaskState::AwaitingConfirmation.is_terminal());
        assert!(!TaskState::Executing.is_terminal());
        assert!(!TaskState::Blocked.is_terminal());
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(TaskState::AwaitingConfirmation.to_string(), "AwaitingConfirmation");
        assert_eq!(TaskState::Executing.to_string(), "Executing");
        assert_eq!(TaskState::RolledBack.to_string(), "RolledBack");
    }

    #[test]
    fn test_model_role_serde() {
        let roles =
            vec![ModelRole::Planner, ModelRole::Coder, ModelRole::Vision, ModelRole::Reviewer];
        for role in roles {
            let json = serde_json::to_string(&role).unwrap();
            let back: ModelRole = serde_json::from_str(&json).unwrap();
            assert_eq!(role, back);
        }
    }

    #[test]
    fn test_task_state_serde() {
        let states = vec![
            TaskState::Received,
            TaskState::Classified,
            TaskState::Planned,
            TaskState::AwaitingConfirmation,
            TaskState::Executing,
            TaskState::Blocked,
            TaskState::Failed,
            TaskState::RolledBack,
            TaskState::Completed,
            TaskState::Archived,
        ];
        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let back: TaskState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }

    #[test]
    fn test_model_role_equality() {
        assert_eq!(ModelRole::Planner, ModelRole::Planner);
        assert_ne!(ModelRole::Planner, ModelRole::Coder);
    }

    #[test]
    fn test_task_state_equality() {
        assert_eq!(TaskState::Received, TaskState::Received);
        assert_ne!(TaskState::Received, TaskState::Classified);
    }

    #[test]
    fn test_all_valid_transitions_exhaustive() {
        let transitions = vec![
            (TaskState::Received, TaskState::Classified),
            (TaskState::Classified, TaskState::Planned),
            (TaskState::Classified, TaskState::Failed),
            (TaskState::Planned, TaskState::AwaitingConfirmation),
            (TaskState::Planned, TaskState::Executing),
            (TaskState::Planned, TaskState::Failed),
            (TaskState::AwaitingConfirmation, TaskState::Executing),
            (TaskState::AwaitingConfirmation, TaskState::Failed),
            (TaskState::Executing, TaskState::Completed),
            (TaskState::Executing, TaskState::Failed),
            (TaskState::Executing, TaskState::Blocked),
            (TaskState::Blocked, TaskState::Executing),
            (TaskState::Blocked, TaskState::Failed),
            (TaskState::Failed, TaskState::RolledBack),
            (TaskState::Failed, TaskState::Archived),
            (TaskState::Completed, TaskState::Archived),
            (TaskState::RolledBack, TaskState::Archived),
        ];
        for (from, to) in transitions {
            assert!(from.can_transition_to(&to), "{} -> {} should be valid", from, to);
        }
    }

    #[test]
    fn test_all_invalid_transitions_exhaustive() {
        let invalid = vec![
            (TaskState::Received, TaskState::Received),
            (TaskState::Received, TaskState::Planned),
            (TaskState::Received, TaskState::Failed),
            (TaskState::Classified, TaskState::Received),
            (TaskState::Classified, TaskState::Classified),
            (TaskState::Classified, TaskState::Completed),
            (TaskState::Planned, TaskState::Received),
            (TaskState::Planned, TaskState::Classified),
            (TaskState::AwaitingConfirmation, TaskState::Received),
            (TaskState::AwaitingConfirmation, TaskState::Planned),
            (TaskState::AwaitingConfirmation, TaskState::AwaitingConfirmation),
            (TaskState::Executing, TaskState::Received),
            (TaskState::Executing, TaskState::Classified),
            (TaskState::Executing, TaskState::Planned),
            (TaskState::Blocked, TaskState::Received),
            (TaskState::Blocked, TaskState::Classified),
            (TaskState::Blocked, TaskState::Planned),
            (TaskState::Blocked, TaskState::Blocked),
            (TaskState::Failed, TaskState::Received),
            (TaskState::Failed, TaskState::Classified),
            (TaskState::Failed, TaskState::Planned),
            (TaskState::Failed, TaskState::Failed),
            (TaskState::Failed, TaskState::Completed),
            (TaskState::Completed, TaskState::Received),
            (TaskState::Completed, TaskState::Classified),
            (TaskState::Completed, TaskState::Planned),
            (TaskState::Completed, TaskState::Completed),
            (TaskState::Completed, TaskState::Failed),
            (TaskState::Completed, TaskState::Executing),
            (TaskState::RolledBack, TaskState::Received),
            (TaskState::RolledBack, TaskState::Failed),
            (TaskState::RolledBack, TaskState::RolledBack),
            (TaskState::Archived, TaskState::Received),
            (TaskState::Archived, TaskState::Classified),
            (TaskState::Archived, TaskState::Planned),
            (TaskState::Archived, TaskState::Failed),
            (TaskState::Archived, TaskState::Completed),
            (TaskState::Archived, TaskState::RolledBack),
            (TaskState::Archived, TaskState::Archived),
        ];
        for (from, to) in invalid {
            assert!(!from.can_transition_to(&to), "{} -> {} should be invalid", from, to);
        }
    }

    #[test]
    fn test_task_record_construction() {
        use crate::task::{TaskInput, TaskRequest};
        let task_id = TaskId::new();
        let request = TaskRequest::new(TaskInput::Text("test".into()));
        let record = TaskRecord {
            task_id,
            request: request.clone(),
            current_state: TaskState::Received,
            assigned_role: None,
            state_history: vec![(TaskState::Received, Utc::now())],
        };
        assert_eq!(record.task_id, task_id);
        assert_eq!(record.current_state, TaskState::Received);
        assert!(record.assigned_role.is_none());
        assert_eq!(record.state_history.len(), 1);
    }

    #[test]
    fn test_task_record_serde() {
        use crate::task::{TaskInput, TaskRequest};
        let task_id = TaskId::new();
        let request = TaskRequest::new(TaskInput::Text("test".into()));
        let record = TaskRecord {
            task_id,
            request,
            current_state: TaskState::Classified,
            assigned_role: Some(ModelRole::Coder),
            state_history: vec![
                (TaskState::Received, Utc::now()),
                (TaskState::Classified, Utc::now()),
            ],
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: TaskRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.task_id, back.task_id);
        assert_eq!(record.current_state, back.current_state);
        assert_eq!(record.assigned_role, back.assigned_role);
    }

    #[test]
    fn test_can_transition_to_symmetric_check() {
        // If A -> B is valid, B -> A should not be (except for Blocked <-> Executing)
        let pairs = vec![
            (TaskState::Received, TaskState::Classified),
            (TaskState::Classified, TaskState::Planned),
            (TaskState::Planned, TaskState::Executing),
            (TaskState::Executing, TaskState::Completed),
            (TaskState::Failed, TaskState::Archived),
        ];
        for (from, to) in pairs {
            assert!(from.can_transition_to(&to));
            assert!(!to.can_transition_to(&from), "{} -> {} should be invalid (reverse)", to, from);
        }
    }

    #[test]
    fn test_blocked_executing_bidirectional() {
        assert!(TaskState::Blocked.can_transition_to(&TaskState::Executing));
        assert!(TaskState::Executing.can_transition_to(&TaskState::Blocked));
    }
}
