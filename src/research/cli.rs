use super::{batch, comparison, inventory as inv, plan, runner};
use crate::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Value, json};
use std::path::Path;

fn arg(name: &'static str) -> Arg {
    Arg::new(name).long(name).required(true)
}
fn flag(name: &'static str) -> Arg {
    Arg::new(name).long(name).action(ArgAction::SetTrue)
}
pub fn commands() -> [Command; 3] {
    [
    Command::new("inventory").about("Seal, verify and copy retained file trees offline; never delete data")
        .subcommand(Command::new("seal").about("Create a fresh immutable inventory of every retained member").arg(arg("source")).arg(arg("out")).arg(Arg::new("kind").long("kind").default_value("data").value_parser(["data","capture","code","config"])).arg(Arg::new("closed-by").long("closed-by")))
        .subcommand(Command::new("verify").about("Refuse changed bytes, member sets, unsafe paths and links").arg(arg("source")).arg(arg("inventory")).arg(Arg::new("sha256").long("sha256")))
        .subcommand(Command::new("copy").about("Relocate exact retained bytes into a fresh directory, offline").arg(arg("source")).arg(arg("inventory")).arg(arg("sha256")).arg(arg("out"))),
    Command::new("experiment").about("Freeze and execute bounded trusted external evaluators; retain failures and explicit reviews")
        .subcommand(Command::new("validate").about("Validate a plan and frozen identities without running commands or creating an experiment").arg(arg("plan")))
        .subcommand(Command::new("create").about("Exclusively reserve a fresh identity and import shared verified snapshots").arg(arg("plan")).arg(arg("root")).arg(flag("dry-run")))
        .subcommand(Command::new("run").about("Run one reserved attempt once with exact argv and bounded owned process groups").arg(arg("dir")).arg(flag("dry-run")))
        .subcommand(Command::new("inspect").about("Verify and inspect real process evidence, artifacts and review state").arg(arg("dir")))
        .subcommand(Command::new("compare").about("Retain exact common/unmatched normalized cases under the same fixed inputs and evaluator").arg(arg("baseline")).arg(arg("candidate")).arg(arg("out")))
        .subcommand(Command::new("review").about("Record a durable private review/disposition; never automatically promote").arg(arg("dir")).arg(arg("disposition").value_parser(["continue","revise","nominate","inconclusive","retire"])).arg(arg("note")))
        .subcommand(Command::new("export").about("Create fresh allowlisted JSON and Markdown lifecycle summaries without private raw evidence").arg(arg("dir")).arg(arg("out"))),
    Command::new("batch").about("Run a bounded pool of independent attempts sharing immutable snapshots")
        .subcommand(Command::new("run").about("Execute every named member or retain an explicit not-started/failure status").arg(arg("root")).arg(arg("plan")))
        .subcommand(Command::new("inspect").about("Verify current member integrity and read historical batch completion").arg(arg("dir")))
]
}
fn value<'a>(args: &'a ArgMatches, key: &str) -> &'a str {
    args.get_one::<String>(key).expect("required argument")
}
fn path<'a>(args: &'a ArgMatches, key: &str) -> &'a Path {
    Path::new(value(args, key))
}
pub fn execute(family: &str, args: &ArgMatches) -> Result<Value> {
    let (action, args) = args
        .subcommand()
        .ok_or("choose a subcommand; read --help")?;
    match (family, action) {
        ("inventory", "seal") => inv::seal(
            path(args, "source"),
            path(args, "out"),
            value(args, "kind"),
            args.get_one::<String>("closed-by").cloned(),
        ),
        ("inventory", "verify") => {
            let inventory = path(args, "inventory");
            let digest = inv::digest(inventory)?;
            if args
                .get_one::<String>("sha256")
                .is_some_and(|expected| expected != &digest)
            {
                return Err("inventory hash mismatch".into());
            }
            let data: inv::Inventory = inv::read(inventory)?;
            inv::verify(path(args, "source"), &data)?;
            Ok(
                json!({"verified":true,"sha256":digest,"files":data.files.len(),"scope":"retained bytes/member integrity; no venue completeness or financial proof"}),
            )
        }
        ("inventory", "copy") => {
            let data = inv::load(path(args, "inventory"), value(args, "sha256"))?;
            inv::copy(path(args, "source"), &data, path(args, "out"))?;
            Ok(
                json!({"copied":true,"files":data.files.len(),"source_manifest_rewritten":false,"scope":"byte relocation only; declared external dependencies may still be needed"}),
            )
        }
        ("experiment", "validate") => plan::validate_file(path(args, "plan")),
        ("experiment", "create") => {
            if args.get_flag("dry-run") {
                plan::validate_file(path(args, "plan"))
            } else {
                plan::create(path(args, "plan"), path(args, "root"))
            }
        }
        ("experiment", "run") => {
            if args.get_flag("dry-run") {
                let (_, p, f) = plan::load(path(args, "dir"))?;
                Ok(
                    json!({"id":p.id,"plan_sha256":f.plan_sha256,"executed":false,"experiment_writes":false,"commands":p.commands}),
                )
            } else {
                runner::run(path(args, "dir"), None, None)
            }
        }
        ("experiment", "inspect") => runner::inspect(path(args, "dir")),
        ("experiment", "compare") => comparison::compare(
            path(args, "baseline"),
            path(args, "candidate"),
            path(args, "out"),
        ),
        ("experiment", "review") => comparison::review(
            path(args, "dir"),
            value(args, "disposition"),
            path(args, "note"),
        ),
        ("experiment", "export") => comparison::export(path(args, "dir"), path(args, "out")),
        ("batch", "run") => batch::run(path(args, "root"), path(args, "plan")),
        ("batch", "inspect") => batch::inspect(path(args, "dir")),
        _ => Err("unknown lifecycle operation".into()),
    }
}
