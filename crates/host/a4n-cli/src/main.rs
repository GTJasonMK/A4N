mod client;
mod config;
mod context;
mod local;
mod output;
mod page;
mod routing;
mod shell;
mod sse;
mod transport;

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use a4n_api_contract::agent::{
    AgentApprovalDecision, AgentBudget, AgentControlAction, AgentDeliveryMode, AgentExecutionMode,
    AgentMemoryInput, AgentMemoryScopeKind, AgentResourceRef, AgentScope, AgentThreadKind,
    ArchiveAgentThreadInput, ControlAgentRunInput, ControlChildAgentInput, CreateAgentThreadInput,
    EnsureMainAgentThreadInput, ForkAgentThreadInput, ResolveAgentApprovalInput,
    RetractAgentMemoryInput, SendAgentMessageInput, SendAgentMessageToChildInput, SpawnAgentInput,
    UpdateAgentThreadInput,
};
use a4n_api_contract::auth::{ChangePasswordInput, RevokeSessionInput};
use a4n_api_contract::command_manifest as catalog;
use a4n_api_contract::package::PackageId;
use a4n_api_contract::project::CreateProjectInput;
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use reqwest::{Method, Response};
use serde_json::{Map, Value, json};
use tokio::io::AsyncWriteExt;

use client::{AgentHeartbeatClient, ApiBody, ApiClient, JsonRequest};
use config::{StoredConfig, resolve_config_path};
use context::SessionContext;
use output::{OutputMode, OutputOptions};

include!("cli/command_definitions.rs");
include!("cli/command_labels.rs");
include!("cli/bootstrap.rs");
include!("cli/handlers/local.rs");
include!("cli/handlers/auth.rs");
include!("cli/handlers/task.rs");
include!("cli/handlers/agent.rs");
include!("cli/handlers/project.rs");
include!("cli/handlers/workflow.rs");
include!("cli/handlers/media.rs");
include!("cli/io.rs");
include!("cli/tests.rs");
