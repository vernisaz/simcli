use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::HashSet,
    env::{self, current_dir},
    ffi::{OsStr, OsString},
    fmt,
    fs::ReadDir,
    path::PathBuf,
};

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
#[derive(PartialEq, Debug, Default)]
#[allow(dead_code)]
pub enum OptTyp {
    Num,
    FNum,
    Str,
    InStr,
    #[default]
    None,
}

/// Specify if a wild card in argument should be treated as for Windows
///
/// * None - not treated
/// * Once - only one time first match
/// * All - occurance
#[derive(PartialEq, Debug, Default)]
pub enum WildCardExpansion {
    #[default]
    None,
    Once,
    All,
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

/// Provides an argument description
///
#[derive(Debug)]
pub struct CliOpt {
    t: OptTyp,
    v: Option<OptVal>,
    nme: String,
    descr: Option<String>,
}

#[derive(Debug, Default)]
struct Glob {
    parent: Option<PathBuf>,
    dir: Option<ReadDir>,
    before: OsString,
    after: OsString,
}
impl Glob {
    fn from(str: &str) -> Self {
        let mut parent = PathBuf::from(str);
        if let Some(file_name) = parent.file_name()
            && let file_name = file_name.display().to_string()
            && let Some((before, after)) = file_name.split_once('*')
        {
            parent.pop();
            Glob {
                dir: if parent.has_root() {
                    parent.read_dir()
                } else {
                    current_dir().unwrap_or_default().join(&parent).read_dir()
                }
                .ok(),
                parent: Some(parent),
                before: OsStr::new(before).to_os_string(),
                after: OsStr::new(after).to_os_string(),
            }
        } else {
            Glob {
                parent: Some(parent),
                ..Default::default()
            }
        }
    }
}

impl Iterator for Glob {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(dir) = &mut self.dir {
            let pattern_len = self.before.len() + self.after.len();
            loop {
                match dir.next() {
                    None => break None,
                    Some(entry) => {
                        if let Ok(entry) = entry {
                            let file_name = entry.file_name();
                            if file_name.len() >= pattern_len
                                && file_name
                                    .as_encoded_bytes()
                                    .starts_with(self.before.as_encoded_bytes())
                                && file_name
                                    .as_encoded_bytes()
                                    .ends_with(self.after.as_encoded_bytes())
                            {
                                if let Some(parent) = &self.parent {
                                    break Some(
                                        parent.join(entry.file_name()).display().to_string(),
                                    );
                                } else {
                                    break Some(file_name.display().to_string());
                                }
                            } else {
                                continue;
                            }
                        }
                    }
                }
            }
        } else if let Some(parent) = &self.parent {
            let res = parent.display().to_string();
            self.parent = None;
            Some(res)
        } else {
            None
        }
    }
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

/// Defines a combined storage for CLI argements description and real arguments data
///
/// # Field details
/// * args - hold a vector of arguments
/// * opts - hold a vector of options
/// * descr - description of the CLI
/// * oper - the operation if any
/// * oper_requested - tells if the CLI expects an operation
/// * oper_descr - optional description of the operation
/// * glob_mode - Windows specific
/// * unprocessed - state of processing
/// * unknown - a vector of unrecognized options
/// all fields managed internally and shouldn't be accesed directly
#[allow(clippy::upper_case_acronyms)]
#[derive(Default)]
pub struct CLI {
    args: Vec<String>,
    opts: Vec<CliOpt>,
    descr: Option<String>,
    oper: Option<String>,
    oper_requested: bool,
    oper_descr: Option<String>,
    glob_mode: WildCardExpansion,
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
            unprocessed: true,
            unknown: vec![],
            ..Default::default()
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
    /// Process wildcard in arguments
    ///
    pub fn process_wildcard(&mut self, mode: WildCardExpansion) -> &mut Self {
        self.glob_mode = mode;
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
                let mut was_input_opt = false;
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
                        && !was_input_opt
                    {
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
                            OptTyp::InStr | OptTyp::None => continue, // TODO maybe add some error handling
                        }
                        was_input_opt = true;
                        string.retain(|c| c != last);
                    }
                }
                if !string.is_empty() {
                    self.unknown.push(string)
                }
            } else if self.oper.is_none() && self.oper_requested {
                self.oper = Some(arg)
            } else if !cfg!(windows) {
                self.args.push(arg)
            } else {
                match self.glob_mode {
                    WildCardExpansion::None => self.args.push(arg),
                    WildCardExpansion::Once => match Glob::from(&arg).next() {
                        None => self.args.push(arg),
                        Some(arg) => self.args.push(arg),
                    },
                    WildCardExpansion::All => {
                        for arg in Glob::from(&arg) {
                            self.args.push(arg)
                        }
                    }
                }
            }
            self.oper_requested = false;
        }
        self.opts.sort();
        self.unprocessed = false
    }
}

/// Defines a combined storage for CLI argements description and real arguments
/// data not requiring to be mutable
///
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
    /// Process wildcard in arguments
    ///
    pub fn process_wildcard(&self, mode: WildCardExpansion) -> &Self {
        let mut cli = self.cli.borrow_mut();
        cli.glob_mode = mode;
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
