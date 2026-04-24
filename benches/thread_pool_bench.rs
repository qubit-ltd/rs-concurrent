/*!
 * Benchmark for [`qubit_concurrent::task::service::ThreadPool`].
 */

use std::{
    convert::Infallible,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, OnceLock},
};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use qubit_concurrent::task::service::{ExecutorService, ThreadPool};
use rayon::{ThreadPoolBuilder, prelude::*};

/// Environment variable used to override benchmark dataset root path.
const BENCH_DATA_DIR_ENV: &str = "THREAD_POOL_BENCH_DATA_DIR";
/// Relative path of the CPU-oriented PBBS dataset file.
const CPU_DATASET_FILE: &str = "cpu/pbbs/wikisamp.xml";
/// Relative path of IO small-file workload directory.
const IO_SMALL_FILES_DIR: &str = "io/small-files";
/// Relative path of IO medium-file workload directory.
const IO_MEDIUM_FILES_DIR: &str = "io/medium-files";
/// Maximum number of CPU tasks loaded from dataset lines.
const CPU_DATASET_TASK_LIMIT: usize = 4_096;
/// Maximum number of small files loaded for IO benchmark.
const IO_SMALL_FILE_LIMIT: usize = 2_048;
/// Maximum number of medium files loaded for IO benchmark.
const IO_MEDIUM_FILE_LIMIT: usize = 128;
/// Relative path of PBBS rand-local graph dataset.
const GRAPH_RAND_LOCAL_FILE: &str = "graph/pbbs/randLocalGraph_J_10_50000";
/// Relative path of PBBS rMat graph dataset.
const GRAPH_RMAT_FILE: &str = "graph/pbbs/rMatGraph_J_12_50000";
/// Number of frontier tasks used by graph-traversal benchmark.
const GRAPH_FRONTIER_TASKS: usize = 4_096;

/// CPU benchmark workload derived from an external dataset.
struct DatasetCpuWorkload {
    /// Stable case name for Criterion benchmark IDs.
    name: &'static str,
    /// Per-task CPU iteration counts derived from dataset records.
    iterations: Vec<usize>,
}

/// IO benchmark workload containing file paths to read.
struct DatasetIoWorkload {
    /// Stable case name for Criterion benchmark IDs.
    name: &'static str,
    /// File paths used as task inputs.
    files: Vec<PathBuf>,
}

/// Graph-traversal workload derived from a PBBS adjacency graph.
struct DatasetGraphWorkload {
    /// Stable case name for Criterion benchmark IDs.
    name: &'static str,
    /// Start offsets of adjacency lists for each vertex.
    offsets: Arc<Vec<usize>>,
    /// Flat adjacency list of destination vertex IDs.
    edges: Arc<Vec<u32>>,
    /// Frontier vertices expanded by benchmark tasks.
    frontier: Vec<usize>,
}

/// Runs one batch of no-op tasks and waits until the pool terminates.
fn run_noop_batch(pool_size: usize, task_count: usize) {
    let pool = ThreadPool::new(pool_size).expect("thread pool should be created");
    let mut handles = Vec::with_capacity(task_count);
    for _ in 0..task_count {
        let handle = pool
            .submit_callable(|| Ok::<usize, Infallible>(1))
            .expect("task should be accepted");
        handles.push(handle);
    }
    let mut sum = 0usize;
    for handle in handles {
        sum += handle.get().expect("task should complete");
    }
    black_box(sum);
    pool.shutdown();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should be created")
        .block_on(pool.await_termination());
}

/// Runs one batch of light CPU tasks and waits until the pool terminates.
fn run_cpu_light_batch(pool_size: usize, task_count: usize) {
    run_cpu_work_batch(pool_size, task_count, 128);
}

/// Performs a deterministic amount of CPU work for one task.
fn compute_cpu_work(inner_iters: usize) -> usize {
    let mut acc = 0usize;
    for i in 0..inner_iters {
        acc = acc.wrapping_add(black_box(i));
    }
    acc
}

/// Runs one batch of CPU tasks with configurable per-task work and waits until
/// the pool terminates.
fn run_cpu_work_batch(pool_size: usize, task_count: usize, inner_iters: usize) {
    let pool = ThreadPool::new(pool_size).expect("thread pool should be created");
    let mut handles = Vec::with_capacity(task_count);
    for _ in 0..task_count {
        let iterations = inner_iters;
        let handle = pool
            .submit_callable(move || Ok::<usize, Infallible>(compute_cpu_work(iterations)))
            .expect("task should be accepted");
        handles.push(handle);
    }
    let mut sum = 0usize;
    for handle in handles {
        sum = sum.wrapping_add(handle.get().expect("task should complete"));
    }
    black_box(sum);
    pool.shutdown();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should be created")
        .block_on(pool.await_termination());
}

/// Runs one batch with Rayon using equivalent task count and per-task work.
fn run_rayon_cpu_work_batch(worker_count: usize, task_count: usize, inner_iters: usize) {
    let pool = ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
        .expect("rayon thread pool should be created");
    let sum = pool.install(|| {
        (0..task_count)
            .into_par_iter()
            .map(|_| compute_cpu_work(inner_iters))
            .reduce(|| 0usize, usize::wrapping_add)
    });
    black_box(sum);
}

/// Drains results produced by tasks submitted to the external `threadpool` crate.
///
/// # Parameters
///
/// * `receiver` - Channel receiving one checksum per completed task.
/// * `task_count` - Number of task results expected from the channel.
///
/// # Panics
///
/// Panics if a submitted task fails to send its result.
fn drain_threadpool_crate_results(receiver: mpsc::Receiver<usize>, task_count: usize) {
    let mut sum = 0usize;
    for _ in 0..task_count {
        let value = receiver
            .recv()
            .expect("threadpool crate task should send result");
        sum = sum.wrapping_add(value);
    }
    black_box(sum);
}

