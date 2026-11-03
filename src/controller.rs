use futures::StreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    Api, Client, ResourceExt,
    api::{Patch, PatchParams},
    runtime::{Controller, controller::Action, watcher::Config},
};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info, instrument};

use crate::crd::buildkit::BuildKit;

const FIELD_MANAGER: &str = "buildkit-operator";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kubernetes API error: {0}")]
    Kube(#[from] kube::Error),
    #[error("BuildKit is missing metadata.{0}")]
    MissingField(&'static str),
}

pub struct Context {
    pub client: Client,
}

#[instrument(skip_all, fields(name = %buildkit.name_any()))]
pub async fn reconcile(buildkit: Arc<BuildKit>, ctx: Arc<Context>) -> Result<Action, Error> {
    let ns = buildkit
        .namespace()
        .ok_or(Error::MissingField("namespace"))?;
    let name = buildkit.name_any();

    // Desired state (must set a controller ownerReference internally)
    let desired = crate::resources::deployment::build_deployment(&buildkit);
    let deploy_name = desired.name_any();

    let deployments: Api<Deployment> = Api::namespaced(ctx.client.clone(), &ns);
    let buildkits: Api<BuildKit> = Api::namespaced(ctx.client.clone(), &ns);

    // SSA is idempotent: no diffing, no pre-GET, and it corrects all drift
    let applied = deployments
        .patch(
            &deploy_name,
            &PatchParams::apply(FIELD_MANAGER).force(),
            &Patch::Apply(&desired),
        )
        .await?;

    // Status derived from the object returned by the patch
    let desired_status = crate::status::build_status(&buildkit, &applied);

    if buildkit.status.as_ref() != Some(&desired_status) {
        let patch = serde_json::json!({
            "apiVersion": BuildKit::api_version(&()),
            "kind": BuildKit::kind(&()),
            "status": desired_status,
        });
        buildkits
            .patch_status(&name, &PatchParams::apply(FIELD_MANAGER).force(), &Patch::Apply(&patch))
            .await?;
        info!("updated status");
    } else {
        debug!("status unchanged");
    }

    // Owned Deployment changes already trigger reconciles; this is just a safety resync
    Ok(Action::requeue(Duration::from_secs(300)))
}

pub fn error_policy(buildkit: Arc<BuildKit>, err: &Error, _ctx: Arc<Context>) -> Action {
    error!(name = %buildkit.name_any(), error = %err, "reconcile failed");
    Action::requeue(Duration::from_secs(30))
}

pub async fn run(client: Client) {
    let buildkits: Api<BuildKit> = Api::all(client.clone());
    let deployments: Api<Deployment> = Api::all(client.clone());
    let ctx = Arc::new(Context { client });

    Controller::new(buildkits, Config::default())
        .owns(deployments, Config::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((obj, _action)) => debug!(name = %obj.name, "reconciled"),
                Err(e) => error!(error = %e, "controller error"),
            }
        })
        .await;
}
