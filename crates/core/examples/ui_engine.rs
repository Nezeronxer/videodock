// Test-only JSON-lines transport to the real Engine; never installed with the app.
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use videodock_core::Engine;

fn main() -> anyhow::Result<()> {
    let directory = std::env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Missing isolated test profile"))?;
    let engine = Engine::new(directory.into())?;
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line?)?;
        let result = if request["snapshot"] == true {
            json!({})
        } else {
            engine
                .dispatch(request)
                .unwrap_or_else(|error| json!({"error": error.to_string()}))
        };
        serde_json::to_writer(
            &mut output,
            &json!({"result":result,"snapshot":engine.snapshot()}),
        )?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}
