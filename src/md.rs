
pub fn parse_md() -> std::io::Result<()> {
    use std::io::BufRead;
    env_logger::init();

    let file = std::fs::File::open("index.en.md")?;
    let lines = std::io::BufReader::new(file).lines();
    let mut has_frontmatter = false;
    let mut in_frontmatter = false;
    let mut frontmatter = String::new();
    let mut body = String::new();

    // Read files
    for (i, line) in lines.enumerate() {
        if let Ok(line) = line {
            if i == 0 && line.starts_with("+++") {
                // begging of frontmatter
                has_frontmatter = true;
                in_frontmatter = true;
            } else if in_frontmatter && line.starts_with("+++") {
                // End of frontmatter
                in_frontmatter = false;
            } else {
                if in_frontmatter {
                    frontmatter += &line;
                    frontmatter += "\n";
                } else {
                    body += &line;
                    body += "\n";
                }
            }
        }
    }

    println!("frontmatter: {:?}\n\n", frontmatter);
    println!("body: {:?}\n\n", body);

    Ok(())
}
