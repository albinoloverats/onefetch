extern crate bytecount;
extern crate colored;
extern crate git2;
extern crate license;
extern crate tokei;

use colored::Color;
use colored::*;
use git2::Repository;
use license::License;
use std::{
    cmp,
    collections::HashMap,
    convert::From,
    env,
    ffi::OsStr,
    fmt,
    fmt::Write,
    fs,
    process::{Command, Stdio},
    result,
    str::FromStr,
};

type Result<T> = result::Result<T, Error>;

struct Info {
    project_name: String,
    version: String,
    dominant_language: Language,
    languages: Vec<(Language, f64)>,
    authors: Vec<String>,
    last_change: String,
    repo: String,
    commits: String,
    number_of_lines: usize,
    license: String,
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buffer = String::new();
        let color = match self.colors().get(0) {
            Some(&c) => c,
            None => Color::White,
        };

        writeln!(
            buffer,
            "{}{}",
            "Project: ".color(color).bold(),
            self.project_name
        )?;

        writeln!(
            buffer,
            "{}{}",
            "Version: ".color(color).bold(),
            self.version
        )?;

        if !self.languages.is_empty() {
            if self.languages.len() > 1 {
                let title = "Languages: ";
                let pad = " ".repeat(title.len());
                let mut s = String::from("");
                for (cnt, language) in self.languages.iter().enumerate() {
                    let formatted_number = format!("{:.*}", 2, language.1);
                    if cnt != 0 && cnt % 3 == 0 {
                        s = s + &format!("\n{}{} ({} %) ", pad, language.0, formatted_number);
                    } else {
                        s = s + &format!("{} ({} %) ", language.0, formatted_number);
                    }
                }
                writeln!(buffer, "{}{}", title.color(color).bold(), s)?;
            } else {
                let title = "Language: ";
                writeln!(
                    buffer,
                    "{}{}",
                    title.color(color).bold(),
                    self.dominant_language
                )?;
            };
        }

        if !self.authors.is_empty() {
            let title = if self.authors.len() > 1 {
                "Authors: "
            } else {
                "Author: "
            };

            writeln!(buffer, "{}{}", title.color(color).bold(), self.authors[0])?;

            let title = " ".repeat(title.len());

            for author in self.authors.iter().skip(1) {
                writeln!(buffer, "{}{}", title.color(color).bold(), author)?;
            }
        }

        writeln!(
            buffer,
            "{}{}",
            "Last change: ".color(color).bold(),
            self.last_change
        )?;

        writeln!(buffer, "{}{}", "Repo: ".color(color).bold(), self.repo)?;
        writeln!(
            buffer,
            "{}{}",
            "Commits: ".color(color).bold(),
            self.commits
        )?;
        writeln!(
            buffer,
            "{}{}",
            "Lines of code: ".color(color).bold(),
            self.number_of_lines
        )?;
        writeln!(
            buffer,
            "{}{}",
            "License: ".color(color).bold(),
            self.license
        )?;

        let logo = self.get_ascii();
        let mut logo_lines = logo.lines();
        let mut info_lines = buffer.lines();
        let left_pad = logo.lines().map(|l| true_len(l)).max().unwrap_or(0);

        for _ in 0..cmp::max(count_newlines(logo), count_newlines(&buffer)) {
            let logo_line = match logo_lines.next() {
                Some(line) => line,
                None => "",
            };

            let info_line = match info_lines.next() {
                Some(line) => line,
                None => "",
            };

            let (logo_line, extra_pad) = colorize_str(logo_line, self.colors());
            // If the string is empty the extra padding should not be added
            let pad = if logo_line.is_empty() {
                left_pad
            } else {
                left_pad + extra_pad
            };
            writeln!(f, "{:<width$} {:^}", logo_line, info_line, width = pad,)?;
        }

        Ok(())
    }
}

fn count_newlines(s: &str) -> usize {
    bytecount::count(s.as_bytes(), b'\n')
}

