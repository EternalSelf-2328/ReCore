//! Reads each `.css` file named on the command line and prints what the
//! interpreter found, for checking a theme by hand.

fn main() {
    let mut failed = false;
    for path in std::env::args().skip(1) {
        match recore_css::load(&path) {
            Ok(theme) => {
                println!("=== {}", path);
                println!("id: {}", theme.id);
                println!("name: {}", theme.name);
                println!("dark: {}", theme.palette.dark);
                println!("chat: {}", theme.palette.chat.hex());
                println!("sidebar: {}", theme.palette.sidebar.hex());
                println!("rail: {}", theme.palette.rail.hex());
                println!("text: {}", theme.palette.text.hex());
                println!("accent: {}", theme.palette.accent.hex());
                println!("radius: {:?}", theme.look.radius);
                println!("font: {:?}", theme.look.font_family);
                println!("options: {}", theme.options.len());
                println!("fonts: {}", theme.fonts.len());
                println!("native: {:?}", theme.native);
                for warning in &theme.warnings {
                    println!("warning: {warning}");
                }
            }
            Err(error) => {
                failed = true;
                eprintln!("=== {path}: {error}");
            }
        }
    }
    std::process::exit(if failed { 1 } else { 0 });
}
