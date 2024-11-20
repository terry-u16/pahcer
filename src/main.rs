pub(crate) mod runner;
pub(crate) mod settings;

use anyhow::Result;
use settings::gen_setting_file;

fn main() -> Result<()> {
    gen_setting_file();
    let settings = settings::load_setting_file()?;
    dbg!(settings);
    Ok(())
}
