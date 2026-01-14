//! E2E test benchmarking utilities.
//!
//! Tracks timing of various operations during E2E tests:
//! - Total test time
//! - Setup time (container connection, seeding)
//! - LLM call time (via VCR or real)
//! - Neo4j query time
//!
//! # Usage
//!
//! ```ignore
//! let benchmark = E2EBenchmark::new("test_name");
//! benchmark.start_phase("setup");
//! // ... setup code ...
//! benchmark.end_phase("setup");
//!
//! // Track individual operations
//! benchmark.record_neo4j_query("get_character", 5);
//! benchmark.record_llm_call("generate", 1500);
//!
//! // Print summary at end
//! benchmark.print_summary();
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use neo4rs::{Graph, Query};

use crate::infrastructure::ports::RepoError;

// =============================================================================
// E2E Benchmark
// =============================================================================

/// Accumulated timing data for E2E test benchmarking.
pub struct E2EBenchmark {
    test_name: String,
    test_start: Instant,
    /// Named phases (setup, seed, test_body, etc.)
    phases: Mutex<HashMap<String, PhaseTiming>>,
    /// Neo4j query timings
    neo4j_queries: Mutex<Vec<QueryTiming>>,
    /// LLM call timings
    llm_calls: Mutex<Vec<LlmTiming>>,
    /// Currently active phase
    active_phase: Mutex<Option<(String, Instant)>>,
}

/// Timing data for a named phase.
#[derive(Debug, Clone)]
pub struct PhaseTiming {
    pub name: String,
    pub duration_ms: u64,
}

/// Timing data for a Neo4j query.
#[derive(Debug, Clone)]
pub struct QueryTiming {
    pub operation: String,
    pub duration_ms: u64,
}

/// Timing data for an LLM call.
#[derive(Debug, Clone)]
pub struct LlmTiming {
    pub operation: String,
    pub duration_ms: u64,
    pub tokens: Option<u32>,
}

impl E2EBenchmark {
    /// Create a new benchmark tracker for a test.
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            test_start: Instant::now(),
            phases: Mutex::new(HashMap::new()),
            neo4j_queries: Mutex::new(Vec::new()),
            llm_calls: Mutex::new(Vec::new()),
            active_phase: Mutex::new(None),
        }
    }

    /// Start timing a named phase.
    pub fn start_phase(&self, name: &str) {
        let mut active = self.active_phase.lock().unwrap();
        *active = Some((name.to_string(), Instant::now()));
    }

    /// End timing the current phase.
    pub fn end_phase(&self, name: &str) {
        let mut active = self.active_phase.lock().unwrap();
        if let Some((phase_name, start)) = active.take() {
            if phase_name == name {
                let duration_ms = start.elapsed().as_millis() as u64;
                let mut phases = self.phases.lock().unwrap();
                phases.insert(
                    name.to_string(),
                    PhaseTiming {
                        name: name.to_string(),
                        duration_ms,
                    },
                );
            }
        }
    }

    /// Record a Neo4j query timing.
    pub fn record_neo4j_query(&self, operation: &str, duration_ms: u64) {
        let mut queries = self.neo4j_queries.lock().unwrap();
        queries.push(QueryTiming {
            operation: operation.to_string(),
            duration_ms,
        });
    }

    /// Record an LLM call timing.
    pub fn record_llm_call(&self, operation: &str, duration_ms: u64, tokens: Option<u32>) {
        let mut calls = self.llm_calls.lock().unwrap();
        calls.push(LlmTiming {
            operation: operation.to_string(),
            duration_ms,
            tokens,
        });
    }

    /// Get total test duration so far.
    pub fn total_duration_ms(&self) -> u64 {
        self.test_start.elapsed().as_millis() as u64
    }

    /// Get summary statistics.
    pub fn summary(&self) -> BenchmarkSummary {
        let total_ms = self.total_duration_ms();

        let phases = self.phases.lock().unwrap();
        let neo4j_queries = self.neo4j_queries.lock().unwrap();
        let llm_calls = self.llm_calls.lock().unwrap();

        let neo4j_total_ms: u64 = neo4j_queries.iter().map(|q| q.duration_ms).sum();
        let llm_total_ms: u64 = llm_calls.iter().map(|c| c.duration_ms).sum();
        let phase_total_ms: u64 = phases.values().map(|p| p.duration_ms).sum();

        // "Own code" time = total - external calls
        let own_code_ms = total_ms.saturating_sub(neo4j_total_ms + llm_total_ms);

        BenchmarkSummary {
            test_name: self.test_name.clone(),
            total_ms,
            phases: phases.values().cloned().collect(),
            neo4j_query_count: neo4j_queries.len(),
            neo4j_total_ms,
            llm_call_count: llm_calls.len(),
            llm_total_ms,
            own_code_ms,
        }
    }

    /// Print a formatted summary to stdout.
    pub fn print_summary(&self) {
        let summary = self.summary();
        println!("\n{}", summary.format());
    }
}