/// Runs one batch with the external `threadpool` crate.
///
/// # Parameters
///
/// * `worker_count` - Number of worker threads.
/// * `task_count` - Number of tasks in this batch.
/// * `inner_iters` - CPU iterations performed by each task.
fn run_threadpool_crate_cpu_work_batch(
    worker_count: usize,
    task_count: usize,
    inner_iters: usize,
) {
    let pool = threadpool::ThreadPool::new(worker_count);
    let (sender, receiver) = mpsc::channel();
    for _ in 0..task_count {
        let sender = sender.clone();
        pool.execute(move || {
            let value = compute_cpu_work(inner_iters);
            sender
                .send(value)
                .expect("threadpool crate receiver should stay alive");
        });
    }
    drop(sender);
    pool.join();
    drain_threadpool_crate_results(receiver, task_count);
}

/// Describes one skewed workload profile.
struct SkewedWorkloadSpec {
    /// Stable benchmark case name.
    name: &'static str,
    /// Iterations for the common (light) task path.
    light_iters: usize,
    /// Iterations for the rare (heavy) task path.
    heavy_iters: usize,
    /// Percentage of tasks routed to the heavy path.
    heavy_percent: u32,
    /// Seed used for deterministic heavy-task placement.
    seed: u64,
}

impl SkewedWorkloadSpec {
    /// Returns the expected average iterations per task.
    ///
    /// # Returns
    ///
    /// Expected iterations from the configured heavy/light mix.
    fn expected_iters_per_task(&self) -> usize {
        let heavy = self.heavy_percent as usize;
        let light = 100usize
            .checked_sub(heavy)
            .expect("heavy percent must be <= 100");
        (self.light_iters * light + self.heavy_iters * heavy) / 100
    }
}

