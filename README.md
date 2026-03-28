# Simple CLI options processor

## Purpose
Surprisingly, but AI doesn't give any simple and powerful command line arguments processor.
The list of `clap`, `pico-args`, `lexopt`, `args`, and  `docopt` looks ridiculous.

Therefore this crate was created.
You do not need to define a structure holding command line arguments.
Instead, SimCLI uses a pull approach. Just define an expectation for arguments, and then pull
some accordingly your needs.

## Features

- mixed list of arguments and options
- use a platform specific option flag, like `-` for Unix and `/` for Windows
- errors detection as in defining options as in the processing them
- automatic help and description generation
- specifying a type and range of arguments
 

## How to use

Define the arguments first,
```Rust
let mut cli = CLI::new();
cli.opt("n", OptTyp::Num)?.description("Number of lines")
    .opt("v", OptTyp::None)?.description("Version").opt("h", OptTyp::None)?;
```

You can pull the required arguments after,
```rust
if cli.get_opt("v") == Some(&OptVal::Empty) {
    return Ok(println!("\nVersion {VERSION}"))
} else if cli.get_opt("h") == Some(&OptVal::Empty) {
    return Ok(println!("simtail [opts] <file path>[ ...<file path>]\n{}", cli.get_description()?))
}
let lns = match cli.get_opt("n") {
    Some(OptVal::Num(n)) => *n as usize,
    _ => 15usize
};
tail_of(&cli.args().first()?, lns)?;
```

If you have arguments in a form like - *-Xname=value*, then you can define them 
using the code bellow,
```rust
cli.opt("D", OptTyp::InStr)?.description("A definition as name=value");
// and then read their presences in the command line
let d_o = cli.get_opt("D");
if let Some(OptVal::Arr(d_o)) = d_o {
    for (i,d) in d_o.into_iter().enumerate() {
        println!("opt[{i}] {d:?}");
    }
}
```

## How to build the crate
The crate can be built either using [RB](https://github.com/vernisaz/rust_bee) (.7b script provided) or
Cargo (.toml descriptor can be easy added, since there are no dependencies).
Do not forget to check out [the common scripts](https://github.com/vernisaz/simscript) when use *RB*.