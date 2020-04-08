use std::env::Args;

// argument option
#[derive(Debug, Clone)]
pub(crate) struct Arg {
    name: char,
    value_name: Option<String>,
    help: String,
}

impl Arg {
    pub(crate) fn new(name: char, value_name: Option<String>, help: String) -> Self {
        Self {
            name,
            value_name,
            help,
        }
    }

    // Check if arg is matched to this argument option
    fn is_matched(&self, arg: &str) -> bool {
        if arg.len() == 2 {
            (arg.chars().nth(0) == Some('-')) && (arg.chars().nth(1) == Some(self.name))
        } else {
            false
        }
    }
}

#[test]
fn test_arg_is_matched() {
    let arg = Arg::new('a', None, "help".to_string());
    assert_eq!(arg.is_matched("-a"), true);
    assert_eq!(arg.is_matched("-b"), false);
}

#[derive(Debug, PartialEq)]
pub(crate) enum MatchedValue {
    FlagArg { name: char },
    Arg { name: char, value: String },
    ArgValueMissing { name: char },
    ParseError { argument: String },
    Value { value: String },
}

pub(crate) struct App<'a> {
    options: Box<&'a [Arg]>,
}

impl App<'_> {
    pub(crate) fn new(options: &[Arg]) -> App {
        App {
            options: Box::new(options),
        }
    }

    /// Parse CLI arguments with options.
    pub(crate) fn parse<T: ToString>(&self, arguments: &[T]) -> Box<[MatchedValue]> {
        let mut values = Vec::with_capacity(self.options.len());

        let mut arguments_iter = arguments.iter();
        while let Some(argument) = arguments_iter.next() {
            let argument = argument.to_string();
            if !argument.starts_with("-") {
                values.push(MatchedValue::Value { value: argument });
            } else if let Some(arg) = self
                .options
                .iter()
                .find(|&opt| opt.is_matched(argument.as_str()))
            {
                let name = arg.name;

                if arg.value_name.is_some() {
                    // if this option takes a value
                    if let Some(val) = arguments_iter.next() {
                        values.push(MatchedValue::Arg {
                            name,
                            value: val.to_string(),
                        });
                    } else {
                        values.push(MatchedValue::ArgValueMissing { name });
                    }
                } else {
                    // if this option does not take a value
                    values.push(MatchedValue::FlagArg { name });
                }
            } else {
                values.push(MatchedValue::ParseError { argument });
            }
        }

        values.into_boxed_slice()
    }
}

#[test]
fn test_app_parse() {
    let options = vec![
        Arg::new('a', None, "option a".to_string()),
        Arg::new('b', Some("VAL".to_string()), "option b".to_string()),
    ];
    let app = App::new(&options);
    let arguments = vec!["-a", "-b", "B1", "-b", "B2", "-a", "VAL"];
    let matches = app.parse(&arguments);
    assert_eq!(matches.len(), 5);
    assert_eq!(matches[0], MatchedValue::FlagArg { name: 'a' });
    assert_eq!(
        matches[1],
        MatchedValue::Arg {
            name: 'b',
            value: "B1".to_string()
        }
    );
    assert_eq!(
        matches[2],
        MatchedValue::Arg {
            name: 'b',
            value: "B2".to_string()
        }
    );
    assert_eq!(matches[3], MatchedValue::FlagArg { name: 'a' });
    assert_eq!(
        matches[4],
        MatchedValue::Value {
            value: "VAL".to_string()
        }
    );

    let arguments = vec!["-c", "-a", "-b"];
    let matches = app.parse(&arguments);
    assert_eq!(matches.len(), 3);
    assert_eq!(
        matches[0],
        MatchedValue::ParseError {
            argument: "-c".to_string()
        }
    );
    assert_eq!(matches[1], MatchedValue::FlagArg { name: 'a' });
    assert_eq!(matches[2], MatchedValue::ArgValueMissing { name: 'b' });

    let arguments = vec!["-b", "-a"];
    let matches = app.parse(&arguments);
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0],
        MatchedValue::Arg {
            name: 'b',
            value: "-a".to_string()
        }
    ); // this is spec!
}
