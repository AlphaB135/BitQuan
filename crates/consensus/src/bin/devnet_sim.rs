#![forbid(unsafe_code)]

//! Devnet difficulty simulation harness with burst guard metrics.

use std::fmt;

use anyhow::{anyhow, Context, Result};
use clap::Parser;

use bitquan_consensus::{asert_next_target, compact_to_target, ConsensusParams};

/// Command-line arguments for the simulation runner.
#[derive(Parser, Debug)]
#[command(
    name = "devnet-sim",
    about = "Simulate ASERT + burst guard response to stepped hash-rate changes."
)]
struct SimArgs {
    /// Comma separated sequence of <blocks>:<hash_rate> entries (e.g. 200:1.0,80:4.0,200:1.0).
    #[arg(
        long,
        default_value = "200:1.0,80:4.0,200:1.0,80:0.5,200:1.0",
        value_name = "PATTERN"
    )]
    pattern: String,

    /// Emit per-block metrics.
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

/// Parsed hash-rate segment.
#[derive(Clone, Copy, Debug)]
struct Segment {
    blocks: u64,
    hash_rate: f64,
}

/// Accumulated statistics for a segment.
#[derive(Default, Debug)]
struct SegmentStats {
    blocks: u64,
    hash_rate: f64,
    interval_sum: f64,
    guard_triggers: u64,
}

/// Captures a guard activation for later analysis.
#[derive(Clone, Copy, Debug)]
struct GuardEvent {
    height: u64,
    ratio: f64,
    time_delta: i64,
    expected_time: f64,
    segment_index: usize,
}

/// Snapshot of a simulated block.
#[derive(Clone, Copy, Debug)]
struct BlockSnapshot {
    height: u64,
    timestamp: i64,
    target: f64,
}

impl fmt::Display for SegmentStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.blocks == 0 {
            return write!(
                f,
                "{:>5} blk @ {:>4.2}x → avg ??? s | guard ???",
                0, self.hash_rate
            );
        }
        let avg_interval = self.interval_sum / self.blocks as f64;
        let guard_per_100 = if self.blocks == 0 {
            0.0
        } else {
            self.guard_triggers as f64 * 100.0 / self.blocks as f64
        };
        write!(
            f,
            "{:>5} blk @ {:>4.2}x → avg {:>6.2} s | guard {:>3} ({:>4.2}/100)",
            self.blocks, self.hash_rate, avg_interval, self.guard_triggers, guard_per_100
        )
    }
}

