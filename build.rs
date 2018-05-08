extern crate rustc_version;

use rustc_version::{version, Version};

fn main() {
    if version().unwrap() >= Version::parse("1.26.0-nightly").unwrap() {
        println!("cargo:rustc-cfg=feature=\"i128\"");
        println!("cargo:rustc-cfg=feature=\"fused\"");
        println!("cargo:rustc-cfg=feature=\"unstable_name_collision\"");
    }
}
