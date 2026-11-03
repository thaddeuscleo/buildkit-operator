use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "buildkit.cleoverse.net",
    version = "v1alpha1",
    kind = "BuildKit",
    plural = "buildkits",
    namespaced,
    printcolumn = r#"{
        "name": "Ready",
        "type": "string",
        "jsonPath": ".status.conditions[?(@.type=='Ready')].status"
    }"#,
    printcolumn = r#"{
        "name": "Replicas",
        "type": "integer",
        "jsonPath": ".status.readyReplicas"
    }"#,
    printcolumn = r#"{
        "name": "Progressing",
        "type": "string",
        "jsonPath": ".status.conditions[?(@.type=='Progressing')].status"
    }"#
)]
#[kube(status = "BuildKitStatus")]

pub struct BuildKitSpec {
    pub image: String,
    pub replicas: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildKitStatus {
    pub ready_replicas: i32,
    pub observed_generation: i64,

    #[serde(default)]
    pub conditions: Vec<Condition>,
}