/// Mixes bits with splitmix64 for deterministic pseudo-random buckets.
///
/// # Parameters
///
/// * `value` - Input seed value to mix.
///
/// # Returns
///
/// A mixed 64-bit value with good avalanche properties.
fn splitmix64(value: u64) -> u64 {
    let mut z = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Computes per-task iteration count for one skewed workload profile.
///
/// # Parameters
///
/// * `task_index` - Stable index of the task in this batch.
/// * `spec` - Skewed profile defining heavy/light mix.
///
/// # Returns
///
/// Iteration count for this task.
fn skewed_iters_for_task(task_index: usize, spec: &SkewedWorkloadSpec) -> usize {
    let mixed = splitmix64((task_index as u64).wrapping_add(spec.seed));
    let bucket = (mixed % 100) as u32;
    if bucket < spec.heavy_percent {
        spec.heavy_iters
    } else {
        spec.light_iters
    }
}

/// Runs one batch of skewed CPU tasks and waits until pool termination.
///
/// # Parameters
///
/// * `pool_size` - Number of worker threads.
/// * `task_count` - Number of tasks submitted in this batch.
/// * `spec` - Skewed workload profile.
fn run_cpu_skewed_batch(pool_size: usize, task_count: usize, spec: &SkewedWorkloadSpec) {
    let pool = ThreadPool::new(pool_size).expect("thread pool should be created");
    let mut handles = Vec::with_capacity(task_count);
    for task_index in 0..task_count {
        let iterations = skewed_iters_for_task(task_index, spec);
        let handle = pool
            .submit_callable(move || Ok::<usize, Infallible>(compute_cpu_work(iterations)))
            .expect("task should be accepted");
        handles.push(handle);
    }
    let mut sum = 0usize;
    for handle in handles {
        sum = sum.wrapping_add(handle.get().expect("task should complete"));
    }
    black_box(sum);
    pool.shutdown();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should be created")
        .block_on(pool.await_termination());
}

/// Runs one skewed batch with Rayon for the same workload profile.
///
/// # Parameters
///
/// * `worker_count` - Number of worker threads in Rayon.
/// * `task_count` - Number of tasks in this batch.
/// * `spec` - Skewed workload profile.
fn run_rayon_cpu_skewed_batch(worker_count: usize, task_count: usize, spec: &SkewedWorkloadSpec) {
    let pool = ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
        .expect("rayon thread pool should be created");
    let sum = pool.install(|| {
        (0..task_count)
            .into_par_iter()
            .map(|task_index| compute_cpu_work(skewed_iters_for_task(task_index, spec)))
            .reduce(|| 0usize, usize::wrapping_add)
    });
    black_box(sum);
}

/// Runs one skewed batch with the external `threadpool` crate.
///
/// # Parameters
///
/// * `worker_count` - Number of worker threads.
/// * `task_count` - Number of tasks in this batch.
/// * `spec` - Skewed workload profile.
fn run_threadpool_crate_cpu_skewed_batch(
    worker_count: usize,
    task_count: usize,
    spec: &SkewedWorkloadSpec,
) {
    let pool = threadpool::ThreadPool::new(worker_count);
    let (sender, receiver) = mpsc::channel();
    for task_index in 0..task_count {
        let iterations = skewed_iters_for_task(task_index, spec);
        let sender = sender.clone();
        pool.execute(move || {
            let value = compute_cpu_work(iterations);
            sender
                .send(value)
                .expect("threadpool crate receiver should stay alive");
        });
    }
    drop(sender);
    pool.join();
    drain_threadpool_crate_results(receiver, task_count);
}

/// Resolves the benchmark dataset root directory.
///
/// # Returns
///
/// Absolute dataset root path. By default it points to
/// `<crate-root>/test-data`, and can be overridden by
/// `THREAD_POOL_BENCH_DATA_DIR`.
fn benchmark_data_root() -> PathBuf {
    std::env::var(BENCH_DATA_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data"))
}

/// Resolves one dataset path relative to the benchmark data root.
///
/// # Parameters
///
/// * `relative_path` - Path segment under dataset root.
///
/// # Returns
///
/// Absolute path of the requested dataset file or directory.
fn dataset_path(relative_path: &str) -> PathBuf {
    benchmark_data_root().join(relative_path)
}

/// Converts one dataset line into deterministic CPU iterations.
///
/// # Parameters
///
/// * `line` - Raw dataset line text.
/// * `line_index` - Stable index of the line in dataset order.
///
/// # Returns
///
/// Iteration count for one CPU task. The mapping intentionally produces a
/// heavy tail to make work-stealing effects observable.
fn derive_dataset_iterations(line: &str, line_index: usize) -> usize {
    let mut prefix_hash = 0u64;
    for &byte in line.as_bytes().iter().take(16) {
        prefix_hash = prefix_hash.wrapping_mul(131).wrapping_add(byte as u64 + 1);
    }
    let mixed = splitmix64(prefix_hash ^ (line_index as u64).wrapping_mul(0x9E37_79B9));
    let light_iters = 96usize + (mixed as usize % 1_024);
    if mixed.is_multiple_of(20) {
        // Around 5% records become heavy tasks to model real load skew.
        4_096usize + (mixed as usize % 8_192)
    } else {
        light_iters
    }
}

/// Builds CPU workload from PBBS text dataset lines.
///
/// # Returns
///
/// `Some(workload)` when dataset file exists and has enough non-empty lines.
/// Returns `None` when dataset is missing or unreadable.
fn build_dataset_cpu_workload() -> Option<DatasetCpuWorkload> {
    let path = dataset_path(CPU_DATASET_FILE);
    if !path.exists() {
        eprintln!("[bench] skip dataset-cpu: missing file {}", path.display());
        return None;
    }
    let file = fs::File::open(&path).ok()?;
    let reader = BufReader::new(file);
    let mut iterations = Vec::with_capacity(CPU_DATASET_TASK_LIMIT);
    for line in reader.lines() {
        if iterations.len() >= CPU_DATASET_TASK_LIMIT {
            break;
        }
        let Ok(line) = line else {
            eprintln!(
                "[bench] skip dataset-cpu: failed to read {}",
                path.display()
            );
            return None;
        };
        if line.trim().is_empty() {
            continue;
        }
        let index = iterations.len();
        iterations.push(derive_dataset_iterations(&line, index));
    }
    if iterations.is_empty() {
        eprintln!(
            "[bench] skip dataset-cpu: no usable lines in {}",
            path.display()
        );
        return None;
    }
    Some(DatasetCpuWorkload {
        name: "pbbs_wikisamp",
        iterations,
    })
}

/// Returns cached CPU dataset workload.
///
/// # Returns
///
/// Cached workload reference when dataset is available, otherwise `None`.
fn dataset_cpu_workload() -> Option<&'static DatasetCpuWorkload> {
    static WORKLOAD: OnceLock<Option<DatasetCpuWorkload>> = OnceLock::new();
    WORKLOAD.get_or_init(build_dataset_cpu_workload).as_ref()
}

/// Collects regular files from one directory in stable sorted order.
///
/// # Parameters
///
/// * `dir` - Directory containing benchmark files.
/// * `limit` - Maximum number of files returned.
///
/// # Returns
///
/// Sorted file list up to `limit`.
///
/// # Errors
///
/// Returns an error message when directory traversal fails.
fn collect_sorted_files(dir: &Path, limit: usize) -> Result<Vec<PathBuf>, String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("failed to read {}: {error}", dir.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to iterate dir entry: {error}"))?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    files.sort_unstable();
    if files.len() > limit {
        files.truncate(limit);
    }
    Ok(files)
}

/// Builds one IO workload from directory files.
///
/// # Parameters
///
/// * `name` - Stable workload name.
/// * `relative_dir` - Relative directory path under dataset root.
/// * `limit` - Maximum number of files loaded.
///
/// # Returns
///
/// `Some(workload)` when dataset directory exists and contains regular files.
/// Returns `None` when data is missing or unreadable.
fn build_dataset_io_workload(
    name: &'static str,
    relative_dir: &str,
    limit: usize,
) -> Option<DatasetIoWorkload> {
    let dir = dataset_path(relative_dir);
    if !dir.exists() {
        eprintln!(
            "[bench] skip dataset-io ({name}): missing directory {}",
            dir.display()
        );
        return None;
    }
    let files = collect_sorted_files(&dir, limit).ok()?;
    if files.is_empty() {
        eprintln!(
            "[bench] skip dataset-io ({name}): no files in {}",
            dir.display()
        );
        return None;
    }
    Some(DatasetIoWorkload { name, files })
}

/// Returns cached small-file IO workload.
///
/// # Returns
///
/// Workload reference when small-file dataset exists, otherwise `None`.
fn dataset_io_small_workload() -> Option<&'static DatasetIoWorkload> {
    static WORKLOAD: OnceLock<Option<DatasetIoWorkload>> = OnceLock::new();
    WORKLOAD
        .get_or_init(|| {
            build_dataset_io_workload("small_files", IO_SMALL_FILES_DIR, IO_SMALL_FILE_LIMIT)
        })
        .as_ref()
}

