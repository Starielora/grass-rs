// Frame stats aggregation. Values are pushed every frame (from the GPU profiler,
// CPU timers, counters, ...) and averaged over a fixed interval; the GUI renders
// the snapshot without knowing any stat by name.

#[derive(Clone, Copy)]
pub enum StatValue {
    Time(std::time::Duration),
    Count(u64),
    Float(f64),
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Time,
    Count,
    Float,
}

impl StatValue {
    fn decompose(self) -> (Kind, f64) {
        match self {
            StatValue::Time(d) => (Kind::Time, d.as_nanos() as f64),
            StatValue::Count(c) => (Kind::Count, c as f64),
            StatValue::Float(f) => (Kind::Float, f),
        }
    }

    fn compose(kind: Kind, v: f64) -> StatValue {
        match kind {
            Kind::Time => StatValue::Time(std::time::Duration::from_nanos(v as u64)),
            Kind::Count => StatValue::Count(v as u64),
            Kind::Float => StatValue::Float(v),
        }
    }

    pub fn format(&self) -> String {
        match self {
            StatValue::Time(d) => {
                let ms = d.as_secs_f64() * 1e3;
                if ms >= 1.0 {
                    format!("{ms:.2} ms")
                } else {
                    format!("{:.0} µs", ms * 1e3)
                }
            }
            StatValue::Count(c) => format!("{c}"),
            StatValue::Float(f) => format!("{f:.2}"),
        }
    }
}

// flushed snapshots kept per stat for sparklines (at 1s flush interval = 1 minute)
const HISTORY_LEN: usize = 60;

pub struct StatRow {
    pub name: &'static str,
    pub depth: u8,
    kind: Kind,
    avg: f64,
    min: f64,
    max: f64,
    history: std::vec::Vec<f32>,
}

impl StatRow {
    pub fn avg(&self) -> StatValue {
        StatValue::compose(self.kind, self.avg)
    }

    pub fn min(&self) -> StatValue {
        StatValue::compose(self.kind, self.min)
    }

    pub fn max(&self) -> StatValue {
        StatValue::compose(self.kind, self.max)
    }

    /// Flushed averages, oldest first. Time stats are in milliseconds,
    /// the rest in their natural unit.
    pub fn history(&self) -> &[f32] {
        &self.history
    }
}

struct Accum {
    name: &'static str,
    depth: u8,
    kind: Kind,
    sum: f64,
    min: f64,
    max: f64,
    count: u32,
}

pub struct StatsAggregator {
    interval: std::time::Duration,
    // push order defines display order
    pending: std::vec::Vec<Accum>,
    snapshot: std::vec::Vec<StatRow>,
    // outlives snapshots so sparklines survive across flushes
    histories: std::vec::Vec<(&'static str, std::vec::Vec<f32>)>,
    last_flush: std::time::Instant,
}

impl StatsAggregator {
    pub fn new(interval: std::time::Duration) -> Self {
        Self {
            interval,
            pending: std::vec::Vec::new(),
            snapshot: std::vec::Vec::new(),
            histories: std::vec::Vec::new(),
            last_flush: std::time::Instant::now(),
        }
    }

    pub fn push(&mut self, name: &'static str, depth: u8, value: StatValue) {
        let (kind, v) = value.decompose();

        match self.pending.iter_mut().find(|a| a.name == name) {
            Some(a) => {
                a.sum += v;
                a.min = a.min.min(v);
                a.max = a.max.max(v);
                a.count += 1;
            }
            None => self.pending.push(Accum {
                name,
                depth,
                kind,
                sum: v,
                min: v,
                max: v,
                count: 1,
            }),
        }
    }

    /// Call once per frame. Every `interval` the pending samples are averaged
    /// into a new snapshot and cleared.
    pub fn tick(&mut self) {
        if self.pending.is_empty() || self.last_flush.elapsed() < self.interval {
            return;
        }

        self.snapshot = self
            .pending
            .iter()
            .map(|a| {
                let avg = a.sum / a.count as f64;

                // sparkline sample in display units (ms for time stats)
                let sample = match a.kind {
                    Kind::Time => (avg / 1e6) as f32,
                    Kind::Count | Kind::Float => avg as f32,
                };
                let index = match self.histories.iter().position(|(name, _)| *name == a.name) {
                    Some(i) => i,
                    None => {
                        self.histories.push((a.name, std::vec::Vec::new()));
                        self.histories.len() - 1
                    }
                };
                let history = &mut self.histories[index].1;
                history.push(sample);
                if history.len() > HISTORY_LEN {
                    history.remove(0);
                }

                StatRow {
                    name: a.name,
                    depth: a.depth,
                    kind: a.kind,
                    avg,
                    min: a.min,
                    max: a.max,
                    history: history.clone(),
                }
            })
            .collect();

        self.pending.clear();
        self.last_flush = std::time::Instant::now();
    }

    pub fn snapshot(&self) -> &[StatRow] {
        &self.snapshot
    }
}
