// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     tracing_subscriber::fmt::init();

//     let basedirs = xdg::BaseDirectories::with_prefix("wallpaper_manager")?;

//     let config = read_config(&basedirs)?;

//     let category = config
//         .categories
//         .iter()
//         .find(|v| v.name == "ruby")
//         .unwrap_or_else(|| {
//             panic!("No category in {:?}", config);
//         });

//     let image = fetch_category(
//         SearchConfig {
//             tags: category.tags.clone(),
//             aspect_ratios: config.aspect_ratios,
//         },
//         WallhavenSupplier,
//     )
//     .await?;

//     let image_path = image.cache(&basedirs).await?;

//     if let Some(command) = config.set_command {
//         let (program, args) = command.split_once(' ').unwrap_or((command.as_str(), ""));
//         let args = args.replace("{path}", image_path.to_str().unwrap());
//         let args = args.split(' ');

//         let result = std::process::Command::new(program)
//             .args(args)
//             .output()
//             .unwrap();
//     };

//     Ok(())
// }

use std::process::ExitCode;

use walltz::Program;

#[tokio::main]
async fn main() -> ExitCode {
    Program::init().await
}