/// Returns cached medium-file IO workload.
///
/// # Returns
///
/// Workload reference when medium-file dataset exists, otherwise `None`.
fn dataset_io_medium_workload() -> Option<&'static DatasetIoWorkload> {
    static WORKLOAD: OnceLock<Option<DatasetIoWorkload>> = OnceLock::new();
    WORKLOAD
        .get_or_init(|| {
            build_dataset_io_workload("medium_files", IO_MEDIUM_FILES_DIR, IO_MEDIUM_FILE_LIMIT)
        })
        .as_ref()
}

/// Reads one required line from a buffered text iterator.
///
/// # Parameters
///
/// * `lines` - Text lines iterator.
/// * `path` - Source file path used for diagnostics.
/// * `label` - Logical field name for error messages.
///
/// # Returns
///
/// Trimmed line text when present and readable, otherwise `None`.
fn read_required_line<R: BufRead>(
    lines: &mut std::io::Lines<R>,
    path: &Path,
    label: &str,
) -> Option<String> {
    let next = lines.next();
    let Some(line) = next else {
        eprintln!(
            "[bench] skip dataset-graph: missing {} in {}",
            label,
            path.display()
        );
        return None;
    };
    let Ok(line) = line else {
        eprintln!(
            "[bench] skip dataset-graph: failed to read {} in {}",
            label,
            path.display()
        );
        return None;
    };
    Some(line.trim().to_string())
}

/// Parses one usize value from a required line.
///
/// # Parameters
///
/// * `lines` - Text lines iterator.
/// * `path` - Source file path used for diagnostics.
/// * `label` - Logical field name for error messages.
///
/// # Returns
///
/// Parsed `usize` when conversion succeeds, otherwise `None`.
fn read_required_usize<R: BufRead>(
    lines: &mut std::io::Lines<R>,
    path: &Path,
    label: &str,
) -> Option<usize> {
    let raw = read_required_line(lines, path, label)?;
    let Ok(value) = raw.parse::<usize>() else {
        eprintln!(
            "[bench] skip dataset-graph: invalid {}={} in {}",
            label,
            raw,
            path.display()
        );
        return None;
    };
    Some(value)
}

/// Parses a PBBS `AdjacencyGraph` text file.
///
/// # Parameters
///
/// * `path` - Dataset file path in PBBS adjacency format.
///
/// # Returns
///
/// Tuple of `(offsets, edges)` when parsing succeeds; otherwise `None`.
///
/// # Format
///
/// The format starts with `AdjacencyGraph`, then vertex/edge counts, followed
/// by `n` offset lines and `m` neighbor-id lines.
fn parse_pbbs_adjacency_graph(path: &Path) -> Option<(Vec<usize>, Vec<u32>)> {
    let file = fs::File::open(path).ok()?;
    let mut lines = BufReader::new(file).lines();

    let header = read_required_line(&mut lines, path, "header")?;
    if header != "AdjacencyGraph" {
        eprintln!(
            "[bench] skip dataset-graph: unsupported header {} in {}",
            header,
            path.display()
        );
        return None;
    }
    let vertex_count = read_required_usize(&mut lines, path, "vertex_count")?;
    let edge_count = read_required_usize(&mut lines, path, "edge_count")?;
    if vertex_count == 0 || edge_count == 0 {
        eprintln!(
            "[bench] skip dataset-graph: empty graph in {}",
            path.display()
        );
        return None;
    }

    let mut offsets = Vec::with_capacity(vertex_count);
    for index in 0..vertex_count {
        let label = format!("offset[{index}]");
        let offset = read_required_usize(&mut lines, path, &label)?;
        offsets.push(offset);
    }
    if offsets.windows(2).any(|window| window[0] > window[1])
        || offsets.last().is_some_and(|last| *last > edge_count)
    {
        eprintln!(
            "[bench] skip dataset-graph: invalid offsets in {}",
            path.display()
        );
        return None;
    }

    let mut edges = Vec::with_capacity(edge_count);
    for index in 0..edge_count {
        let label = format!("edge[{index}]");
        let raw = read_required_line(&mut lines, path, &label)?;
        let Ok(neighbor) = raw.parse::<u32>() else {
            eprintln!(
                "[bench] skip dataset-graph: invalid edge value {} in {}",
                raw,
                path.display()
            );
            return None;
        };
        edges.push(neighbor);
    }
    Some((offsets, edges))
}

/// Computes the adjacency range for one vertex.
///
/// # Parameters
///
/// * `offsets` - Vertex offset array.
/// * `edge_count` - Number of adjacency entries.
/// * `vertex` - Vertex index.
///
/// # Returns
///
/// `(start, end)` offsets into the adjacency edge array.
fn adjacency_range(offsets: &[usize], edge_count: usize, vertex: usize) -> (usize, usize) {
    let start = offsets[vertex];
    let end = if vertex + 1 < offsets.len() {
        offsets[vertex + 1]
    } else {
        edge_count
    };
    (start, end)
}

