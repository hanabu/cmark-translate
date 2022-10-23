mod cmark_xml;
mod deepl;
mod trans;

use clap::{CommandFactory, Parser};

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Translate a CommonMark file
    Translate {
        /// Source language (ISO639-1 2 letter code)
        #[arg(short, long)]
        from: String,
        /// Target language (ISO639-1 2 letter code)
        #[arg(short, long)]
        to: String,
        /// Input CommonMark file
        input: std::path::PathBuf,
        /// Output translated CommonMark file
        output: std::path::PathBuf,
    },
    /// Manage glossaries
    Glossary {
        #[command(subcommand)]
        command: GlossaryCommands,
    },
    /// Show DeepL usage
    Usage,
}

#[derive(clap::Subcommand)]
enum GlossaryCommands {
    /// Register glossary TSV file
    Register {
        /// Glossary name
        #[arg(short, long)]
        name: String,
        /// Source language (ISO639-1 2 letter code)
        #[arg(short, long)]
        from: String,
        /// Target language (ISO639-1 2 letter code)
        #[arg(short, long)]
        to: String,
        /// Input glossary TSV file
        input: std::path::PathBuf,
    },
    /// List registered glossaries
    List,
    /// Delete registered glossary
    Delete {
        /// ID of glossary
        id: String,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    use std::str::FromStr;
    env_logger::init();

    // parse commandline
    let cli = Cli::parse();

    // Load DeepL config
    let deepl = if let Some(cfg_file) = cli.config {
        deepl::Deepl::with_config(&cfg_file)
    } else {
        deepl::Deepl::new()
    }
    .unwrap();

    match cli.command {
        Some(Commands::Translate {
            from,
            to,
            input,
            output,
        }) => {
            // Translate CommonMark file
            let lang_from = deepl::Language::from_str(&from)?;
            let lang_to = deepl::Language::from_str(&to)?;

            trans::translate_cmark_file(&deepl, lang_from, lang_to, &input, &output).await?;
        }
        Some(Commands::Glossary { command }) => {
            // Glossary management
            match command {
                GlossaryCommands::Register {
                    name,
                    from,
                    to,
                    input,
                } => {
                    let from_lang = deepl::Language::from_str(&from)?;
                    let to_lang = deepl::Language::from_str(&to)?;
                    deepl
                        .register_glossaries_from_file(&name, from_lang, to_lang, input)
                        .await?;
                }
                GlossaryCommands::List => {
                    // List glossaries
                    let glossaries = deepl.list_glossaries().await.unwrap();
                    for glossary in glossaries {
                        println!("{:?}\n", glossary);
                    }
                }
                GlossaryCommands::Delete { id } => {
                    deepl.remove_glossary(&id).await.unwrap();
                }
            }
        }
        Some(Commands::Usage) => {
            let used_chars = deepl.get_usage().await.unwrap();
            println!("{} characters used.", used_chars);
        }
        _ => {
            // Print help
            Cli::command().print_help()?;
        }
    }

    Ok(())
}
