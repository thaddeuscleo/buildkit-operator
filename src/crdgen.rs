use crate::crd::buildkit::BuildKit;
use kube::CustomResourceExt;

mod crd;

fn main() {
    let crd = BuildKit::crd();

    print!("{}", serde_json::to_string(&crd).unwrap())
}
