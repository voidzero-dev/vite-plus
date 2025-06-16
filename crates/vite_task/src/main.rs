use std::{
    env::{args, current_dir},
    path::PathBuf,
};

use vite_task::Args;

fn main() -> anyhow::Result<()> {
    vite_task::main(
        PathBuf::from("/Users/patr0nus/code/vuejs_core"),
        Args { tasks: vec!["build-dts".into()] },
    )
    // vite_task::main(
    //     current_dir()?,
    //     Args { tasks: args().skip(1).collect() },
    // )
}
