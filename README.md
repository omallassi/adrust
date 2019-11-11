# ADRust : adr-tools in Rust

The idea is to provide a cli to managed ADR. One more? yes...reality is that was mostly an excuse to play with `Rust`. 

Here are the main features: 
* [wip] Manage ADR lifecycle (create, obsoletes...). ADR should be written in _asciidoc_
* [not started yet] an `init` command
* [not started yet] Integrate with Git
* [not started yet] Manage Tags and why not search
* [not started yet] Support different types of templates
* [not started yet] Why not integrate with Microsoft Teams

## Installation 
The current code line is tested on `MacOs / Rust 1.39` and build with `cargo`. Once all this pre-requisites installed, cloning the repo, `cargo test`and `cargo build` should be enough. 

Run `adr config list` to view the default configuration. __Displayed `Path` should exist prior to usage__, as an `init` command is missing. 



## Play...

```
[omallassi@omallassi-mac adrust]$./target/debug/adr -h

adr 0.1.0
A CLI to help you manage your ADR in git

USAGE:
    adr [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    completed-by     Complete a decision with another decision
    config           List All the Configuration Items
    decided          update the Status to Decide
    help             Prints this message or the help of the given subcommand(s)
    list             Lists all Decision Records
    new              will create a new Decision Record
    superseded-by    update the Status to Decide
```

In more details, 

| Command        | Description           |
| ------------- | ------------- |
| `adr list`      | will list all the ADR |
| `adr config list`     | will list the configuration. Configuration is stored in [config_dir](https://docs.rs/directories/2.0.2/directories/struct.ProjectDirs.html#method.config_dir) |
| `adr config set --name prop --value val`      | will set the configuration property |
| `adr new --name my-decision`      | will create a new decision  |
| `adr decided --name my-decision.md`      | will transition an ADR to decided |
| `adr superseded-by --name my-decision.md --by my-new-decision.md`      | will supersed an ADR `by` the specified one |


## ADR Template & lifecycle
For now, template should be in _asciidoc_. Look at `./templates/adr-temaplate-v0.1.adoc` (in particularly the header) for more details. ADR lifecycle is managed based on the `cl-*` information. 