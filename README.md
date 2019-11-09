# adr-tools

The idea is to provide a cli to managed ADR. One more? yes you are right. In fact that was also an excuse to play with `Rust`. 

Here are the main features: 
* Manage ADR lifecycle (create, obsoletes...)
* Integrate with Git
* Why not integrate with Microsoft Teams
* Manage Tags and why not search

## Installation 
The current code line is tested on `MacOs / Rust 1.39` and build with `cargo`. Once all this pre-requisites installes, cloning the repo, `cargo test`and `cargo build` should be enough

## Play...

```
[omallassi@omallassi-mac adr-tools]$./target/debug/adr -h

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
| Tables        | Are           | 
| ------------- |-------------| 
| `adr list`      | will list all the ADR
| `adr config list`     | will list the configuration. Configuration is stored in [config_dir](https://docs.rs/directories/2.0.2/directories/struct.ProjectDirs.html#method.config_dir) | 
| `adr config set --name prop --value val`      | will set the configuration property | 
| `adr new --name my-decision`      | will create a new decision  | 
| `adr decided --name my-decision.md`      | will transition an ADR to decided | 
| `adr superseded-by --name my-decision.md --by my-new-decision.md`      | will supersed an ADR `by` the specified one | 

