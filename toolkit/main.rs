use std::env;

use serde::{Deserialize, Serialize};
use sonic_rs::{JsonContainerTrait, JsonValueTrait};

#[derive(Serialize, Deserialize)]
struct Stats {
    mem_max: i64,
    cpu_max: f64,
    p99_9: f64,
    p95: f64,
    p90: f64,
    min: f64,
    max: f64,
    avg: f64,
    med: f64,
    count: i64,
    rps: f64,
    success_rate: f64,
}

#[derive(Serialize, Deserialize)]
struct PlanningTierStats {
    tier: String,
    /// Paired (cold - warm) latency: time spent preparing a never-seen operation.
    overhead_avg: f64,
    overhead_med: f64,
    overhead_p95: f64,
    overhead_p99: f64,
    cold_med: f64,
    cold_p95: f64,
    warm_med: f64,
    warm_p95: f64,
}

#[derive(Serialize, Deserialize)]
struct PlanningStats {
    success_rate: f64,
    tiers: Vec<PlanningTierStats>,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if &args[1] == "report" {
        report(&args[2]);
    } else if &args[1] == "summary" {
        summary();
    } else if &args[1] == "data" {
        data();
    } else if &args[1] == "planning-report" {
        planning_report(&args[2]);
    } else if &args[1] == "planning-summary" {
        planning_summary(args.get(2).map(|s| s.as_str()).unwrap_or("gateways"));
    } else {
        eprintln!("Usage: cargo run -p toolkit report <gateway-dir>");
        eprintln!("Usage: cargo run -p toolkit summary");
        eprintln!("Usage: cargo run -p toolkit data");
        eprintln!("Usage: cargo run -p toolkit planning-report <gateway-dir>");
        eprintln!("Usage: cargo run -p toolkit planning-summary [dir]");
    }
}

fn data() {
    // collect gateways/*/data.csv
    let gateways = std::fs::read_dir("gateways").unwrap();

    let mut buffer = String::new();
    let mut wrote_header = false;

    for gateway in gateways {
        if let Ok(gateway) = gateway {
            if !gateway.metadata().unwrap().is_dir() {
                continue;
            }

            let gateway_name = gateway.file_name().into_string().unwrap();
            let csv = std::fs::read_to_string(&format!("{}/data.csv", gateway.path().display()));
            if csv.is_err() {
                continue;
            }
            let csv: String = csv.unwrap();
            let lines = csv.lines();
            for (index, line) in lines.enumerate() {
                if index == 0 {
                    if !wrote_header {
                        buffer.push_str("Gateway,");
                        buffer.push_str(line);
                        buffer.push_str("\n");
                        wrote_header = true;
                    }
                } else {
                    buffer.push_str(&gateway_name);
                    buffer.push_str(",");
                    buffer.push_str(line);
                    buffer.push_str("\n");
                }
            }
        }
    }

    std::fs::write("data.csv", buffer).unwrap();
}

fn summary() {
    // collect gateways/*/stats.json
    let gateways = std::fs::read_dir("gateways").unwrap();
    let mut reports: Vec<(String, Stats)> = Vec::new();
    for gateway in gateways {
        if let Ok(gateway) = gateway {
            if !gateway.metadata().unwrap().is_dir() {
                continue;
            }

            let stats = read_file(&format!("{}/stats.json", gateway.path().display()));
            let stats: Stats = sonic_rs::from_str(&stats).unwrap();
            reports.push((gateway.file_name().to_string_lossy().into_owned(), stats));
        }
    }

    reports.sort_by(|a, b| a.1.rps.partial_cmp(&b.1.rps).unwrap());
    reports.reverse();

    println!(
        "| {:<10} | {:<7} | {:<10} | {:<10} | {:<7} | {:<11} | {:<12} | {} |",
        "Gateway",
        "RPS",
        "P99.9 (ms)",
        "P95 (ms)",
        "Count",
        "CPU (max %)",
        "MEM (max MB)",
        "Success Rate (%)"
    );
    println!(
        "| {} | {} | {} | {} | {} | {} | {} | {} |",
        "----------",
        "-------",
        "----------",
        "----------",
        "-------",
        "-----------",
        "------------",
        "----------------"
    );
    for (name, stats) in reports {
        println!(
            "| {:<10} | {:<7.2} | {:<10.2} | {:<10.2} | {:<7} | {:<11.2} | {:<12} | {:<16.2} |",
            name,
            stats.rps,
            stats.p99_9,
            stats.p95,
            stats.count,
            stats.cpu_max,
            stats.mem_max,
            stats.success_rate * 100.0
        );
    }
}

