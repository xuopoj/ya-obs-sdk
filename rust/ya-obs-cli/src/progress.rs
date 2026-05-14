use indicatif::{ProgressBar, ProgressStyle};

pub fn bytes_bar(total: u64) -> ProgressBar {
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template(
            "{spinner} {bytes}/{total_bytes} ({bytes_per_sec}) [{wide_bar}] {percent}%",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    bar
}
