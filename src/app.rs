use rand::{
    distributions::{Distribution, Uniform},
    rngs::ThreadRng,
};
use tui::widgets::ListState;

const TASKS: [Message; 71] = [
    Message {
        message: "message1",
        speaker: "speaker1",
    },
    Message {
        message: "message2",
        speaker: "speaker2",
    },
    Message {
        message: "message3",
        speaker: "speaker3",
    },
    Message {
        message: "message4",
        speaker: "speaker4",
    },
    Message {
        message: "message5",
        speaker: "speaker5",
    },
    Message {
        message: "message6",
        speaker: "speaker6",
    },
    Message {
        message: "message7",
        speaker: "speaker7",
    },
    Message {
        message: "message8",
        speaker: "speaker8",
    },
    Message {
        message: "message9",
        speaker: "speaker9",
    },
    Message {
        message: "message10",
        speaker: "speaker10",
    },
    Message {
        message: "message11",
        speaker: "speaker11",
    },
    Message {
        message: "message12",
        speaker: "speaker12",
    },
    Message {
        message: "message13",
        speaker: "speaker13",
    },
    Message {
        message: "message14",
        speaker: "speaker14",
    },
    Message {
        message: "message15",
        speaker: "speaker15",
    },
    Message {
        message: "message16",
        speaker: "speaker16",
    },
    Message {
        message: "message17",
        speaker: "speaker17",
    },
    Message {
        message: "message18",
        speaker: "speaker18",
    },
    Message {
        message: "message19",
        speaker: "speaker19",
    },
    Message {
        message: "message20",
        speaker: "speaker20",
    },
    Message {
        message: "message21",
        speaker: "speaker21",
    },
    Message {
        message: "message22",
        speaker: "speaker22",
    },
    Message {
        message: "message23",
        speaker: "speaker23",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
    Message {
        message: "message24",
        speaker: "speaker24",
    },
];

const LOGS: [(&str, &str); 26] = [
    ("Event1", "INFO"),
    ("Event2", "INFO"),
    ("Event3", "CRITICAL"),
    ("Event4", "ERROR"),
    ("Event5", "INFO"),
    ("Event6", "INFO"),
    ("Event7", "WARNING"),
    ("Event8", "INFO"),
    ("Event9", "INFO"),
    ("Event10", "INFO"),
    ("Event11", "CRITICAL"),
    ("Event12", "INFO"),
    ("Event13", "INFO"),
    ("Event14", "INFO"),
    ("Event15", "INFO"),
    ("Event16", "INFO"),
    ("Event17", "ERROR"),
    ("Event18", "ERROR"),
    ("Event19", "INFO"),
    ("Event20", "INFO"),
    ("Event21", "WARNING"),
    ("Event22", "INFO"),
    ("Event23", "INFO"),
    ("Event24", "WARNING"),
    ("Event25", "INFO"),
    ("Event26", "INFO"),
];

const EVENTS: [(&str, u64); 24] = [
    ("B1", 9),
    ("B2", 12),
    ("B3", 5),
    ("B4", 8),
    ("B5", 2),
    ("B6", 4),
    ("B7", 5),
    ("B8", 9),
    ("B9", 14),
    ("B10", 15),
    ("B11", 1),
    ("B12", 0),
    ("B13", 4),
    ("B14", 6),
    ("B15", 4),
    ("B16", 6),
    ("B17", 4),
    ("B18", 7),
    ("B19", 13),
    ("B20", 8),
    ("B21", 11),
    ("B22", 9),
    ("B23", 3),
    ("B24", 5),
];

pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Clone)]
pub struct RandomSignal {
    distribution: Uniform<u64>,
    rng: ThreadRng,
}

impl RandomSignal {
    pub fn new(lower: u64, upper: u64) -> RandomSignal {
        RandomSignal {
            distribution: Uniform::new(lower, upper),
            rng: rand::thread_rng(),
        }
    }
}

impl Iterator for RandomSignal {
    type Item = u64;
    fn next(&mut self) -> Option<u64> {
        Some(self.distribution.sample(&mut self.rng))
    }
}

#[derive(Clone)]
pub struct SinSignal {
    x: f64,
    interval: f64,
    period: f64,
    scale: f64,
}

impl SinSignal {
    pub fn new(interval: f64, period: f64, scale: f64) -> SinSignal {
        SinSignal {
            x: 0.0,
            interval,
            period,
            scale,
        }
    }
}

impl Iterator for SinSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, (self.x * 1.0 / self.period).sin() * self.scale);
        self.x += self.interval;
        Some(point)
    }
}

pub struct TabsState<'a> {
    pub titles: Vec<&'a str>,
    pub index: usize,
}

