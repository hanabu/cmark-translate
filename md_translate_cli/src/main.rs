#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    cmark_xml::parse_md().unwrap();
    Ok(())
}
