use std::{env, path::Path};

fn main() {
    let web_dist = Path::new("../../web/dist");
    println!("cargo::rerun-if-changed={}", web_dist.display());

    if env::var("PROFILE").as_deref() == Ok("release") && !web_dist.join("index.html").is_file() {
        panic!("production web assets are missing; run `cd web && npm ci && npm run build` first");
    }
}
