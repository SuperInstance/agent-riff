//! # agent-riff
//!
//! Competitive riffing for agents. Not propose→critique→approve. Build→respond→escalate.
//! Two agents trying to outplay each other produce something neither could alone.
//! The competition IS the collaboration. The output IS the communication.

#![forbid(unsafe_code)]

use std::collections::VecDeque;

/// Quality assessment of a riff output: Weak (-1), Ok (0), Strong (+1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Weak = -1,
    Ok = 0,
    Strong = 1,
}

impl Quality {
    pub fn to_i8(self) -> i8 { self as i8 }
    pub fn from_i8(v: i8) -> Option<Self> {
        match v { -1 => Some(Quality::Weak), 0 => Some(Quality::Ok), 1 => Some(Quality::Strong), _ => None }
    }
}

/// A single riff — an agent's output in one round.
#[derive(Debug, Clone)]
pub struct Riff {
    pub agent_id: u32,
    pub round: u32,
    pub content_hash: u64,    // Hash of the output (domain-specific)
    pub quality: Quality,
    pub surprise: f64,         // How different from previous riffs (0.0-1.0)
    pub direction: i8,         // -1=reversed direction, 0=extended, +1=new direction
}

impl Riff {
    pub fn new(agent_id: u32, round: u32, quality: Quality, surprise: f64, direction: i8) -> Self {
        Self { agent_id, round, content_hash: 0, quality, surprise: surprise.clamp(0.0, 1.0), direction: direction.clamp(-1, 1) }
    }
}

/// A riff round — one exchange between agents.
#[derive(Debug, Clone)]
pub struct Round {
    pub number: u32,
    pub riffs: Vec<Riff>,
    pub total_surprise: f64,
    pub quality_gap: i8,       // Difference between best and worst riff
}

impl Round {
    fn new(number: u32) -> Self { Self { number, riffs: Vec::new(), total_surprise: 0.0, quality_gap: 0 } }
    fn add(&mut self, riff: Riff) {
        self.total_surprise += riff.surprise;
        self.riffs.push(riff);
        self.recalc();
    }
    fn recalc(&mut self) {
        if self.riffs.is_empty() { return; }
        let qualities: Vec<i8> = self.riffs.iter().map(|r| r.quality.to_i8()).collect();
        self.quality_gap = qualities.iter().max().unwrap_or(&0) - qualities.iter().min().unwrap_or(&0);
    }
    /// Did this round produce surprising output?
    pub fn was_productive(&self) -> bool { self.total_surprise > 0.3 || self.quality_gap > 0 }
    /// Best riff in this round.
    pub fn best(&self) -> Option<&Riff> { self.riffs.iter().max_by_key(|r| r.quality.to_i8()) }
}

/// How an agent responds to a previous riff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    /// Push the same direction further (amplify).
    Escalate,
    /// Push in a perpendicular direction (reframe).
    Pivot,
    /// Push in the opposite direction (challenge).
    Invert,
    /// Take the most provocative part and go somewhere new.
    Provoked,
}

impl ResponseMode {
    /// Choose response mode based on previous round's surprise level.
    pub fn auto(prev_surprise: f64, streak: u32) -> Self {
        if streak > 5 { ResponseMode::Pivot }       // Getting stale — reframe
        else if prev_surprise < 0.2 { ResponseMode::Provoked }  // Not surprising enough — provoke
        else if prev_surprise > 0.7 { ResponseMode::Escalate }  // Hot streak — keep pushing
        else { ResponseMode::Invert }                // Medium surprise — challenge
    }
    /// Direction offset this mode produces.
    pub fn direction(self) -> i8 {
        match self { ResponseMode::Escalate => 0, ResponseMode::Pivot => 1, ResponseMode::Invert => -1, ResponseMode::Provoked => 1 }
    }
}