/// Summary of benchmark results.
#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    pub test_name: String,
    pub total_ms: u64,
    pub phases: Vec<PhaseTiming>,
    pub neo4j_query_count: usize,
    pub neo4j_total_ms: u64,
    pub llm_call_count: usize,
    pub llm_total_ms: u64,
    pub own_code_ms: u64,
}

impl BenchmarkSummary {
    /// Format the summary as a human-readable string.
    pub fn format(&self) -> String {
        let mut lines = vec![
            format!("=== Benchmark: {} ===", self.test_name),
            format!("Total time:     {:>8} ms", self.total_ms),
            String::new(),
        ];

        // Phases breakdown
        if !self.phases.is_empty() {
            lines.push("Phases:".to_string());
            for phase in &self.phases {
                let pct = (phase.duration_ms as f64 / self.total_ms as f64) * 100.0;
                lines.push(format!(
                    "  {:<12} {:>8} ms ({:>5.1}%)",
                    phase.name, phase.duration_ms, pct
                ));
            }
            lines.push(String::new());
        }

        // External calls
        lines.push("External calls:".to_string());
        let neo4j_pct = (self.neo4j_total_ms as f64 / self.total_ms as f64) * 100.0;
        lines.push(format!(
            "  Neo4j:       {:>8} ms ({:>5.1}%) - {} queries",
            self.neo4j_total_ms, neo4j_pct, self.neo4j_query_count
        ));

        let llm_pct = (self.llm_total_ms as f64 / self.total_ms as f64) * 100.0;
        lines.push(format!(
            "  LLM:         {:>8} ms ({:>5.1}%) - {} calls",
            self.llm_total_ms, llm_pct, self.llm_call_count
        ));

        lines.push(String::new());

        // Own code time
        let own_pct = (self.own_code_ms as f64 / self.total_ms as f64) * 100.0;
        lines.push(format!(
            "Own code:       {:>8} ms ({:>5.1}%)",
            self.own_code_ms, own_pct
        ));

        lines.join("\n")
    }

    /// Format as a compact single line for CI/batch output.
    pub fn format_compact(&self) -> String {
        format!(
            "{}: {}ms total (neo4j: {}ms/{}, llm: {}ms/{}, own: {}ms)",
            self.test_name,
            self.total_ms,
            self.neo4j_total_ms,
            self.neo4j_query_count,
            self.llm_total_ms,
            self.llm_call_count,
            self.own_code_ms
        )
    }
}

// =============================================================================
// Timed Graph Wrapper
// =============================================================================

/// A wrapper around neo4rs::Graph that tracks query execution times.
///
/// This can be used to instrument Neo4j queries for benchmarking.
/// Currently not integrated into E2ETestContext but available for manual use.
#[allow(dead_code)]
pub struct TimedGraph {
    inner: Graph,
    /// Accumulated query time in milliseconds
    total_query_ms: AtomicU64,
    /// Number of queries executed
    query_count: AtomicU64,
    /// Optional benchmark to report to
    benchmark: Option<std::sync::Arc<E2EBenchmark>>,
}