impl<'a> TabsState<'a> {
    pub fn new(titles: Vec<&'a str>) -> TabsState {
        TabsState { titles, index: 0 }
    }
    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub struct Signal<S: Iterator> {
    source: S,
    pub points: Vec<S::Item>,
    tick_rate: usize,
}

impl<S> Signal<S>
where
    S: Iterator,
{
    fn on_tick(&mut self) {
        for _ in 0..self.tick_rate {
            self.points.remove(0);
        }
        self.points
            .extend(self.source.by_ref().take(self.tick_rate));
    }
}

pub struct Signals {
    pub sin1: Signal<SinSignal>,
    pub sin2: Signal<SinSignal>,
    pub window: [f64; 2],
}

impl Signals {
    fn on_tick(&mut self) {
        self.sin1.on_tick();
        self.sin2.on_tick();
        self.window[0] += 1.0;
        self.window[1] += 1.0;
    }
}

pub struct Server<'a> {
    pub name: &'a str,
    pub location: &'a str,
    pub coords: (f64, f64),
    pub status: &'a str,
}

#[derive(Clone, Copy)]
pub struct Message<'a> {
    pub message: &'a str,
    pub speaker: &'a str,
}

pub struct App<'a> {
    pub title: &'a str,
    pub should_quit: bool,
    pub tabs: TabsState<'a>,
    pub input: String,
    /// Current input mode
    pub input_mode: InputMode,
    /// History of recorded messages
    pub messages: Vec<String>,
    // pub show_chart: bool,
    // pub progress: f64,
    // pub sparkline: Signal<RandomSignal>,
    pub tasks: StatefulList<Message<'a>>,
    // pub logs: StatefulList<(&'a str, &'a str)>,
    // pub signals: Signals,
    // pub barchart: Vec<(&'a str, u64)>,
    // pub servers: Vec<Server<'a>>,
    // pub enhanced_graphics: bool,
}

impl<'a> App<'a> {
    pub fn new(title: &'a str, enhanced_graphics: bool) -> App<'a> {
        // let mut rand_signal = RandomSignal::new(0, 100);
        // let sparkline_points = rand_signal.by_ref().take(300).collect();
        // let mut sin_signal = SinSignal::new(0.2, 3.0, 18.0);
        // let sin1_points = sin_signal.by_ref().take(100).collect();
        // let mut sin_signal2 = SinSignal::new(0.1, 2.0, 10.0);
        // let sin2_points = sin_signal2.by_ref().take(200).collect();
        App {
            title,
            should_quit: false,
            tabs: TabsState::new(vec!["Tab0", "Tab1", "Tab2"]),
            // show_chart: true,
            // progress: 0.0,
            // sparkline: Signal {
            //     source: rand_signal,
            //     points: sparkline_points,
            //     tick_rate: 1,
            // },
            tasks: StatefulList::with_items(TASKS.to_vec()),
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            // logs: StatefulList::with_items(LOGS.to_vec()),
            // signals: Signals {
            //     sin1: Signal {
            //         source: sin_signal,
            //         points: sin1_points,
            //         tick_rate: 5,
            //     },
            //     sin2: Signal {
            //         source: sin_signal2,
            //         points: sin2_points,
            //         tick_rate: 10,
            //     },
            //     window: [0.0, 20.0],
            // },
            // barchart: EVENTS.to_vec(),
            // servers: vec![
            //     Server {
            //         name: "NorthAmerica-1",
            //         location: "New York City",
            //         coords: (40.71, -74.00),
            //         status: "Up",
            //     },
            //     Server {
            //         name: "Europe-1",
            //         location: "Paris",
            //         coords: (48.85, 2.35),
            //         status: "Failure",
            //     },
            //     Server {
            //         name: "SouthAmerica-1",
            //         location: "São Paulo",
            //         coords: (-23.54, -46.62),
            //         status: "Up",
            //     },
            //     Server {
            //         name: "Asia-1",
            //         location: "Singapore",
            //         coords: (1.35, 103.86),
            //         status: "Up",
            //     },
            // ],
            // enhanced_graphics,
        }
    }

    pub fn on_up(&mut self) {
        self.tasks.previous();
    }

    pub fn on_down(&mut self) {
        self.tasks.next();
    }

    pub fn on_right(&mut self) {
        self.tabs.next();
    }

    pub fn on_left(&mut self) {
        self.tabs.previous();
    }

    pub fn on_enter(&mut self) {
        match self.input_mode {
            InputMode::Editing => self.messages.push(self.input.drain(..).collect()),
            _ => {}
        }
    }

    pub fn on_esc(&mut self) {
        match self.input_mode {
            InputMode::Editing => self.input_mode = InputMode::Normal,
            _ => {}
        }
    }

    pub fn on_backspace(&mut self) {
        match self.input_mode {
            InputMode::Editing => {
                self.input.pop();
            }
            _ => {}
        }
    }

    pub fn on_key(&mut self, c: char) {
        match self.input_mode {
            InputMode::Normal => match c {
                'q' => {
                    self.should_quit = true;
                }
                'e' => self.input_mode = InputMode::Editing,
                _ => {}
            },
            InputMode::Editing => self.input.push(c),
        }
    }

    pub fn on_tick(&mut self) {
        // Update progress
        // self.progress += 0.001;
        // if self.progress > 1.0 {
        //     self.progress = 0.0;
        // }

        // self.sparkline.on_tick();
        // self.signals.on_tick();

        // let log = self.logs.items.pop().unwrap();
        // self.logs.items.insert(0, log);

        // let event = self.barchart.pop().unwrap();
        // self.barchart.insert(0, event);
    }
}
