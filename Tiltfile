allow_k8s_contexts('espresso-k3s-sno')
if k8s_context() != 'espresso-k3s-sno':
    fail('Select the espresso-k3s-sno development context before running Tilt.')

default_registry('192.168.1.120:30500')
custom_build(
    'buildkit-operator:local',
    'docker buildx build --builder k8s-amd64 --platform linux/amd64 --push -t "$EXPECTED_REF" .',
    deps=['Dockerfile', '.dockerignore', 'Cargo.toml', 'Cargo.lock', 'src'],
    skips_local_docker=True,
    disable_push=True,
)

watch_file('src/crd')
watch_file('Cargo.toml')
watch_file('Cargo.lock')
k8s_yaml(local('cargo run --quiet --locked --bin crdgen'))
k8s_resource(
    new_name='buildkit-crd',
    objects=['buildkits.buildkit.cleoverse.net:customresourcedefinition'],
    labels=['operator'],
)
local_resource(
    'crd-established',
    cmd=['kubectl', '--context', k8s_context(), 'wait', '--for=condition=Established',
         '--timeout=60s', 'crd/buildkits.buildkit.cleoverse.net'],
    resource_deps=['buildkit-crd'],
    labels=['operator'],
)

k8s_yaml(['config/rbac.yaml', 'config/deployment.yaml'])
k8s_resource(
    'buildkit-operator',
    objects=['buildkit-operator:serviceaccount', 'buildkit-operator:clusterrole',
             'buildkit-operator:clusterrolebinding'],
    resource_deps=['crd-established'],
    labels=['operator'],
)

k8s_kind('BuildKit', pod_readiness='wait')
k8s_yaml('deploy/buildkit.yaml')
k8s_resource(
    'example',
    extra_pod_selectors=[{'app': 'example'}],
    resource_deps=['buildkit-operator'],
    labels=['workload'],
)
