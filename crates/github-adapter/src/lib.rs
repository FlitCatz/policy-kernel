#![forbid(unsafe_code)]

mod gh;
mod parse;
mod process;

pub use gh::{GhCliApi, GitHubApi};
pub use parse::{
    parse_agent_ready, parse_merge, parse_pull_request, parse_trusted_check, parse_work_item,
    publish_pre_merge_check, serialize_check_binding, validate_merge_protection, validate_workflow,
    AgentReadyExpectation, CheckBinding, CheckExpectation, LabelReceipt, PublicationReceipt,
    PublishedCheck, TrustedCheck, WorkItemExpectation, WorkflowExpectation, WorkflowIdentity,
};

use policy_contracts::IdentityError;
use std::{io, time::Duration};

pub const CHECK_NAME: &str = "policy-kernel/pre-merge";
pub const MAX_CHECK_BINDING_BYTES: usize = 4_096;

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("invalid GitHub route segment: {0}")]
    InvalidRouteSegment(String),
    #[error("GitHub observation binding failed: {0}")]
    Binding(String),
    #[error("serialized payload is {actual} bytes; limit is {limit} bytes")]
    PayloadTooLarge { actual: usize, limit: usize },
    #[error("GitHub CLI output exceeded {limit} bytes")]
    OutputTooLarge { limit: usize },
    #[error("GitHub CLI timed out after {0:?}")]
    Timeout(Duration),
    #[error("GitHub CLI failed with status {status:?}: {stderr}")]
    ProcessFailure { status: Option<i32>, stderr: String },
    #[error("GitHub CLI I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid identity: {0}")]
    Identity(#[from] IdentityError),
}

pub(crate) fn binding_error(message: impl Into<String>) -> AdapterError {
    AdapterError::Binding(message.into())
}
