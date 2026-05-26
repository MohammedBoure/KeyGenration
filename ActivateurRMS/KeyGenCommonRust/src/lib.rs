use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

pub const APP_TYPES: [&str; 3] = ["Restaurant", "Lab", "Jewelry"];
pub const ENV_PATH_VARIABLE: &str = "KEYGEN_ENV_FILE";

#[derive(Clone, Debug)]
pub struct RuntimeEnvironment {
    path: PathBuf,
    values: BTreeMap<String, String>,
}

impl RuntimeEnvironment {
    pub fn load() -> io::Result<Self> {
        let path = environment_file_path()?;
        let values = if path.is_file() {
            parse_dotenv(&fs::read_to_string(&path)?)?
        } else {
            BTreeMap::new()
        };
        Ok(Self { path, values })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn exists(&self) -> bool {
        self.path.is_file()
    }

    pub fn value(&self, name: &str) -> Option<String> {
        env::var(name)
            .ok()
            .or_else(|| self.values.get(name).cloned())
    }
}

pub fn env_assignment(name: &str, value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("{name}=\"{escaped}\"\n")
}

fn environment_file_path() -> io::Result<PathBuf> {
    if let Some(path) = env::var_os(ENV_PATH_VARIABLE) {
        return Ok(PathBuf::from(path));
    }
    env::current_exe()?
        .parent()
        .map(|directory| directory.join(".env"))
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "executable folder is unavailable"))
}

fn parse_dotenv(contents: &str) -> io::Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for (index, source_line) in contents.lines().enumerate() {
        let line = source_line.trim().trim_start_matches('\u{feff}').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let (name, source_value) = line.split_once('=').ok_or_else(|| {
            invalid_line(index, "expected a KEY=VALUE assignment in the .env file")
        })?;
        let name = name.trim();
        if !valid_name(name) {
            return Err(invalid_line(index, "invalid environment variable name"));
        }
        values.insert(name.to_owned(), parse_value(source_value.trim(), index)?);
    }
    Ok(values)
}

fn parse_value(value: &str, index: usize) -> io::Result<String> {
    if let Some(remainder) = value.strip_prefix('"') {
        return parse_quoted_value(remainder, '"', true, index);
    }
    if let Some(remainder) = value.strip_prefix('\'') {
        return parse_quoted_value(remainder, '\'', false, index);
    }
    Ok(value.trim().to_owned())
}

fn parse_quoted_value(
    value: &str,
    quote: char,
    expand_escapes: bool,
    index: usize,
) -> io::Result<String> {
    let Some(end) = value.rfind(quote) else {
        return Err(invalid_line(index, "unterminated quoted value"));
    };
    let trailing = value[end + quote.len_utf8()..].trim();
    if !trailing.is_empty() && !trailing.starts_with('#') {
        return Err(invalid_line(index, "unexpected data after quoted value"));
    }
    let contents = &value[..end];
    if !expand_escapes {
        return Ok(contents.to_owned());
    }
    let mut parsed = String::new();
    let mut characters = contents.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            parsed.push(character);
            continue;
        }
        match characters.next() {
            Some('\\') => parsed.push('\\'),
            Some('"') => parsed.push('"'),
            Some(character) => {
                parsed.push('\\');
                parsed.push(character);
            }
            None => parsed.push('\\'),
        }
    }
    Ok(parsed)
}

fn valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn invalid_line(index: usize, message: &str) -> io::Error {
    io::Error::new(
        ErrorKind::InvalidData,
        format!(".env line {}: {message}", index + 1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_settings_and_tokens_with_equals() {
        let values = parse_dotenv(
            "# package settings\n\
             KEYGEN_CLOUD_API_URL=https://keys.example.test\n\
             KEYGEN_API_SECRET_TOKEN=\"abc_def==\"\n\
             export KEYGEN_LISTEN_ADDRESS='127.0.0.1:45632'\n",
        )
        .unwrap();

        assert_eq!(
            values.get("KEYGEN_CLOUD_API_URL").map(String::as_str),
            Some("https://keys.example.test")
        );
        assert_eq!(
            values.get("KEYGEN_API_SECRET_TOKEN").map(String::as_str),
            Some("abc_def==")
        );
        assert_eq!(
            values.get("KEYGEN_LISTEN_ADDRESS").map(String::as_str),
            Some("127.0.0.1:45632")
        );
    }

    #[test]
    fn generated_assignments_round_trip_special_values() {
        let contents = env_assignment("KEYGEN_API_SECRET_TOKEN", "a=\"b\"\\c");
        let values = parse_dotenv(&contents).unwrap();
        assert_eq!(
            values.get("KEYGEN_API_SECRET_TOKEN").map(String::as_str),
            Some("a=\"b\"\\c")
        );
    }

    #[test]
    fn preserves_windows_paths_in_quoted_values() {
        let values =
            parse_dotenv("KEYGEN_DATA_DIR=\"C:\\ProgramData\\target\\runtime\"\n").unwrap();
        assert_eq!(
            values.get("KEYGEN_DATA_DIR").map(String::as_str),
            Some("C:\\ProgramData\\target\\runtime")
        );
    }
}
