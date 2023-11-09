use owo_colors::{OwoColorize, Style};

pub fn setup_logger(level: Option<log::LevelFilter>) -> anyhow::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let style = match record.level().as_str() {
                "TRACE" => Style::new().purple(),
                "DEBUG" => Style::new().blue(),
                "WARN" => Style::new().yellow(),
                "ERROR" => Style::new().red(),
                _ => Style::new().bright_green(),
            };
            out.finish(format_args!(
                "{:<5} {} {} {}",
                record.level().style(style),
                record.target(),
                "~".fg_rgb::<128, 128, 128>(),
                message
            ))
        })
        .level(log::LevelFilter::Off)
        .level_for(
            "proxy_rs",
            if level.is_none() {
                log::LevelFilter::Info
            } else {
                level.unwrap()
            },
        )
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