/// Transforms a string with color format into one with proper
/// escape characters for color display.
///
/// Colors are specified with {0}, {1}... where the number represents
/// the nth element in the colors Vec provided to the function.  
/// If there are more colors in the ascii than in the Vec it
/// defaults to white.  
/// The usize in the tuple refers to the extra padding needed
/// which comes from the added escape characters.
fn colorize_str(line: &str, colors: Vec<Color>) -> (String, usize) {
    // Extract colors from string coded with {n}
    let mut colors_in_str: Vec<Color> = line.split('{').fold(Vec::new(), |mut acc, s| {
        if s.len() > 2 {
            let i = s.chars().nth(0).unwrap_or('0').to_digit(10).unwrap_or(0);
            acc.push(*colors.get(i as usize).unwrap_or(&Color::White));
        }
        acc
    });

    if colors_in_str.is_empty() {
        colors_in_str.push(match colors.get(0) {
            Some(&c) => c,
            None => Color::White,
        });
    }

    let mut colors_iter = colors_in_str.iter();

    let out_str = line.split('{').fold(String::new(), |mut acc, s| {
        if s.len() > 2 {
            let s: String = s.chars().skip(2).collect();
            let c = match colors_iter.next() {
                Some(&c) => c,
                None => Color::White,
            };
            acc.push_str(&format!("{}", s.color(c)));
        }
        acc
    });
    (out_str, colors_in_str.len() * 9)
}

/// Returns the true length of a string after substracting the {n}
/// color declarations.
fn true_len(line: &str) -> usize {
    line.split('{')
        .fold(String::new(), |mut acc, s| {
            if s.len() > 2 {
                acc.push_str(&s.chars().skip(2).collect::<String>());
            } else {
                acc.push_str(s);
            }
            acc
        })
        .len()
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum Language {
    Assembly,
    C,
    Clojure,
    Cpp,
    Csharp,
    Go,
    Haskell,
    Java,
    Lisp,
    Lua,
    Python,
    R,
    Ruby,
    Rust,
    Scala,
    Shell,
    Swift,
    TypeScript,
    JavaScript,
    Php,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Language::Assembly => write!(f, "Assembly"),
            Language::C => write!(f, "C"),
            Language::Clojure => write!(f, "Clojure"),
            Language::Cpp => write!(f, "C++"),
            Language::Csharp => write!(f, "C#"),
            Language::Go => write!(f, "Go"),
            Language::Haskell => write!(f, "Haskell"),
            Language::Java => write!(f, "Java"),
            Language::Lisp => write!(f, "Lisp"),
            Language::Lua => write!(f, "Lua"),
            Language::Python => write!(f, "Python"),
            Language::R => write!(f, "R"),
            Language::Ruby => write!(f, "Ruby"),
            Language::Rust => write!(f, "Rust"),
            Language::Scala => write!(f, "Scala"),
            Language::Shell => write!(f, "Shell"),
            Language::Swift => write!(f, "Swift"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::JavaScript => write!(f, "JavaScript"),
            Language::Php => write!(f, "Php"),
        }
    }
}

fn main() -> Result<()> {
    if !is_git_installed() {
        return Err(Error::GitNotInstalled);
    }

    let mut args = env::args();

    if args.next().is_none() {
        return Err(Error::TooFewArgs);
    };

    let dir = if let Some(arg) = args.next() {
        arg
    } else {
        String::from(".")
    };

    if args.next().is_some() {
        return Err(Error::TooManyArgs);
    };

    let tokei_langs = project_languages(&dir);
    let languages_stat = get_languages_stat(&tokei_langs).ok_or(Error::SourceCodeNotFound)?;
    let mut languages_stat_vec: Vec<(_, _)> = languages_stat.into_iter().collect();
    languages_stat_vec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().reverse());
    let dominant_language = languages_stat_vec[0].0.clone();

    let authors = get_authors(&dir, 3);
    let config = get_configuration(&dir)?;
    let version = get_version(&dir)?;
    let commits = get_commits(&dir)?;
    let last_change = get_last_change(&dir)?;

    let info = Info {
        project_name: config.repository_name,
        version,
        dominant_language,
        languages: languages_stat_vec,
        authors,
        last_change,
        repo: config.repository_url,
        commits,
        number_of_lines: get_total_loc(&tokei_langs),
        license: project_license(&dir)?,
    };

    println!("{}", info);
    Ok(())
}

