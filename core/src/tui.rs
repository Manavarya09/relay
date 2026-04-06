//! Beautiful terminal UI for Relay — spinners, boxes, interactive prompts.

use colored::Colorize;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

// ─── Banner ─────────────────────────────────────────────────────────────────

pub fn print_banner() {
    let banner = r#"
  ╔═══════════════════════════════════════════════╗
  ║                                               ║
  ║   ⚡ R E L A Y                                ║
  ║   Cross-agent context handoff                 ║
  ║                                               ║
  ╚═══════════════════════════════════════════════╝
"#;
    eprintln!("{}", banner.cyan());
}

// ─── Spinners ───────────────────────────────────────────────────────────────

pub fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn step(num: usize, total: usize, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template(&format!(
            "  {{spinner:.cyan}} [{}/{}] {{msg}}",
            num, total
        ))
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

// ─── Boxes ──────────────────────────────────────────────────────────────────

pub fn print_box(title: &str, content: &str) {
    let term_width = Term::stdout().size().1 as usize;
    let width = term_width.min(72).max(40);
    let inner = width - 4;

    // Top border
    eprintln!("  ╭{}╮", "─".repeat(inner + 2));

    // Title
    let title_padded = format!(" {} ", title);
    let pad = inner.saturating_sub(title_padded.len()) + 1;
    eprintln!("  │{}{}│", title_padded.bold().cyan(), " ".repeat(pad));

    // Separator
    eprintln!("  ├{}┤", "─".repeat(inner + 2));

    // Content lines
    for line in content.lines() {
        let display_line = if line.len() > inner {
            let mut end = inner.saturating_sub(1);
            while end > 0 && !line.is_char_boundary(end) { end -= 1; }
            format!("{}…", &line[..end])
        } else {
            line.to_string()
        };
        let pad = inner.saturating_sub(display_line.len()) + 1;
        eprintln!("  │ {}{} │", display_line, " ".repeat(pad.saturating_sub(1)));
    }

    // Bottom border
    eprintln!("  ╰{}╯", "─".repeat(inner + 2));
}

pub fn print_section(icon: &str, title: &str) {
    eprintln!();
    eprintln!("  {} {}", icon, title.bold());
    eprintln!("  {}", "─".repeat(50).dimmed());
}

// ─── Agent Select ───────────────────────────────────────────────────────────

pub fn select_agent(agents: &[(String, bool, String)]) -> Option<String> {
    let items: Vec<String> = agents
        .iter()
        .map(|(name, available, reason)| {
            if *available {
                format!("✅  {} — {}", name, reason)
            } else {
                format!("❌  {} — {}", name, reason)
            }
        })
        .collect();

    eprintln!();
    let selection = dialoguer::FuzzySelect::with_theme(
        &dialoguer::theme::ColorfulTheme::default(),
    )
    .with_prompt("  Select target agent")
    .items(&items)
    .default(0)
    .interact_opt()
    .ok()
    .flatten()?;

    let (name, available, _) = &agents[selection];
    if !*available {
        eprintln!(
            "\n  {} {} is not available.",
            "⚠️ ".yellow(),
            name.bold()
        );
        return None;
    }

    Some(name.clone())
}

// ─── Status Display ─────────────────────────────────────────────────────────

pub fn print_snapshot(snapshot: &crate::SessionSnapshot) {
    eprintln!();
    let term_width = Term::stdout().size().1 as usize;
    let width = term_width.min(72).max(40);
    eprintln!("  {}", "═".repeat(width).cyan());
    eprintln!(
        "  {}  {}",
        "📋".to_string(),
        "Session Snapshot".bold().cyan()
    );
    eprintln!("  {}", "═".repeat(width).cyan());

    // Project + time
    eprintln!();
    eprintln!("  {}  {}", "📁", snapshot.project_dir.dimmed());
    eprintln!("  {}  {}", "🕐", snapshot.timestamp.dimmed());

    // Current task
    print_section("🎯", "Current Task");
    eprintln!("  {}", snapshot.current_task);

    // Todos
    if !snapshot.todos.is_empty() {
        print_section("📝", "Progress");
        for t in &snapshot.todos {
            let (icon, style) = match t.status.as_str() {
                "completed" => ("✅", t.content.dimmed().to_string()),
                "in_progress" => ("🔄", t.content.yellow().bold().to_string()),
                _ => ("⏳", t.content.normal().to_string()),
            };
            eprintln!("  {icon}  {style}");
        }
    }

    // Last error
    if let Some(ref err) = snapshot.last_error {
        print_section("🚨", "Last Error");
        for line in err.lines().take(5) {
            eprintln!("  {}", line.red());
        }
    }

    // Decisions
    if !snapshot.decisions.is_empty() {
        print_section("💡", "Key Decisions");
        for d in &snapshot.decisions {
            eprintln!("  • {}", d.dimmed());
        }
    }

    // Git
    if let Some(ref git) = snapshot.git_state {
        print_section("🔀", "Git State");
        eprintln!("  Branch: {}", git.branch.green());
        eprintln!("  {}", git.status_summary);
        if !git.recent_commits.is_empty() {
            eprintln!();
            for c in git.recent_commits.iter().take(3) {
                eprintln!("  {}", c.dimmed());
            }
        }
    }

    // Changed files
    if !snapshot.recent_files.is_empty() {
        print_section("📄", &format!("Changed Files ({})", snapshot.recent_files.len()));
        for f in snapshot.recent_files.iter().take(10) {
            eprintln!("  {f}");
        }
    }

    // Conversation
    if !snapshot.conversation.is_empty() {
        print_section(
            "💬",
            &format!("Conversation ({} turns)", snapshot.conversation.len()),
        );
        let start = snapshot.conversation.len().saturating_sub(10);
        for turn in &snapshot.conversation[start..] {
            let (prefix, _color) = match turn.role.as_str() {
                "user" => ("👤 YOU ", turn.content.normal().to_string()),
                "assistant" => ("🤖 AI  ", turn.content.cyan().to_string()),
                "assistant_tool" => ("🔧 TOOL", turn.content.dimmed().to_string()),
                "tool_result" => ("📤 OUT ", turn.content.dimmed().to_string()),
                _ => ("   ", turn.content.normal().to_string()),
            };
            let short = if turn.content.len() > 90 {
                let mut end = 85;
                while end > 0 && !turn.content.is_char_boundary(end) { end -= 1; }
                format!("{}…", &turn.content[..end])
            } else {
                turn.content.clone()
            };
            let styled = match turn.role.as_str() {
                "user" => short.normal().to_string(),
                "assistant" => short.cyan().to_string(),
                "assistant_tool" => short.dimmed().to_string(),
                "tool_result" => short.dimmed().to_string(),
                _ => short,
            };
            eprintln!("  {} {}", prefix.dimmed(), styled);
        }
    }

    eprintln!();
    eprintln!("  {}", "═".repeat(width).cyan());
}

// ─── Agents Display ─────────────────────────────────────────────────────────

pub fn print_agents(
    priority: &[String],
    statuses: &[crate::AgentStatus],
) {
    eprintln!();
    let term_width = Term::stdout().size().1 as usize;
    let width = term_width.min(72).max(40);
    eprintln!("  {}", "═".repeat(width).cyan());
    eprintln!("  {}  {}", "🤖", "Available Agents".bold().cyan());
    eprintln!("  {}", "═".repeat(width).cyan());
    eprintln!();
    eprintln!(
        "  Priority: {}",
        priority
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" → ")
            .dimmed()
    );
    eprintln!();

    for s in statuses {
        if s.available {
            eprintln!(
                "  \u{2705}  {:<12} {}",
                s.name.green().bold(),
                s.reason.dimmed()
            );
            if let Some(ref v) = s.version {
                eprintln!("     {:<12} {}", "", format!("v{v}").dimmed());
            }
        } else {
            eprintln!(
                "  \u{274c}  {:<12} {}",
                s.name.dimmed(),
                s.reason.dimmed()
            );
        }
    }

    let available = statuses.iter().filter(|s| s.available).count();
    eprintln!();
    if available == 0 {
        eprintln!(
            "  \u{26a0}\u{fe0f}  {}",
            "No agents available. Run 'relay init' to configure.".yellow()
        );
    } else {
        eprintln!(
            "  \u{1f680} {} agent{} ready for handoff",
            available.to_string().green().bold(),
            if available == 1 { "" } else { "s" }
        );
    }
    eprintln!();
}

// ─── Handoff Result ─────────────────────────────────────────────────────────

pub fn print_handoff_success(agent: &str, file: &str) {
    eprintln!();
    eprintln!(
        "  \u{2705} {}",
        format!("Handed off to {agent}").green().bold()
    );
    eprintln!("  \u{1f4c4} {}", file.dimmed());
    eprintln!();
}

pub fn print_handoff_fail(message: &str, file: &str) {
    eprintln!();
    eprintln!("  \u{274c} {}", message.red());
    eprintln!();
    eprintln!("  \u{1f4a1} Context saved \u{2014} copy-paste into any AI:");
    eprintln!("     {}", file.cyan());
    eprintln!();
}
