//! `cargo run -p regia-domain --features synthetic --example act -- --store target/demo-store.jsonl --circle <uuid-v7> --audit-store target/demo-audit.jsonl --q anna`
//!
//! Drives the WF-03 reference state machine from a captured task to a content-free audit
//! record. It is a demo of the domain contract, not a policy engine, a workflow runtime or a
//! storage decision: hashes and the policy decision are supplied by the caller, and no
//! cryptographic primitive is implemented here (ADR-007 remains proposed).

use std::{
    env,
    fmt::Write as _,
    fs::{self, OpenOptions},
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::ExitCode,
};

use regia_domain::{
    Approval, AuditDecision, AuditEvent, AuditRecord, AuditResult, CircleId, Envelope, Object,
    ObjectId, PolicyContextHash, PolicyVersion, PrincipalId, ProposalHash, ReadDecision, ReadView,
    Reversibility, RiskClass, SourceRef, Task, Timestamp, TransitionContext, Withheld, Workflow,
    WorkflowEvent, WorkflowState,
    synthetic::{ActionDraft, propose_action_for_task},
};
use time::{Duration, OffsetDateTime};

/// A declared synthetic principal: stable across runs so the audit can attribute an action.
const DEMO_PRINCIPAL: &str = "01a21ee0-d408-7000-8000-000000000002";
const DEFAULT_POLICY_HASH: &str = "demo-policy-v1";
const DEFAULT_PROPOSAL_HASH: &str = "demo-proposal-v1";

/// The caller-supplied policy decision. There is no implicit allow.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PolicyChoice {
    Allow,
    Deny,
}

struct Args {
    store: PathBuf,
    audit_store: PathBuf,
    circle: CircleId,
    object: Option<ObjectId>,
    needle: Option<String>,
    policy: PolicyChoice,
    principal: PrincipalId,
    risk_class: RiskClass,
    reversibility: Reversibility,
    policy_version: String,
    policy_hash: String,
    proposal_hash: String,
    idempotency_key: String,
    action_type: String,
}

fn parse_policy(value: &str) -> Result<PolicyChoice, String> {
    match value {
        "allow" => Ok(PolicyChoice::Allow),
        "deny" => Ok(PolicyChoice::Deny),
        other => Err(format!("--policy must be allow or deny, got {other}")),
    }
}

fn parse_risk_class(value: &str) -> Result<RiskClass, String> {
    match value {
        "low" => Ok(RiskClass::Low),
        "elevated" => Ok(RiskClass::Elevated),
        "critical" => Ok(RiskClass::Critical),
        other => Err(format!(
            "--risk-class must be low|elevated|critical, got {other}"
        )),
    }
}

fn parse_reversibility(value: &str) -> Result<Reversibility, String> {
    match value {
        "cancellable" => Ok(Reversibility::Cancellable),
        "compensable" => Ok(Reversibility::Compensable),
        "irreversible" => Ok(Reversibility::Irreversible),
        other => Err(format!(
            "--reversibility must be cancellable|compensable|irreversible, got {other}"
        )),
    }
}

