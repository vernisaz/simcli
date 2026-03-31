use std::{collections::HashSet, env, fmt};

#[cfg(unix)]
const OPT_PREFIX: char = '-';
#[cfg(windows)]
const OPT_PREFIX: char = '/';
const VERSION: &str = env!("VERSION");

/// Returns a version of the create
///
pub fn get_version() -> &'static str {
    VERSION
}

/// Specify types of command line options
///
/// * Num - integer number
/// * FNum - float number
/// * Str - string
/// * InStr - property definition in a form like name=value
/// * None - no value
#[derive(PartialEq)]
#[allow(dead_code)]
pub enum OptTyp {
    Num,
    FNum,
    Str,
    InStr,
    None,
}

/// Specify possible values of command line options
///
#[derive(PartialEq, Debug)]
pub enum OptVal {
    Num(i64),
    FNum(f64),
    Str(String),
    Arr(HashSet<(String, String)>),
    Empty,
    Unmatch,
}
#[derive(Debug)]
pub struct OptError {
    cause: String,
}
impl fmt::Display for OptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Err: {}", self.cause)
    }
}
impl std::error::Error for OptError {}
impl Default for CLI {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides an argument description
///
pub struct CliOpt {
    t: OptTyp,
    v: Option<OptVal>,
    nme: String,
    descr: Option<String>,
}

/// Defines combined storage for CLI argements description and real arguments data
///
#[allow(clippy::upper_case_acronyms)]
pub struct CLI {
    // TODO: type CLI_MUT =  (RefCell<CLI>) for to avoid using &mut self
    args: Vec<String>, // RefCell
    opts: Vec<CliOpt>, // RefCell
    descr: Option<String>,
    oper: Option<String>,
    oper_requested: bool,
    oper_descr: Option<String>,
    unprocessed: bool,    // Cell
    unknown: Vec<String>, // RefCell
}
impl CLI {
    /// Create an empty CLI arguments descriptor
    ///
    pub fn new() -> Self {
        CLI {
            args: vec![],
            opts: vec![],
            descr: None,
            oper: Default::default(),
            oper_requested: false,
            oper_descr: Default::default(),
            unprocessed: true,
            unknown: vec![],
        }
    }

    /// Creates a new argument option
    ///
    /// # Examples
    ///
    /// ```
    /// let _ = cli.opt("c", OptTyp::None).inspect_err(|e| eprintln!("{e}"));
    /// ```
    pub fn opt(&mut self, name: &str, t: OptTyp) -> Result<&mut Self, OptError> {
        if !self.unprocessed {
            return Err(OptError {
                cause: format!("the option {name} can't be set after parsing arguments"),
            });
        }
        for opt in &self.opts {
            if opt.nme == name {
                return Err(OptError {
                    cause: format!("repeating option {name}"),
                });
            }
        }
        self.opts.push(CliOpt {
            t,
            nme: name.to_string(),
            descr: None,
            v: None,
        });
        Ok(self)
    }
    /// Specify common CLI description
    ///
    pub fn description(&mut self, descr: &str) -> &mut Self {
        match self.opts.last_mut() {
            Some(element) => element.descr = Some(descr.to_string()),
            _ => self.descr = Some(descr.to_string()),
        }
        self
    }
    /// Use an operation as the first arguments
    ///
    pub fn use_oper(&mut self) -> &mut Self {
        self.oper_requested = true;
        self
    }
    /// Specify an operation description
    ///
    pub fn oper_description(&mut self, descr: &str) -> Result<&mut Self, OptError> {
        if !self.oper_requested {
            Err(OptError {
                cause: format!("an operation description can be defined after a set - use_oper()"),
            })
        } else {
            self.oper_descr = Some(descr.to_string());
            Ok(self)
        }
    }
    /// Get the CLI description
    ///
    pub fn get_description(&self) -> Option<String> {
        let mut descr = String::new();
        if let Some(some_descr) = &self.descr {
            descr += some_descr
        }
        if let Some(some_descr) = &self.oper_descr {
            descr += &format!("\n{some_descr}")
        }
        for opt in &self.opts {
            descr += &format!("\n{OPT_PREFIX}{}", opt.nme);
            if let Some(some_descr) = &opt.descr {
                descr += &format!("\t{some_descr}")
            }
        }
        if descr.is_empty() { None } else { Some(descr) }
    }
    /// Get a CLI option
    ///
    pub fn get_opt(&mut self, name: &str) -> Option<&OptVal> {
        if self.unprocessed {
            self.parse()
        }
        for opt in &self.opts {
            if opt.nme == name {
                return opt.v.as_ref();
            }
        }
        None
    }
    /// Returns first argument as an operation
    ///
    /// some CLI tools, as git consider first argument as operation/command
    ///
    /// the argument will be also added in arguments vec itself
    pub fn get_oper(&mut self) -> Option<&String> {
        if self.unprocessed {
            self.parse()
        }
        self.oper.as_ref()
    }
    /// Get CLI arguments
    ///
    pub fn args(&mut self) -> &Vec<String> {
        if self.unprocessed {
            self.parse()
        }
        &self.args
    }

