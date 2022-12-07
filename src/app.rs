use chrono::Local;
use num_enum::TryFromPrimitive;
use std::{
    borrow::{Borrow, Cow},
    ops::AddAssign,
    sync::Arc,
    time::SystemTimeError,
};

use chashmap::{CHashMap, ReadGuard};
use json::JsonValue;
use rand::{
    distributions::{Distribution, Uniform},
    rngs::ThreadRng,
};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::info;
use tui::widgets::ListState;

use crate::client::{
    self,
    my_custom_runtime::{block_on_return, spawn},
    ActionResult, ClientMethod, ContactMessage, ContactUserInfo, InputMessage, MessageChannel,
    ResponseChannel, SystemRequest, RESPONSE_WAITING_LIST,
};

extern crate chrono;
use chrono::prelude::*;

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

#[derive(TryFromPrimitive)]
#[repr(u16)]
pub enum AppBlock {
    GroupList = 0,
    DialogDetail = 1,
}

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

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
    pub mark: u16,
}

impl<T> StatefulList<T> {
    pub fn new(mark: u16) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
            mark,
        }
    }
    pub fn with_items(mark: u16, items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
            mark,
        }
    }

    pub fn next(&mut self) {
        if self.items.len() == 0 {
            return;
        }
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
        if self.items.len() == 0 {
            return;
        }
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

#[derive(Clone)]
pub struct Message {
    pub message: String,
    pub speaker: String,
}

// impl ResponseChannel for App {
//     fn response(&mut self, message: String) {
//         if let Some(r) = parse_json(message) {
//             if r.success {
//                 self.groups.items.extend(r.data.unwrap());
//             }
//         }
//         // self.app.tabs
//     }
// }

// unsafe impl Send for App {}
// unsafe impl Sync for App {}

pub struct App<'a> {
    pub title: String,
    pub should_quit: bool,
    pub input: String,
    /// Current input mode
    pub input_mode: InputMode,
    /// History of recorded messages
    pub messages: Vec<String>,
    // pub show_chart: bool,
    // pub progress: f64,
    // pub sparkline: Signal<RandomSignal>,
    pub tasks: StatefulList<Message>,
    pub groups: StatefulList<ContactUserInfo<'a>>,
    pub message_callback: MessageChannel,
    pub focus: u16,
    pub target_id: Option<Cow<'a, str>>,
    pub target_display_name: Option<Box<String>>,
    pub message_shard: CHashMap<String, Vec<Message>>,
    pub message_unread: CHashMap<String, u16>,
    pub message_latest_time: CHashMap<String, i64>,
}

// pub struct TestResponseChannel {
//     app: &'static StatefulList<ContactUserInfo>,
// }

// impl<'a> ResponseChannel for TestResponseChannel {
//     fn response(&mut self, message: String) {
//         if let Some(r) = parse_json(message) {
//             if r.success {
//                 self.app.items.extend(r.data.unwrap());
//             }
//         }
//         // self.app.tabs
//     }
// }

impl<'a> App<'a> {
    fn message_unread_count_up(&mut self, contact: &ContactMessage) {
        if !self.message_unread.contains_key(contact.unique_id.as_ref()) {
            self.message_unread
                .insert_new(contact.unique_id.as_ref().to_owned(), 0u16);
        }
        if let Some(mut guard) = self.message_unread.get_mut(contact.unique_id.as_ref()) {
            guard.add_assign(1);
        }
        if let Some(idx) = &self.target_id {
            self.message_unread.insert(idx.as_ref().to_owned(), 0u16);
        }
    }

    fn message_latest_update(&mut self, contact: &ContactMessage) {
        let dt = Local::now();
        let timestamp = dt.timestamp_millis();
        self.message_latest_time
            .insert(contact.unique_id.as_ref().to_owned(), timestamp);
    }

    fn message_shard(&mut self, contact: &ContactMessage) {
        if !self.message_shard.contains_key(contact.unique_id.as_ref()) {
            self.message_shard.insert_new(
                contact.unique_id.as_ref().to_string(),
                Vec::<Message>::new(),
            );
        }
        if let Some(mut guard) = self.message_shard.get_mut(contact.unique_id.as_ref()) {
            guard.push(Message {
                message: contact.text.to_string(),
                speaker: contact.display_name.to_string(),
            });
        }
    }

