use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use a4n_service_protocol::{
    ArtifactEvent, ArtifactOperation, CancelRequestedEvent, DEFAULT_GENERATION_ACTIVE_SESSION_MAX,
    GENERATION_EVENT_SCHEMA_VERSION, GenerationEvent, GenerationTaskEvent, GenerationTaskSnapshot,
    GenerationTaskStatus, GenerationUsage, JS_SAFE_INTEGER_I64, JS_SAFE_INTEGER_U64,
    MAX_GENERATION_EVENT_BYTES, ProgressEvent, TextDeltaEvent, TextStreamSnapshot, UsageEvent,
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex as AsyncMutex, Notify, watch};

mod model;
mod publisher;
mod registry;
mod replay;
mod session;

#[cfg(test)]
mod tests;

pub use model::GenerationSessionError;
pub use replay::{GenerationReplay, GenerationSubscription};

pub(crate) use model::{
    GenerationDurableCommit, GenerationSequenceLeaseUpdate, SessionCheckpointCandidate,
};
pub(crate) use publisher::{
    CommittedEventPermit, GenerationEventPublisher, GenerationSequenceLeaseReplenisher,
    SessionGenerationEventPublisher, apply_committed_envelope_to_snapshot,
};
pub(crate) use registry::{
    GenerationSessionConfig, GenerationSessionRegistry, SessionAdmissionPermit,
};
pub(crate) use replay::now_ms;
pub(crate) use session::GenerationTaskSession;

use publisher::{ApplyMode, OrdinaryAction, OrdinaryInput, append_ordinary_locked, apply_event};
use registry::{
    HostSubscriptionPermit, MemoryReservation, SEQUENCE_EMERGENCY_SLOTS, SEQUENCE_LEASE_BLOCK,
    SessionCapacityLease,
};
use replay::{
    duration_ms_i64, encoded_event_bytes, replay_after_locked, replay_chunk_indexes,
    session_memory_bytes, trim_replay, validate_checkpoint_revision, validate_durable_commit,
    validate_persisted_at_ms, validate_replay,
};
use session::SessionState;
