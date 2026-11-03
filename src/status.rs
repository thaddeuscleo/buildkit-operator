use k8s_openapi::{
    api::apps::v1::Deployment,
    apimachinery::pkg::apis::meta::v1::{Condition, Time},
};

use crate::crd::buildkit::{BuildKit, BuildKitStatus};

pub fn make_condition(
    previous: Option<&Condition>,
    condition_type: &str,
    status: &str,
    reason: &str,
    message: &str,
    generation: i64,
) -> Condition {
    let last_transition_time = match previous {
        Some(old) if old.status == status => old.last_transition_time.clone(),
        _ => Time(jiff::Timestamp::now()),
    };

    Condition {
        type_: condition_type.to_string(),
        status: status.to_string(),
        reason: reason.to_string(),
        message: message.to_string(),
        observed_generation: Some(generation),
        last_transition_time,
    }
}

pub fn build_status(buildkit: &BuildKit, deployment: &Deployment) -> BuildKitStatus {
    let generation = buildkit.metadata.generation.unwrap_or(0);
    let desired = buildkit.spec.replicas;
    let deployment_status = deployment.status.as_ref();

    let ready = deployment_status
        .and_then(|s| s.ready_replicas)
        .unwrap_or(0);

    let updated = deployment_status
        .and_then(|s| s.updated_replicas)
        .unwrap_or(0);

    let available = deployment_status
        .and_then(|s| s.available_replicas)
        .unwrap_or(0);

    let observed = deployment_status
        .and_then(|s| s.observed_generation)
        .unwrap_or(0);

    let deployment_generation = deployment.metadata.generation.unwrap_or(0);

    let rollout_current = observed >= deployment_generation;

    let failed = deployment_status
        .and_then(|s| s.conditions.as_ref())
        .is_some_and(|conditions| {
            conditions.iter().any(|condition| {
                condition.type_ == Some("Progressing".to_string())
                    && condition.status == Some("False".to_string())
                    && condition.reason.as_deref() == Some("ProgressDeadlineExceeded")
            })
        });

    let is_ready = !failed
        && rollout_current
        && ready == desired
        && updated == desired
        && available == desired;

    let is_progressing = !is_ready && !failed;

    let previous = buildkit.status.as_ref();

    let find_previous =
        |kind: &str| previous.and_then(|s| s.conditions.iter().find(|c| c.type_ == kind));

    let (ready_reason, ready_message) = if is_ready {
        ("DeploymentReady", "All BuildKit replicas are ready")
    } else if failed {
        (
            "DeploymentFailed",
            "Deployment exceeded its progress deadline",
        )
    } else {
        ("DeploymentNotReady", "Waiting for BuildKit replicas")
    };

    let ready_condition = make_condition(
        find_previous("Ready"),
        "Ready",
        if is_ready { "True" } else { "False" },
        ready_reason,
        ready_message,
        generation,
    );

    let progressing_condition = make_condition(
        find_previous("Progressing"),
        "Progressing",
        if is_progressing { "True" } else { "False" },
        if failed {
            "ProgressDeadlineExceeded"
        } else if is_ready {
            "RolloutComplete"
        } else {
            "RolloutInProgress"
        },
        if failed {
            "Deployment rollout failed"
        } else if is_ready {
            "Deployment rollout completed"
        } else {
            "Waiting for Deployment rollout"
        },
        generation,
    );

    BuildKitStatus {
        ready_replicas: ready,
        observed_generation: generation,
        conditions: vec![ready_condition, progressing_condition],
    }
}