fn parse_args() -> Result<Args, String> {
    let mut store = None;
    let mut audit_store = None;
    let mut circle = None;
    let mut object = None;
    let mut needle = None;
    let mut policy = PolicyChoice::Deny;
    let mut principal = DEMO_PRINCIPAL
        .parse::<PrincipalId>()
        .map_err(|e| format!("invalid demo principal: {e}"))?;
    let mut risk_class = RiskClass::Low;
    let mut reversibility = Reversibility::Cancellable;
    let mut policy_version = "demo-policy-v1".to_owned();
    let mut policy_hash = DEFAULT_POLICY_HASH.to_owned();
    let mut proposal_hash = DEFAULT_PROPOSAL_HASH.to_owned();
    let mut idempotency_key = "demo-idem-v1".to_owned();
    let mut action_type = "create_task".to_owned();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--store" => store = Some(PathBuf::from(value("--store")?)),
            "--audit-store" => audit_store = Some(PathBuf::from(value("--audit-store")?)),
            "--circle" => {
                let raw = value("--circle")?;
                circle = Some(
                    raw.parse::<CircleId>()
                        .map_err(|e| format!("--circle: {e}"))?,
                );
            }
            "--object" => {
                let raw = value("--object")?;
                object = Some(
                    raw.parse::<ObjectId>()
                        .map_err(|e| format!("--object: {e}"))?,
                );
            }
            "--q" => needle = Some(value("--q")?),
            "--policy" => policy = parse_policy(&value("--policy")?)?,
            "--principal" => {
                let raw = value("--principal")?;
                principal = raw
                    .parse::<PrincipalId>()
                    .map_err(|e| format!("--principal: {e}"))?;
            }
            "--risk-class" => risk_class = parse_risk_class(&value("--risk-class")?)?,
            "--reversibility" => reversibility = parse_reversibility(&value("--reversibility")?)?,
            "--policy-version" => policy_version = value("--policy-version")?,
            "--policy-hash" => policy_hash = value("--policy-hash")?,
            "--proposal-hash" => proposal_hash = value("--proposal-hash")?,
            "--idempotency-key" => idempotency_key = value("--idempotency-key")?,
            "--action-type" => action_type = value("--action-type")?,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let store = store.ok_or("--store is required")?;
    let audit_store = audit_store.ok_or("--audit-store is required")?;
    if store == audit_store {
        return Err("--store and --audit-store must differ".to_owned());
    }
    // Fail closed: without a circle nothing is read from the store.
    let circle = circle.ok_or("--circle is required")?;
    if object.is_none() && needle.is_none() {
        return Err("--object or --q is required".to_owned());
    }
    Ok(Args {
        store,
        audit_store,
        circle,
        object,
        needle,
        policy,
        principal,
        risk_class,
        reversibility,
        policy_version,
        policy_hash,
        proposal_hash,
        idempotency_key,
        action_type,
    })
}

/// The one selected task and the counts of refused/withheld lines.
struct Selection {
    task: Task,
    refused: usize,
    withheld_label: usize,
    withheld_circle: usize,
}

/// Selects exactly one visible task; an ambiguous match is a refusal.
fn select_task(args: &Args) -> Result<Selection, String> {
    let jsonl = fs::read_to_string(&args.store)
        .map_err(|e| format!("cannot read {}: {e}", args.store.display()))?;
    let view = ReadView::owner(args.circle);
    let needle_lower = args.needle.as_ref().map(|n| n.to_lowercase());
    let mut refused = 0usize;
    let mut withheld_label = 0usize;
    let mut withheld_circle = 0usize;
    let mut candidates: Vec<Task> = Vec::new();

    for line in jsonl.lines() {
        let envelope: Envelope = if let Ok(e) = serde_json::from_str(line) {
            e
        } else {
            refused += 1;
            continue;
        };
        match view.evaluate(&envelope) {
            ReadDecision::Withheld(Withheld::Label) => {
                withheld_label += 1;
                continue;
            }
            ReadDecision::Withheld(Withheld::Circle) => {
                withheld_circle += 1;
                continue;
            }
            ReadDecision::Show => {}
        }
        let Object::Task(task) = envelope.object() else {
            continue;
        };
        let selected = match (&args.object, &needle_lower) {
            (Some(id), _) => task.header.id == *id,
            (None, Some(needle)) => task.title.value.to_lowercase().contains(needle),
            (None, None) => false,
        };
        if selected {
            candidates.push(task.clone());
        }
    }
    if candidates.is_empty() {
        return Err(format!(
            "no visible task selected; refused lines: {refused}, withheld: {} (label: {withheld_label}, circle: {withheld_circle})",
            withheld_label + withheld_circle
        ));
    }
    if candidates.len() != 1 {
        return Err(format!(
            "expected exactly one visible task, found {}; use --object to disambiguate",
            candidates.len()
        ));
    }
    Ok(Selection {
        task: candidates
            .pop()
            .ok_or_else(|| "no visible task selected".to_owned())?,
        refused,
        withheld_label,
        withheld_circle,
    })
}