    fn receive_push_notification(&mut self) {
        let receiver: &mut Receiver<String> = &mut self.message_callback.push_notification_receiver;
        if let Ok(message) = receiver.try_recv() {
            if let Ok(jvalue) = json::parse(message.trim()) {
                if let Ok(contact) = ContactMessage::try_from(jvalue) {
                    info!("parse message success: {}", message.clone());
                    self.message_unread_count_up(&contact);
                    self.message_shard(&contact);
                    self.message_latest_update(&contact);
                    self.groups.items.sort_by(|a, b| {
                        let left =
                            if let Some(num) = self.message_latest_time.get(a.unique_id.as_ref()) {
                                num.to_owned()
                            } else {
                                0i64
                            };
                        let right =
                            if let Some(num) = self.message_latest_time.get(b.unique_id.as_ref()) {
                                num.to_owned()
                            } else {
                                0i64
                            };
                        right.cmp(&left)
                    });
                    if let Some(unique) = &self.target_id {
                        if unique.eq(contact.unique_id.as_ref()) {
                            self.tasks.items.push(Message {
                                message: contact.text.to_string(),
                                speaker: contact.display_name.to_string(),
                            });
                        }
                    }
                }
            } else {
                info!("parse message error: {}", message.clone());
            }
        }
    }

    pub fn refresh_contact_list(&self) {
        client::list_user_and_group(
            &self.message_callback,
            // Box::new(TestResponseChannel {
            //     app: &mut self.groups,
            // }),
        )
    }

    fn dispatch_event(&mut self) {
        let old_map = RESPONSE_WAITING_LIST.load().clear();
        if old_map.len() == 0 {
            return;
        }
        old_map.into_iter().for_each(|(_, value)| {
            if let Some(result) = client::parse_json(value.clone()) {
                if result.success {
                    if let Ok(method_enum) = TryInto::<ClientMethod>::try_into(result.method) {
                        match method_enum {
                            ClientMethod::listUserAndGroup => {
                                self.groups.items.truncate(0);
                                self.groups.items.append(&mut result.data.unwrap());
                            }
                            ClientMethod::sendChatMessage => {}
                        }
                    }
                }
            }
        });
    }

    pub fn new(title: &str, enhanced_graphics: bool, call_back: MessageChannel) -> App<'a> {
        App {
            title: String::from(title),
            should_quit: false,
            message_callback: call_back,
            tasks: StatefulList::new(AppBlock::DialogDetail as u16),
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            groups: StatefulList::new(AppBlock::GroupList as u16),
            focus: 0,
            target_id: None,
            message_shard: CHashMap::new(),
            message_unread: CHashMap::new(),
            message_latest_time: CHashMap::new(),
            target_display_name: None,
        }
    }

    pub fn on_up(&mut self) {
        if self.focus == self.tasks.mark {
            self.tasks.previous();
        } else if self.focus == self.groups.mark {
            self.groups.previous();
        }
    }

    pub fn on_down(&mut self) {
        if self.focus == self.tasks.mark {
            self.tasks.next();
        } else if self.focus == self.groups.mark {
            self.groups.next();
        }
    }

    pub fn on_right(&mut self) {
        self.focus = self.focus.saturating_add(1);
        if self.focus >= 2 {
            self.focus = 1;
        }
    }

    pub fn on_left(&mut self) {
        self.focus = self.focus.saturating_sub(1);
    }

    pub fn on_enter(&mut self) {
        match self.input_mode {
            InputMode::Editing => {
                let msg: String = self.input.drain(..).collect();
                if msg.len() > 0 {
                    if let Some(target_id) = self.target_id.to_owned() {
                        self.message_callback.callback(InputMessage {
                            message: msg,
                            group: target_id.to_string(),
                        })
                    }
                }
            }
            InputMode::Normal => {
                if self.groups.mark == self.focus {
                    if let Some(idx) = self.groups.state.selected() {
                        let unique_id = &self.groups.items[idx].unique_id;
                        let display_name = &self.groups.items[idx].display_name;
                        info!("choose target id={}", unique_id);
                        self.target_id = Some(unique_id.to_owned());
                        self.target_display_name = Some(display_name.clone());
                        if let Some(messages) = self.message_shard.get(unique_id.as_ref()) {
                            self.tasks.items = messages.to_vec();
                            self.tasks.state.select(Some(self.tasks.items.len() - 1));
                        } else {
                            self.tasks.items.truncate(0);
                            self.tasks.state.select(None);
                        }
                    } else {
                        return;
                    }
                }
            }
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
        // self.waiting_message();
        self.receive_push_notification();
        self.message_callback.message_dispatch();
        self.dispatch_event();
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
