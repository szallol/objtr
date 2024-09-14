use anyhow::{Context, Result};
use obj::{load_obj, Obj};
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<()> {
    let input = BufReader::new(File::open(
        "c:/work/_help/terra_obj/BlockBABB/BlockBABB.obj ",
    )?);
    let model: Obj = load_obj(input)?;
    model.vertices.iter().for_each(|v| {
        println!("v: {:?}", v);
    });

    Ok(())
}