fn main() -> Result<()> {
    let args = SimArgs::parse();
    let segments = parse_pattern(&args.pattern)?;

    if segments.is_empty() {
        return Err(anyhow!("no segments parsed from pattern"));
    }

    let params = ConsensusParams::phase3_defaults();
    let baseline_bits = 0x1d00ffff;
    let baseline_target = compact_to_target(baseline_bits);
    let mut current_target = baseline_target;

    let mut blocks =
        Vec::with_capacity(segments.iter().map(|s| s.blocks as usize).sum::<usize>() + 1);
    blocks.push(BlockSnapshot {
        height: 0,
        timestamp: 0,
        target: baseline_target,
    });

    let mut segment_stats: Vec<SegmentStats> = segments
        .iter()
        .map(|seg| SegmentStats {
            blocks: 0,
            hash_rate: seg.hash_rate,
            interval_sum: 0.0,
            guard_triggers: 0,
        })
        .collect();

    let mut guard_events = Vec::new();
    let mut total_interval = 0.0;

    for (seg_index, segment) in segments.iter().enumerate() {
        for _ in 0..segment.blocks {
            let prev = blocks.last().expect("genesis block present");

            let difficulty_ratio = baseline_target / current_target;
            let mean_interval =
                (params.target_block_time as f64) * difficulty_ratio / segment.hash_rate;
            let dt_seconds = mean_interval.max(1.0).round() as i64;

            let timestamp = prev.timestamp + dt_seconds;
            let height = prev.height + 1;

            if args.verbose {
                println!(
                    "[block {:>6}] dt={:>4} s | target={:.6e} | diff_ratio={:.4} | hash= {:.2}x",
                    height, dt_seconds, current_target, difficulty_ratio, segment.hash_rate
                );
            }

            blocks.push(BlockSnapshot {
                height,
                timestamp,
                target: current_target,
            });

            total_interval += dt_seconds as f64;

            if let Some(stats) = segment_stats.get_mut(seg_index) {
                stats.blocks += 1;
                stats.interval_sum += dt_seconds as f64;
            }

            let window = params.burst_guard_window;
            let anchor_index = if height >= window {
                (height - window) as usize
            } else {
                0usize
            };
            let anchor = blocks.get(anchor_index).context("anchor lookup failed")?;

            let height_delta = height as i64 - anchor.height as i64;
            let time_delta = timestamp - anchor.timestamp;
            let next_target =
                asert_next_target(anchor.target, height_delta, time_delta, &params, None);

            let expected_time = params.target_block_time as f64 * height_delta as f64;
            let guard_triggered = height_delta as u64 >= params.burst_guard_window
                && time_delta > 0
                && (time_delta as f64) < expected_time * params.burst_guard_floor_ratio;

            if guard_triggered {
                guard_events.push(GuardEvent {
                    height: height + 1,
                    ratio: time_delta as f64 / expected_time,
                    time_delta,
                    expected_time,
                    segment_index: seg_index,
                });
                if let Some(stats) = segment_stats.get_mut(seg_index) {
                    stats.guard_triggers += 1;
                }
            }

            if args.verbose {
                let avg_interval = if height_delta > 0 {
                    time_delta as f64 / height_delta as f64
                } else {
                    params.target_block_time as f64
                };
                let ratio = if expected_time > 0.0 {
                    time_delta as f64 / expected_time
                } else {
                    1.0
                };
                println!(
                    "[ASERT] height={} guard={} window={} avg={:.2}s ratio={:.3} target={:.6e} next_target={:.6e}",
                    height,
                    if guard_triggered { "ON " } else { "off" },
                    height_delta,
                    avg_interval,
                    ratio,
                    current_target,
                    next_target
                );
            }

            current_target = next_target;
        }
    }

    let total_blocks = blocks.len() as u64 - 1;
    let avg_interval = total_interval / total_blocks as f64;

    println!("=== Devnet Difficulty Simulation ===");
    println!("pattern         : {}", args.pattern);
    println!("blocks simulated: {}", total_blocks);
    println!(
        "average interval: {:.2} s (target {} s)",
        avg_interval, params.target_block_time
    );
    let total_guard = guard_events.len() as f64;
    let guard_per_100 = if total_blocks == 0 {
        0.0
    } else {
        total_guard * 100.0 / total_blocks as f64
    };
    println!(
        "guard activations: {} total ({:.2} per 100 blocks)",
        guard_events.len(),
        guard_per_100
    );

    let steady_threshold = 0.05;
    let mut steady_blocks = 0.0;
    let mut steady_guard = 0.0;
    let mut steady_interval = 0.0;

    for stats in &segment_stats {
        let is_steady = (stats.hash_rate - 1.0).abs() <= steady_threshold;
        if is_steady {
            steady_blocks += stats.blocks as f64;
            steady_guard += stats.guard_triggers as f64;
            steady_interval += stats.interval_sum;
        }
    }

    if steady_blocks > 0.0 {
        println!(
            "steady-state avg: {:.2} s | guard {:.2} per 100 blocks",
            steady_interval / steady_blocks,
            (steady_guard * 100.0 / steady_blocks)
        );
    } else {
        println!("steady-state avg: n/a (no ~1x hash segments)");
    }

    if let Some((idx, stats)) = segment_stats
        .iter()
        .enumerate()
        .rev()
        .find(|(_, seg)| (seg.hash_rate - 1.0).abs() <= steady_threshold && seg.blocks > 0)
    {
        let avg_interval = stats.interval_sum / stats.blocks as f64;
        let guard_rate = stats.guard_triggers as f64 * 100.0 / stats.blocks as f64;
        println!(
            "final steady seg #{idx}: avg {:.2} s | guard {:.2} per 100 blocks",
            avg_interval, guard_rate
        );
    }

    let mut flapping = 0usize;
    let mut worst_ratio = 1.0_f64;
    for (idx, event) in guard_events.iter().enumerate() {
        worst_ratio = worst_ratio.min(event.ratio);
        if idx > 0 {
            let prev = guard_events[idx - 1];
            if event.height > prev.height
                && event.height - prev.height <= params.burst_guard_window / 2
            {
                flapping += 1;
            }
        }
    }

    println!(
        "flapping events  : {} (threshold ≤ {} blocks between triggers)",
        flapping,
        params.burst_guard_window / 2
    );
    if guard_events.is_empty() {
        println!("guard floor usage: none (all windows above floor ratio)");
    } else {
        println!(
            "minimum ratio    : {:.3} of expected window time",
            worst_ratio
        );
    }
    if flapping > 0 {
        println!(
            "note             : burst guard flapping detected; consider widening hysteresis \
             (e.g. release when ratio > 0.38)"
        );
    } else {
        println!("note             : no burst guard flapping detected");
    }

    println!("\n--- Segment breakdown ---");
    for (idx, stats) in segment_stats.iter().enumerate() {
        println!("seg {:>2}: {}", idx, stats);
    }

    if !guard_events.is_empty() {
        println!("\n--- Guard activations ---");
        for event in &guard_events {
            println!(
                "height {:>6} | ratio {:.3} | Δt={}s vs expected {:.1}s | segment {}",
                event.height,
                event.ratio,
                event.time_delta,
                event.expected_time,
                event.segment_index
            );
        }
    }

    Ok(())
}

fn parse_pattern(pattern: &str) -> Result<Vec<Segment>> {
    let mut segments = Vec::new();

    for part in pattern.split(',') {
        let entry = part.trim();
        if entry.is_empty() {
            continue;
        }
        let mut split = entry.split(':');
        let blocks_str = split
            .next()
            .ok_or_else(|| anyhow!("missing block count in pattern entry '{}'", entry))?;
        let rate_str = split
            .next()
            .ok_or_else(|| anyhow!("missing hash-rate in pattern entry '{}'", entry))?;

        if split.next().is_some() {
            return Err(anyhow!(
                "too many fields in pattern entry '{}', expected <blocks>:<hash>",
                entry
            ));
        }

        let blocks: u64 = blocks_str
            .parse()
            .with_context(|| format!("invalid block count '{}'", blocks_str))?;
        if blocks == 0 {
            return Err(anyhow!("segment '{}' has zero blocks", entry));
        }

        let hash_rate: f64 = rate_str
            .parse()
            .with_context(|| format!("invalid hash-rate '{}'", rate_str))?;
        if hash_rate <= 0.0 {
            return Err(anyhow!("segment '{}' must have positive hash-rate", entry));
        }

        segments.push(Segment { blocks, hash_rate });
    }

    Ok(segments)
}
