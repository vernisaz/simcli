use std::{cell::RefCell, cmp::Ordering, collections::HashSet, env, fmt};

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
#[derive(PartialEq, Debug)]
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
#[derive(PartialEq, Debug, Clone)]
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
#[derive(Debug)]
pub struct CliOpt {
    t: OptTyp,
    v: Option<OptVal>,
    nme: String,
    descr: Option<String>,
}

impl PartialEq for CliOpt {
    fn eq(&self, other: &Self) -> bool {
        let self_nam = if self.nme.as_bytes()[0] == b'-' {
            &self.nme[1..]
        } else {
            &self.nme[..]
        };
        let other_nam = if other.nme.as_bytes()[0] == b'-' {
            &other.nme[1..]
        } else {
            &other.nme[..]
        };
        *self_nam == *other_nam
    }
}
impl Eq for CliOpt {}

// Implement PartialOrd
impl PartialOrd for CliOpt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other)) // Delegate to Ord's cmp
    }
}

// Implement Ord (total ordering)
impl Ord for CliOpt {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_nam = if self.nme.as_bytes()[0] == b'-' {
            &self.nme[1..]
        } else {
            &self.nme[..]
        };
        let other_nam = if other.nme.as_bytes()[0] == b'-' {
            &other.nme[1..]
        } else {
            &other.nme[..]
        };
        self_nam.cmp(other_nam)
    }
}

/// Defines combined storage for CLI argements description and real arguments data
///
#[allow(clippy::upper_case_acronyms)]
pub struct CLI {
    args: Vec<String>,
    opts: Vec<CliOpt>,
    descr: Option<String>,
    oper: Option<String>,
    oper_requested: bool,
    oper_descr: Option<String>,
    unprocessed: bool,
    unknown: Vec<String>,
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
    /// Use an operation as the first argument
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
                cause: "an operation description can be defined after a set - use_oper()"
                    .to_string(),
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
    /// Some CLI tools, like git, consider the first argument as an operation/command
    ///
    /// The argument will be excluded from the arguments list
    pub fn get_oper(&mut self) -> Option<&String> {
        if self.unprocessed {
            self.parse()
        }
        self.oper.as_ref()
    }
    /// Get CLI arguments
    ///
    /// The returned list doesn't include the command, if it is defined
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
                let mut string = sarg.to_string();
                for opt in &mut self.opts {
                    //eprintln!("checking {} ags {string}", opt.nme);
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
                        string.clear();
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
                                unreachable!(
                                    "Can't argument in format -Xname=value become a different type"
                                )
                                //opt.v = Some(OptVal::Arr(HashSet::new()))
                            }
                        }
                        string.clear();
                    } else if opt.t == OptTyp::None
                        && opt.nme.chars().count() == 1
                        && sarg.contains(&opt.nme)
                    {
                        opt.v = Some(OptVal::Empty);
                        string.retain(|c| c != opt.nme.chars().next().unwrap());
                    } else if let Some(last) = string.chars().last()
                        && opt.nme.chars().count() == 1
                        && opt.nme.chars().next().unwrap() == last
                    {
                        string.retain(|c| c != last);
                        match opt.t {
                            OptTyp::Num => {
                                if let Some(val) = args.next() {
                                    match val.parse::<i64>() {
                                        Ok(num) => opt.v = Some(OptVal::Num(num)),
                                        _ => opt.v = Some(OptVal::Unmatch),
                                    }
                                }
                            }
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
                            OptTyp::InStr | OptTyp::None => (),
                        }
                    }
                }
                if !string.is_empty() {
                    self.unknown.push(string)
                }
            } else if self.oper.is_none() && self.oper_requested {
                self.oper = Some(arg.clone())
            } else {
                self.args.push(arg)
            }
            self.oper_requested = false;
        }
        self.opts.sort();
        self.unprocessed = false
    }
}

pub struct CliNoMut {
    cli: RefCell<CLI>,
}
impl Default for CliNoMut {
    fn default() -> Self {
        Self::new()
    }
}

impl CliNoMut {
    /// Create an empty CLI arguments descriptor
    ///
    pub fn new() -> Self {
        CliNoMut {
            cli: RefCell::new(CLI::new()),
        }
    }

    /// Creates a new argument option
    ///
    /// # Examples
    ///
    /// ```
    /// let _ = cli.opt("c", OptTyp::None).inspect_err(|e| eprintln!("{e}"));
    /// ```
    pub fn opt(&self, name: &str, t: OptTyp) -> Result<&Self, OptError> {
        let mut cli = self.cli.borrow_mut();
        match cli.opt(name, t) {
            Ok(_) => Ok(self),
            Err(err) => Err(err),
        }
    }
    /// Specify common CLI description
    ///
    pub fn description(&self, descr: &str) -> &Self {
        let mut cli = self.cli.borrow_mut();
        let _ = cli.description(descr);
        self
    }
    /// Use an operation as the first argument
    ///
    pub fn use_oper(&self) -> &Self {
        let mut cli = self.cli.borrow_mut();
        cli.oper_requested = true;
        self
    }
    /// Specify an operation description
    ///
    pub fn oper_description(&self, descr: &str) -> Result<&Self, OptError> {
        let mut cli = self.cli.borrow_mut();
        match cli.oper_description(descr) {
            Ok(_) => Ok(self),
            Err(err) => Err(err),
        }
    }
    /// Get the CLI description
    ///
    pub fn get_description(&self) -> Option<String> {
        let cli = self.cli.borrow();
        cli.get_description()
    }
    /// Get a CLI option
    ///
    pub fn get_opt(&self, name: &str) -> Option<OptVal> {
        self.cli.borrow_mut().get_opt(name).cloned()
    }
    /// Returns first argument as an operation
    ///
    /// Some CLI tools, as git, consider the first argument as an operation/command
    ///
    /// the argument will be also added in arguments vec itself
    pub fn get_oper(&self) -> Option<String> {
        self.cli.borrow_mut().get_oper().cloned()
    }
    /// Get CLI arguments
    ///
    /// The operation isn't included when specified
    pub fn args(&self) -> Vec<String> {
        self.cli.borrow_mut().args().clone()
    }

    /// Get errors
    ///
    /// Returns a vector of unrecognized options or None
    pub fn get_errors(&self) -> Option<Vec<String>> {
        self.cli.borrow_mut().get_errors().cloned()
    }
}
