use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/embedded_client_env.rs"));

pub const ENV_PATH_VARIABLE: &str = "KEYGEN_ENV_FILE";
pub const TOKEN_IDS_VARIABLE: &str = "KEYGEN_TOKEN_IDS";

#[derive(Clone, PartialEq, Eq)]
pub struct GenerationToken {
    pub id: String,
    pub name: String,
    pub secret: String,
}

impl GenerationToken {
    pub fn name_variable(&self) -> String {
        format!("KEYGEN_TOKEN_{}_NAME", self.id)
    }

    pub fn secret_variable(&self) -> String {
        format!("KEYGEN_TOKEN_{}_SECRET", self.id)
    }
}

impl fmt::Debug for GenerationToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GenerationToken")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone)]
pub struct RuntimeEnvironment {
    path: PathBuf,
    values: BTreeMap<String, String>,
    embedded_values: BTreeMap<String, String>,
}

impl RuntimeEnvironment {
    pub fn load() -> io::Result<Self> {
        let path = environment_file_path()?;
        let values = if path.is_file() {
            parse_dotenv(&fs::read_to_string(&path)?)?
        } else {
            BTreeMap::new()
        };
        let embedded_values = parse_dotenv(EMBEDDED_CLIENT_ENV)?;
        Ok(Self {
            path,
            values,
            embedded_values,
        })
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
            .or_else(|| self.embedded_values.get(name).cloned())
    }

    pub fn generation_tokens(&self) -> io::Result<Vec<GenerationToken>> {
        let ids = self
            .value(TOKEN_IDS_VARIABLE)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_configuration("KEYGEN_TOKEN_IDS is required"))?;
        let mut tokens = Vec::new();

        for configured_id in ids.split(',').map(str::trim) {
            if configured_id.is_empty()
                || !configured_id
                    .chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric())
            {
                return Err(invalid_configuration(
                    "KEYGEN_TOKEN_IDS must contain comma-separated letters, numbers, or underscores",
                ));
            }

            let id = configured_id.to_ascii_uppercase();
            if tokens.iter().any(|token: &GenerationToken| token.id == id) {
                return Err(invalid_configuration(
                    "KEYGEN_TOKEN_IDS contains a duplicated token identifier",
                ));
            }

            let name_variable = format!("KEYGEN_TOKEN_{id}_NAME");
            let secret_variable = format!("KEYGEN_TOKEN_{id}_SECRET");
            let name = required_token_value(self, &name_variable)?;
            if tokens
                .iter()
                .any(|token: &GenerationToken| token.name.eq_ignore_ascii_case(&name))
            {
                return Err(invalid_configuration(
                    "configured token names must be unique",
                ));
            }
            let secret = required_token_value(self, &secret_variable)?;
            tokens.push(GenerationToken { id, name, secret });
        }

        if tokens.is_empty() {
            return Err(invalid_configuration(
                "KEYGEN_TOKEN_IDS must contain at least one token identifier",
            ));
        }
        Ok(tokens)
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

fn invalid_configuration(message: &str) -> io::Error {
    io::Error::new(
        ErrorKind::InvalidData,
        format!(".env configuration: {message}"),
    )
}

fn required_token_value(environment: &RuntimeEnvironment, variable: &str) -> io::Result<String> {
    environment
        .value(variable)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_configuration(&format!("{variable} is required")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_postgresql_settings_and_passwords_with_equals() {
        let values = parse_dotenv(
            "# package settings\n\
             PGHOST=database.example.test\n\
             PGPASSWORD=\"abc_def==\"\n\
             export KEYGEN_LISTEN_ADDRESS='127.0.0.1:45632'\n",
        )
        .unwrap();

        assert_eq!(
            values.get("PGHOST").map(String::as_str),
            Some("database.example.test")
        );
        assert_eq!(
            values.get("PGPASSWORD").map(String::as_str),
            Some("abc_def==")
        );
        assert_eq!(
            values.get("KEYGEN_LISTEN_ADDRESS").map(String::as_str),
            Some("127.0.0.1:45632")
        );
    }

    #[test]
    fn generated_assignments_round_trip_special_values() {
        let contents = env_assignment("PGPASSWORD", "a=\"b\"\\c");
        let values = parse_dotenv(&contents).unwrap();
        assert_eq!(
            values.get("PGPASSWORD").map(String::as_str),
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

    #[test]
    fn runtime_file_value_overrides_embedded_build_value() {
        let environment = RuntimeEnvironment {
            path: PathBuf::new(),
            values: parse_dotenv("KEYGEN_LISTEN_ADDRESS=127.0.0.1:45633\n").unwrap(),
            embedded_values: parse_dotenv("KEYGEN_LISTEN_ADDRESS=127.0.0.1:45632\n").unwrap(),
        };

        assert_eq!(
            environment.value("KEYGEN_LISTEN_ADDRESS").as_deref(),
            Some("127.0.0.1:45633")
        );
    }

    #[test]
    fn loads_stacked_generation_tokens_from_runtime_configuration() {
        let environment = RuntimeEnvironment {
            path: PathBuf::new(),
            values: parse_dotenv(
                "KEYGEN_TOKEN_IDS=restaurant,new_product\n\
                 KEYGEN_TOKEN_RESTAURANT_NAME=Restaurant\n\
                 KEYGEN_TOKEN_RESTAURANT_SECRET=first-private-token\n\
                 KEYGEN_TOKEN_NEW_PRODUCT_NAME=New Product\n\
                 KEYGEN_TOKEN_NEW_PRODUCT_SECRET=second-private-token\n",
            )
            .unwrap(),
            embedded_values: BTreeMap::new(),
        };

        let tokens = environment.generation_tokens().unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].id, "RESTAURANT");
        assert_eq!(tokens[0].name, "Restaurant");
        assert_eq!(tokens[1].name, "New Product");
        assert_eq!(tokens[1].secret, "second-private-token");
        assert_eq!(
            tokens[1].secret_variable(),
            "KEYGEN_TOKEN_NEW_PRODUCT_SECRET"
        );
    }

    #[test]
    fn rejects_missing_or_duplicated_generation_token_settings() {
        let missing_secret = RuntimeEnvironment {
            path: PathBuf::new(),
            values: parse_dotenv(
                "KEYGEN_TOKEN_IDS=demo\n\
                 KEYGEN_TOKEN_DEMO_NAME=Demo\n",
            )
            .unwrap(),
            embedded_values: BTreeMap::new(),
        };
        assert!(missing_secret.generation_tokens().is_err());

        let duplicated_name = RuntimeEnvironment {
            path: PathBuf::new(),
            values: parse_dotenv(
                "KEYGEN_TOKEN_IDS=first,second\n\
                 KEYGEN_TOKEN_FIRST_NAME=Demo\n\
                 KEYGEN_TOKEN_FIRST_SECRET=one\n\
                 KEYGEN_TOKEN_SECOND_NAME=demo\n\
                 KEYGEN_TOKEN_SECOND_SECRET=two\n",
            )
            .unwrap(),
            embedded_values: BTreeMap::new(),
        };
        assert!(duplicated_name.generation_tokens().is_err());
    }
}
