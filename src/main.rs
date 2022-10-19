mod deepl;
mod md;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    md::parse_md();
    Ok(())
}
