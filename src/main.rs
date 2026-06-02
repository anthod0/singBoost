use singboost::{AppPaths, KernelCommand};

fn main() {
    let paths = AppPaths::from_current_exe().expect("failed to resolve application directory");
    let run_command = KernelCommand::run(&paths);

    println!(
        "SingBoost initialized. Kernel command: {} {}",
        run_command.program.display(),
        run_command.args.join(" ")
    );
}
