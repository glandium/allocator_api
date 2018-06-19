extern crate rustc_version;

use rustc_version::{version, Version};

fn main() {
    if version().unwrap() >= Version::parse("1.27.0-beta").unwrap() {
        println!("cargo:rustc-cfg=feature=\"nonnull_cast\"");
    }
    #[cfg(feature = "std")]
    {
        if version().unwrap() >= Version::parse("1.28.0-alpha").unwrap() {
            println!("cargo:rustc-cfg=feature=\"global_alloc\"");
        }
    }
}