    /// Get errors
    ///
    /// Returns a vector of unrecognized options or None
    pub fn get_errors(&mut self) -> Option<&Vec<String>> {
        if self.unprocessed {
            self.parse()
        }
        if self.unknown.is_empty() {
            None
        } else {
            Some(&self.unknown)
        }
    }

    fn parse(&mut self) {
        let mut args = env::args();
        args.next(); // swallow first
        while let Some(arg) = args.next() {
            if let Some(sarg) = arg.strip_prefix(OPT_PREFIX) {
                // TODO eat extra -'s
                let mut consumed = false;
                for opt in &mut self.opts {
                    if opt.nme == sarg {
                        match opt.t {
                            OptTyp::Num => {
                                if let Some(val) = args.next() {
                                    match val.parse::<i64>() {
                                        Ok(num) => opt.v = Some(OptVal::Num(num)),
                                        _ => opt.v = Some(OptVal::Unmatch),
                                    }
                                }
                            }
                            OptTyp::None => opt.v = Some(OptVal::Empty),
                            OptTyp::FNum => {
                                if let Some(val) = args.next() {
                                    match val.parse::<f64>() {
                                        Ok(num) => opt.v = Some(OptVal::FNum(num)),
                                        _ => opt.v = Some(OptVal::Unmatch),
                                    }
                                }
                            }
                            OptTyp::Str => {
                                if let Some(str) = args.next() {
                                    opt.v = Some(OptVal::Str(str))
                                }
                            }
                            OptTyp::InStr => (),
                        }
                        consumed = true;
                    } else if opt.t == OptTyp::InStr && sarg.starts_with(&opt.nme) {
                        if opt.v.is_none() {
                            opt.v = Some(OptVal::Arr(HashSet::new()))
                        }
                        match &mut opt.v {
                            &mut Some(OptVal::Arr(ref mut set)) => {
                                if let Some(pair) =
                                    sarg.strip_prefix(&opt.nme).unwrap().split_once('=')
                                {
                                    set.insert((pair.0.to_string(), pair.1.to_string()));
                                } else {
                                    set.insert((
                                        sarg.strip_prefix(&opt.nme).unwrap().to_string(),
                                        String::new(),
                                    ));
                                }
                            }
                            _ => {
                                // somehow to report data inconsistency
                                opt.v = Some(OptVal::Arr(HashSet::new()))
                            }
                        }
                        consumed = true;
                    } else if opt.nme.len() == 1 && sarg.contains(&opt.nme) && opt.t == OptTyp::None
                    {
                        opt.v = Some(OptVal::Empty);
                        consumed = true;
                    }
                }
                if !consumed {
                    self.unknown.push(arg)
                }
            } else {
                if self.oper.is_none() && self.oper_requested {
                    self.oper = Some(arg.clone())
                } else {
                    self.args.push(arg)
                }
            }
            self.oper_requested = false;
        }
        self.unprocessed = false
    }
}
