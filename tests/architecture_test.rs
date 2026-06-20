//! Enforces the Clean Architecture dependency rule from ADR-0001 by scanning
//! `use` statements in the source tree. See ADR-0006 for why this exists.

use std::fs;
use std::path::Path;

const INFRASTRUCTURE_CRATES: &[&str] = &[
    "tokio",
    "axum",
    "tower",
    "tower_governor",
    "tonic",
    "tonic_prost",
    "prost",
    "sqlx",
    "redis",
    "argon2",
    "password_hash",
    "jsonwebtoken",
    "config",
    "dotenvy",
    "tracing_subscriber",
    "tracing_opentelemetry",
    "opentelemetry",
    "opentelemetry_sdk",
    "opentelemetry_otlp",
    "opentelemetry_semantic_conventions",
    "metrics",
    "metrics_exporter_prometheus",
];

fn rust_files_in(dir: &str) -> Vec<String> {
    let mut files = Vec::new();
    collect_rust_files(Path::new(dir), &mut files);
    files
}

fn collect_rust_files(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path.to_string_lossy().into_owned());
        }
    }
}

fn use_statements(file: &str) -> Vec<String> {
    fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.trim_start().starts_with("use "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn domain_must_not_depend_on_infrastructure_crates() {
    let mut violations = Vec::new();
    for file in rust_files_in("src/domain") {
        for use_line in use_statements(&file) {
            for crate_name in INFRASTRUCTURE_CRATES {
                if use_line.contains(&format!("use {crate_name}::"))
                    || use_line.contains(&format!("use {crate_name};"))
                {
                    violations.push(format!("{file}: {}", use_line.trim()));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "domain/ must be framework-agnostic — found infrastructure imports:\n{}",
        violations.join("\n")
    );
}

#[test]
fn domain_must_not_depend_on_application_or_outer_layers() {
    let mut violations = Vec::new();
    for file in rust_files_in("src/domain") {
        for use_line in use_statements(&file) {
            if use_line.contains("crate::application")
                || use_line.contains("crate::infrastructure")
                || use_line.contains("crate::interfaces")
            {
                violations.push(format!("{file}: {}", use_line.trim()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "domain/ must not depend on outer layers — found:\n{}",
        violations.join("\n")
    );
}

#[test]
fn application_must_not_depend_on_infrastructure_crates() {
    let mut violations = Vec::new();
    for file in rust_files_in("src/application") {
        for use_line in use_statements(&file) {
            for crate_name in INFRASTRUCTURE_CRATES {
                if use_line.contains(&format!("use {crate_name}::"))
                    || use_line.contains(&format!("use {crate_name};"))
                {
                    violations.push(format!("{file}: {}", use_line.trim()));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "application/ must stay portable across infrastructure adapters — found:\n{}",
        violations.join("\n")
    );
}

#[test]
fn application_must_not_depend_on_infrastructure_or_interfaces_modules() {
    let mut violations = Vec::new();
    for file in rust_files_in("src/application") {
        for use_line in use_statements(&file) {
            if use_line.contains("crate::infrastructure") || use_line.contains("crate::interfaces")
            {
                violations.push(format!("{file}: {}", use_line.trim()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "application/ must not depend on infrastructure/ or interfaces/ — found:\n{}",
        violations.join("\n")
    );
}