/// The riff session — the jam where agents compete.
#[derive(Debug, Clone)]
pub struct RiffSession {
    pub agents: Vec<u32>,
    pub rounds: Vec<Round>,
    pub current_round: u32,
    pub mode: ResponseMode,
    pub streak: u32,            // Consecutive productive rounds
    pub stale_threshold: u32,   // Rounds without surprise before declaring stale
    pub landing_threshold: f64, // Surprise level that indicates "it landed"
    pub finished: bool,
    pub history: VecDeque<Quality>,  // Recent quality history
}

impl RiffSession {
    pub fn new(agents: Vec<u32>) -> Self {
        Self { agents, rounds: Vec::new(), current_round: 0, mode: ResponseMode::Escalate,
               streak: 0, stale_threshold: 5, landing_threshold: 0.8, finished: false, history: VecDeque::new() }
    }

    /// Start a new round.
    pub fn new_round(&mut self) -> &mut Round {
        let r = Round::new(self.current_round);
        self.rounds.push(r);
        self.current_round += 1;
        self.rounds.last_mut().unwrap()
    }

    /// Add a riff to the current round.
    pub fn riff(&mut self, agent_id: u32, quality: Quality, surprise: f64) {
        let direction = self.mode.direction();
        let riff = Riff::new(agent_id, self.current_round.saturating_sub(1), quality, surprise, direction);
        if let Some(round) = self.rounds.last_mut() { round.add(riff); }
        self.history.push_back(quality);
        if self.history.len() > 20 { self.history.pop_front(); }
    }

    /// Evaluate the round and choose next response mode.
    pub fn evaluate(&mut self) -> RoundSummary {
        let round = match self.rounds.last() {
            Some(r) => r,
            None => return RoundSummary { surprise: 0.0, productive: false, landed: false, mode: self.mode },
        };
        let surprise = round.total_surprise;
        let productive = round.was_productive();
        if productive { self.streak += 1; } else { self.streak = 0; }

        // Check if the riff has "landed" — a moment of unexpected quality
        let landed = surprise > self.landing_threshold &&
            round.riffs.iter().any(|r| r.quality == Quality::Strong);

        // Auto-select next mode
        self.mode = ResponseMode::auto(surprise, self.streak);

        // Check stale — too many unproductive rounds
        if self.streak == 0 && self.current_round > self.stale_threshold { self.finished = true; }

        RoundSummary { surprise, productive, landed, mode: self.mode }
    }

    /// Overall session metrics.
    pub fn metrics(&self) -> SessionMetrics {
        let total_rounds = self.rounds.len();
        let productive_rounds = self.rounds.iter().filter(|r| r.was_productive()).count();
        let total_surprise: f64 = self.rounds.iter().map(|r| r.total_surprise).sum();
        let strong_riffs = self.rounds.iter().flat_map(|r| r.riffs.iter()).filter(|r| r.quality == Quality::Strong).count();
        let total_riffs = self.rounds.iter().map(|r| r.riffs.len()).sum::<usize>().max(1);
        let avg_surprise = if total_rounds > 0 { total_surprise / total_rounds as f64 } else { 0.0 };

        SessionMetrics {
            total_rounds,
            productive_rounds,
            total_surprise,
            avg_surprise,
            strong_riff_ratio: strong_riffs as f64 / total_riffs as f64,
            streak: self.streak,
            landed_count: self.rounds.iter().filter(|r| r.total_surprise > self.landing_threshold).count(),
        }
    }
}

/// Summary of a single round's evaluation.
#[derive(Debug, Clone)]
pub struct RoundSummary {
    pub surprise: f64,
    pub productive: bool,
    pub landed: bool,
    pub mode: ResponseMode,
}

