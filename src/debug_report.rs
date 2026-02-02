use astorion::{NodeSummary, ParseDetails};

mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const DIM: &str = "\x1b[2m";
    pub const BOLD: &str = "\x1b[1m";

    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const CYAN: &str = "\x1b[36m";
    pub const GRAY: &str = "\x1b[90m";

    pub struct Palette {
        enabled: bool,
    }

    impl Palette {
        pub fn new(enabled: bool) -> Self {
            Self { enabled }
        }

        pub fn paint(&self, s: impl AsRef<str>, color: &str) -> String {
            if self.enabled { format!("{}{}{}", color, s.as_ref(), RESET) } else { s.as_ref().to_string() }
        }

        pub fn bold(&self, s: impl AsRef<str>) -> String {
            if self.enabled { format!("{}{}{}", BOLD, s.as_ref(), RESET) } else { s.as_ref().to_string() }
        }

        pub fn dim(&self, s: impl AsRef<str>) -> String {
            if self.enabled { format!("{}{}{}", DIM, s.as_ref(), RESET) } else { s.as_ref().to_string() }
        }
    }
}

pub fn print_run(input: &str, details: &ParseDetails, color: bool) {
    let palette = ansi::Palette::new(color);
    println!("\n{}", palette.bold(palette.paint(format!("⚙  Parsing: \"{}\"", input), ansi::CYAN)));

    // Saturation summary
    println!("\n{}", palette.paint("━━━ Saturation ━━━", ansi::GRAY));
    print_saturation(details, &palette);

    if details.regex_profile.is_some() {
        println!("\n{}", palette.paint("━━━ Regex Profiling ━━━", ansi::GRAY));
        print_regex_profile(details, &palette);
    }

    // Results
    println!("\n{}", palette.paint("━━━ Results ━━━", ansi::GRAY));
    if details.all_candidates.is_empty() {
        println!("{}", palette.dim("  No tokens produced"));
        println!("\n{}", palette.paint("Possible reasons:", ansi::YELLOW));
        println!("  • Rules were filtered out (check bucket/phrase requirements)");
        println!("  • Regex patterns didn't match");
        println!("  • Production functions returned None");
        println!("\n{}", palette.dim("  Tip: Set RUSTLING_DEBUG_RULES=1 to see rule filtering details"));
    } else {
        // Keep CLI output compact: print the final resolved candidates.
        print_results(details, &palette);
    }

    // Timing
    println!("\n{}", palette.paint("━━━ Timing ━━━", ansi::GRAY));
    println!(
        "  Total: {}  │  Saturation: {}  │  Resolve: {}",
        palette.paint(format!("{:?}", details.total), ansi::GREEN),
        palette.paint(format!("{:?}", details.saturation_total), ansi::CYAN),
        palette.dim(format!("{:?}", details.resolve)),
    );
    println!();
}

fn print_saturation(details: &ParseDetails, palette: &ansi::Palette) {
    for pass in &details.saturation {
        let label = if pass.pass == 0 { "Pass 0 (regex):".to_string() } else { format!("Pass {}:", pass.pass) };

        println!(
            "  {} {}",
            palette.paint(label, ansi::BLUE),
            if pass.produced > 0 {
                palette.paint(format!("✓ {} tokens", pass.produced), ansi::GREEN)
            } else {
                palette.dim(format!("✗ {} tokens", pass.produced))
            }
        );

        for node in pass.samples.iter().take(5) {
            println!("    {}", fmt_node_compact(node, palette));
        }
        if pass.samples.len() > 5 {
            println!("    {}", palette.dim(format!("... +{} more", pass.samples.len() - 5)));
        }
    }
}

fn print_results(details: &ParseDetails, palette: &ansi::Palette) {
    for (idx, ent) in details.all_candidates.iter().enumerate() {
        println!(
            "  {} {} {} {}",
            palette.paint(format!("[{}]", idx), ansi::GRAY),
            palette.bold(palette.paint(&ent.value, ansi::GREEN)),
            palette.dim("│"),
            palette.paint(format!("span {}..{}", ent.start, ent.end), ansi::YELLOW),
        );
        println!(
            "      {} {}  {} {}",
            palette.dim("dim:"),
            palette.paint(&ent.name, ansi::BLUE),
            palette.dim("│ rule:"),
            palette.paint(&ent.rule, ansi::CYAN)
        );
    }
}

fn print_regex_profile(details: &ParseDetails, palette: &ansi::Palette) {
    let Some(profile) = &details.regex_profile else {
        return;
    };

    println!(
        "  Total regex time: {}  │  Matches: {}",
        palette.paint(format!("{:?}", profile.total_time), ansi::GREEN),
        palette.paint(profile.total_matches.to_string(), ansi::BLUE)
    );

    if profile.rules.is_empty() {
        println!("  {}", palette.dim("No regex rules executed"));
        return;
    }

    for rule in &profile.rules {
        println!(
            "  {} {}  {} {}  {} {}",
            palette.paint(rule.rule, ansi::CYAN),
            palette.dim(format!("{:?}", rule.total_time)),
            palette.dim("evals:"),
            palette.paint(rule.evaluations.to_string(), ansi::YELLOW),
            palette.dim("matches:"),
            palette.paint(rule.matches.to_string(), ansi::YELLOW)
        );
    }
}

fn fmt_node_compact(node: &NodeSummary, palette: &ansi::Palette) -> String {
    format!(
        "{} {} {}",
        palette.paint(format!("{}..{}", node.start, node.end), ansi::YELLOW),
        palette.paint(&node.rule, ansi::BLUE),
        palette.dim(node.preview.clone())
    )
}
