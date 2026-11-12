//! POL-01 spike: one request model, one oracle, several engines.
//!
//! The application layer builds a minimal, complete [`Request`]; engines never fetch data (report §4.5).
//! Every error path is a `deny` (AGENTS.md). Throwaway code: nothing here is a decision (ADR-005 is pending).

pub mod cedar;
pub mod oracle;

use regia_domain::{CircleId, PrincipalId, Role};

/// What the principal wants to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Read content.
    Read,
    /// Learn that the object exists (title, count, snippet) without reading it.
    Discover,
    /// Create an object.
    Create,
    /// Change an object.
    Update,
    /// Delete an object.
    Delete,
    /// Export content out of the system.
    Export,
    /// Share with others.
    Share,
    /// Execute a tool or workflow step.
    Execute,
}

impl Action {
    /// All actions, for exhaustive tests.
    pub const ALL: [Self; 8] = [
        Self::Read,
        Self::Discover,
        Self::Create,
        Self::Update,
        Self::Delete,
        Self::Export,
        Self::Share,
        Self::Execute,
    ];

    /// Stable wire name used in policies.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Discover => "discover",
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Export => "export",
            Self::Share => "share",
            Self::Execute => "execute",
        }
    }

    /// Everything except read and discover has an effect and needs the mutation guard.
    #[must_use]
    pub const fn is_mutating(self) -> bool {
        !matches!(self, Self::Read | Self::Discover)
    }
}

/// A membership as the policy sees it. A revoked membership never grants anything (P-09).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The circle.
    pub circle: CircleId,
    /// Role held.
    pub role: Role,
    /// Revoked memberships deny every action on that circle.
    pub revoked: bool,
}

/// Who asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// Identity.
    pub id: PrincipalId,
    /// Memberships known to the caller.
    pub memberships: Vec<Member>,
}

/// What is asked about. `label` and `category` are raw text on purpose: the policy must deny on a missing
/// or unknown value rather than trust the caller to have parsed it (P-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    /// Owning circle.
    pub circle: CircleId,
    /// Enclosing circles, nearest first. Supplied by the caller; the engine does not walk the graph.
    pub ancestors: Vec<CircleId>,
    /// Confidentiality label name; `None` is a missing attribute.
    pub label: Option<String>,
    /// The person who owns the object.
    pub owner: Option<PrincipalId>,
    /// Explicitly named principals (for `restricted`).
    pub grantees: Vec<PrincipalId>,
    /// Content category, relevant to pilot exclusions (owner answer Q9).
    pub category: Option<String>,
}

/// Where the request came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A person acting directly.
    User,
    /// Derived from imported or model-produced content (prompt-injection surface, T-06).
    Untrusted,
}

/// Request context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    /// Provenance of the request.
    pub origin: Origin,
    /// A human approved this exact action.
    pub approved: bool,
}

/// Everything an engine may look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Who.
    pub principal: Principal,
    /// What.
    pub action: Action,
    /// On which object.
    pub resource: Resource,
    /// Under which circumstances.
    pub context: Context,
}

/// Categories refused at creation during the pilot (owner answer Q9).
pub const EXCLUDED_CATEGORIES: [&str; 5] =
    ["health", "minors", "finance", "credentials", "id_document"];

/// The outcome, bound to the policy version that produced it (P-08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// `true` only when a rule explicitly allowed.
    pub allow: bool,
    /// Names of the rules that decided; never empty (explainability).
    pub reasons: Vec<String>,
    /// Version of the policy set that was evaluated.
    pub policy_version: String,
}

impl Decision {
    /// A deny with a reason. Used for every error path.
    #[must_use]
    pub fn deny(reason: impl Into<String>, policy_version: impl Into<String>) -> Self {
        Self {
            allow: false,
            reasons: vec![reason.into()],
            policy_version: policy_version.into(),
        }
    }
}

/// An authorisation engine under test.
pub trait Engine {
    /// Engine name for reports.
    fn name(&self) -> &'static str;
    /// Decide. Implementations map every internal error to a deny.
    fn decide(&self, request: &Request) -> Decision;
}

/// The engines under comparison, oracle first.
///
/// # Panics
/// If a bundled policy set is invalid; that is a build defect, never a runtime fallback.
#[must_use]
#[allow(clippy::expect_used)] // a bundled policy set that fails validation is a build defect
pub fn engines() -> Vec<Box<dyn Engine>> {
    vec![
        Box::new(oracle::Oracle::new("v1")),
        Box::new(cedar::CedarEngine::new("v1").expect("bundled Cedar policies are valid")),
    ]
}
