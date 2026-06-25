#[tokio::main]
async fn main() {
    if let Err(error) = hugdocker::cli::run().await {
        eprintln!("错误: {error}");
        std::process::exit(1);
    }
}