/// Advances the reference state machine; a denial is never mapped to an allow.
fn drive(
    workflow: &mut Workflow,
    ctx: &TransitionContext<'_>,
    policy: PolicyChoice,
    approver: PrincipalId,
    proposal_hash: &ProposalHash,
    target_circle: CircleId,
    output: &mut String,
) -> Result<(AuditDecision, AuditResult), String> {
    macro_rules! step {
        ($event:expr) => {{
            let event = $event;
            workflow
                .transition(&event, ctx)
                .map_err(|e| format!("transition refused: {e}"))?;
            let _ = writeln!(output, "event: {event:?}  state: {:?}", workflow.state());
        }};
    }

    step!(WorkflowEvent::ContextResolved);
    step!(WorkflowEvent::PlanDrafted);
    step!(WorkflowEvent::SchemaValidated);
    step!(WorkflowEvent::PolicyEvaluated);

    match policy {
        PolicyChoice::Deny => {
            step!(WorkflowEvent::Denied);
            Ok((AuditDecision::Deny, AuditResult::NotAttempted))
        }
        PolicyChoice::Allow => {
            step!(WorkflowEvent::ApprovalRequired);
            print!("approve this action? [y/N] ");
            io::stdout()
                .flush()
                .map_err(|e| format!("cannot flush stdout: {e}"))?;
            let mut answer = String::new();
            io::stdin()
                .read_line(&mut answer)
                .map_err(|e| format!("cannot read approval: {e}"))?;
            if answer.trim().eq_ignore_ascii_case("y") {
                let approval = Approval {
                    approver,
                    at: ctx.now,
                    expires_at: Timestamp(ctx.now.0 + Duration::minutes(5)),
                    proposal_hash: proposal_hash.clone(),
                    target_circle,
                };
                step!(WorkflowEvent::Approved { approval });
                step!(WorkflowEvent::ExecutionStarted);
                step!(WorkflowEvent::Succeeded);
                step!(WorkflowEvent::Reconciled);
                step!(WorkflowEvent::Completed);
                Ok((AuditDecision::Allow, AuditResult::Succeeded))
            } else {
                let _ = writeln!(output, "approval: withheld");
                Ok((AuditDecision::Deny, AuditResult::NotAttempted))
            }
        }
    }
}

/// Everything the workflow needs, with the caller-supplied hashes kept opaque.
struct Prepared {
    workflow: Workflow,
    requester: PrincipalId,
    proposal_hash: ProposalHash,
    policy_hash: PolicyContextHash,
    action_id: ObjectId,
    target_circle: CircleId,
    source_refs: Vec<SourceRef>,
    policy_version: PolicyVersion,
}

fn prepare(task: &Task, args: &Args, now: Timestamp) -> Result<Prepared, String> {
    let requester = args.principal;
    let expires_at = Timestamp(now.0 + Duration::minutes(10));
    let proposal = propose_action_for_task(
        task,
        requester,
        ActionDraft {
            action_type: &args.action_type,
            policy_context_hash: &args.policy_hash,
            idempotency_key: &args.idempotency_key,
            risk_class: args.risk_class,
            reversibility: args.reversibility,
            expires_at,
        },
    )
    .map_err(|e| format!("cannot propose action: {e}"))?;
    let action_id = proposal.id;
    let target_circle = proposal.target_circle;
    let source_refs = proposal.source_refs.clone();
    let policy_hash = PolicyContextHash::new(&args.policy_hash).map_err(|e| e.to_string())?;
    let proposal_hash = ProposalHash::new(&args.proposal_hash).map_err(|e| e.to_string())?;
    let policy_version = PolicyVersion::new(&args.policy_version).map_err(|e| e.to_string())?;
    Ok(Prepared {
        workflow: Workflow::new(proposal),
        requester,
        proposal_hash,
        policy_hash,
        action_id,
        target_circle,
        source_refs,
        policy_version,
    })
}

fn append_audit(path: &Path, line: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    writeln!(file, "{line}").map_err(|e| format!("cannot append audit: {e}"))
}

