extern crate embed_resource;
use std::env;

fn main() {
    // Build scripts legitimately read cargo's environment; the workspace-wide
    // ban in clippy.toml targets the simulation crates (PLAN_v4 D9).
    #[allow(clippy::disallowed_methods)]
    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        // on windows we will set our game icon as icon for the executable
        // (path is relative to this package directory, where build scripts run)
        embed_resource::compile("../../build/windows/icon.rc");
    }
}
