extern crate embed_resource;
use std::env;

fn main() {
    // Build scripts legitimately read cargo's environment; the workspace-wide
    // ban in clippy.toml targets the simulation crates (PLAN_v4 D9).
    #[allow(clippy::disallowed_methods)]
    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        // The game icon as the executable's icon (the path is relative to
        // this package directory, where build scripts run). On a Linux host
        // it is embedded only when <arch>-w64-mingw32-windres exists
        // (x86_64-pc-windows-gnu) or RC_<target> names a resource compiler
        // (tools/portable sets it to a windres wrapper); otherwise
        // embed-resource skips it silently, and the window icon is still
        // set at run time.
        embed_resource::compile("../../build/windows/icon.rc");
    }
}
