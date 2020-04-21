use core::fmt;
use std::fmt::Formatter;

// argument option
#[derive(Debug, Clone)]
pub(crate) struct Arg<ID: Copy> {
    id: ID,
    short_name: char,
    value_name: Option<String>,
    help: String,
}

impl<ID: Copy> Arg<ID> {
    pub(crate) fn new(id: ID, short_name: char, value_name: Option<String>, help: String) -> Self {
        Arg {
            id,
            short_name,
            value_name,
            help,
        }
    }

    // Check if arg is matched to this argument option
    fn is_matched(&self, arg: &str) -> bool {
        if arg.len() == 2 {
            (arg.chars().nth(0) == Some('-')) && (arg.chars().nth(1) == Some(self.short_name))
        } else {
            false
        }
    }
}

#[test]
fn test_arg_is_matched() {
    let arg = Arg::new(0, 'a', None, "help".to_string());
    assert_eq!(arg.is_matched("-a"), true);
    assert_eq!(arg.is_matched("-b"), false);
}

/// Argument value
#[derive(Debug, PartialEq)]
pub(crate) enum ArgValue<ID: Copy + PartialEq> {
    Arg { id: ID, value: Option<String> },
    Value { value: String },
}

/// Parse error
#[derive(Debug, PartialEq)]
pub(crate) enum ArgParseError {
    ArgValueMissing { name: char },
    ParseError { argument: String },
}

impl fmt::Display for ArgParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ArgParseError::*;
        match self {
            ArgValueMissing { name } => {
                f.write_str(format!("\'-{}\': parameter value missing", name).as_str())
            }
            ParseError { argument } => {
                f.write_str(format!("\'{}\': wrong argument", argument).as_str())
            }
        }
    }
}

pub(crate) struct App<'a, ID: Copy + PartialEq> {
    options: Box<&'a [Arg<ID>]>,
}

impl<ID: Copy + PartialEq> App<'_, ID> {
    pub(crate) fn new(options: &[Arg<ID>]) -> App<ID> {
        App {
            options: Box::new(options),
        }
    }

    /// Find the arg object for specific id.
    pub(crate) fn find_arg(&self, id: ID) -> Option<Arg<ID>> {
        self.options.iter().find(|&v| v.id == id).map(|v| v.clone())
    }

    /// Parse CLI arguments with options.
    pub(crate) fn parse<T: ToString>(
        &self,
        arguments: &[T],
    ) -> Result<Box<[ArgValue<ID>]>, ArgParseError> {
        let mut values = Vec::with_capacity(self.options.len());

        let mut arguments_iter = arguments.iter();
        while let Some(argument) = arguments_iter.next() {
            let argument = argument.to_string();
            if !argument.starts_with("-") {
                values.push(ArgValue::Value { value: argument });
            } else if let Some(arg) = self
                .options
                .iter()
                .find(|&opt| opt.is_matched(argument.as_str()))
            {
                let id = arg.id;
                let short_name = arg.short_name;

                if arg.value_name.is_some() {
                    // if this option takes a value
                    if let Some(val) = arguments_iter.next() {
                        values.push(ArgValue::Arg {
                            id,
                            value: Some(val.to_string()),
                        });
                    } else {
                        return Err(ArgParseError::ArgValueMissing { name: short_name });
                    }
                } else {
                    // if this option does not take a value
                    values.push(ArgValue::Arg { id, value: None });
                }
            } else {
                return Err(ArgParseError::ParseError { argument });
            }
        }

        Ok(values.into_boxed_slice())
    }

    /// get option message
    pub(crate) fn help_option_message(&self) -> String {
        let mut text = String::new();

        for option in self.options.iter() {
            let left = if let Some(t) = &option.value_name {
                format!("    -{} {}", option.short_name, t)
            } else {
                format!("    -{}", option.short_name)
            };
            text.push_str(left.as_str());
            let indent_offset = 30.max(left.len() + 1);
            for _ in 0..(indent_offset - left.len()) {
                text.push(' ');
            }
            text.push_str(
                option
                    .help
                    .replace("\n", format!("\n{}", " ".repeat(indent_offset)).as_str())
                    .as_str(),
            );
            text.push('\n');
        }
        text
    }
}

#[test]
fn test_app_parse() {
    let options = vec![
        Arg::new(0, 'a', None, "option a".to_string()),
        Arg::new(1, 'b', Some("VAL".to_string()), "option b".to_string()),
    ];
    let app = App::new(&options);
    let arguments = vec!["-a", "-b", "B1", "-b", "B2", "-a", "VAL"];
    let matches = app.parse(&arguments).unwrap();
    assert_eq!(matches.len(), 5);
    assert_eq!(matches[0], ArgValue::Arg { id: 0, value: None });
    assert_eq!(
        matches[1],
        ArgValue::Arg {
            id: 1,
            value: Some("B1".to_string())
        }
    );
    assert_eq!(
        matches[2],
        ArgValue::Arg {
            id: 1,
            value: Some("B2".to_string())
        }
    );
    assert_eq!(matches[3], ArgValue::Arg { id: 0, value: None });
    assert_eq!(
        matches[4],
        ArgValue::Value {
            value: "VAL".to_string()
        }
    );

    let arguments = vec!["-c", "-a", "-b"];
    let matches = app.parse(&arguments);
    assert_eq!(
        matches,
        Err(ArgParseError::ParseError {
            argument: "-c".to_string()
        })
    );

    let arguments = vec!["-a", "-b"];
    let matches = app.parse(&arguments);
    assert_eq!(matches, Err(ArgParseError::ArgValueMissing { name: 'b' }));

    let arguments = vec!["-b", "-a"];
    let matches = app.parse(&arguments).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0],
        ArgValue::Arg {
            id: 1,
            value: Some("-a".to_string())
        }
    ); // this is spec!
}
