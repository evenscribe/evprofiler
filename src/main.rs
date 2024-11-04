mod profilestore;
pub(crate) mod profilestorepb {
    tonic::include_proto!("parca.profilestore.v1alpha1");
}

fn main() {
    println!("Hello, world!");
}