fn project_languages(dir: &str) -> tokei::Languages {
    let mut languages = tokei::Languages::new();
    let required_languages = get_all_language_types();
    languages.get_statistics(&[&dir], vec![".git", "target"], Some(required_languages));
    languages
}

fn get_languages_stat(languages: &tokei::Languages) -> Option<HashMap<Language, f64>> {
    let mut stats = HashMap::new();

    let sum_language_code: usize = languages.remove_empty().iter().map(|(_, v)| v.code).sum();

    if sum_language_code == 0 {
        None
    } else {
        for (k, v) in languages.remove_empty().iter() {
            let code = v.code as f64;
            stats.insert(
                Language::from(**k),
                (code / sum_language_code as f64) * 100.00,
            );
        }
        Some(stats)
    }
}

fn project_license(dir: &str) -> Result<String> {
    let output = fs::read_dir(dir)
        .map_err(|_| Error::ReadDirectory)?
        .filter_map(result::Result::ok)
        .map(|entry| entry.path())
        .filter(
            |entry| {
                entry.is_file()
                    && entry
                        .file_name()
                        .map(OsStr::to_string_lossy)
                        .unwrap_or_else(|| "".into())
                        .starts_with("LICENSE")
                    || entry
                        .file_name()
                        .map(OsStr::to_string_lossy)
                        .unwrap_or_else(|| "".into())
                        .starts_with("LICENCE")
            }, // TODO: multiple prefixes, like COPYING?
        )
        .map(|entry| {
            license::Kind::from_str(&fs::read_to_string(entry).unwrap_or_else(|_| "".into()))
        })
        .filter_map(result::Result::ok)
        .map(|license| license.name().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    if output == "" {
        Ok("??".into())
    } else {
        Ok(output)
    }
}

fn get_version(dir: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("describe")
        .arg("--abbrev=0")
        .arg("--tags")
        .output()
        .expect("Failed to execute git.");

    let output = String::from_utf8_lossy(&output.stdout);

    if output == "" {
        Ok("??".into())
    } else {
        Ok(output.to_string().replace('\n', ""))
    }
}

fn get_last_change(dir: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("log")
        .arg("-1")
        .arg("--format=%cr")
        .output()
        .expect("Failed to execute git.");

    let output = String::from_utf8_lossy(&output.stdout);

    if output == "" {
        Ok("??".into())
    } else {
        Ok(output.to_string().replace('\n', ""))
    }
}

fn get_commits(dir: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-list")
        .arg("--count")
        .arg("HEAD")
        .output()
        .expect("Failed to execute git.");

    let output = String::from_utf8_lossy(&output.stdout);

    if output == "" {
        Ok("0".into())
    } else {
        Ok(output.to_string().replace('\n', ""))
    }
}

fn is_git_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .status()
        .is_ok()
}

#[derive(Debug)]
struct Configuration {
    pub repository_name: String,
    pub repository_url: String,
}

fn get_configuration(dir: &str) -> Result<Configuration> {
    let repo = Repository::open(dir).map_err(|_| Error::NotGitRepo)?;
    let config = repo.config().map_err(|_| Error::NoGitData)?;
    let mut remote_url = String::new();
    let mut repository_name = String::new();
    let mut remote_upstream: Option<String> = None;

    for entry in &config.entries(None).unwrap() {
        let entry = entry.unwrap();
        match entry.name().unwrap() {
            "remote.origin.url" => remote_url = entry.value().unwrap().to_string(),
            "remote.upstream.url" => remote_upstream = Some(entry.value().unwrap().to_string()),
            _ => (),
        }
    }

    if let Some(url) = remote_upstream {
        remote_url = url.clone();
    }

    let url = remote_url.clone();
    let name_parts: Vec<&str> = url.split('/').collect();

    if !name_parts.is_empty() {
        repository_name = name_parts[name_parts.len() - 1].to_string();
        if repository_name.is_empty() {
            repository_name = name_parts[name_parts.len() - 2].to_string();
        }
    }

    if repository_name.contains(".git") {
        let repo_name = repository_name.clone();
        let parts: Vec<&str> = repo_name.split(".git").collect();
        repository_name = parts[0].to_string();
    }

    Ok(Configuration {
        repository_name: repository_name.clone(),
        repository_url: name_parts.join("/"),
    })
}

