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
}
