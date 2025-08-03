use std::{collections::HashSet, io::Read};

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, registry};

pub fn level_from_args(
    args: &std::collections::HashSet<&str>,
) -> tracing::level_filters::LevelFilter {
    const LEVELS: &[(&str, tracing::level_filters::LevelFilter)] = &[
        ("trace", tracing::level_filters::LevelFilter::TRACE),
        ("debug", tracing::level_filters::LevelFilter::DEBUG),
        ("info", tracing::level_filters::LevelFilter::INFO),
        ("warn", tracing::level_filters::LevelFilter::WARN),
        ("error", tracing::level_filters::LevelFilter::ERROR),
    ];

    for (name, level) in LEVELS {
        if args.contains(name) {
            return *level;
        }
    }
    tracing::level_filters::LevelFilter::INFO
}

pub fn is_llm() -> bool {
    std::env::var("LLM_AGENT").unwrap_or_default() == "1"
}

pub fn init_tracing() -> impl tracing::Subscriber + Send + Sync + 'static {
    let mut level = LevelFilter::INFO;
    if is_llm() {
        level = LevelFilter::OFF;
    }

    registry().with(fmt::layer().with_filter(level))
}

pub fn capture_traces(args: &HashSet<&str>, f: impl FnOnce()) -> String {
    unsafe { std::env::set_var("NO_COLOR", "1") }
    let mut reader = tempfile::NamedTempFile::new().unwrap();

    let writer = reader.reopen().unwrap();
    {
        let level = level_from_args(args);
        let mut console_level = level;
        let (writer, _guard) = tracing_appender::non_blocking(writer);

        let file_layer = fmt::Layer::new()
            .with_file(args.contains("file"))
            .with_line_number(args.contains("line"))
            .with_writer(writer)
            .without_time()
            .with_ansi(false);

        if is_llm() {
            console_level = LevelFilter::OFF;
        }
        let console_layer = fmt::Layer::new()
            .with_file(args.contains("file"))
            .with_line_number(args.contains("line"))
            .with_ansi(true);

        let subscriber = registry()
            .with(console_layer.with_filter(console_level))
            .with(file_layer.with_filter(level));

        tracing::subscriber::with_default(subscriber, f);
    }

    let mut buf = String::new();
    reader.read_to_string(&mut buf).unwrap();
    buf
}
