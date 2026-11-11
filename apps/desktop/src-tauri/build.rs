fn main() {
    embed_manifest_for_tests();
    tauri_build::build()
}

/// Embeds the Common Controls v6 application manifest into test binaries.
///
/// Tauri links `TaskDialogIndirect` from `comctl32.dll`, which only exists in
/// the v6 flavour of that library. The v6 flavour is loaded through the
/// application manifest, and the manifest is only attached to the shipped
/// executable — not to the test harnesses Cargo builds. Without it the
/// Windows loader resolves the v5 library and the test binary aborts with
/// `STATUS_ENTRYPOINT_NOT_FOUND` before a single assertion runs.
///
/// This mirrors the workaround the Tauri project applies to its own test
/// suite (tauri-apps/tauri#4383).
#[cfg(all(target_os = "windows", target_env = "msvc"))]
fn embed_manifest_for_tests() {
    const MANIFEST: &str = "windows-app-manifest.xml";

    let manifest = std::env::current_dir()
        .expect("failed to resolve the crate directory")
        .join(MANIFEST);
    println!("cargo:rerun-if-changed={}", manifest.display());

    // The manifest must be embedded, not referenced, so the loader finds it
    // without a side-by-side installation next to the test binary.
    //
    // `rustc-link-arg-tests` scopes this to the test harnesses only: the
    // shipped executable already receives the manifest through the Windows
    // resource file that `tauri_build` compiles, and embedding it twice would
    // produce a duplicate resource.
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest.display()
    );
}

#[cfg(not(all(target_os = "windows", target_env = "msvc")))]
fn embed_manifest_for_tests() {}