/// Overall session metrics.
#[derive(Debug, Clone)]
pub struct SessionMetrics {
    pub total_rounds: usize,
    pub productive_rounds: usize,
    pub total_surprise: f64,
    pub avg_surprise: f64,
    pub strong_riff_ratio: f64,
    pub streak: u32,
    pub landed_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn riff_creation() {
        let r = Riff::new(0, 1, Quality::Strong, 0.7, 1);
        assert_eq!(r.agent_id, 0);
        assert_eq!(r.round, 1);
        assert_eq!(r.quality, Quality::Strong);
        assert!(r.surprise <= 1.0);
    }
    #[test] fn round_add_and_eval() {
        let mut round = Round::new(0);
        round.add(Riff::new(0, 0, Quality::Ok, 0.5, 0));
        round.add(Riff::new(1, 0, Quality::Strong, 0.8, 1));
        assert_eq!(round.riffs.len(), 2);
        assert!(round.was_productive());
        assert!(round.best().unwrap().quality == Quality::Strong);
    }
    #[test] fn round_quality_gap() {
        let mut round = Round::new(0);
        round.add(Riff::new(0, 0, Quality::Weak, 0.1, 0));
        round.add(Riff::new(1, 0, Quality::Strong, 0.5, 1));
        assert_eq!(round.quality_gap, 2); // strong(1) - weak(-1) = 2
    }
    #[test] fn response_mode_auto() {
        assert_eq!(ResponseMode::auto(0.1, 0), ResponseMode::Provoked); // low surprise
        assert_eq!(ResponseMode::auto(0.8, 0), ResponseMode::Escalate); // high surprise
        assert_eq!(ResponseMode::auto(0.5, 6), ResponseMode::Pivot);    // stale streak
        assert_eq!(ResponseMode::auto(0.4, 0), ResponseMode::Invert);   // medium
    }
    #[test] fn session_basic_flow() {
        let mut s = RiffSession::new(vec![0, 1]);
        s.new_round();
        s.riff(0, Quality::Ok, 0.3);
        s.riff(1, Quality::Strong, 0.6);
        let summary = s.evaluate();
        assert!(summary.productive);
        assert_eq!(s.current_round, 1);
    }
    #[test] fn session_multi_round() {
        let mut s = RiffSession::new(vec![0, 1]);
        for round in 0..3 {
            s.new_round();
            s.riff(0, Quality::Strong, 0.7);
            s.riff(1, Quality::Strong, 0.8);
            s.evaluate();
        }
        let m = s.metrics();
        assert_eq!(m.total_rounds, 3);
        assert!(m.avg_surprise > 0.5);
        assert!(!s.finished);
    }
    #[test] fn session_stale_detection() {
        let mut s = RiffSession::new(vec![0, 1]);
        s.stale_threshold = 3;
        for _ in 0..4 {
            s.new_round();
            s.riff(0, Quality::Weak, 0.05); // low surprise, not productive
            s.riff(1, Quality::Weak, 0.05);
            s.evaluate();
        }
        assert!(s.finished);
    }
    #[test] fn session_landing() {
        let mut s = RiffSession::new(vec![0, 1]);
        s.landing_threshold = 0.5;
        s.new_round();
        s.riff(0, Quality::Strong, 0.9);
        s.riff(1, Quality::Strong, 0.8);
        let summary = s.evaluate();
        assert!(summary.landed);
    }
    #[test] fn session_metrics() {
        let mut s = RiffSession::new(vec![0, 1]);
        s.new_round();
        s.riff(0, Quality::Strong, 0.7);
        s.riff(1, Quality::Ok, 0.4);
        s.evaluate();
        let m = s.metrics();
        assert_eq!(m.total_rounds, 1);
        assert!(m.total_surprise > 1.0);
        assert_eq!(m.productive_rounds, 1);
    }
    #[test] fn response_mode_directions() {
        assert_eq!(ResponseMode::Escalate.direction(), 0);
        assert_eq!(ResponseMode::Pivot.direction(), 1);
        assert_eq!(ResponseMode::Invert.direction(), -1);
        assert_eq!(ResponseMode::Provoked.direction(), 1);
    }
    #[test] fn quality_roundtrip() {
        for v in [-1i8, 0, 1] { assert_eq!(Quality::from_i8(v).unwrap().to_i8(), v); }
    }
    #[test] fn surprise_clamped() {
        let r = Riff::new(0, 0, Quality::Ok, 1.5, 0);
        assert!(r.surprise <= 1.0);
        let r2 = Riff::new(0, 0, Quality::Ok, -0.5, 0);
        assert!(r2.surprise >= 0.0);
    }
}
