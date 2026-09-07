use std::fs;
use std::io::{self, Read};
use std::process;

use clap::{Parser, Subcommand};
use logcompress::{compress, CompressConfig, CompressResult};

#[derive(Parser)]
#[command(
    name = "logcompress",
    version,
    about = "Query-aware log compression for LLM context"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search logs with a natural language query
    Search {
        /// Search query
        #[arg(short, long)]
        query: String,

        /// Log file path (use - for stdin)
        #[arg(short, long)]
        file: String,

        /// Max result lines
        #[arg(long, default_value_t = 50)]
        top: usize,

        /// Context lines around each hit (±N)
        #[arg(long, default_value_t = 2)]
        context: usize,

        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,

        /// Print stats after output
        #[arg(long)]
        stats: bool,

        /// Disable trace-based expansion
        #[arg(long)]
        no_trace: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search {
            query,
            file,
            top,
            context,
            format,
            stats,
            no_trace,
        } => {
            let input = read_input(&file);
            let config = CompressConfig {
                top_k: top,
                context_lines: context,
                trace_expansion: !no_trace,
                ..Default::default()
            };

            let result = compress(&query, &input, &config);

            match format.as_str() {
                "json" => print_json(&result),
                _ => print_text(&result),
            }

            if stats {
                print_stats(&result);
            }
        }
    }
}

fn read_input(file: &str) -> String {
    if file == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        });
        buf
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", file, e);
            process::exit(1);
        })
    }
}

fn print_text(result: &CompressResult) {
    for line in &result.lines {
        println!("{:>6} | {}", line.line_number, line.content);
    }
}

fn print_json(result: &CompressResult) {
    println!("{}", serde_json::to_string_pretty(result).unwrap());
}

fn print_stats(result: &CompressResult) {
    eprintln!("---");
    eprintln!(
        "Input: {} lines | Output: {} lines | Ratio: {:.1}% | Time: {:.1}ms",
        result.stats.input_lines,
        result.stats.output_lines,
        result.stats.compression_ratio * 100.0,
        result.elapsed_us as f64 / 1000.0,
    );
    eprintln!(
        "Query terms: {} | Candidates: {}",
        result.stats.unique_query_terms, result.stats.candidate_lines,
    );
}