// Return first n most active commiters as authors within this project.
fn get_authors(dir: &str, n: usize) -> Vec<String> {
    use std::collections::HashMap;
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("log")
        .arg("--format='%aN'")
        .output()
        .expect("Failed to execute git.");

    // create map for storing author name as a key and their commit count as value
    let mut authors = HashMap::new();
    let output = String::from_utf8_lossy(&output.stdout);
    for line in output.lines() {
        let commit_count = authors.entry(line.to_string()).or_insert(0);
        *commit_count += 1;
    }

    // sort authors by commit count where the one with most commit count is first
    let mut authors: Vec<(String, usize)> = authors.into_iter().collect();
    authors.sort_by_key(|(_, c)| *c);
    authors.reverse();

    // truncate the vector so we only get the count of authors we specified as 'n'
    authors.truncate(n);

    // get only authors without their commit count
    // and string "'" prefix and suffix
    let authors: Vec<String> = authors
        .into_iter()
        .map(|(author, _)| author.trim_matches('\'').to_string())
        .collect();

    authors
}

fn get_total_loc(languages: &tokei::Languages) -> usize {
    languages
        .values()
        .collect::<Vec<&tokei::Language>>()
        .iter()
        .fold(0, |sum, val| sum + val.code)
}

/// Convert from tokei LanguageType to known Language type .
impl From<tokei::LanguageType> for Language {
    fn from(language: tokei::LanguageType) -> Self {
        match language {
            tokei::LanguageType::Assembly => Language::Assembly,
            tokei::LanguageType::C => Language::C,
            tokei::LanguageType::Clojure => Language::Clojure,
            tokei::LanguageType::Cpp => Language::Cpp,
            tokei::LanguageType::CSharp => Language::Csharp,
            tokei::LanguageType::Go => Language::Go,
            tokei::LanguageType::Haskell => Language::Haskell,
            tokei::LanguageType::Java => Language::Java,
            tokei::LanguageType::Lisp => Language::Lisp,
            tokei::LanguageType::Lua => Language::Lua,
            tokei::LanguageType::Python => Language::Python,
            tokei::LanguageType::R => Language::R,
            tokei::LanguageType::Ruby => Language::Ruby,
            tokei::LanguageType::Rust => Language::Rust,
            tokei::LanguageType::Scala => Language::Scala,
            tokei::LanguageType::Sh => Language::Shell,
            tokei::LanguageType::Swift => Language::Swift,
            tokei::LanguageType::TypeScript => Language::TypeScript,
            tokei::LanguageType::JavaScript => Language::JavaScript,
            tokei::LanguageType::Php => Language::Php,
            _ => unimplemented!(),
        }
    }
}

fn get_all_language_types() -> Vec<tokei::LanguageType> {
    vec![
        tokei::LanguageType::Assembly,
        tokei::LanguageType::C,
        tokei::LanguageType::Clojure,
        tokei::LanguageType::Cpp,
        tokei::LanguageType::CSharp,
        tokei::LanguageType::Go,
        tokei::LanguageType::Haskell,
        tokei::LanguageType::Java,
        tokei::LanguageType::Lisp,
        tokei::LanguageType::Lua,
        tokei::LanguageType::Python,
        tokei::LanguageType::R,
        tokei::LanguageType::Ruby,
        tokei::LanguageType::Rust,
        tokei::LanguageType::Scala,
        tokei::LanguageType::Sh,
        tokei::LanguageType::Swift,
        tokei::LanguageType::TypeScript,
        tokei::LanguageType::JavaScript,
        tokei::LanguageType::Php,
    ]
}