fn report(pwd: &str) {
    let mem_cpu_raw = read_file(&format!("{}/data.csv", pwd));
    let lines = mem_cpu_raw.split('\n').collect::<Vec<&str>>();

    let mut max_mem: i64 = 0;
    let mut max_cpu: f64 = 0.0;

    for line in lines {
        let line = line.trim();

        if line.len() == 0 {
            continue;
        }

        if line.contains("Seconds") {
            continue;
        }

        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() >= 3 {
            let cpu = cols[cols.len() - 2].parse::<f64>().unwrap_or(0.0);
            let mem = cols[cols.len() - 1].parse::<i64>().unwrap_or(0);
            max_cpu = max_cpu.max(cpu);
            max_mem = max_mem.max(mem);
        }
    }

    max_mem = max_mem / 1000;

    let k6_stats_raw = read_file(&format!("{}/k6_summary.json", pwd));
    let k6_stats: sonic_rs::Value =
        sonic_rs::from_str(&k6_stats_raw).expect("Unable to parse k6_summary.json");

    let k6_stats = k6_stats.as_object().unwrap();
    let metrics = k6_stats.get(&"metrics").unwrap().as_object().unwrap();
    let http_req_duration = metrics
        .get(&"http_req_duration")
        .unwrap()
        .as_object()
        .unwrap()
        .get(&"values")
        .unwrap()
        .as_object()
        .unwrap();

    let http_reqs = metrics
        .get(&"http_reqs")
        .unwrap()
        .as_object()
        .unwrap()
        .get(&"values")
        .unwrap()
        .as_object()
        .unwrap();

    let success_rate = metrics
        .get(&"success_rate")
        .unwrap()
        .as_object()
        .unwrap()
        .get(&"values")
        .unwrap()
        .as_object()
        .unwrap();

    let stats = Stats {
        mem_max: max_mem,
        cpu_max: max_cpu,
        med: http_req_duration.get(&"med").unwrap().as_f64().unwrap(),
        max: http_req_duration.get(&"max").unwrap().as_f64().unwrap(),
        p90: http_req_duration.get(&"p(90)").unwrap().as_f64().unwrap(),
        p95: http_req_duration.get(&"p(95)").unwrap().as_f64().unwrap(),
        p99_9: http_req_duration.get(&"p(99.9)").unwrap().as_f64().unwrap(),
        avg: http_req_duration.get(&"avg").unwrap().as_f64().unwrap(),
        min: http_req_duration.get(&"min").unwrap().as_f64().unwrap(),
        count: http_reqs.get(&"count").unwrap().as_i64().unwrap(),
        rps: http_reqs.get(&"rate").unwrap().as_f64().unwrap(),
        success_rate: success_rate.get(&"rate").unwrap().as_f64().unwrap(),
    };

    std::fs::write(
        format!("{}/stats.json", pwd),
        sonic_rs::to_string_pretty(&stats).unwrap(),
    )
    .expect("Unable to write stats.json")
}