/// Builds a frontier with explicit heavy-tail vertex selection.
///
/// # Parameters
///
/// * `offsets` - Vertex offset array.
/// * `edge_count` - Number of adjacency entries.
/// * `task_count` - Number of frontier vertices to sample.
///
/// # Returns
///
/// Frontier vertex IDs. About 25% samples come from top 1% high-degree
/// vertices, and the rest from lower-degree vertices.
fn build_skewed_frontier(offsets: &[usize], edge_count: usize, task_count: usize) -> Vec<usize> {
    let vertex_count = offsets.len();
    let mut vertices = (0..vertex_count).collect::<Vec<_>>();
    vertices.sort_unstable_by_key(|vertex| {
        let (start, end) = adjacency_range(offsets, edge_count, *vertex);
        std::cmp::Reverse(end - start)
    });
    let heavy_count = (vertex_count / 100).max(1).min(vertex_count);
    let heavy_pool = &vertices[..heavy_count];
    let light_pool = &vertices[vertex_count / 2..];
    let fallback_pool = if light_pool.is_empty() {
        heavy_pool
    } else {
        light_pool
    };

    let mut frontier = Vec::with_capacity(task_count);
    for index in 0..task_count {
        let mixed = splitmix64(0xC0FF_EE00_u64.wrapping_add(index as u64));
        // Skew strategy: route a minority of tasks to high-degree hubs and
        // leave most tasks on lower-degree vertices to amplify variance.
        let pool = if mixed % 100 < 25 {
            heavy_pool
        } else {
            fallback_pool
        };
        frontier.push(pool[mixed as usize % pool.len()]);
    }
    frontier
}

/// Builds a uniformly sampled frontier over all vertices.
///
/// # Parameters
///
/// * `vertex_count` - Number of vertices in graph.
/// * `task_count` - Number of frontier vertices to sample.
///
/// # Returns
///
/// Frontier vertex IDs sampled with deterministic pseudo-random mapping.
fn build_uniform_frontier(vertex_count: usize, task_count: usize) -> Vec<usize> {
    let mut frontier = Vec::with_capacity(task_count);
    for index in 0..task_count {
        let mixed = splitmix64(0xFACE_B00C_u64.wrapping_add(index as u64));
        frontier.push(mixed as usize % vertex_count);
    }
    frontier
}

/// Loads one PBBS graph dataset and constructs a traversal workload.
///
/// # Parameters
///
/// * `name` - Stable workload name.
/// * `relative_file` - Relative dataset path under benchmark root.
/// * `skewed_frontier` - Whether to generate heavy-tail frontier sampling.
///
/// # Returns
///
/// `Some(workload)` when dataset exists and is parsed successfully; otherwise
/// `None`.
fn build_dataset_graph_workload(
    name: &'static str,
    relative_file: &str,
    skewed_frontier: bool,
) -> Option<DatasetGraphWorkload> {
    let path = dataset_path(relative_file);
    if !path.exists() {
        eprintln!(
            "[bench] skip dataset-graph ({name}): missing file {}",
            path.display()
        );
        return None;
    }
    let (offsets, edges) = parse_pbbs_adjacency_graph(&path)?;
    let frontier = if skewed_frontier {
        build_skewed_frontier(&offsets, edges.len(), GRAPH_FRONTIER_TASKS)
    } else {
        build_uniform_frontier(offsets.len(), GRAPH_FRONTIER_TASKS)
    };
    Some(DatasetGraphWorkload {
        name,
        offsets: Arc::new(offsets),
        edges: Arc::new(edges),
        frontier,
    })
}

/// Returns cached PBBS rand-local graph traversal workload.
///
/// # Returns
///
/// Workload reference when dataset exists, otherwise `None`.
fn dataset_graph_rand_local_workload() -> Option<&'static DatasetGraphWorkload> {
    static WORKLOAD: OnceLock<Option<DatasetGraphWorkload>> = OnceLock::new();
    WORKLOAD
        .get_or_init(|| {
            build_dataset_graph_workload("pbbs_rand_local", GRAPH_RAND_LOCAL_FILE, false)
        })
        .as_ref()
}

/// Returns cached PBBS rMat graph traversal workload.
///
/// # Returns
///
/// Workload reference when dataset exists, otherwise `None`.
fn dataset_graph_rmat_workload() -> Option<&'static DatasetGraphWorkload> {
    static WORKLOAD: OnceLock<Option<DatasetGraphWorkload>> = OnceLock::new();
    WORKLOAD
        .get_or_init(|| build_dataset_graph_workload("pbbs_rmat_skewed", GRAPH_RMAT_FILE, true))
        .as_ref()
}

/// Runs one CPU batch where per-task cost comes from dataset-derived iterations.
///
/// # Parameters
///
/// * `pool_size` - Number of worker threads.
/// * `iterations` - Per-task iteration counts.
fn run_cpu_dataset_batch(pool_size: usize, iterations: &[usize]) {
    let pool = ThreadPool::new(pool_size).expect("thread pool should be created");
    let mut handles = Vec::with_capacity(iterations.len());
    for &inner_iters in iterations {
        let handle = pool
            .submit_callable(move || Ok::<usize, Infallible>(compute_cpu_work(inner_iters)))
            .expect("task should be accepted");
        handles.push(handle);
    }
    let mut sum = 0usize;
    for handle in handles {
        sum = sum.wrapping_add(handle.get().expect("task should complete"));
    }
    black_box(sum);
    pool.shutdown();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should be created")
        .block_on(pool.await_termination());
}

/// Runs one Rayon CPU batch using the same dataset-derived iterations.
///
/// # Parameters
///
/// * `worker_count` - Number of Rayon threads.
/// * `iterations` - Per-task iteration counts.
fn run_rayon_cpu_dataset_batch(worker_count: usize, iterations: &[usize]) {
    let pool = ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
        .expect("rayon thread pool should be created");
    let sum = pool.install(|| {
        iterations
            .par_iter()
            .map(|inner_iters| compute_cpu_work(*inner_iters))
            .reduce(|| 0usize, usize::wrapping_add)
    });
    black_box(sum);
}