impl Info {
    pub fn get_ascii(&self) -> &str {
        match self.dominant_language {
            Language::Assembly => include_str!("../resources/assembly.ascii"),
            Language::C => include_str!("../resources/c.ascii"),
            Language::Clojure => include_str!("../resources/clojure.ascii"),
            Language::Cpp => include_str!("../resources/cpp.ascii"),
            Language::Csharp => include_str!("../resources/csharp.ascii"),
            Language::Go => include_str!("../resources/go.ascii"),
            Language::Haskell => include_str!("../resources/haskell.ascii"),
            Language::Java => include_str!("../resources/java.ascii"),
            Language::Lisp => include_str!("../resources/lisp.ascii"),
            Language::Lua => include_str!("../resources/lua.ascii"),
            Language::Python => include_str!("../resources/python.ascii"),
            Language::R => include_str!("../resources/r.ascii"),
            Language::Ruby => include_str!("../resources/ruby.ascii"),
            Language::Rust => include_str!("../resources/rust.ascii"),
            Language::Scala => include_str!("../resources/scala.ascii"),
            Language::Shell => include_str!("../resources/shell.ascii"),
            Language::Swift => include_str!("../resources/swift.ascii"),
            Language::TypeScript => include_str!("../resources/typescript.ascii"),
            Language::JavaScript => include_str!("../resources/javascript.ascii"),
            Language::Php => include_str!("../resources/php.ascii"),
            // _ => include_str!("../resources/unknown.ascii"),
        }
    }

    fn colors(&self) -> Vec<Color> {
        match self.dominant_language {
            Language::Assembly => vec![Color::Cyan],
            Language::C => vec![Color::BrightBlue, Color::Blue],
            Language::Clojure => vec![Color::BrightBlue, Color::BrightGreen],
            Language::Cpp => vec![Color::Yellow, Color::Cyan],
            Language::Csharp => vec![Color::White],
            Language::Go => vec![Color::White],
            Language::Haskell => vec![Color::BrightBlue, Color::BrightMagenta, Color::Blue],
            Language::Java => vec![Color::BrightBlue, Color::Red],
            Language::Lisp => vec![Color::Yellow],
            Language::Lua => vec![Color::Blue],
            Language::Python => vec![Color::Blue, Color::Yellow],
            Language::R => vec![Color::White, Color::Blue],
            Language::Ruby => vec![Color::Magenta],
            Language::Rust => vec![Color::White, Color::BrightRed],
            Language::Scala => vec![Color::Blue],
            Language::Shell => vec![Color::Green],
            Language::Swift => vec![Color::BrightRed],
            Language::TypeScript => vec![Color::Cyan],
            Language::JavaScript => vec![Color::BrightYellow],
            Language::Php => vec![Color::BrightWhite],
        }
    }
}

/// Custom error type
enum Error {
    /// Sourcecode could be located
    SourceCodeNotFound,
    /// Git is not installed or did not function properly
    GitNotInstalled,
    /// Did not find any git data in the directory
    NoGitData,
    /// An IO error occoured while reading ./
    ReadDirectory,
    /// Not in a Git Repo
    NotGitRepo,
    /// Too few arguments
    TooFewArgs,
    /// Too many arguments
    TooManyArgs,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let content = match self {
            Error::SourceCodeNotFound => "Could not find any source code in this directory",
            Error::GitNotInstalled => "Git failed to execute",
            Error::NoGitData => "Could not retrieve git configuration data",
            Error::ReadDirectory => "Could not read directory",
            Error::NotGitRepo => "This is not a Git Repo",
            Error::TooFewArgs => "Too few arguments. Expected program name and a single argument.",
            Error::TooManyArgs => "Too many arguments. Expected a single argument.",
        };
        write!(f, "{}", content)
    }
}
