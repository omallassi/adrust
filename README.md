# adr-tools
Yes, this is a POC, yes this is dirty, yes, it is not configurable (you know path are hard-coded etc...), Yes, this is written in Rust, No it is not yet integrated with git but I will (actually I did for apis-catalog...) but: 

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

you can then run, for instance 

`adr list` - will list all the ADR

`adr config list` - will list the configuration. Configuration is stored in [config_dir](https://docs.rs/directories/2.0.2/directories/struct.ProjectDirs.html#method.config_dir)

`adr config set --name prop --value val` - will set the configuration property. 

`adr new --name my-decision` - will create a new decision 

`adr decided --name my-decision.md` - will transition an ADR to decided

`adr superseded-by --name my-decision.md --by my-new-decision.md` - will supersed an ADR `by` the specified one

## ToDo's
so much
- [ ] Integration with Git
- [ ] Seq Number in the ADR
- [ ] publish on Teams - https://docs.microsoft.com/fr-fr/graph/api/channel-post-messages?view=graph-rest-beta&tabs=http
- ...