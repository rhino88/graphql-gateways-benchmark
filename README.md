# GraphQL Federation Gateways Benchmark

> 📊 **Live Results Available Here:**
> 👉 [the-guild.dev/graphql/hive/federation-gateway-performance](http://the-guild.dev/graphql/hive/federation-gateway-performance)

This project contains a suite of tools to benchmark and compare the performance of different GraphQL federation gateways.

## What This Benchmark Measures

This benchmark evaluates GraphQL federation gateways across multiple performance dimensions:

- **Throughput (RPS)**: Requests per second handled by each gateway
- **Latency**: Response times including P95 and P99.9 percentiles
- **Resource Usage**: Maximum CPU and memory consumption during load
- **Reliability**: Success rate under different load conditions
- **Query Planning**: Time spent preparing (parsing, validating, normalizing and query planning) an operation the gateway has never seen before

The benchmark supports three testing modes:
- **Constant Load**: Steady traffic with 50 virtual users to measure baseline efficiency
- **Stress Testing**: Gradually increasing load to find breaking points and maximum capacity
- **Query Planning**: Sequential cold vs. warm requests to measure per-operation planning cost

## Overview

The benchmark process is orchestrated by a `Makefile` and a series of bash scripts that coordinate multiple tools. For each gateway defined in the `gateways/` directory, the process is as follows:

### Tools and Scripts

- **`Makefile`**: Provides convenient commands (`install`, `run-subgraphs`, `test`, `test-all`) that orchestrate the entire benchmarking workflow
- **`test.sh`**: Main test orchestration script that manages gateway lifecycle, CPU pinning, and resource monitoring
- **`monitor.sh`**: Continuous monitoring script that tracks CPU and memory usage of gateway processes and collects k6 performance metrics via REST API
- **`k6.js`**: Load testing script that defines the GraphQL queries and load patterns (constant or stress testing modes)
- **`toolkit`**: Rust-based CLI utility for processing raw data and generating reports

### Benchmark Process

1.  **Gateway Setup**: The `test.sh` script starts the gateway server using `setsid` for process group management and optionally pins it to specific CPU cores using `taskset`
2.  **Warmup Phase**: A brief warmup period with constant load to stabilize the gateway before measurement begins
3.  **Monitoring**: The `monitor.sh` script continuously tracks:
     - CPU and memory usage of the gateway process group
     - k6 performance metrics (VUs, RPS, P95 latency, success rate) via k6's REST API
     - All data is logged to `data.csv` with timestamps
4.  **Load Testing**: The `k6` tool executes the specified test mode (constant or stress) while monitoring continues, saving detailed results to `k6_summary.json`
5.  **Data Processing**: The `toolkit` utility processes the raw monitoring data (`data.csv`) and k6 results (`k6_summary.json`) to generate a unified `stats.json` with key metrics
6.  **Report Generation**: The `toolkit` provides summary views aggregating results across all tested gateways into comparative tables

## Project Structure

- `gateways/`: Contains subdirectories for each gateway to be benchmarked. Each gateway needs an `install.sh` to set up dependencies and a `run.sh` to start the server.
- `toolkit/`: A Rust crate with a command-line tool to process and summarize benchmark results.
- `subgraphs/`: Contains the Rust source for the GraphQL subgraphs used for testing.
- `k6.js`: The k6 script for load testing.
- `k6-planning.js`: The k6 script for the query planning benchmark.
- `monitor.sh`: Script to monitor CPU and memory usage of a process.
- `test.sh`: Script to run the benchmark for a single gateway.
- `Makefile`: Provides convenience commands to run the benchmarks.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [k6](https://k6.io/docs/getting-started/installation/)
- A running instance of the subgraphs.

## System Requirements

**This benchmark only works on macOS and Linux.** Windows is not supported.

The benchmark uses Unix-specific tools and commands that are not available on Windows, including:
- Process monitoring utilities (`taskset`, `setsid`)
- CPU core detection commands (`nproc`, `sysctl`)
- Shell scripts with Unix-specific functionality

**CPU Pinning Behavior**: If `taskset` and `setsid` are not available on your system, the benchmark will still run but processes will not be pinned to specific CPU cores. In this case, both k6 (load generator) and the gateway processes will run on all available cores, which may result in less consistent performance measurements due to CPU contention and context switching.

## Usage

1.  **Install Gateway Dependencies**:
    This will run the `install.sh` script for each gateway.
    ```bash
    make install
    ```

2.  **Run the Subgraphs**:
    The subgraphs need to be running for the gateways to connect to them.
    ```bash
    make run-subgraphs
    ```
    This command will block, so run it in a separate terminal.

3.  **Run Benchmarks**:
    You can test a single gateway or all of them. You must specify a testing mode: `constant`, `stress` or `planning`.

    *   **Test a single gateway:**
        ```bash
        make test gateway=<gateway_name> mode=<constant|stress|planning>
        ```
        Replace `<gateway_name>` with one of: `apollo-gateway`, `apollo-router`, `apollo-router-v3`, `apollo-router-v3-incremental`, `cosmo`, `grafbase`, `hive-gateway`, `hive-gateway-router-runtime`, or `hive-router`.

        Examples:
        ```bash
        make test gateway=apollo-router mode=constant
        make test gateway=hive-router mode=stress
        make test gateway=apollo-router-v3-incremental mode=planning
        ```

    *   **Test all gateways:**
        This will run the test for every gateway in the `gateways` directory and then print a summary table.
        ```bash
        make test-all mode=<constant|stress|planning>
        ```

        Examples:
        ```bash
        make test-all mode=constant
        make test-all mode=stress
        make test-all mode=planning
        ```

4.  **View Summary Manually**:
    If you have already run the tests and just want to see the summary table again:
    ```bash
    cargo run -p toolkit summary
    cargo run -p toolkit planning-summary   # query planning results
    ```

    The summary output will look something like this:

    ```
    | Gateway    | RPS     | P99 (ms)   | P95 (ms)   | Count   | CPU (max %) | MEM (max MB) | Success Rate (%) |
    | ---------- | ------- | ---------- | ---------- | ------- | ----------- | ------------ | ---------------- |
    | hive-router       | 1826.91 | 78.56      | 48.34      | 109861  | 166.00      | 53           | 100.00           |
    | cosmo      | 570.79  | 348.17     | 128.25     | 34327   | 263.00      | 119          | 100.00           |
    | grafbase   | 451.24  | 400.35     | 139.79     | 27148   | 136.00      | 94           | 100.00           |
    | apollo-router     | 317.45  | 495.61     | 201.34     | 19098   | 273.00      | 193          | 100.00           |
    ```

     ---
    🌐 **Want the full, up-to-date benchmark results (with all gateways compared)?**
    Check them out here: [the-guild.dev/graphql/hive/federation-gateway-performance](http://the-guild.dev/graphql/hive/federation-gateway-performance)

## Testing Modes

- **Constant Mode** (`mode=constant`): Maintains steady traffic with 50 virtual users for 60 seconds to measure baseline performance and efficiency under normal load conditions.

- **Stress Mode** (`mode=stress`): Gradually increases load to find the breaking point of each gateway, helping identify maximum capacity and reliability under pressure.

- **Planning Mode** (`mode=planning`): Measures query planning cost. See [Query Planning Benchmark](#query-planning-benchmark).

## Query Planning Benchmark

Gateways cache the result of parsing, validating and planning an operation, so the throughput benchmarks above (which send the same query over and over) almost never plan anything. The planning benchmark (`k6-planning.js`) measures what the first request for a new operation costs.

A single VU runs a fixed number of iterations (`PLANNING_ITERATIONS`, default `1000`, after `PLANNING_WARMUP_ITERATIONS`, default `50`). In each iteration, for each query tier, it sends:

- **warm**: the exact same document every time, served from the gateway's caches.
- **cold**: the same document with a unique operation name and unique aliases. Aliases shape the response, so they can't be normalized away, and every gateway's plan cache misses.

Both requests do identical subgraph work, so the paired difference `cold - warm` (**planning overhead**) is the time the gateway spent preparing a new operation: parsing, validation, normalization and query planning. This works the same way for every gateway, but it isn't pure planner time. For example, Apollo Router's own `apollo.router.query_planning.plan.duration` metric accounts for roughly half of its overhead on these queries. Iterations alternate which request goes first so neither one consistently benefits from running second.

Query tiers:

| Tier | Description |
| --- | --- |
| `simple` | A single root field resolved by one subgraph |
| `federated` | The query used by the throughput benchmarks: 4 subgraphs, `@requires`, `@provides`, nested entity fetches |
| `complex` | 6 root fields with overlapping selections and deep chains of entity jumps across all subgraphs |

Results are written to `gateways/<gateway>/planning_stats.json` and summarized with `cargo run -p toolkit planning-summary`.

### Apollo Router v3 incremental planner

`apollo-router-v3` and `apollo-router-v3-incremental` both run Apollo Router `v3.0.0-preview.0`. The only difference is that `apollo-router-v3-incremental` enables the new incremental (BULB) query planner (`supergraph.query_planning.incremental_planner.enabled: true`, default `beam_width: 16` and `fuel: 5000`). It builds plans field by field with bounded backtracking instead of exhaustively enumerating plan candidates.

The incremental planner targets large supergraphs, where exhaustively enumerating plan candidates gets expensive. The 4-subgraph schema used here is small, so expect the two variants to be close.