fn planning_report(pwd: &str) {
    let raw = read_file(&format!("{}/k6_planning_summary.json", pwd));
    let summary: sonic_rs::Value =
        sonic_rs::from_str(&raw).expect("Unable to parse k6_planning_summary.json");
    let summary = summary.as_object().unwrap();
    let metrics = summary.get(&"metrics").unwrap().as_object().unwrap();

    let metric = |name: &str, stat: &str| -> f64 {
        metrics
            .get(&name)
            .and_then(|m| m.as_object())
            .and_then(|m| m.get(&"values"))
            .and_then(|v| v.as_object())
            .and_then(|v| v.get(&stat))
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::NAN)
    };

    let tiers = summary
        .get(&"tiers")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            let tier = t.as_str().unwrap().to_string();
            let overhead = format!("planning_{}_overhead", tier);
            let cold = format!("planning_{}_cold", tier);
            let warm = format!("planning_{}_warm", tier);
            PlanningTierStats {
                overhead_avg: metric(&overhead, "avg"),
                overhead_med: metric(&overhead, "med"),
                overhead_p95: metric(&overhead, "p(95)"),
                overhead_p99: metric(&overhead, "p(99)"),
                cold_med: metric(&cold, "med"),
                cold_p95: metric(&cold, "p(95)"),
                warm_med: metric(&warm, "med"),
                warm_p95: metric(&warm, "p(95)"),
                tier,
            }
        })
        .collect();

    let stats = PlanningStats {
        success_rate: metric("success_rate", "rate"),
        tiers,
    };

    std::fs::write(
        format!("{}/planning_stats.json", pwd),
        sonic_rs::to_string_pretty(&stats).unwrap(),
    )
    .expect("Unable to write planning_stats.json");

    let name = std::path::Path::new(pwd)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    print_planning_tables(vec![(name, stats)]);
}

/// Collects `<dir>/*/planning_stats.json` and prints one table per query tier.
/// When `SCENARIO_ARTIFACTS_PREFIX` is set (CI artifacts), it is stripped from
/// the directory names.
fn planning_summary(dir: &str) {
    let prefix = env::var("SCENARIO_ARTIFACTS_PREFIX").unwrap_or_default();
    let mut reports: Vec<(String, PlanningStats)> = Vec::new();

    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        if !entry.metadata().unwrap().is_dir() {
            continue;
        }
        let Ok(raw) =
            std::fs::read_to_string(entry.path().join("planning_stats.json"))
        else {
            continue;
        };
        let stats: PlanningStats = sonic_rs::from_str(&raw).unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        let name = name.strip_prefix(&prefix).unwrap_or(&name).to_string();
        reports.push((name, stats));
    }

    print_planning_tables(reports);
}

fn print_planning_tables(reports: Vec<(String, PlanningStats)>) {
    let Some((_, first)) = reports.first() else {
        eprintln!("No planning_stats.json found");
        return;
    };
    let tiers: Vec<String> = first.tiers.iter().map(|t| t.tier.clone()).collect();

    for tier in tiers {
        let mut rows: Vec<(&str, &PlanningTierStats, f64)> = reports
            .iter()
            .filter_map(|(name, stats)| {
                stats
                    .tiers
                    .iter()
                    .find(|t| t.tier == tier)
                    .map(|t| (name.as_str(), t, stats.success_rate))
            })
            .collect();
        rows.sort_by(|a, b| a.1.overhead_med.total_cmp(&b.1.overhead_med));

        println!("### Query planning: `{}` query", tier);
        println!();
        println!(
            "| Gateway | Planning overhead p50 (ms) | p95 (ms) | p99 (ms) | Cold p50 (ms) | Warm p50 (ms) | Success Rate (%) |"
        );
        println!("| --- | --- | --- | --- | --- | --- | --- |");
        for (name, t, success_rate) in rows {
            println!(
                "| {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.2} |",
                name,
                t.overhead_med,
                t.overhead_p95,
                t.overhead_p99,
                t.cold_med,
                t.warm_med,
                success_rate * 100.0
            );
        }
        println!();
    }
}

fn read_file(filepath: &str) -> String {
    std::fs::read_to_string(filepath).unwrap_or_else(|_| panic!("Unable to read {}", filepath))
}