/// Runs one dataset-derived CPU batch with the external `threadpool` crate.
///
/// # Parameters
///
/// * `worker_count` - Number of worker threads.
/// * `iterations` - Per-task iteration counts.
fn run_threadpool_crate_cpu_dataset_batch(worker_count: usize, iterations: &[usize]) {
    let pool = threadpool::ThreadPool::new(worker_count);
    let (sender, receiver) = mpsc::channel();
    for &inner_iters in iterations {
        let sender = sender.clone();
        pool.execute(move || {
            let value = compute_cpu_work(inner_iters);
            sender
                .send(value)
                .expect("threadpool crate receiver should stay alive");
        });
    }
    drop(sender);
    pool.join();
    drain_threadpool_crate_results(receiver, iterations.len());
}

/// Reads one file and computes a deterministic checksum.
///
/// # Parameters
///
/// * `path` - File path to read.
///
/// # Returns
///
/// Byte-sum checksum used to prevent dead-code elimination.
///
/// # Panics
///
/// Panics when file cannot be read.
fn file_checksum(path: &Path) -> usize {
    let bytes = fs::read(path).unwrap_or_else(|error| {
        panic!("failed to read benchmark file {}: {error}", path.display())
    });
    bytes
        .into_iter()
        .fold(0usize, |acc, byte| acc.wrapping_add(byte as usize))
}

/// Runs one IO batch with ThreadPool by reading one file per task.
///
/// # Parameters
///
/// * `pool_size` - Number of worker threads.
/// * `files` - File paths read by submitted tasks.
fn run_io_file_batch(pool_size: usize, files: &[PathBuf]) {
    let pool = ThreadPool::new(pool_size).expect("thread pool should be created");
    let mut handles = Vec::with_capacity(files.len());
    for path in files {
        let path = path.clone();
        let handle = pool
            .submit_callable(move || Ok::<usize, Infallible>(file_checksum(&path)))
            .expect("task should be accepted");
        handles.push(handle);
    }
    let mut sum = 0usize;
    for handle in handles {
        sum = sum.wrapping_add(handle.get().expect("task should complete"));
    }
    black_box(sum);
    pool.shutdown();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should be created")
        .block_on(pool.await_termination());
}

/// Runs one IO batch with Rayon by reading one file per task.
///
/// # Parameters
///
/// * `worker_count` - Number of Rayon threads.
/// * `files` - File paths read in parallel.
fn run_rayon_io_file_batch(worker_count: usize, files: &[PathBuf]) {
    let pool = ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
        .expect("rayon thread pool should be created");
    let sum = pool.install(|| {
        files
            .par_iter()
            .map(|path| file_checksum(path))
            .reduce(|| 0usize, usize::wrapping_add)
    });
    black_box(sum);
}

/// Runs one IO batch with the external `threadpool` crate.
///
/// # Parameters
///
/// * `worker_count` - Number of worker threads.
/// * `files` - File paths read by submitted tasks.
fn run_threadpool_crate_io_file_batch(worker_count: usize, files: &[PathBuf]) {
    let pool = threadpool::ThreadPool::new(worker_count);
    let (sender, receiver) = mpsc::channel();
    for path in files {
        let path = path.clone();
        let sender = sender.clone();
        pool.execute(move || {
            let value = file_checksum(&path);
            sender
                .send(value)
                .expect("threadpool crate receiver should stay alive");
        });
    }
    drop(sender);
    pool.join();
    drain_threadpool_crate_results(receiver, files.len());
}

/// Traverses adjacency of one frontier vertex and returns a checksum.
///
/// # Parameters
///
/// * `offsets` - Vertex offset array.
/// * `edges` - Flat adjacency edge array.
/// * `vertex` - Frontier vertex to expand.
///
/// # Returns
///
/// Deterministic checksum over adjacency entries of `vertex`.
fn traverse_frontier_vertex(offsets: &[usize], edges: &[u32], vertex: usize) -> usize {
    let (start, end) = adjacency_range(offsets, edges.len(), vertex);
    let mut acc = vertex.wrapping_mul(17);
    // This loop models the hot path of BFS-style frontier expansion where
    // task cost is proportional to out-degree.
    for &neighbor in &edges[start..end] {
        acc = acc.wrapping_add((neighbor as usize).wrapping_mul(31));
    }
    acc
}

/// Runs one graph-traversal batch with ThreadPool.
///
/// # Parameters
///
/// * `pool_size` - Number of worker threads.
/// * `workload` - Graph workload with adjacency and frontier data.
fn run_graph_dataset_batch(pool_size: usize, workload: &DatasetGraphWorkload) {
    let pool = ThreadPool::new(pool_size).expect("thread pool should be created");
    let mut handles = Vec::with_capacity(workload.frontier.len());
    for &vertex in &workload.frontier {
        let offsets = Arc::clone(&workload.offsets);
        let edges = Arc::clone(&workload.edges);
        let handle = pool
            .submit_callable(move || {
                Ok::<usize, Infallible>(traverse_frontier_vertex(&offsets, &edges, vertex))
            })
            .expect("task should be accepted");
        handles.push(handle);
    }
    let mut sum = 0usize;
    for handle in handles {
        sum = sum.wrapping_add(handle.get().expect("task should complete"));
    }
    black_box(sum);
    pool.shutdown();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should be created")
        .block_on(pool.await_termination());
}

/// Runs one graph-traversal batch with Rayon.
///
/// # Parameters
///
/// * `worker_count` - Number of Rayon threads.
/// * `workload` - Graph workload with adjacency and frontier data.
fn run_rayon_graph_dataset_batch(worker_count: usize, workload: &DatasetGraphWorkload) {
    let pool = ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
        .expect("rayon thread pool should be created");
    let offsets = workload.offsets.as_slice();
    let edges = workload.edges.as_slice();
    let sum = pool.install(|| {
        workload
            .frontier
            .par_iter()
            .map(|vertex| traverse_frontier_vertex(offsets, edges, *vertex))
            .reduce(|| 0usize, usize::wrapping_add)
    });
    black_box(sum);
}

