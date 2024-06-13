use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use indexmap::{
    map::{Keys, Values},
    IndexMap,
};
use regex::Regex;
use req::*;
use schemars::schema_for;
use stringlit::s;

pub const WORD_DESCRIPTION: &str = //
    r#"The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED",
"MAY", and "OPTIONAL" in this document are to be interpreted as described in
[RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).
"#;

pub const HIGHLIGHTED_WORDS: [&str; 10] = [
    "must not",
    "must",
    "required",
    "shall not",
    "shall",
    "should not",
    "should",
    "recommended",
    "may",
    "optional",
];

fn nl() -> String {
    s!("")
}

fn check_requirements(
    test_results: &str,
    output: &mut IndexMap<String, (bool, Vec<String>)>,
    requirements: &IndexMap<String, Requirement>,
    allowed_requirements: &[Regex],
) {
    for (id, _) in requirements {
        if allowed_requirements.iter().any(|r| r.is_match(id)) {
            let search_string = format!("{}: failed", id.trim());
            if test_results.contains(&search_string) {
                let errors = test_results.lines().filter_map(|l| {
                    if l.starts_with(&search_string) {
                        l.split_once(":")
                            .map(|(_, txt)| txt)
                            .and_then(|txt| txt.split_once("-").map(|(_, err)| err.to_string()))
                    } else {
                        None
                    }
                });
                output.insert(id.trim().to_string(), (false, errors.collect()));
            } else if test_results.contains(&format!("{}: passed", id.trim())) {
                output
                    .entry(id.trim().to_string())
                    .or_insert((true, Vec::new()));
            };
        }
    }
}

fn has_valid_requirements(
    mut requirements: Keys<String, Requirement>,
    allowed_requirements: &[Regex],
) -> bool {
    requirements.any(|id| allowed_requirements.iter().any(|r| r.is_match(id)))
}

fn has_valid_topics(mut topics: Values<String, Topic>, allowed_requirements: &[Regex]) -> bool {
    topics.any(|topic| {
        has_valid_requirements(topic.requirements.keys(), allowed_requirements)
            || has_valid_topics(topic.subtopics.values(), allowed_requirements)
    })
}

fn check_topics(
    test_results: &[PathBuf],
    output: &mut Vec<String>,
    topics: &IndexMap<String, Topic>,
    allowed_requirements: &[Regex],
    level: usize,
) -> anyhow::Result<()> {
    if !has_valid_topics(topics.values(), allowed_requirements) {
        return Ok(());
    }
    for (id, topic) in topics {
        if !has_valid_topics(topic.subtopics.values(), allowed_requirements)
            && !has_valid_requirements(topic.requirements.keys(), allowed_requirements)
        {
            continue;
        }
        output.push(format!(
            "{} _{}_ - {}",
            "#".repeat(level),
            id.trim(),
            topic.name
        ));

        let mut test_status = IndexMap::new();
        for test_result in test_results {
            let test_result = std::fs::read_to_string(test_result)?;
            if !topic.requirements.is_empty() {
                check_requirements(
                    &test_result,
                    &mut test_status,
                    &topic.requirements,
                    allowed_requirements,
                );
            }
        }

        if !topic.requirements.is_empty() {
            for (id, req) in &topic.requirements {
                let (status, errors) = if let Some((status, errors)) = test_status.get(id) {
                    if *status {
                        (":white_check_mark:", errors.to_owned())
                    } else {
                        (":x:", errors.to_owned())
                    }
                } else {
                    (":warning:", Vec::new())
                };
                output.push(format!("- _{}_ - {}: {status}", id.trim(), req.name));
                for err in errors {
                    output.push(format!("  - {}", err.trim()));
                }
            }

            output.push(nl());
        }

        if !topic.subtopics.is_empty() {
            check_topics(
                test_results,
                output,
                &topic.subtopics,
                allowed_requirements,
                level + 1,
            )?;
            output.push(nl());
        }
    }
    Ok(())
}

fn add_requirements(output: &mut Vec<String>, requirements: &IndexMap<String, Requirement>) {
    for (id, requirement) in requirements {
        output.push(format!(
            "- **_{}_ - {}:** {}",
            id.trim(),
            requirement.name.trim(),
            requirement.description.trim()
        ));
        for info in &requirement.additional_info {
            output.push(format!("  - {}", info.trim(),));
        }
    }
}