#[allow(dead_code)]
impl TimedGraph {
    /// Create a new timed graph wrapper.
    pub fn new(graph: Graph) -> Self {
        Self {
            inner: graph,
            total_query_ms: AtomicU64::new(0),
            query_count: AtomicU64::new(0),
            benchmark: None,
        }
    }

    /// Create a timed graph that reports to a benchmark.
    pub fn with_benchmark(graph: Graph, benchmark: std::sync::Arc<E2EBenchmark>) -> Self {
        Self {
            inner: graph,
            total_query_ms: AtomicU64::new(0),
            query_count: AtomicU64::new(0),
            benchmark: Some(benchmark),
        }
    }

    /// Get the underlying graph for operations that need direct access.
    pub fn inner(&self) -> &Graph {
        &self.inner
    }

    /// Get total accumulated query time.
    pub fn total_query_ms(&self) -> u64 {
        self.total_query_ms.load(Ordering::Relaxed)
    }

    /// Get number of queries executed.
    pub fn query_count(&self) -> u64 {
        self.query_count.load(Ordering::Relaxed)
    }

    /// Execute a write query (no results) and track timing.
    pub async fn run(&self, query: Query) -> Result<(), RepoError> {
        let start = Instant::now();
        let result = self
            .inner
            .run(query)
            .await
            .map_err(|e| RepoError::database("run", e));
        let elapsed_ms = start.elapsed().as_millis() as u64;

        self.total_query_ms.fetch_add(elapsed_ms, Ordering::Relaxed);
        self.query_count.fetch_add(1, Ordering::Relaxed);

        if let Some(benchmark) = &self.benchmark {
            benchmark.record_neo4j_query("run", elapsed_ms);
        }

        result
    }
}

// =============================================================================
// Global Benchmark Registry (for multi-test aggregation)
// =============================================================================

use std::sync::OnceLock;

/// Global registry of benchmark results across tests.
static BENCHMARK_REGISTRY: OnceLock<Mutex<Vec<BenchmarkSummary>>> = OnceLock::new();

/// Register a benchmark result for later aggregation.
pub fn register_benchmark(summary: BenchmarkSummary) {
    let registry = BENCHMARK_REGISTRY.get_or_init(|| Mutex::new(Vec::new()));
    let mut summaries = registry.lock().unwrap();
    summaries.push(summary);
}

/// Print aggregated benchmark results.
///
/// Call this at the end of a test run to see totals across all tests.
pub fn print_aggregate_benchmarks() {
    let Some(registry) = BENCHMARK_REGISTRY.get() else {
        println!("No benchmarks recorded");
        return;
    };

    let summaries = registry.lock().unwrap();
    if summaries.is_empty() {
        println!("No benchmarks recorded");
        return;
    }

    let total_ms: u64 = summaries.iter().map(|s| s.total_ms).sum();
    let total_neo4j_ms: u64 = summaries.iter().map(|s| s.neo4j_total_ms).sum();
    let total_llm_ms: u64 = summaries.iter().map(|s| s.llm_total_ms).sum();
    let total_own_ms: u64 = summaries.iter().map(|s| s.own_code_ms).sum();
    let total_neo4j_queries: usize = summaries.iter().map(|s| s.neo4j_query_count).sum();
    let total_llm_calls: usize = summaries.iter().map(|s| s.llm_call_count).sum();

    println!("\n=== Aggregate Benchmark Results ({} tests) ===", summaries.len());
    println!("Total time:     {:>8} ms", total_ms);
    println!();
    println!("External calls:");
    let neo4j_pct = (total_neo4j_ms as f64 / total_ms as f64) * 100.0;
    println!(
        "  Neo4j:       {:>8} ms ({:>5.1}%) - {} queries",
        total_neo4j_ms, neo4j_pct, total_neo4j_queries
    );
    let llm_pct = (total_llm_ms as f64 / total_ms as f64) * 100.0;
    println!(
        "  LLM:         {:>8} ms ({:>5.1}%) - {} calls",
        total_llm_ms, llm_pct, total_llm_calls
    );
    println!();
    let own_pct = (total_own_ms as f64 / total_ms as f64) * 100.0;
    println!("Own code:       {:>8} ms ({:>5.1}%)", total_own_ms, own_pct);
    println!();

    // Per-test breakdown (compact)
    println!("Per-test breakdown:");
    for summary in summaries.iter() {
        println!("  {}", summary.format_compact());
    }
}