/// Runs one graph-traversal batch with the external `threadpool` crate.
///
/// # Parameters
///
/// * `worker_count` - Number of worker threads.
/// * `workload` - Graph workload with adjacency and frontier data.
fn run_threadpool_crate_graph_dataset_batch(worker_count: usize, workload: &DatasetGraphWorkload) {
    let pool = threadpool::ThreadPool::new(worker_count);
    let (sender, receiver) = mpsc::channel();
    for &vertex in &workload.frontier {
        let offsets = Arc::clone(&workload.offsets);
        let edges = Arc::clone(&workload.edges);
        let sender = sender.clone();
        pool.execute(move || {
            let value = traverse_frontier_vertex(&offsets, &edges, vertex);
            sender
                .send(value)
                .expect("threadpool crate receiver should stay alive");
        });
    }
    drop(sender);
    pool.join();
    drain_threadpool_crate_results(receiver, workload.frontier.len());
}

/// Benchmarks throughput under different worker counts and task types.
fn bench_thread_pool_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_pool_throughput");
    let workers = [1usize, 2, 4, 8];
    let task_count = 2_000usize;
    group.throughput(Throughput::Elements(task_count as u64));
    for worker_count in workers {
        group.bench_with_input(
            BenchmarkId::new("noop_tasks", worker_count),
            &worker_count,
            |b, &wc| b.iter(|| run_noop_batch(wc, task_count)),
        );
        group.bench_with_input(
            BenchmarkId::new("cpu_light_tasks", worker_count),
            &worker_count,
            |b, &wc| b.iter(|| run_cpu_light_batch(wc, task_count)),
        );
    }
    group.finish();
}

/// Compares thread pool throughput against Rayon and the `threadpool` crate.
fn bench_thread_pool_vs_libraries(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_pool_vs_libraries");
    let workers = [1usize, 4, 8];
    let granularities = [256usize, 2_048];
    let total_iters = 2_048_000usize;
    for worker_count in workers {
        for inner_iters in granularities {
            let task_count = total_iters / inner_iters;
            group.throughput(Throughput::Elements(task_count as u64));
            let thread_pool_id = format!("thread_pool/workers={worker_count}/iters={inner_iters}");
            group.bench_with_input(
                BenchmarkId::from_parameter(thread_pool_id),
                &worker_count,
                |b, &wc| b.iter(|| run_cpu_work_batch(wc, task_count, inner_iters)),
            );
            let rayon_id = format!("rayon/workers={worker_count}/iters={inner_iters}");
            group.bench_with_input(
                BenchmarkId::from_parameter(rayon_id),
                &worker_count,
                |b, &wc| b.iter(|| run_rayon_cpu_work_batch(wc, task_count, inner_iters)),
            );
            let threadpool_id = format!("threadpool/workers={worker_count}/iters={inner_iters}");
            group.bench_with_input(
                BenchmarkId::from_parameter(threadpool_id),
                &worker_count,
                |b, &wc| {
                    b.iter(|| run_threadpool_crate_cpu_work_batch(wc, task_count, inner_iters))
                },
            );
        }
    }
    group.finish();
}

/// Benchmarks scheduling overhead vs task granularity under fixed total work.
fn bench_thread_pool_granularity(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_pool_granularity");
    let workers = [1usize, 4, 8];
    let granularities = [32usize, 256, 2_048];
    let total_iters = 2_048_000usize;
    for worker_count in workers {
        for inner_iters in granularities {
            let task_count = total_iters / inner_iters;
            let id = format!("workers={worker_count}/iters={inner_iters}");
            group.throughput(Throughput::Elements(task_count as u64));
            group.bench_with_input(BenchmarkId::from_parameter(id), &worker_count, |b, &wc| {
                b.iter(|| run_cpu_work_batch(wc, task_count, inner_iters))
            });
        }
    }
    group.finish();
}

/// Compares ThreadPool, Rayon, and the `threadpool` crate on skewed workloads
/// where a small fraction of tasks are much heavier than the rest.
fn bench_thread_pool_skewed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_pool_skewed_workload");
    let workers = [4usize, 8];
    let profiles = [
        SkewedWorkloadSpec {
            name: "heavy5_light95",
            light_iters: 128,
            heavy_iters: 8_192,
            heavy_percent: 5,
            seed: 0xA11C_E5E1,
        },
        SkewedWorkloadSpec {
            name: "heavy1_light99",
            light_iters: 64,
            heavy_iters: 32_768,
            heavy_percent: 1,
            seed: 0xBADC_0FFE,
        },
    ];
    let total_iters = 4_194_304usize;
    for worker_count in workers {
        for profile in &profiles {
            let expected = profile.expected_iters_per_task().max(1);
            let task_count = (total_iters / expected).max(1);
            group.throughput(Throughput::Elements(task_count as u64));
            let thread_pool_id = format!(
                "thread_pool/workers={worker_count}/profile={}",
                profile.name
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(thread_pool_id),
                &worker_count,
                |b, &wc| b.iter(|| run_cpu_skewed_batch(wc, task_count, profile)),
            );
            let rayon_id = format!("rayon/workers={worker_count}/profile={}", profile.name);
            group.bench_with_input(
                BenchmarkId::from_parameter(rayon_id),
                &worker_count,
                |b, &wc| b.iter(|| run_rayon_cpu_skewed_batch(wc, task_count, profile)),
            );
            let threadpool_id =
                format!("threadpool/workers={worker_count}/profile={}", profile.name);
            group.bench_with_input(
                BenchmarkId::from_parameter(threadpool_id),
                &worker_count,
                |b, &wc| {
                    b.iter(|| run_threadpool_crate_cpu_skewed_batch(wc, task_count, profile))
                },
            );
        }
    }
    group.finish();
}

