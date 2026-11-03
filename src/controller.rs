use futures::StreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use std::{sync::Arc, time::Duration};

use kube::{
    Api, Client,
    api::{Patch, PatchParams},
    runtime::{Controller, controller::Action, watcher::Config},
};

use crate::crd::buildkit::BuildKit;

#[derive(Clone)]
pub struct Context {
    pub client: Client,
}

pub async fn reconcile(buildkit: Arc<BuildKit>, ctx: Arc<Context>) -> Result<Action, kube::Error> {
    let deployment = crate::resources::deployment::build_deployment(&buildkit);
    let deployment_name = deployment
        .metadata
        .name
        .as_deref()
        .expect("Deployment name must exist");

    let namespace = buildkit.metadata.namespace.as_deref().unwrap_or("default");
    let deployments: Api<Deployment> = Api::namespaced(ctx.client.clone(), namespace);
    let existing = deployments.get_opt(deployment_name).await?;

    let needs_apply = match existing {
        None => true,
        Some(ref current) => {
            let current_spec = &current.spec;

            let replicas = current_spec.replicas;

            let image = current_spec
                .template
                .spec
                .as_ref()
                .and_then(|pod| pod.containers.first())
                .and_then(|container| container.image.as_deref());

            let owner_uid = current
                .metadata
                .owner_references
                .as_ref()
                .and_then(|owners| owners.iter().find(|owner| owner.controller == Some(true)))
                .map(|owner| owner.uid.as_str());

            replicas != Some(buildkit.spec.replicas)
                || image != Some(buildkit.spec.image.as_str())
                || owner_uid != buildkit.metadata.uid.as_deref()
        }
    };

    if needs_apply {
        deployments
            .patch(
                deployment_name,
                &PatchParams::apply("buildkit-operator").force(),
                &Patch::Apply(&deployment),
            )
            .await?;
        println!("Applied Deployment: {}", deployment_name);
    } else {
        println!("Deployment already matches desired state");
    }

    // Status Update
    let current_deployment = deployments.get(deployment_name).await?;
    let deployment_status = current_deployment.status.as_ref();

    let ready_replicas = deployment_status
        .and_then(|status| status.ready_replicas)
        .unwrap_or(0);

    println!(
        "BuildKit {} has {} ready replicas",
        deployment_name, ready_replicas
    );

    let name = buildkit.metadata.name.as_deref().unwrap();

    let desired_status = crate::status::build_status(&buildkit, &current_deployment);

    let buildkits: Api<BuildKit> = Api::namespaced(ctx.client.clone(), namespace);
    if buildkit.status.as_ref() != Some(&desired_status) {
        let patch = serde_json::json!({
            "status": desired_status
        });

        buildkits
            .patch_status(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await?;

        println!("Updated BuildKit conditions: {}", name);
    }

    // Ok(Action::await_change())
    Ok(Action::requeue(Duration::from_secs(300)))
}

pub fn error_policy(buildkit: Arc<BuildKit>, error: &kube::Error, _ctx: Arc<Context>) -> Action {
    eprintln!(
        "Error reconciling {}: {}",
        buildkit.metadata.name.as_deref().unwrap_or("unknown"),
        error
    );

    Action::requeue(Duration::from_secs(30))
}

pub async fn run(client: Client) {
    let buildkits: Api<BuildKit> = Api::all(client.clone());
    let deployments: Api<Deployment> = Api::all(client.clone());

    let ctx = Arc::new(Context { client: client });

    Controller::new(buildkits, Config::default())
        .owns(deployments, Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok(o) => println!("reconciled {:?}", o),
                Err(e) => println!("reconcile failed: {:?}", e),
            }
        })
        .await;
}