// =============================================================================
// Timing Helpers
// =============================================================================

/// Time an async operation and return (result, duration_ms).
pub async fn timed<F, T>(f: F) -> (T, u64)
where
    F: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = f.await;
    let duration_ms = start.elapsed().as_millis() as u64;
    (result, duration_ms)
}

/// Time a sync operation and return (result, duration_ms).
pub fn timed_sync<F, T>(f: F) -> (T, u64)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let duration_ms = start.elapsed().as_millis() as u64;
    (result, duration_ms)
}

// =============================================================================
// LLM Benchmark Decorator
// =============================================================================

use async_trait::async_trait;
use crate::infrastructure::ports::{LlmError, LlmPort, LlmRequest, LlmResponse, ToolDefinition};

/// Decorator that adds benchmark timing to any LlmPort implementation.
pub struct BenchmarkLlmDecorator {
    inner: std::sync::Arc<dyn LlmPort>,
    benchmark: std::sync::Arc<E2EBenchmark>,
}

impl BenchmarkLlmDecorator {
    pub fn new(
        inner: std::sync::Arc<dyn LlmPort>,
        benchmark: std::sync::Arc<E2EBenchmark>,
    ) -> Self {
        Self { inner, benchmark }
    }
}

#[async_trait]
impl LlmPort for BenchmarkLlmDecorator {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let start = Instant::now();
        let response = self.inner.generate(request).await?;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let tokens = response.usage.as_ref().map(|u| u.total_tokens);
        self.benchmark.record_llm_call("generate", elapsed_ms, tokens);

        Ok(response)
    }

    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        let start = Instant::now();
        let response = self.inner.generate_with_tools(request, tools).await?;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let tokens = response.usage.as_ref().map(|u| u.total_tokens);
        self.benchmark
            .record_llm_call("generate_with_tools", elapsed_ms, tokens);

        Ok(response)
    }
}

// =============================================================================
// Environment Helper
// =============================================================================

/// Check if benchmarking is enabled via environment variable.
pub fn is_benchmark_enabled() -> bool {
    std::env::var("E2E_BENCHMARK").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_phases() {
        let benchmark = E2EBenchmark::new("test_example");

        benchmark.start_phase("setup");
        std::thread::sleep(Duration::from_millis(10));
        benchmark.end_phase("setup");

        let summary = benchmark.summary();
        assert_eq!(summary.phases.len(), 1);
        assert!(summary.phases[0].duration_ms >= 10);
    }

    #[test]
    fn test_benchmark_recording() {
        let benchmark = E2EBenchmark::new("test_recording");

        benchmark.record_neo4j_query("get_character", 50);
        benchmark.record_neo4j_query("get_location", 30);
        benchmark.record_llm_call("generate", 1500, Some(100));

        let summary = benchmark.summary();
        assert_eq!(summary.neo4j_query_count, 2);
        assert_eq!(summary.neo4j_total_ms, 80);
        assert_eq!(summary.llm_call_count, 1);
        assert_eq!(summary.llm_total_ms, 1500);
    }

    #[test]
    fn test_summary_format() {
        let benchmark = E2EBenchmark::new("format_test");
        benchmark.record_neo4j_query("query", 100);
        benchmark.record_llm_call("generate", 500, None);

        let summary = benchmark.summary();
        let formatted = summary.format();

        assert!(formatted.contains("format_test"));
        assert!(formatted.contains("Neo4j"));
        assert!(formatted.contains("LLM"));
    }
}
