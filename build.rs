use std::{env, path::Path, process::Command};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let frontend_dir = Path::new(&manifest_dir).join("frontend");

    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/pnpm-lock.yaml");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/vite.config.ts");
    println!("cargo:rerun-if-changed=frontend/tailwind.config.ts");
    println!("cargo:rerun-if-changed=frontend/postcss.config.js");
    println!("cargo:rerun-if-changed=frontend/src");

    if env::var("SQL_WEB_SKIP_FRONTEND_BUILD").is_ok() {
        return;
    }

    if !frontend_dir.exists() {
        panic!("frontend directory does not exist");
    }

    let pnpm = env::var("PNPM").unwrap_or_else(|_| "pnpm".to_string());

    if !frontend_dir.join("node_modules").exists() {
        let mut install = Command::new(&pnpm);
        install.current_dir(&frontend_dir).arg("install");
        if frontend_dir.join("pnpm-lock.yaml").exists() {
            install.arg("--frozen-lockfile");
        }
        run(install, "install frontend dependencies");
    }

    let mut build = Command::new(&pnpm);
    build.current_dir(&frontend_dir).arg("build");
    run(build, "build frontend assets");
}

fn run(mut command: Command, description: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to {description}: {error}"));

    if !status.success() {
        panic!("failed to {description}: {status}");
    }
}
