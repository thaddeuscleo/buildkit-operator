use k8s_openapi::api::{
    apps::v1::{Deployment, DeploymentSpec},
    core::v1::{
        AppArmorProfile, Container, PodSpec, PodTemplateSpec, SeccompProfile, SecurityContext,
    },
};
use kube::{Resource, api::ObjectMeta};

use crate::crd::buildkit::BuildKit;

pub fn build_deployment(buildkit: &BuildKit) -> Deployment {
    let name = buildkit.metadata.name.as_ref().unwrap();
    let namespace = buildkit.metadata.namespace.as_ref().unwrap();

    let labels = std::collections::BTreeMap::from([("app".to_string(), name.to_string())]);

    Deployment {
        metadata: ObjectMeta {
            name: Some(format!("{}-buildkit", name)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels.clone()),
            owner_references: Some(vec![buildkit.controller_owner_ref(&()).unwrap()]),
            ..Default::default()
        },
        spec: DeploymentSpec {
            replicas: Some(buildkit.spec.replicas),
            selector: k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector {
                match_labels: Some(labels.clone()),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(labels),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "buildkitd".to_string(),
                        image: Some(buildkit.spec.image.clone()),
                        security_context: Some(SecurityContext {
                            run_as_user: Some(1000),
                            run_as_group: Some(1000),
                            seccomp_profile: Some(SeccompProfile {
                                type_: "Unconfined".to_string(),
                                ..Default::default()
                            }),
                            app_armor_profile: Some(AppArmorProfile {
                                type_: "Unconfined".to_string(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
