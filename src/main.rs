mod cmark_xml;
mod deepl;
mod glossary;
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
        /// Formality - formal or informal
        #[arg(long)]
        formality: Option<String>,
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
        /// Input glossary TSV file - First row should contain language codes
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    // parse commandline
    let cli = Cli::parse();

    // Load DeepL config
    let deepl = if let Some(cfg_file) = cli.config {
        deepl::Deepl::with_config(&cfg_file)
    } else {
        deepl::Deepl::new()
    };

    match cli.command {
        Some(Commands::Translate {
            from,
            to,
            formality,
            input,
            output,
        }) => {
            // Translate CommonMark file
            let lang_from = deepl::Language::from_str(&from)?;
            let lang_to = deepl::Language::from_str(&to)?;
            let formality = formality.map_or(Ok(deepl::Formality::Default), |f| {
                deepl::Formality::from_str(&f)
            })?;

            trans::translate_cmark_file(
                &deepl.unwrap(),
                lang_from,
                lang_to,
                formality,
                &input,
                &output,
            )
            .await?;
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

                    let glossaries = glossary::read_glossary(
                        input,
                        from_lang.as_langcode(),
                        to_lang.as_langcode(),
                    )
                    .unwrap();

                    let glossary = deepl
                        .unwrap()
                        .register_glossaries(&name, from_lang, to_lang, &glossaries)
                        .await
                        .unwrap();
                    println!(
                        "Total {} entries are registered as ID = {}",
                        glossary.entry_count, glossary.glossary_id
                    );
                }
                GlossaryCommands::List => {
                    // List glossaries
                    let glossaries = deepl.unwrap().list_glossaries().await.unwrap();
                    for glossary in glossaries {
                        println!("{:?}\n", glossary);
                    }
                }
                GlossaryCommands::Delete { id } => {
                    deepl.unwrap().remove_glossary(&id).await.unwrap();
                }
            }
        }
        Some(Commands::Usage) => {
            let used_chars = deepl.unwrap().get_usage().await.unwrap();
            println!("{} characters used.", used_chars);
        }
        _ => {
            // Print help
            Cli::command().print_help()?;
        }
    }

    Ok(())
}