/// The facts a caller needs to render, written to the audit store without content.
struct Outcome {
    now: Timestamp,
    requester: PrincipalId,
    action_id: ObjectId,
    target_circle: CircleId,
    decision: AuditDecision,
    result: AuditResult,
    policy_version: PolicyVersion,
    source_refs: Vec<SourceRef>,
    state: WorkflowState,
    refused: usize,
    withheld_label: usize,
    withheld_circle: usize,
}

fn finish(args: &Args, outcome: Outcome) -> Result<(), String> {
    let Outcome {
        now,
        requester,
        action_id,
        target_circle,
        decision,
        result,
        policy_version,
        source_refs,
        state,
        refused,
        withheld_label,
        withheld_circle,
    } = outcome;
    let audit = AuditEvent::new(AuditRecord {
        at: now,
        actor: requester,
        action: action_id,
        decision,
        policy_version,
        result,
        source_refs,
    });
    let audit_line =
        serde_json::to_string(&audit).map_err(|e| format!("serialisation failed: {e}"))?;
    let mut output = String::new();
    let _ = writeln!(output, "id: {action_id}");
    let _ = writeln!(output, "circle: {target_circle}");
    let _ = writeln!(output, "principal: {}", args.principal);
    let _ = writeln!(output, "state: {state:?}");
    let _ = writeln!(output, "decision: {decision:?}  result: {result:?}");
    let _ = writeln!(output, "sources: {}", audit.source_refs().len());
    let _ = writeln!(
        output,
        "refused lines: {refused}  withheld: {} (label: {withheld_label}, circle: {withheld_circle})",
        withheld_label + withheld_circle
    );
    let _ = writeln!(output, "audit: {}", args.audit_store.display());
    if args.policy == PolicyChoice::Allow
        && (args.policy_hash == DEFAULT_POLICY_HASH || args.proposal_hash == DEFAULT_PROPOSAL_HASH)
    {
        let _ = writeln!(
            output,
            "warning: demo hash defaults in use; approval binding is not meaningful"
        );
    }

    // Last line of defence: never write or print content, even if a future change leaks it.
    if audit_line.contains("CANARY-") || output.contains("CANARY-") {
        return Err("canary found; refusing to write or print".to_owned());
    }
    append_audit(&args.audit_store, &audit_line)?;
    print!("{output}");
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("act: {e}");
            eprintln!(
                "usage: act --store <file.jsonl> --audit-store <file.jsonl> --circle <uuid-v7> (--object <id> | --q <text>) [--policy allow|deny] [--principal <uuid-v7>] [--risk-class low|elevated|critical] [--reversibility cancellable|compensable|irreversible] [--policy-version <text>] [--policy-hash <text>] [--proposal-hash <text>] [--idempotency-key <text>] [--action-type <text>]"
            );
            return ExitCode::from(2);
        }
    };

    let now = Timestamp(OffsetDateTime::now_utc());
    let Selection {
        task,
        refused,
        withheld_label,
        withheld_circle,
    } = match select_task(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("act: {e}");
            return ExitCode::FAILURE;
        }
    };

    let Prepared {
        mut workflow,
        requester,
        proposal_hash,
        policy_hash,
        action_id,
        target_circle,
        source_refs,
        policy_version,
    } = match prepare(&task, &args, now) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("act: {e}");
            return ExitCode::FAILURE;
        }
    };
    let ctx = TransitionContext {
        now,
        proposal_hash: &proposal_hash,
        policy_context_hash: &policy_hash,
    };
    let mut output = String::new();
    let (decision, result) = match drive(
        &mut workflow,
        &ctx,
        args.policy,
        args.principal,
        &proposal_hash,
        target_circle,
        &mut output,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("act: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = finish(
        &args,
        Outcome {
            now,
            requester,
            action_id,
            target_circle,
            decision,
            result,
            policy_version,
            source_refs,
            state: workflow.state(),
            refused,
            withheld_label,
            withheld_circle,
        },
    ) {
        eprintln!("act: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