fn add_topics(output: &mut Vec<String>, topics: &IndexMap<String, Topic>, level: usize) {
    for (id, topic) in topics {
        output.push(format!(
            "{} _{}_ - {}",
            "#".repeat(level),
            id.trim(),
            topic.name.trim()
        ));
        if !topic.requirements.is_empty() {
            add_requirements(output, &topic.requirements);
            output.push(nl());
        }
        if !topic.subtopics.is_empty() {
            add_topics(output, &topic.subtopics, level + 1);
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Outputs the JSON schema for the input data
    Schema,
    /// Outputs demo data in YAML format
    Demo,
    #[clap(alias = "md")]
    /// Transform requirements into Markdown
    Markdown {
        /// The path to the requirements file
        requirements: PathBuf,
    },
    /// Transform requirements into HTML
    Html {
        /// The path to the requirements file
        requirements: PathBuf,
    },
    /// Check test output against requirements
    Check {
        #[arg(short, long, default_value = "REQ-.*")]
        /// Regex to select which requirements should be checked
        allowed_requirements: Vec<String>,
        /// The path to the requirements file
        requirements: PathBuf,
        /// The path to the test output files
        #[arg(required=true, num_args=1..)]
        test_results: Vec<PathBuf>,
    },
    /// Generate shell completions
    Completions {
        /// The shell to generate the completions for
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
}

#[derive(Parser)]
#[command(version)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

fn parse(value: &str) -> anyhow::Result<Project> {
    Ok(serde_yaml::from_str(value)
        .or_else(|_| serde_json::from_str(value))
        .or_else(|_| rsn::from_str(value))
        .or_else(|_| toml::from_str(value))?)
}

fn to_markdown(requirements: PathBuf, add_toc: bool) -> anyhow::Result<String> {
    let project: Project = parse(&std::fs::read_to_string(requirements)?)?;

    let mut output = vec![format!("# Requirements for {}", project.name.trim()), nl()];
    if add_toc {
        output.extend([s!("[[_TOC_]]"), nl()]);
    }
    output.extend([
        WORD_DESCRIPTION.trim().to_string(),
        nl(),
        format!("**VERSION: {}**", project.version),
        nl(),
        s!("## Description"),
        project.description.trim().to_string(),
        nl(),
    ]);

    if !project.topics.is_empty() {
        output.push(s!("## Requirements"));
        add_topics(&mut output, &project.topics, 3);
    }

    if !project.definitions.is_empty() {
        output.push(s!("## Definitions"));
        for definition in project.definitions {
            output.push(format!(
                "- {}: {}",
                definition.name.trim(),
                definition.value.trim()
            ));
            for info in definition.additional_info {
                output.push(format!("  - {}", info.trim()))
            }
        }
        output.push(nl());
    }

    if !project.config_defaults.is_empty() {
        output.push(s!("## Config Defaults"));
        for default in project.config_defaults {
            output.push(format!("- **{}**", default.name.trim()));
            output.push(format!("  - Type: {}", default.typ.trim()));
            if let Some(unit) = default.unit {
                output.push(format!("  - Unit: {}", unit.trim()));
            }
            if let Some(valid_values) = default.valid_values {
                output.push(format!(
                    "  - Valid Values: _{}_",
                    valid_values.join(", ").trim()
                ));
            }
            if let Some(default_value) = default.default_value {
                output.push(format!(
                    "  - Default Value: _{}_{}",
                    default_value.trim(),
                    default
                        .hint
                        .map(|h| format!(" {}", h.trim()))
                        .unwrap_or_default()
                ));
            } else {
                output.push(format!(
                    "  - **Required**: This value **_MUST_** be provided as a start parameter.{}",
                    default
                        .hint
                        .map(|h| format!(" {}", h.trim()))
                        .unwrap_or_default()
                ));
            }
            output.push(nl());
        }
    }

    let mut output = output.join("\n");
    for word in HIGHLIGHTED_WORDS {
        output = output.replace(word, &format!("**_{}_**", word.to_uppercase()));
    }
    Ok(output)
}

fn main() -> anyhow::Result<()> {
    let Args { command } = Args::parse();
    match command {
        Command::Demo => {
            println!("{}", serde_yaml::to_string(&demo_project())?);
        }
        Command::Html { requirements } => {
            let output = to_markdown(requirements, false)?;
            let template = include_str!("../template.html");
            println!(
                "{}",
                template.replace(
                    "{{content}}",
                    &markdown::to_html_with_options(&output, &markdown::Options::gfm())
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                )
            );
        }
        Command::Schema => {
            let schema = schema_for!(Project);
            println!("{}", serde_json::to_string_pretty(&schema).unwrap());
        }
        Command::Markdown { requirements } => {
            let output = to_markdown(requirements, true)?;
            println!("{output}");
        }
        Command::Check {
            allowed_requirements,
            requirements,
            test_results,
        } => {
            let re = allowed_requirements
                .into_iter()
                .map(|r| Regex::new(&r).expect("Invalid regex!"));
            let re: Vec<_> = re.collect();
            let project: Project = parse(&std::fs::read_to_string(requirements)?)?;
            let mut output = vec![format!("# Test Results - {}", project.name)];
            check_topics(&test_results, &mut output, &project.topics, &re, 2)?;

            let output = output.join("\n");
            println!("{output}");
        }
        Command::Completions { shell } => {
            shell.generate(&mut Args::command(), &mut std::io::stdout());
        }
    }

    Ok(())
}
