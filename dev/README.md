# Develop with Tilt

This workflow targets the `espresso-k3s-sno` development cluster at `192.168.1.120`. Install Tilt, Rust/Cargo, kubectl, and Docker Buildx with the native amd64 Kubernetes builder `k8s-amd64`. The operator uses the same Dockerfile, ServiceAccount, RBAC, and Deployment as the normal Kubernetes installation.

## Bootstrap the development registry

Run on your development machine:

```bash
kubectl --context espresso-k3s-sno apply -f dev/registry.yaml
kubectl --context espresso-k3s-sno -n buildkit-dev rollout status deployment/registry
curl --fail http://192.168.1.120:30500/v2/
```

The registry serves unauthenticated HTTP. Restrict port 30500 to your trusted development network. Its storage is ephemeral: replacing the registry Pod loses uploaded images. Rebuild the operator in Tilt after registry data loss. The registry is outside Tilt's lifecycle.

Tilt builds and pushes directly from the `k8s-amd64` Buildx builder. The local Docker daemon does not build or push the operator image. Verify the builder exists:

```bash
docker buildx inspect k8s-amd64
```

For the existing builder Deployment `buildkit/k8s-amd640`, install the configuration and mount it:

```bash
kubectl --context espresso-k3s-sno -n buildkit create configmap buildkit-registry-config --from-file=buildkitd.toml=dev/buildkitd.toml --dry-run=client -o yaml | kubectl --context espresso-k3s-sno apply -f -
kubectl --context espresso-k3s-sno -n buildkit patch deployment k8s-amd640 --patch-file dev/buildkit-config-patch.yaml
kubectl --context espresso-k3s-sno -n buildkit rollout status deployment/k8s-amd640
```

For later changes to the ConfigMap, restart the builder Deployment to reload the mounted file. Builder rollouts can interrupt active builds and discard ephemeral build caches.

The remote BuildKit daemon must allow the HTTP development registry. Merge `dev/buildkitd.toml` into its configuration. When creating a new builder, use:

```bash
docker buildx create --name k8s-amd64 --driver kubernetes --platform linux/amd64 --driver-opt namespace=buildkit-dev,nodeselector=kubernetes.io/arch=amd64 --buildkitd-config dev/buildkitd.toml
docker buildx inspect k8s-amd64 --bootstrap
```

Create the builder while the `espresso-k3s-sno` context is selected and after the registry namespace exists. If the builder already exists, update its BuildKit configuration through its provisioning setup; creating another builder with the same name does not update it. Keep existing BuildKit configuration when merging the registry entry.

On the k3s node, use `sudoedit /etc/rancher/k3s/registries.yaml` to merge the following configuration, preserving existing mirrors and credentials:

```yaml
mirrors:
  "192.168.1.120:30500":
    endpoint:
      - "http://192.168.1.120:30500"
```

Restart k3s on the node with `sudo systemctl restart k3s`. This interrupts the single-node cluster temporarily. Apply equivalent registry settings to every node if the cluster expands. See the [k3s registry configuration guide](https://docs.k3s.io/installation/private-registry).

Back on your development machine, wait for recovery:

```bash
kubectl --context espresso-k3s-sno wait --for=condition=Ready node/espresso-k3s-sno --timeout=120s
kubectl --context espresso-k3s-sno -n buildkit-dev rollout status deployment/registry
```

## Run the inner loop

From the repository root:

```bash
kubectl config use-context espresso-k3s-sno
tilt up
```

Tilt builds `linux/amd64` images for the k3s node, pushes unique image tags to the development registry, generates and installs the CRD, waits for establishment, and deploys the operator and example BuildKit instance. Compilation runs natively on the amd64 builder. Later builds reuse its BuildKit Cargo caches.

Edit Rust sources or Cargo manifests to trigger a rebuild and operator rollout. Manifest edits trigger redeployment. README edits do not trigger an image build. CRD source edits also regenerate the in-memory CRD manifest without rewriting `deploy/crd.json`.

Use the Tilt UI or `tilt logs buildkit-operator` for operator logs and `tilt logs example` for worker logs. Verify reconciliation:

```bash
kubectl --context espresso-k3s-sno -n default rollout status deployment/buildkit-operator
kubectl --context espresso-k3s-sno -n default rollout status deployment/example-buildkit
kubectl --context espresso-k3s-sno -n default get buildkit example
```

If a push reports an HTTPS/HTTP mismatch, check the remote BuildKit registry configuration. If a Pod reports `ImagePullBackOff`, check k3s registry configuration and registry reachability from the node. If a build reports `exec format error`, confirm the builder Pod is scheduled on an amd64 node.

The example uses the rootless BuildKit image and requests unconfined security profiles. Cluster admission policies must permit those worker settings.

## Stop and clean up

Stopping `tilt up` leaves workloads running. `tilt down` deletes Tilt-managed resources, including the BuildKit CRD and **all instances of that CRD across namespaces**. Use this only on the development cluster, and do not run it if the CRD contains resources you need to preserve.

The registry remains available after `tilt down`. To remove it and discard its images explicitly:

```bash
kubectl --context espresso-k3s-sno delete -f dev/registry.yaml
```

## Validate changes

Evaluate the Tiltfile without deploying:

```bash
tilt alpha tiltfile-result
```

After bootstrap, run `tilt ci --timeout=15m` to exercise image build, push, deployment, and readiness. Inspect operator logs for RBAC errors. During an interactive session, make a temporary Rust log-message edit, confirm that Tilt builds a new image and replaces the operator Pod, then restore the edit and confirm the next rollout.
