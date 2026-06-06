//! Riff Evolution — Start simple, evolve through competitive riffing.
//!
//! Shows 5 generations of riffing where agents try to outplay each other,
//! each generation getting more complex and surprising.

use agent_riff::*;

fn main() {
    println!("🎸 ══════════════════════════════════════════════════════════");
    println!("🎸  RIFF EVOLUTION — 5 Generations of Competitive Riffing");
    println!("🎸 ══════════════════════════════════════════════════════════\n");

    let mut session = RiffSession::new(vec![0, 1]);
    session.stale_threshold = 10; // Give it room to grow

    println!("Two agents compete. Each round, they try to outplay the other.");
    println!("The response mode evolves based on how surprising each round is.\n");

    // Simulate 5 generations, each with multiple rounds
    let generations = 5;
    let rounds_per_gen = 3;

    // Quality and surprise escalate across generations
    let quality_schedule = [
        // (agent_0_quality, agent_0_surprise, agent_1_quality, agent_1_surprise)
        (Quality::Ok, 0.2, Quality::Ok, 0.15),           // Gen 1: tentative
        (Quality::Ok, 0.4, Quality::Strong, 0.5),         // Gen 2: warming up
        (Quality::Strong, 0.6, Quality::Strong, 0.7),     // Gen 3: competitive
        (Quality::Strong, 0.8, Quality::Ok, 0.3),         // Gen 4: agent 0 pulls ahead
        (Quality::Strong, 0.9, Quality::Strong, 0.95),    // Gen 5: both on fire
    ];

    for generation in 0..generations {
        let (q0_base, s0_base, q1_base, s1_base) = quality_schedule[generation];
        println!("━━━ GENERATION {} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", generation + 1);
        println!("  Mode: {:?}\n", session.mode);

        for round_num in 0..rounds_per_gen {
            session.new_round();

            // Add some variation within each generation
            let variation = round_num as f64 * 0.1;
            let s0 = (s0_base as f64 + variation).min(1.0);
            let s1 = (s1_base as f64 + variation + 0.05).min(1.0);

            session.riff(0, q0_base, s0);
            session.riff(1, q1_base, s1);

            let summary = session.evaluate();

            let quality_bar = |q: Quality| -> &str {
                match q { Quality::Weak => "▁", Quality::Ok => "▃", Quality::Strong => "▇" }
            };

            println!("  Round {} | A0: {} surprise={:.2} | A1: {} surprise={:.2} | {} → {:?}",
                session.current_round,
                quality_bar(q0_base), s0,
                quality_bar(q1_base), s1,
                if summary.productive { "✅ productive" } else { "⚪ flat" },
                summary.mode,
            );

            if summary.landed {
                println!("           🎯 LANDED! This riff hit the moment!");
            }
        }

        // Generation summary
        let metrics = session.metrics();
        println!("\n  📊 Gen {} Summary: surprise={:.2} | strong_ratio={:.0}% | streak={}",
            generation + 1,
            metrics.avg_surprise,
            metrics.strong_riff_ratio * 100.0,
            metrics.streak,
        );

        // Show direction of next generation
        println!("  Next mode: {:?}\n", session.mode);
    }

    // Final report
    let metrics = session.metrics();
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║  RIFF EVOLUTION — FINAL REPORT                        ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    println!("║  Total rounds:      {:>33}  ║", metrics.total_rounds);
    println!("║  Productive rounds: {:>33}  ║", metrics.productive_rounds);
    println!("║  Total surprise:    {:>33.2}  ║", metrics.total_surprise);
    println!("║  Avg surprise:      {:>33.2}  ║", metrics.avg_surprise);
    println!("║  Strong riff ratio: {:>32.0}%  ║", metrics.strong_riff_ratio * 100.0);
    println!("║  Best streak:       {:>33}  ║", metrics.streak);
    println!("║  Landed moments:    {:>33}  ║", metrics.landed_count);
    println!("║  Session finished:  {:>33}  ║", if session.finished { "Yes (went stale)" } else { "No (still hot)" });
    println!("╚═══════════════════════════════════════════════════════╝");

    println!("\n💡 Each generation escalates the surprise. When surprise drops,");
    println!("   the response mode shifts: Escalate → Invert → Pivot → Provoked.");
    println!("   The competition IS the collaboration. The output IS the communication.");
}