/// Compares ThreadPool, Rayon, and the `threadpool` crate on CPU workloads
/// derived from PBBS text data.
fn bench_thread_pool_dataset_cpu(c: &mut Criterion) {
    let Some(workload) = dataset_cpu_workload() else {
        return;
    };
    let mut group = c.benchmark_group("thread_pool_dataset_cpu");
    let workers = [4usize, 8];
    let task_counts = [512usize, 2_048];
    for worker_count in workers {
        for task_count in task_counts {
            if task_count > workload.iterations.len() {
                continue;
            }
            let iterations = &workload.iterations[..task_count];
            group.throughput(Throughput::Elements(task_count as u64));
            let thread_pool_id = format!(
                "thread_pool/workers={worker_count}/dataset={}/tasks={task_count}",
                workload.name
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(thread_pool_id),
                &worker_count,
                |b, &wc| b.iter(|| run_cpu_dataset_batch(wc, iterations)),
            );
            let rayon_id = format!(
                "rayon/workers={worker_count}/dataset={}/tasks={task_count}",
                workload.name
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(rayon_id),
                &worker_count,
                |b, &wc| b.iter(|| run_rayon_cpu_dataset_batch(wc, iterations)),
            );
            let threadpool_id = format!(
                "threadpool/workers={worker_count}/dataset={}/tasks={task_count}",
                workload.name
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(threadpool_id),
                &worker_count,
                |b, &wc| b.iter(|| run_threadpool_crate_cpu_dataset_batch(wc, iterations)),
            );
        }
    }
    group.finish();
}

/// Compares ThreadPool, Rayon, and the `threadpool` crate on dataset-backed IO
/// workloads.
fn bench_thread_pool_dataset_io(c: &mut Criterion) {
    let mut workloads = Vec::new();
    if let Some(small) = dataset_io_small_workload() {
        workloads.push(small);
    }
    if let Some(medium) = dataset_io_medium_workload() {
        workloads.push(medium);
    }
    if workloads.is_empty() {
        return;
    }
    let mut group = c.benchmark_group("thread_pool_dataset_io");
    let workers = [4usize, 8];
    for workload in workloads {
        group.throughput(Throughput::Elements(workload.files.len() as u64));
        for worker_count in workers {
            let thread_pool_id = format!(
                "thread_pool/workers={worker_count}/dataset={}",
                workload.name
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(thread_pool_id),
                &worker_count,
                |b, &wc| b.iter(|| run_io_file_batch(wc, &workload.files)),
            );
            let rayon_id = format!("rayon/workers={worker_count}/dataset={}", workload.name);
            group.bench_with_input(
                BenchmarkId::from_parameter(rayon_id),
                &worker_count,
                |b, &wc| b.iter(|| run_rayon_io_file_batch(wc, &workload.files)),
            );
            let threadpool_id =
                format!("threadpool/workers={worker_count}/dataset={}", workload.name);
            group.bench_with_input(
                BenchmarkId::from_parameter(threadpool_id),
                &worker_count,
                |b, &wc| b.iter(|| run_threadpool_crate_io_file_batch(wc, &workload.files)),
            );
        }
    }
    group.finish();
}

/// Compares ThreadPool, Rayon, and the `threadpool` crate on PBBS
/// graph-traversal workloads.
fn bench_thread_pool_dataset_graph_traversal(c: &mut Criterion) {
    let mut workloads = Vec::new();
    if let Some(workload) = dataset_graph_rand_local_workload() {
        workloads.push(workload);
    }
    if let Some(workload) = dataset_graph_rmat_workload() {
        workloads.push(workload);
    }
    if workloads.is_empty() {
        return;
    }
    let mut group = c.benchmark_group("thread_pool_dataset_graph_traversal");
    let workers = [4usize, 8];
    for workload in workloads {
        group.throughput(Throughput::Elements(workload.frontier.len() as u64));
        for worker_count in workers {
            let thread_pool_id = format!(
                "thread_pool/workers={worker_count}/dataset={}",
                workload.name
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(thread_pool_id),
                &worker_count,
                |b, &wc| b.iter(|| run_graph_dataset_batch(wc, workload)),
            );
            let rayon_id = format!("rayon/workers={worker_count}/dataset={}", workload.name);
            group.bench_with_input(
                BenchmarkId::from_parameter(rayon_id),
                &worker_count,
                |b, &wc| b.iter(|| run_rayon_graph_dataset_batch(wc, workload)),
            );
            let threadpool_id =
                format!("threadpool/workers={worker_count}/dataset={}", workload.name);
            group.bench_with_input(
                BenchmarkId::from_parameter(threadpool_id),
                &worker_count,
                |b, &wc| b.iter(|| run_threadpool_crate_graph_dataset_batch(wc, workload)),
            );
        }
    }
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = bench_thread_pool_throughput,
        bench_thread_pool_granularity,
        bench_thread_pool_vs_libraries,
        bench_thread_pool_skewed_workload,
        bench_thread_pool_dataset_cpu,
        bench_thread_pool_dataset_io,
        bench_thread_pool_dataset_graph_traversal
);
criterion_main!(benches);
