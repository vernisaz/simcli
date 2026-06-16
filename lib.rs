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
    ArrStr,
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
    ArrStr(Vec<String>),
    Empty,
    Unmatch,
}
#[derive(Debug, Default)]
enum OptStat {
    Duplicate,
    DupAlias,
    NoOption,
    #[default]
    Other,
}
#[derive(Default, Debug)]
#[allow(dead_code)]
pub struct OptError {
    problem_type: OptStat,
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
    other: Option<HashSet<String>>,
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
///
/// all fields are managed internally and shouldn't be accesed directly
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
    unknown: Vec<String>,  // and add misused
    src_args: Vec<String>, // TODO separate field as immutable
}
impl CLI {
    /// Create an empty CLI arguments descriptor
    ///
    pub fn new() -> Self {
        let mut src_args = vec![];
        let mut args = env::args();
        args.next(); // swallow first
        src_args.extend(args);
        CLI {
            args: vec![],
            opts: vec![],
            unprocessed: true,
            unknown: vec![],
            src_args,
            ..Default::default()
        }
    }

    /// Create an empty CLI arguments descriptor to parse a vector of arguments
    ///
    pub fn from(src_args: Vec<String>) -> Self {
        CLI {
            args: vec![],
            opts: vec![],
            unprocessed: true,
            unknown: vec![],
            src_args,
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
                ..Default::default()
            });
        }
        if self.get_opt_def(name).is_ok() {
            return Err(OptError {
                cause: format!("repeating option {name}"),
                problem_type: OptStat::Duplicate,
            });
        }
        // opts are case insensitive on Windows
        #[cfg(target_os = "windows")]
        let name = name.to_ascii_lowercase();
        self.opts.push(CliOpt {
            t,
            nme: name.to_string(),
            other: None,
            descr: None,
            v: None,
        });
        Ok(self)
    }
    /// Specify a common CLI description
    ///
    pub fn description(&mut self, descr: &str) -> &mut Self {
        match self.opts.last_mut() {
            Some(element) => element.descr = Some(descr.to_string()),
            _ => self.descr = Some(descr.to_string()),
        }
        self
    }
    /// Add an alias to just the created argument option
    ///
    /// Several aliases can be added, aliases are usually created as a multi symbols alternatives
    /// to an one symbol option
    ///
    /// # Examples
    ///
    /// ```
    /// let cli = cli.opt("v", OptTyp::None).inspect_err(|e| eprintln!("{e}"))?;
    /// cli.alias("-version")?.description("Provides a version of the product");
    /// ```
    pub fn alias(&mut self, name: &str) -> Result<&mut Self, OptError> {
        if !self.unprocessed {
            return Err(OptError {
                cause: format!("the alias {name} can't be set after parsing arguments"),
                ..Default::default()
            });
        }
        if self.get_opt_def(name).is_ok() {
            return Err(OptError {
                cause: format!("repeating alias {name}"),
                problem_type: OptStat::DupAlias,
            });
        }
        match self.opts.last_mut() {
            Some(element) => {
                if element.other.is_none() {
                    element.other = Some(HashSet::new())
                }
                if let Some(ref mut aliases) = element.other {
                    aliases.insert(name.to_string());
                }
            }
            _ => {
                return Err(OptError {
                    cause: "no current element to set alias to".to_string(),
                    problem_type: OptStat::NoOption,
                });
            }
        }
        Ok(self)
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
                cause: "an operation description can be defined after setting - use_oper()"
                    .to_string(),
                ..Default::default()
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
                descr += &format!("\t{some_descr}");
                if let Some(aliases) = &opt.other {
                    for alias in aliases {
                        descr += &format!("\n{OPT_PREFIX}{alias}\t''");
                    }
                }
            }
        }
        if descr.is_empty() { None } else { Some(descr) }
    }
    /// Get a CLI option
    ///
    pub fn get_opt(&mut self, name: &str) -> Option<&OptVal> {
        // beter to return Result<Option<&OptVal>, error>
        if self.unprocessed {
            self.parse()
        }

        match self.get_opt_def(name).ok() {
            Some(opt) => opt.v.as_ref(),
            _ => None,
        }
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

    /// Used internally to check if the option is defined
    fn get_opt_def(&self, name: &str) -> Result<&CliOpt, OptStat> {
        //eprintln!("searching {name}");
        // opts are case insensitive on Windows
        #[cfg(target_os = "windows")]
        let name = &name.to_ascii_lowercase();
        let mut found = None;
        for opt in &self.opts {
            if opt.nme == *name {
                if found.is_none() {
                    found = Some(opt);
                } else {
                    return Err(OptStat::Duplicate);
                }
            } else if let Some(other) = &opt.other
                && other.contains(name)
            {
                if found.is_none() {
                    found = Some(opt);
                } else {
                    return Err(OptStat::DupAlias);
                }
            }
        }
        match found {
            None => Err(OptStat::NoOption),
            Some(opt) => Ok(opt),
        }
    }

    fn matches(opt: &CliOpt, name: &str) -> bool {
        if opt.nme == name || cfg!(windows) && opt.nme == name.to_ascii_lowercase() {
            return true;
        }
        if let Some(other) = &opt.other
            && (other.contains(name) || cfg!(windows) && other.contains(&name.to_ascii_lowercase()))
        {
            true
        } else {
            false
        }
    }

    fn parse(&mut self) {
        let mut args = self.src_args.clone().into_iter();
        while let Some(arg) = args.next() {
            if arg == "--" {
                self.args.extend(args);
                break;
            }
            if let Some(sarg) = arg.strip_prefix(OPT_PREFIX) {
                let mut string = sarg.to_string();
                if string.is_empty() {
                    self.unknown.push(String::new());
                    continue;
                }
                let mut was_input_opt = false;
                for opt in &mut self.opts {
                    //eprintln!("checking {} ags {string}", opt.nme);
                    // opts are case insensitive for Windows
                    if CLI::matches(opt, sarg) {
                        if opt.v.is_none() || opt.t == OptTyp::ArrStr {
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
                                OptTyp::ArrStr => {
                                    if let Some(str) = args.next() {
                                        if opt.v.is_none() {
                                            opt.v = Some(OptVal::ArrStr(vec![]))
                                        }
                                        match &mut opt.v {
                                            &mut Some(OptVal::ArrStr(ref mut vec)) => {
                                                vec.push(str);
                                            }
                                            _ => {
                                                // somehow to report data inconsistency
                                                unreachable!("Can't add an argument to non vec")
                                                //opt.v = Some(OptVal::Arr(HashSet::new()))
                                            }
                                        }
                                    }
                                }
                                OptTyp::InStr => (),
                            }
                        } else {
                            self.unknown.push(string.clone()) // not right because it's a duplicate argument
                        }
                        string.clear();
                    } else if opt.t == OptTyp::InStr
                        && (cfg!(windows) && sarg.to_ascii_lowercase().starts_with(&opt.nme)
                            || sarg.starts_with(&opt.nme))
                    {
                        if opt.v.is_none() {
                            opt.v = Some(OptVal::Arr(HashSet::new()))
                        }
                        match &mut opt.v {
                            &mut Some(OptVal::Arr(ref mut set)) => {
                                let opt_def = sarg[opt.nme.len()..].to_string();
                                if let Some(pair) = opt_def.split_once('=') {
                                    set.insert((pair.0.to_string(), pair.1.to_string()));
                                } else {
                                    set.insert((opt_def, String::new()));
                                }
                            }
                            _ => {
                                // somehow to report data inconsistency
                                unreachable!(
                                    "Can't specify an argument in format -Xname=value become a different type"
                                )
                                //opt.v = Some(OptVal::Arr(HashSet::new()))
                            }
                        }
                        string.clear();
                    } else if opt.t == OptTyp::None
                        && opt.nme.chars().count() == 1
                        && !sarg.starts_with('-')
                        && string.contains(&opt.nme)
                    {
                        //eprintln!("found {opt:?} ags {string}", opt.nme);
                        opt.v = Some(OptVal::Empty);
                        string.retain(|c| c != opt.nme.chars().next().unwrap());
                    } else if let Some(last) = string.chars().last()
                        && opt.nme.chars().count() == 1
                        && opt.nme.chars().next().unwrap() == last
                        && !was_input_opt
                    {
                        if opt.v.is_none() || opt.t == OptTyp::ArrStr {
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
                                OptTyp::ArrStr => {
                                    if let Some(str) = args.next() {
                                        if opt.v.is_none() {
                                            opt.v = Some(OptVal::ArrStr(vec![]))
                                        }
                                        match &mut opt.v {
                                            &mut Some(OptVal::ArrStr(ref mut vec)) => {
                                                vec.push(str);
                                            }
                                            _ => {
                                                // somehow to report data inconsistency
                                                unreachable!("Can't add an argument to non vec")
                                                //opt.v = Some(OptVal::Arr(HashSet::new()))
                                            }
                                        }
                                    }
                                }
                                OptTyp::InStr | OptTyp::None => continue, // TODO maybe add some error handling
                            }
                        } else {
                            self.unknown.push(last.to_string()) // not right because it's a duplicate argument
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
    /// Creates an alias for the last created option
    ///
    /// # Examples
    ///
    /// ```
    /// let _ = cli.alias("-color").inspect_err(|e| eprintln!("{e}"));
    /// ```
    pub fn alias(&self, name: &str) -> Result<&Self, OptError> {
        let mut cli = self.cli.borrow_mut();
        match cli.alias(name) {
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
