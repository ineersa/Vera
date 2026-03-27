//! `vera upgrade` — inspect or apply the binary update plan.

use anyhow::{Result, bail};
use serde::Serialize;

use crate::update_check::{self, InstallMethodSource};

#[derive(Debug, Serialize)]
struct UpgradeReport {
    current_version: String,
    latest_version: Option<String>,
    update_available: bool,
    install_method: Option<String>,
    install_method_source: String,
    detected_install_methods: Vec<String>,
    update_command: String,
    apply_supported: bool,
    applied: bool,
}

pub fn run(apply: bool, json_output: bool) -> Result<()> {
    let status = update_check::binary_version_status(true);
    let mut report = UpgradeReport {
        current_version: status.current_version.to_string(),
        latest_version: status.latest_version.clone(),
        update_available: status.update_available(),
        install_method: status.install_method.clone(),
        install_method_source: install_method_source_name(status.install_method_source).to_string(),
        detected_install_methods: status.detected_install_methods.clone(),
        update_command: status.update_command(),
        apply_supported: status.can_apply_update(),
        applied: false,
    };

    if !apply {
        return print_report(&report, json_output);
    }

    if !status.update_available() {
        if report.latest_version.is_none() {
            bail!("could not determine the latest Vera version; rerun `vera upgrade` later");
        }
        return print_report(&report, json_output);
    }

    if !status.can_apply_update() {
        bail!(apply_error(&status));
    }

    let method = status
        .install_method
        .as_deref()
        .expect("apply requires a resolved install method");
    update_check::apply_update(method)?;
    report.applied = true;
    print_report(&report, json_output)
}

fn print_report(report: &UpgradeReport, json_output: bool) -> Result<()> {
    if json_output {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }

    println!("Current version: {}", report.current_version);
    if let Some(latest) = report.latest_version.as_deref() {
        println!("Latest version:  {latest}");
    } else {
        println!("Latest version:  unavailable");
    }
    println!(
        "Update status:    {}",
        if report.update_available {
            "update available"
        } else {
            "already up to date"
        }
    );
    println!(
        "Install method:   {} ({})",
        report.install_method.as_deref().unwrap_or("unknown"),
        report.install_method_source
    );
    if !report.detected_install_methods.is_empty() {
        println!(
            "Detected methods: {}",
            report.detected_install_methods.join(", ")
        );
    }
    println!("Update command:   {}", report.update_command);

    if report.applied {
        println!("Applied:          yes");
    } else if report.apply_supported {
        println!("Apply support:    yes (`vera upgrade --apply`)");
    } else {
        println!("Apply support:    no (manual update required)");
        print_manual_commands();
    }

    Ok(())
}

fn apply_error(status: &update_check::BinaryVersionStatus) -> String {
    match status.install_method_source {
        InstallMethodSource::Ambiguous => format!(
            "multiple install methods were detected ({}); refusing to guess.\nRun one of these manually:\n{}",
            status.detected_install_methods.join(", "),
            manual_command_lines()
        ),
        InstallMethodSource::Unknown => format!(
            "could not determine how Vera was installed.\nRun one of these manually:\n{}",
            manual_command_lines()
        ),
        _ => "could not determine a supported install method".to_string(),
    }
}

fn print_manual_commands() {
    println!("Manual options:");
    for method in update_check::supported_update_methods() {
        println!(
            "  {:<4} {}",
            format!("{method}:"),
            update_check::suggested_update_command(Some(method))
        );
    }
}

fn manual_command_lines() -> String {
    update_check::supported_update_methods()
        .iter()
        .map(|method| {
            format!(
                "  {}: {}",
                method,
                update_check::suggested_update_command(Some(method))
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn install_method_source_name(source: InstallMethodSource) -> &'static str {
    match source {
        InstallMethodSource::Provenance => "provenance",
        InstallMethodSource::Heuristic => "heuristic",
        InstallMethodSource::Ambiguous => "ambiguous",
        InstallMethodSource::Unknown => "unknown",
    }
}
